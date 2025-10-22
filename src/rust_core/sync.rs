//! Main sync engine orchestration
//!
//! This module ties together diff, chunk, manifest, and archive modules
//! to implement the complete sync workflow.

use crate::archive::{Archive, ArchiveError};
use crate::chunk::{Chunk, ChunkError, ChunkOptions, ChunkProcessor};
use crate::crypto::CryptoKey;
use crate::diff::{ChangeType, DiffEngine, DiffOptions, FileChange};
use crate::manifest::{FileEntry, FileType, Manifest, ManifestError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// Sync error types
#[derive(Debug, Error)]
pub enum SyncError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Diff error
    #[error("Diff error: {0}")]
    Diff(String),

    /// Chunk error
    #[error("Chunk error: {0}")]
    Chunk(#[from] ChunkError),

    /// Manifest error
    #[error("Manifest error: {0}")]
    Manifest(#[from] ManifestError),

    /// Archive error
    #[error("Archive error: {0}")]
    Archive(#[from] ArchiveError),

    /// Encryption key not found
    #[error("Encryption key not found")]
    KeyNotFound,

    /// Sync was cancelled
    #[error("Sync was cancelled")]
    Cancelled,

    /// Source path not found
    #[error("Source path not found: {0}")]
    SourceNotFound(PathBuf),

    /// Destination not accessible
    #[error("Destination not accessible: {0}")]
    DestinationNotAccessible(PathBuf),
}

/// Result type for sync operations
pub type Result<T> = std::result::Result<T, SyncError>;

/// Sync options
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Diff options
    pub diff_options: DiffOptions,

    /// Chunk options
    pub chunk_options: ChunkOptions,

    /// Dry run (don't actually sync)
    pub dry_run: bool,

    /// Device ID
    pub device_id: String,

    /// Progress callback interval (files)
    pub progress_interval: usize,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            diff_options: DiffOptions::default(),
            chunk_options: ChunkOptions::default(),
            dry_run: false,
            device_id: String::new(),
            progress_interval: 10,
        }
    }
}

/// Progress information for sync operations
#[derive(Debug, Clone, Default)]
pub struct SyncProgress {
    /// Files processed so far
    pub files_processed: u64,

    /// Total files to process
    pub total_files: u64,

    /// Bytes processed so far
    pub bytes_processed: u64,

    /// Total bytes to process
    pub total_bytes: u64,

    /// Current file being processed
    pub current_file: Option<PathBuf>,

    /// Current phase
    pub phase: SyncPhase,
}

/// Sync phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncPhase {
    /// Initializing
    #[default]
    Initializing,

    /// Scanning files
    Scanning,

    /// Computing differences
    Computing,

    /// Chunking files
    Chunking,

    /// Encrypting data
    Encrypting,

    /// Writing to archive
    Writing,

    /// Finalizing
    Finalizing,

    /// Completed
    Completed,
}

/// Callback for progress updates
pub type ProgressCallback = Arc<dyn Fn(&SyncProgress) + Send + Sync>;

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Snapshot ID created
    pub snapshot_id: String,

    /// Number of files synced
    pub files_synced: usize,

    /// Number of bytes synced (original)
    pub bytes_synced: u64,

    /// Number of bytes written (compressed/encrypted)
    pub bytes_written: u64,

    /// Number of chunks created
    pub chunks_created: usize,

    /// Number of chunks deduplicated
    pub chunks_deduplicated: usize,

    /// Sync duration
    pub duration: std::time::Duration,
}

impl SyncResult {
    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes_synced > 0 {
            self.bytes_written as f64 / self.bytes_synced as f64
        } else {
            1.0
        }
    }

    /// Get deduplication ratio
    pub fn deduplication_ratio(&self) -> f64 {
        let total_chunks = self.chunks_created + self.chunks_deduplicated;
        if total_chunks > 0 {
            self.chunks_created as f64 / total_chunks as f64
        } else {
            1.0
        }
    }
}

/// Main sync engine
pub struct SyncEngine {
    options: SyncOptions,
    progress: Arc<SyncProgressTracker>,
    cancelled: Arc<AtomicBool>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(options: SyncOptions) -> Self {
        Self {
            options,
            progress: Arc::new(SyncProgressTracker::new()),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancel the sync operation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if sync was cancelled
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Sync source directory to destination archive
    pub fn sync(
        &mut self,
        source_path: &Path,
        dest_path: &Path,
        encryption_key: Option<&CryptoKey>,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<SyncResult> {
        let start_time = std::time::Instant::now();

        // Validate paths
        if !source_path.exists() {
            return Err(SyncError::SourceNotFound(source_path.to_path_buf()));
        }

        if !dest_path.exists() {
            return Err(SyncError::DestinationNotAccessible(dest_path.to_path_buf()));
        }

        // Phase 1: Initialize
        self.progress.set_phase(SyncPhase::Initializing);
        if let Some(ref cb) = progress_callback {
            cb(&self.progress.snapshot());
        }

        // Open or create archive
        let mut archive = if dest_path.join(".airgapsync").exists() {
            Archive::open(dest_path.to_path_buf(), self.options.device_id.clone())?
        } else {
            Archive::initialize(dest_path.to_path_buf(), self.options.device_id.clone())?
        };

        // Phase 2: Scan and diff
        self.progress.set_phase(SyncPhase::Scanning);
        if let Some(ref cb) = progress_callback {
            cb(&self.progress.snapshot());
        }

        let diff_engine = DiffEngine::new(self.options.diff_options.clone());

        // Get previous manifest if exists
        let prev_manifest = archive
            .latest_snapshot()
            .ok()
            .flatten()
            .and_then(|id| archive.load_manifest(&id).ok());

        let prev_files = prev_manifest
            .as_ref()
            .map(|m| {
                m.files
                    .iter()
                    .map(|(k, v)| (k.clone(), v.metadata.clone()))
                    .collect()
            })
            .unwrap_or_default();

        // Phase 3: Compute changes
        self.progress.set_phase(SyncPhase::Computing);
        if let Some(ref cb) = progress_callback {
            cb(&self.progress.snapshot());
        }

        let changes = diff_engine.diff(source_path, &prev_files)
            .map_err(|e| SyncError::Diff(e.to_string()))?;

        // Filter to files that need syncing
        let files_to_sync: Vec<_> = changes
            .iter()
            .filter(|c| {
                matches!(c.change_type, ChangeType::Added | ChangeType::Modified)
            })
            .collect();

        let total_files = files_to_sync.len() as u64;
        let total_bytes: u64 = files_to_sync.iter().map(|c| c.size()).sum();

        self.progress.set_totals(total_files, total_bytes);

        if self.options.dry_run {
            // Dry run - just report what would be synced
            return Ok(SyncResult {
                snapshot_id: "dry-run".to_string(),
                files_synced: files_to_sync.len(),
                bytes_synced: total_bytes,
                bytes_written: 0,
                chunks_created: 0,
                chunks_deduplicated: 0,
                duration: start_time.elapsed(),
            });
        }

        // Phase 4: Create new manifest
        let mut manifest = Manifest::new(
            self.options.device_id.clone(),
            source_path.to_path_buf(),
        );

        manifest.metadata.compression_enabled = self.options.chunk_options.compress;
        manifest.metadata.chunk_size = self.options.chunk_options.chunk_size;
        if encryption_key.is_some() {
            manifest.metadata.encryption_algorithm = Some("aes-256-gcm".to_string());
        }

        // Create snapshot in archive
        let snapshot_id = archive.create_snapshot(&manifest)?;

        // Phase 5: Process files
        let chunk_processor = ChunkProcessor::new(self.options.chunk_options.clone())?;
        let mut chunks_created = 0;
        let mut chunks_deduplicated = 0;
        let mut bytes_written = 0u64;

        for file_change in &files_to_sync {
            if self.is_cancelled() {
                return Err(SyncError::Cancelled);
            }

            let file_meta = file_change.source_meta.as_ref().unwrap();

            if file_meta.is_dir {
                // Add directory to manifest
                let entry = FileEntry::directory(file_meta.clone());
                manifest.add_file(file_meta.path.clone(), entry);
                continue;
            }

            self.progress.set_current_file(Some(file_meta.path.clone()));

            let full_path = source_path.join(&file_meta.path);

            // Phase 6: Chunk file
            self.progress.set_phase(SyncPhase::Chunking);
            if let Some(ref cb) = progress_callback {
                if self.progress.files_processed() % self.options.progress_interval as u64 == 0 {
                    cb(&self.progress.snapshot());
                }
            }

            let mut chunks = chunk_processor.chunk_file(&full_path)?;

            // Phase 7: Process chunks (compress/encrypt)
            self.progress.set_phase(SyncPhase::Encrypting);

            chunk_processor.process_chunks(&mut chunks, encryption_key)?;

            // Phase 8: Store chunks
            self.progress.set_phase(SyncPhase::Writing);

            let mut chunk_ids = Vec::new();

            for chunk in &chunks {
                let was_new = archive.store_chunk(chunk)?;

                if was_new {
                    chunks_created += 1;
                    bytes_written += chunk.current_size() as u64;
                } else {
                    chunks_deduplicated += 1;
                }

                chunk_ids.push(chunk.id().to_string());
            }

            // Add file to manifest
            let entry = FileEntry::regular_file(file_meta.clone(), chunk_ids);
            manifest.add_file(file_meta.path.clone(), entry);

            self.progress.increment_files(1);
            self.progress.increment_bytes(file_meta.size);

            if let Some(ref cb) = progress_callback {
                if self.progress.files_processed() % self.options.progress_interval as u64 == 0 {
                    cb(&self.progress.snapshot());
                }
            }
        }

        // Copy unchanged files from previous manifest
        if let Some(prev) = prev_manifest {
            for change in &changes {
                if change.change_type == ChangeType::Unchanged {
                    if let Some(prev_entry) = prev.get_file(&change.path) {
                        manifest.add_file(change.path.clone(), prev_entry.clone());
                    }
                }
            }
        }

        // Phase 9: Finalize
        self.progress.set_phase(SyncPhase::Finalizing);

        manifest.compressed_bytes = bytes_written;

        // Update manifest in archive
        let manifest_path = dest_path
            .join(".airgapsync")
            .join(format!("device-{}", self.options.device_id))
            .join("snapshots")
            .join(&snapshot_id)
            .join("manifest.json");

        manifest.save_json(&manifest_path)?;

        // Phase 10: Complete
        self.progress.set_phase(SyncPhase::Completed);
        if let Some(ref cb) = progress_callback {
            cb(&self.progress.snapshot());
        }

        Ok(SyncResult {
            snapshot_id,
            files_synced: files_to_sync.len(),
            bytes_synced: total_bytes,
            bytes_written,
            chunks_created,
            chunks_deduplicated,
            duration: start_time.elapsed(),
        })
    }

    /// Restore a snapshot to a destination path
    pub fn restore(
        &mut self,
        archive_path: &Path,
        snapshot_id: &str,
        dest_path: &Path,
        encryption_key: Option<&CryptoKey>,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<RestoreResult> {
        let start_time = std::time::Instant::now();

        // Open archive
        let archive = Archive::open(archive_path.to_path_buf(), self.options.device_id.clone())?;

        // Load manifest
        let manifest = archive.load_manifest(snapshot_id)?;

        self.progress.set_totals(manifest.file_count as u64, manifest.total_bytes);
        self.progress.set_phase(SyncPhase::Initializing);

        // Create destination directory
        fs::create_dir_all(dest_path)?;

        let chunk_processor = ChunkProcessor::new(self.options.chunk_options.clone())?;
        let mut files_restored = 0;
        let mut bytes_restored = 0u64;

        for (file_path, entry) in &manifest.files {
            if self.is_cancelled() {
                return Err(SyncError::Cancelled);
            }

            let dest_file_path = dest_path.join(file_path);

            // Create parent directory
            if let Some(parent) = dest_file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            if entry.is_directory() {
                fs::create_dir_all(&dest_file_path)?;
                continue;
            }

            self.progress.set_current_file(Some(file_path.clone()));
            self.progress.set_phase(SyncPhase::Writing);

            // Retrieve chunks
            let mut chunks: Vec<Chunk> = Vec::new();
            for chunk_id in &entry.chunks {
                let chunk = archive.retrieve_chunk(snapshot_id, chunk_id)?;
                chunks.push(chunk);
            }

            // Reassemble file
            chunk_processor.reassemble_chunks(&mut chunks, &dest_file_path, encryption_key)?;

            files_restored += 1;
            bytes_restored += entry.total_size;

            self.progress.increment_files(1);
            self.progress.increment_bytes(entry.total_size);

            if let Some(ref cb) = progress_callback {
                if files_restored % self.options.progress_interval == 0 {
                    cb(&self.progress.snapshot());
                }
            }
        }

        self.progress.set_phase(SyncPhase::Completed);
        if let Some(ref cb) = progress_callback {
            cb(&self.progress.snapshot());
        }

        Ok(RestoreResult {
            files_restored,
            bytes_restored,
            duration: start_time.elapsed(),
        })
    }
}

/// Progress tracker (thread-safe)
struct SyncProgressTracker {
    files_processed: AtomicU64,
    total_files: AtomicU64,
    bytes_processed: AtomicU64,
    total_bytes: AtomicU64,
    current_file: std::sync::Mutex<Option<PathBuf>>,
    phase: std::sync::Mutex<SyncPhase>,
}

impl SyncProgressTracker {
    fn new() -> Self {
        Self {
            files_processed: AtomicU64::new(0),
            total_files: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            current_file: std::sync::Mutex::new(None),
            phase: std::sync::Mutex::new(SyncPhase::default()),
        }
    }

    fn set_totals(&self, files: u64, bytes: u64) {
        self.total_files.store(files, Ordering::SeqCst);
        self.total_bytes.store(bytes, Ordering::SeqCst);
    }

    fn increment_files(&self, count: u64) {
        self.files_processed.fetch_add(count, Ordering::SeqCst);
    }

    fn increment_bytes(&self, count: u64) {
        self.bytes_processed.fetch_add(count, Ordering::SeqCst);
    }

    fn files_processed(&self) -> u64 {
        self.files_processed.load(Ordering::SeqCst)
    }

    fn set_current_file(&self, file: Option<PathBuf>) {
        *self.current_file.lock().unwrap() = file;
    }

    fn set_phase(&self, phase: SyncPhase) {
        *self.phase.lock().unwrap() = phase;
    }

    fn snapshot(&self) -> SyncProgress {
        SyncProgress {
            files_processed: self.files_processed.load(Ordering::SeqCst),
            total_files: self.total_files.load(Ordering::SeqCst),
            bytes_processed: self.bytes_processed.load(Ordering::SeqCst),
            total_bytes: self.total_bytes.load(Ordering::SeqCst),
            current_file: self.current_file.lock().unwrap().clone(),
            phase: *self.phase.lock().unwrap(),
        }
    }
}

/// Result of a restore operation
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// Number of files restored
    pub files_restored: usize,

    /// Number of bytes restored
    pub bytes_restored: u64,

    /// Restore duration
    pub duration: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sync_engine_creation() {
        let options = SyncOptions {
            device_id: "test-device".to_string(),
            ..Default::default()
        };

        let engine = SyncEngine::new(options);
        assert!(!engine.is_cancelled());
    }

    #[test]
    fn test_sync_engine_cancel() {
        let options = SyncOptions {
            device_id: "test-device".to_string(),
            ..Default::default()
        };

        let engine = SyncEngine::new(options);
        engine.cancel();
        assert!(engine.is_cancelled());
    }

    #[test]
    fn test_sync_dry_run() {
        let source_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();

        // Create some test files
        fs::write(source_dir.path().join("test1.txt"), b"content1").unwrap();
        fs::write(source_dir.path().join("test2.txt"), b"content2").unwrap();

        let options = SyncOptions {
            device_id: "test-device".to_string(),
            dry_run: true,
            ..Default::default()
        };

        let mut engine = SyncEngine::new(options);

        let result = engine.sync(
            source_dir.path(),
            dest_dir.path(),
            None,
            None,
        ).unwrap();

        assert_eq!(result.snapshot_id, "dry-run");
        assert_eq!(result.files_synced, 2);
    }
}

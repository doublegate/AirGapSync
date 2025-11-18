//! Sync orchestration
//!
//! This module provides the main synchronization logic, coordinating diff
//! detection, encryption, and storage operations.

use crate::crypto::CryptoKey;
use crate::diff::{ChangeSet, ChangeType, DiffEngine};
use crate::metadata::{FileMetadata, Manifest};
use crate::storage::{StorageBackend, StorageConfig, StoredFile};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Sync error types
#[derive(Debug, Error)]
pub enum SyncError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Diff error
    #[error("Diff error: {0}")]
    Diff(String),

    /// Encryption error
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,
}

/// Result type for sync operations
pub type Result<T> = std::result::Result<T, SyncError>;

/// Sync operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Full sync (all files)
    Full,

    /// Incremental sync (only changes)
    Incremental,

    /// Dry run (no actual changes)
    DryRun,
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Source directory
    pub source_path: PathBuf,

    /// Destination (storage) path
    pub dest_path: PathBuf,

    /// Device ID
    pub device_id: String,

    /// Sync mode
    pub mode: SyncMode,

    /// Enable verification
    pub verify: bool,

    /// Enable deduplication
    pub enable_dedup: bool,

    /// Chunk size
    pub chunk_size: usize,

    /// Maximum concurrent operations
    pub max_concurrency: usize,
}

impl SyncConfig {
    /// Create a new sync configuration
    pub fn new<P: Into<PathBuf>, Q: Into<PathBuf>>(
        source_path: P,
        dest_path: Q,
        device_id: String,
    ) -> Self {
        Self {
            source_path: source_path.into(),
            dest_path: dest_path.into(),
            device_id,
            mode: SyncMode::Incremental,
            verify: true,
            enable_dedup: true,
            chunk_size: 1024 * 1024,
            max_concurrency: 4,
        }
    }

    /// Set sync mode
    pub fn with_mode(mut self, mode: SyncMode) -> Self {
        self.mode = mode;
        self
    }

    /// Enable/disable verification
    pub fn with_verification(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }
}

/// Progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    /// Total files to process
    pub total_files: usize,

    /// Files processed so far
    pub processed_files: usize,

    /// Total bytes to transfer
    pub total_bytes: u64,

    /// Bytes transferred so far
    pub transferred_bytes: u64,

    /// Current operation
    pub current_operation: String,

    /// Current file being processed
    pub current_file: Option<PathBuf>,

    /// Start time (Unix timestamp)
    pub start_time: u64,

    /// Elapsed seconds
    pub elapsed_seconds: u64,

    /// Estimated seconds remaining
    pub estimated_remaining: u64,
}

impl SyncProgress {
    /// Create new progress tracker
    pub fn new(total_files: usize, total_bytes: u64) -> Self {
        Self {
            total_files,
            processed_files: 0,
            total_bytes,
            transferred_bytes: 0,
            current_operation: "Initializing".to_string(),
            current_file: None,
            start_time: Utc::now().timestamp() as u64,
            elapsed_seconds: 0,
            estimated_remaining: 0,
        }
    }

    /// Update progress
    pub fn update(&mut self, bytes_transferred: u64) {
        self.transferred_bytes += bytes_transferred;
        self.processed_files += 1;

        let now = Utc::now().timestamp() as u64;
        self.elapsed_seconds = now.saturating_sub(self.start_time);

        // Estimate remaining time
        if self.transferred_bytes > 0 {
            let rate = self.transferred_bytes as f64 / self.elapsed_seconds.max(1) as f64;
            let remaining_bytes = self.total_bytes.saturating_sub(self.transferred_bytes);
            self.estimated_remaining = (remaining_bytes as f64 / rate) as u64;
        }
    }

    /// Get progress percentage
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }
}

/// Sync result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Snapshot ID
    pub snapshot_id: String,

    /// Success status
    pub success: bool,

    /// Number of files synced
    pub files_synced: usize,

    /// Total bytes transferred
    pub bytes_transferred: u64,

    /// Duration in seconds
    pub duration_seconds: u64,

    /// Changeset applied
    pub changeset: ChangeSet,

    /// Any errors encountered
    pub errors: Vec<String>,
}

/// Progress callback type
pub type ProgressCallback = Arc<dyn Fn(&SyncProgress) + Send + Sync>;

/// Sync engine
pub struct SyncEngine {
    config: SyncConfig,
    storage: StorageBackend,
    diff_engine: DiffEngine,
    progress_callback: Option<ProgressCallback>,
    cancelled: Arc<Mutex<bool>>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(config: SyncConfig, _crypto_key: CryptoKey) -> Result<Self> {
        let storage_config = StorageConfig {
            base_path: config.dest_path.clone(),
            chunk_size: config.chunk_size,
            enable_dedup: config.enable_dedup,
            enable_compression: true,
        };

        let storage = StorageBackend::new(storage_config)
            .map_err(|e| SyncError::Storage(e.to_string()))?;

        let diff_engine = DiffEngine::new();

        Ok(Self {
            config,
            storage,
            diff_engine,
            progress_callback: None,
            cancelled: Arc::new(Mutex::new(false)),
        })
    }

    /// Set progress callback
    pub fn set_progress_callback<F>(&mut self, callback: F)
    where
        F: Fn(&SyncProgress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
    }

    /// Perform synchronization
    pub fn sync(&mut self, crypto_key: &CryptoKey) -> Result<SyncResult> {
        let start_time = Utc::now().timestamp() as u64;

        // Generate snapshot ID
        let snapshot_id = format!("{}-{}", self.config.device_id, Utc::now().format("%Y%m%d-%H%M%S"));

        // Scan source directory
        self.update_progress("Scanning source directory", None);
        let current_manifest = self.diff_engine
            .scan_directory(&self.config.source_path, self.config.device_id.clone())
            .map_err(|e| SyncError::Diff(e.to_string()))?;

        // Load previous manifest if exists
        let previous_manifest = if self.config.mode == SyncMode::Incremental {
            match self.load_latest_manifest() {
                Ok(m) => Some(m),
                Err(_) => None,
            }
        } else {
            None
        };

        // Compute changes
        self.update_progress("Computing changes", None);
        let changeset = if let Some(ref prev) = previous_manifest {
            self.diff_engine
                .diff_manifests(prev, &current_manifest)
                .map_err(|e| SyncError::Diff(e.to_string()))?
        } else {
            // First sync or full sync - treat all files as new
            let mut cs = ChangeSet::new();
            for file in &current_manifest.files {
                cs.add_change(crate::diff::Change::added(file.clone()));
            }
            cs
        };

        // Create progress tracker
        let mut progress = SyncProgress::new(
            changeset.total_changes(),
            changeset.stats.total_bytes,
        );

        // Process changes
        let mut errors = Vec::new();
        let mut files_synced = 0;
        let mut bytes_transferred = 0u64;

        for change in &changeset.changes {
            // Check for cancellation
            if *self.cancelled.lock().unwrap() {
                return Err(SyncError::Cancelled);
            }

            progress.current_file = Some(change.path.clone());

            match change.change_type {
                ChangeType::Added | ChangeType::Modified => {
                    let file_path = self.config.source_path.join(&change.path);

                    if self.config.mode != SyncMode::DryRun {
                        match self.sync_file(&file_path, change.new_metadata.as_ref().unwrap(), crypto_key) {
                            Ok(_) => {
                                files_synced += 1;
                                bytes_transferred += change.size;
                                progress.update(change.size);
                            }
                            Err(e) => {
                                errors.push(format!("{}: {}", change.path.display(), e));
                            }
                        }
                    } else {
                        files_synced += 1;
                        progress.update(change.size);
                    }
                }
                ChangeType::Deleted => {
                    // For now, we don't delete from backup
                    // Could implement with retention policy
                }
                ChangeType::Moved => {
                    // Update manifest metadata
                    files_synced += 1;
                }
                ChangeType::MetadataChanged => {
                    // Update metadata only
                    files_synced += 1;
                }
            }

            self.notify_progress(&progress);
        }

        // Save manifest
        if self.config.mode != SyncMode::DryRun {
            self.storage
                .save_manifest(&current_manifest, &snapshot_id)
                .map_err(|e| SyncError::Storage(e.to_string()))?;
        }

        let end_time = Utc::now().timestamp() as u64;

        Ok(SyncResult {
            snapshot_id,
            success: errors.is_empty(),
            files_synced,
            bytes_transferred,
            duration_seconds: end_time - start_time,
            changeset,
            errors,
        })
    }

    /// Sync a single file
    fn sync_file(
        &mut self,
        file_path: &Path,
        metadata: &FileMetadata,
        crypto_key: &CryptoKey,
    ) -> Result<StoredFile> {
        self.storage
            .store_file(file_path, metadata.clone(), crypto_key)
            .map_err(|e| SyncError::Storage(e.to_string()))
    }

    /// Load the latest manifest
    fn load_latest_manifest(&self) -> Result<Manifest> {
        let snapshots = self.storage
            .list_snapshots()
            .map_err(|e| SyncError::Storage(e.to_string()))?;

        if let Some(latest) = snapshots.last() {
            self.storage
                .load_manifest(latest)
                .map_err(|e| SyncError::Storage(e.to_string()))
        } else {
            Err(SyncError::Storage("No previous manifest found".to_string()))
        }
    }

    /// Update progress state
    fn update_progress(&self, operation: &str, file: Option<PathBuf>) {
        if let Some(ref callback) = self.progress_callback {
            let progress = SyncProgress {
                total_files: 0,
                processed_files: 0,
                total_bytes: 0,
                transferred_bytes: 0,
                current_operation: operation.to_string(),
                current_file: file,
                start_time: Utc::now().timestamp() as u64,
                elapsed_seconds: 0,
                estimated_remaining: 0,
            };
            callback(&progress);
        }
    }

    /// Notify progress callback
    fn notify_progress(&self, progress: &SyncProgress) {
        if let Some(ref callback) = self.progress_callback {
            callback(progress);
        }
    }

    /// Cancel sync operation
    pub fn cancel(&self) {
        *self.cancelled.lock().unwrap() = true;
    }

    /// List available snapshots
    pub fn list_snapshots(&self) -> Result<Vec<String>> {
        self.storage
            .list_snapshots()
            .map_err(|e| SyncError::Storage(e.to_string()))
    }

    /// Verify a snapshot
    pub fn verify_snapshot(&self, snapshot_id: &str) -> Result<bool> {
        // Load manifest
        let _manifest = self.storage
            .load_manifest(snapshot_id)
            .map_err(|e| SyncError::Storage(e.to_string()))?;

        // TODO: Verify all chunks exist and have correct hashes
        Ok(true)
    }

    /// Restore from snapshot
    pub fn restore(
        &self,
        snapshot_id: &str,
        restore_path: &Path,
        _crypto_key: &CryptoKey,
    ) -> Result<()> {
        // Load manifest
        let manifest = self.storage
            .load_manifest(snapshot_id)
            .map_err(|e| SyncError::Storage(e.to_string()))?;

        // Create restore directory
        std::fs::create_dir_all(restore_path)?;

        // TODO: Implement restore logic
        // For each file in manifest:
        //   - Create directory structure
        //   - Retrieve and decrypt chunks
        //   - Restore file
        //   - Set permissions and timestamps

        log::info!("Restore from {} to {} - {} files",
            snapshot_id,
            restore_path.display(),
            manifest.files.len()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sync_config() {
        let config = SyncConfig::new("/source", "/dest", "device1".to_string());
        assert_eq!(config.device_id, "device1");
        assert_eq!(config.mode, SyncMode::Incremental);
    }

    #[test]
    fn test_sync_progress() {
        let mut progress = SyncProgress::new(10, 1000);
        assert_eq!(progress.total_files, 10);
        assert_eq!(progress.percentage(), 0.0);

        progress.update(500);
        assert_eq!(progress.transferred_bytes, 500);
        assert_eq!(progress.percentage(), 50.0);
    }

    #[test]
    fn test_sync_mode() {
        let config = SyncConfig::new("/src", "/dst", "dev1".to_string())
            .with_mode(SyncMode::DryRun);
        assert_eq!(config.mode, SyncMode::DryRun);
    }
}

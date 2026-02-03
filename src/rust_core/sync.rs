//! Sync engine for orchestrating file synchronization operations
//!
//! This module provides the main synchronization functionality, coordinating
//! between diff detection, chunking, encryption, and transfer to removable media.

use crate::config::{Config, DeviceConfig, NotificationConfig, AdvancedConfig,
                    GeneralConfig, SourceConfig, PolicyConfig, SecurityConfig};
use crate::crypto::{Algorithm, CryptoKey};
use crate::diff::{DiffEngine, FileChange};
use crate::chunk::{ChunkProcessor};
use crate::snapshot::{Snapshot, SnapshotManager};
use crate::keychain::KeychainManager;
use crate::audit::{AuditLogger, AuditEvent};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self};
use std::sync::{Arc, Mutex};
use std::time::{Instant};
use thiserror::Error;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use rayon::prelude::*;

/// Errors that can occur during sync operations
#[derive(Error, Debug)]
pub enum SyncError {
    /// IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    
    /// Source path not found
    #[error("Source path not found: {0}")]
    SourceNotFound(String),
    
    /// Device not mounted
    #[error("Device not mounted at path: {0}")]
    DeviceNotMounted(String),
    
    /// Encryption error
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    /// Diff error
    #[error("Diff error: {0}")]
    Diff(#[from] crate::diff::DiffError),
    
    /// Chunk error
    #[error("Chunk error: {0}")]
    Chunk(#[from] crate::chunk::ChunkError),
    
    /// Snapshot error
    #[error("Snapshot error: {0}")]
    Snapshot(#[from] crate::snapshot::SnapshotError),
    
    /// Keychain error
    #[error("Keychain error: {0}")]
    Keychain(#[from] crate::keychain::KeychainError),
    
    /// Sync interrupted
    #[error("Sync operation was interrupted")]
    Interrupted,
    
    /// Verification failed
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    
    /// Walk directory error
    #[error("Walk directory error: {0}")]
    WalkError(#[from] walkdir::Error),
    
    /// Audit error
    #[error("Audit error: {0}")]
    Audit(String),
}

/// Sync operation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOptions {
    /// Run in dry-run mode (preview changes without syncing)
    pub dry_run: bool,
    
    /// Enable verbose output
    pub verbose: bool,
    
    /// Number of parallel workers
    pub parallel_workers: usize,
    
    /// Chunk size in bytes
    pub chunk_size: usize,
    
    /// Verify after write
    pub verify_after_write: bool,
    
    /// Resume from previous sync
    pub resume: bool,
    
    /// Compression level (0-9, 0 = no compression)
    pub compression_level: u32,
    
    /// Show progress bars
    pub show_progress: bool,
    
    /// Maximum retries for failed operations
    pub max_retries: u32,
    
    /// Exclude patterns
    pub exclude_patterns: Vec<String>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            verbose: false,
            parallel_workers: 4,
            chunk_size: 1024 * 1024, // 1MB chunks
            verify_after_write: true,
            resume: true,
            compression_level: 6,
            show_progress: true,
            max_retries: 3,
            exclude_patterns: vec![
                ".DS_Store".to_string(),
                "*.tmp".to_string(),
                "Thumbs.db".to_string(),
                ".git/*".to_string(),
            ],
        }
    }
}

/// Progress callback for sync operations
pub type ProgressCallback = Arc<dyn Fn(SyncProgress) + Send + Sync>;

/// Sync progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    /// Current operation
    pub operation: String,
    
    /// Current file being processed
    pub current_file: Option<PathBuf>,
    
    /// Total files to process
    pub total_files: u64,
    
    /// Files processed so far
    pub processed_files: u64,
    
    /// Total bytes to process
    pub total_bytes: u64,
    
    /// Bytes processed so far
    pub processed_bytes: u64,
    
    /// Current transfer rate (bytes/sec)
    pub transfer_rate: u64,
    
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<u64>,
    
    /// Start time
    pub start_time: DateTime<Utc>,
    
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Sync result statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Total files synced
    pub files_synced: u64,
    
    /// Total bytes transferred
    pub bytes_transferred: u64,
    
    /// Files skipped
    pub files_skipped: u64,
    
    /// Files failed
    pub files_failed: u64,
    
    /// Total duration (seconds)
    pub duration_seconds: u64,
    
    /// Average transfer rate (bytes/sec)
    pub average_transfer_rate: u64,
    
    /// Snapshot ID created
    pub snapshot_id: Option<String>,
    
    /// Errors encountered
    pub errors: Vec<String>,
    
    /// Warnings
    pub warnings: Vec<String>,
}

/// Main sync engine
pub struct SyncEngine {
    config: Config,
    keychain: KeychainManager,
    diff_engine: DiffEngine,
    chunk_processor: ChunkProcessor,
    snapshot_manager: SnapshotManager,
    audit_logger: AuditLogger,
    progress_callback: Option<ProgressCallback>,
    multi_progress: MultiProgress,
    should_stop: Arc<Mutex<bool>>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(config: Config) -> Result<Self, SyncError> {
        let keychain = KeychainManager::new();
        let diff_engine = DiffEngine::new();
        let chunk_processor = ChunkProcessor::new();
        
        // Use home directory for snapshot storage
        let data_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::Config("Could not determine home directory".to_string()))?
            .join(".airgapsync")
            .join("snapshots");
        
        let snapshot_manager = SnapshotManager::new(&data_dir)?;
        
        // Create audit logger
        let audit_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::Config("Could not determine home directory".to_string()))?
            .join(".airgapsync")
            .join("audit");
        let audit_logger = AuditLogger::new(&audit_dir)
            .map_err(|e| SyncError::Audit(format!("Failed to create audit logger: {}", e)))?;
        
        Ok(Self {
            config,
            keychain,
            diff_engine,
            chunk_processor,
            snapshot_manager,
            audit_logger,
            multi_progress: MultiProgress::new(),
            progress_callback: None,
            should_stop: Arc::new(Mutex::new(false)),
        })
    }
    
    /// Set progress callback
    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        self.progress_callback = Some(callback);
    }
    
    /// Stop the sync operation
    pub fn stop(&self) {
        *self.should_stop.lock().unwrap() = true;
    }
    
    /// Check if sync should stop
    fn should_stop(&self) -> bool {
        *self.should_stop.lock().unwrap()
    }
    
    /// Sync to a specific device
    pub fn sync_to_device(
        &mut self,
        device_id: &str,
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncError> {
        let start_time = Instant::now();
        let start_datetime = Utc::now();
        
        // Find device in config
        let device = self.config.device.iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| SyncError::DeviceNotFound(device_id.to_string()))?
            .clone();
        
        // Verify device is mounted
        if !Path::new(&device.mount_point).exists() {
            return Err(SyncError::DeviceNotMounted(device.mount_point.display().to_string()));
        }
        
        // Log sync start
        let _ = self.audit_logger.log_event(AuditEvent::SyncStarted {
            device_id: device_id.to_string(),
            source_path: self.config.source.path.clone().into(),
            dry_run: options.dry_run,
        });
        
        // Initialize progress tracking
        let mut progress = SyncProgress {
            operation: "Initializing".to_string(),
            current_file: None,
            total_files: 0,
            processed_files: 0,
            total_bytes: 0,
            processed_bytes: 0,
            transfer_rate: 0,
            eta_seconds: None,
            start_time: start_datetime,
            errors: Vec::new(),
        };
        
        self.report_progress(&progress);
        
        // Get encryption key for device
        let encryption_key = self.get_or_create_device_key(&device)?;
        
        // Get latest snapshot for device (if resuming)
        let previous_snapshot = if options.resume {
            self.snapshot_manager.get_latest_snapshot(&device.id)?
        } else {
            None
        };
        
        // Scan source directory and compute diff
        progress.operation = "Scanning source directory".to_string();
        self.report_progress(&progress);
        
        let source_path = Path::new(&self.config.source.path);
        let file_changes = self.scan_and_diff(
            source_path,
            previous_snapshot.as_ref(),
            &options.exclude_patterns,
            &mut progress,
        )?;
        
        if self.should_stop() {
            return Err(SyncError::Interrupted);
        }
        
        // Calculate total work
        progress.total_files = file_changes.len() as u64;
        progress.total_bytes = file_changes.iter()
            .map(|fc| fc.size())
            .sum();
        
        if options.dry_run {
            // Dry run - just report what would be done
            return Ok(SyncResult {
                files_synced: progress.total_files,
                bytes_transferred: progress.total_bytes,
                files_skipped: 0,
                files_failed: 0,
                duration_seconds: start_time.elapsed().as_secs(),
                average_transfer_rate: 0,
                snapshot_id: None,
                errors: Vec::new(),
                warnings: Vec::new(),
            });
        }
        
        // Process files
        progress.operation = "Syncing files".to_string();
        self.report_progress(&progress);
        
        let sync_result = self.process_files(
            &file_changes,
            &device,
            &encryption_key,
            options,
            &mut progress,
        )?;
        
        if self.should_stop() {
            return Err(SyncError::Interrupted);
        }
        
        // Create snapshot
        progress.operation = "Creating snapshot".to_string();
        self.report_progress(&progress);
        
        let snapshot = self.create_snapshot(
            &device.id,
            &file_changes,
            sync_result.files_synced,
            sync_result.bytes_transferred,
        )?;
        
        // Clean up old snapshots based on retention policy
        if self.config.policy.retain_snapshots > 0 {
            let retain_count = self.config.policy.retain_snapshots;
            self.snapshot_manager.cleanup_old_snapshots(&device.id, retain_count as usize)?;
        }
        
        let duration = start_time.elapsed().as_secs();
        let average_rate = if duration > 0 {
            sync_result.bytes_transferred / duration
        } else {
            0
        };
        
        // Log sync completion
        let _ = self.audit_logger.log_event(AuditEvent::SyncCompleted {
            device_id: device.id.clone(),
            files_added: sync_result.files_synced as usize,
            files_modified: 0, // TODO: Track modified files separately
            files_deleted: 0,  // TODO: Track deleted files separately
            bytes_transferred: sync_result.bytes_transferred,
            duration_secs: duration as f64,
        });
        
        Ok(SyncResult {
            snapshot_id: Some(snapshot.id),
            duration_seconds: duration,
            average_transfer_rate: average_rate,
            ..sync_result
        })
    }
    
    /// Get or create encryption key for device
    fn get_or_create_device_key(&mut self, device: &DeviceConfig) -> Result<CryptoKey, SyncError> {
        // Try to get existing key
        match self.keychain.get_key(&device.id) {
            Ok(encryption_key) => {
                // Extract algorithm from metadata
                let algorithm = match &device.encryption.algorithm {
                    crate::config::EncryptionAlgorithm::Aes256Gcm => Algorithm::Aes256Gcm,
                    crate::config::EncryptionAlgorithm::ChaCha20Poly1305 => Algorithm::ChaCha20Poly1305,
                };
                
                // Create key from stored data
                let key = CryptoKey::new(encryption_key.key_material.clone(), algorithm)
                    .map_err(|e| SyncError::Encryption(e.to_string()))?;
                Ok(key)
            }
            Err(_) => {
                // Create new key
                // Use device encryption config or default
                let algorithm = match &device.encryption.algorithm {
                    crate::config::EncryptionAlgorithm::Aes256Gcm => Algorithm::Aes256Gcm,
                    crate::config::EncryptionAlgorithm::ChaCha20Poly1305 => Algorithm::ChaCha20Poly1305,
                };
                
                let key = CryptoKey::generate(algorithm)
                    .map_err(|e| SyncError::Encryption(e.to_string()))?;
                
                // Store in keychain
                let key_metadata = crate::keychain::KeyMetadata {
                    algorithm: match algorithm {
                        Algorithm::Aes256Gcm => "AES-256".to_string(),
                        Algorithm::ChaCha20Poly1305 => "ChaCha20".to_string(),
                    },
                    created_at: chrono::Utc::now(),
                    version: 1,
                    rotated_at: None,
                    device_id: device.id.clone(),
                };
                
                let encryption_key = crate::keychain::EncryptionKey {
                    key_material: key.as_bytes().to_vec(),
                    metadata: key_metadata,
                };
                
                self.keychain.store_key(&device.id, &encryption_key)?;
                
                // Log key generation
                let _ = self.audit_logger.log_event(AuditEvent::KeyGenerated {
                    device_id: device.id.clone(),
                    algorithm: match algorithm {
                        Algorithm::Aes256Gcm => "AES-256-GCM".to_string(),
                        Algorithm::ChaCha20Poly1305 => "ChaCha20-Poly1305".to_string(),
                    },
                });
                
                Ok(key)
            }
        }
    }
    
    /// Scan source directory and compute diff
    fn scan_and_diff(
        &self,
        source_path: &Path,
        previous_snapshot: Option<&Snapshot>,
        exclude_patterns: &[String],
        progress: &mut SyncProgress,
    ) -> Result<Vec<FileChange>, SyncError> {
        let mut file_changes = Vec::new();
        let walker = WalkDir::new(source_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_exclude(e.path(), exclude_patterns));
        
        let pb = if self.config.general.verbose {
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap());
            pb.set_message("Scanning files...");
            self.multi_progress.add(pb)
        } else {
            ProgressBar::hidden()
        };
        
        for entry in walker {
            if self.should_stop() {
                break;
            }
            
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                pb.set_message(format!("Scanning: {}", path.display()));
                
                // Compute file change
                match self.diff_engine.compute_file_change(
                    path,
                    source_path,
                    previous_snapshot,
                ) {
                    Ok(Some(change)) => file_changes.push(change),
                    Ok(None) => {}, // File unchanged
                    Err(e) => {
                        progress.errors.push(format!("Error scanning {}: {}", path.display(), e));
                    }
                }
            }
        }
        
        pb.finish_with_message(format!("Scanned {} files", file_changes.len()));
        
        Ok(file_changes)
    }
    
    /// Check if path should be excluded
    fn should_exclude(&self, path: &Path, patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();
        patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
        })
    }
    
    /// Process files for sync
    fn process_files(
        &self,
        file_changes: &[FileChange],
        device: &DeviceConfig,
        encryption_key: &CryptoKey,
        options: &SyncOptions,
        progress: &mut SyncProgress,
    ) -> Result<SyncResult, SyncError> {
        let mut result = SyncResult {
            files_synced: 0,
            bytes_transferred: 0,
            files_skipped: 0,
            files_failed: 0,
            duration_seconds: 0,
            average_transfer_rate: 0,
            snapshot_id: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        
        let dest_base = Path::new(&device.mount_point).join(".airgapsync");
        fs::create_dir_all(&dest_base)?;
        
        // Create thread pool for parallel processing
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(options.parallel_workers)
            .build()
            .map_err(|e| SyncError::Config(format!("Failed to create thread pool: {}", e)))?;
        
        let pb = if options.show_progress {
            let pb = ProgressBar::new(progress.total_bytes);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            self.multi_progress.add(pb)
        } else {
            ProgressBar::hidden()
        };
        
        // Process files in parallel
        let chunk_size = options.chunk_size;
        let compression_level = options.compression_level;
        let verify = options.verify_after_write;
        let max_retries = options.max_retries;
        
        let results: Vec<_> = pool.install(|| {
            file_changes.par_iter().map(|change| {
                if self.should_stop() {
                    return Err(SyncError::Interrupted);
                }
                
                pb.set_message(format!("Processing: {}", change.path().display()));
                
                let mut retry_count = 0;
                loop {
                    match self.process_single_file(
                        change,
                        &dest_base,
                        encryption_key,
                        chunk_size,
                        compression_level,
                        verify,
                    ) {
                        Ok(bytes) => {
                            pb.inc(bytes);
                            return Ok((1u64, bytes));
                        }
                        Err(e) => {
                            retry_count += 1;
                            if retry_count > max_retries {
                                return Err(e);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100 * retry_count as u64));
                        }
                    }
                }
            }).collect()
        });
        
        // Aggregate results
        for res in results {
            match res {
                Ok((files, bytes)) => {
                    result.files_synced += files;
                    result.bytes_transferred += bytes;
                    progress.processed_files += files;
                    progress.processed_bytes += bytes;
                }
                Err(e) => {
                    result.files_failed += 1;
                    result.errors.push(e.to_string());
                }
            }
        }
        
        pb.finish_with_message("Sync complete");
        
        Ok(result)
    }
    
    /// Process a single file
    fn process_single_file(
        &self,
        change: &FileChange,
        dest_base: &Path,
        encryption_key: &CryptoKey,
        chunk_size: usize,
        compression_level: u32,
        verify: bool,
    ) -> Result<u64, SyncError> {
        match change {
            FileChange::Added { path, .. } |
            FileChange::Modified { path, .. } => {
                // Process file in chunks
                let chunks = self.chunk_processor.process_file(
                    path,
                    chunk_size,
                    compression_level,
                    encryption_key,
                )?;
                
                // Write chunks to destination
                let mut total_bytes = 0u64;
                for (idx, chunk) in chunks.iter().enumerate() {
                    let chunk_path = self.get_chunk_path(dest_base, &chunk.hash);
                    
                    // Skip if chunk already exists (deduplication)
                    if !chunk_path.exists() {
                        fs::write(&chunk_path, &chunk.encrypted_data)?;
                        total_bytes += chunk.encrypted_data.len() as u64;
                        
                        if verify {
                            // Verify written data
                            let read_data = fs::read(&chunk_path)?;
                            if read_data != chunk.encrypted_data {
                                return Err(SyncError::VerificationFailed(
                                    format!("Chunk {} verification failed", idx)
                                ));
                            }
                        }
                    }
                }
                
                Ok(total_bytes)
            }
            FileChange::Deleted { .. } => {
                // Deletions are tracked in snapshot but no data transfer
                Ok(0)
            }
        }
    }
    
    /// Get chunk storage path
    fn get_chunk_path(&self, base: &Path, hash: &str) -> PathBuf {
        // Use first 2 chars of hash for directory sharding
        let dir = base.join("chunks").join(&hash[..2]);
        fs::create_dir_all(&dir).ok();
        dir.join(hash)
    }
    
    /// Create snapshot after sync
    fn create_snapshot(
        &mut self,
        device_id: &str,
        file_changes: &[FileChange],
        files_synced: u64,
        bytes_transferred: u64,
    ) -> Result<Snapshot, SyncError> {
        let snapshot = Snapshot::new(
            device_id.to_string(),
            file_changes.to_vec(),
            files_synced,
            bytes_transferred,
        );
        
        self.snapshot_manager.save_snapshot(&snapshot)?;
        Ok(snapshot)
    }
    
    /// Report progress
    fn report_progress(&self, progress: &SyncProgress) {
        if let Some(callback) = &self.progress_callback {
            callback(progress.clone());
        }
    }
}

/// Sync builder for convenient API
pub struct SyncBuilder {
    config: Option<Config>,
    options: SyncOptions,
    progress_callback: Option<ProgressCallback>,
}

impl SyncBuilder {
    /// Create new sync builder
    pub fn new() -> Self {
        Self {
            config: None,
            options: SyncOptions::default(),
            progress_callback: None,
        }
    }
    
    /// Set configuration
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }
    
    /// Set sync options
    pub fn with_options(mut self, options: SyncOptions) -> Self {
        self.options = options;
        self
    }
    
    /// Set progress callback
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }
    
    /// Build and run sync
    pub fn sync_to_device(self, device_id: &str) -> Result<SyncResult, SyncError> {
        let config = self.config.ok_or_else(|| {
            SyncError::Config("Configuration not provided".to_string())
        })?;
        
        let mut engine = SyncEngine::new(config)?;
        
        if let Some(callback) = self.progress_callback {
            engine.set_progress_callback(callback);
        }
        
        engine.sync_to_device(device_id, &self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_sync_options_default() {
        let options = SyncOptions::default();
        assert!(!options.dry_run);
        assert_eq!(options.parallel_workers, 4);
        assert_eq!(options.chunk_size, 1024 * 1024);
        assert!(options.verify_after_write);
    }
    
    #[test]
    fn test_sync_builder() {
        let builder = SyncBuilder::new()
            .with_options(SyncOptions {
                dry_run: true,
                ..Default::default()
            });
        
        assert!(builder.options.dry_run);
    }
    
    #[test]
    fn test_should_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            general: GeneralConfig::default(),
            source: SourceConfig {
                path: temp_dir.path().to_path_buf(),
                exclude: vec![],
                follow_symlinks: false,
                include_hidden: false,
            },
            device: vec![],
            policy: PolicyConfig::default(),
            security: SecurityConfig::default(),
            schedule: None,
            notifications: NotificationConfig::default(),
            advanced: AdvancedConfig::default(),
        };
        let engine = SyncEngine::new(config).unwrap();
        
        let patterns = vec![
            "*.tmp".to_string(),
            ".git/*".to_string(),
            ".DS_Store".to_string(),
        ];
        
        assert!(engine.should_exclude(Path::new("test.tmp"), &patterns));
        assert!(engine.should_exclude(Path::new(".git/config"), &patterns));
        assert!(engine.should_exclude(Path::new(".DS_Store"), &patterns));
        assert!(!engine.should_exclude(Path::new("test.txt"), &patterns));
    }
}
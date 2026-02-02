//! Snapshot management for backup state tracking
//!
//! This module provides functionality to create, store, and manage snapshots
//! of sync operations, enabling incremental backups and recovery.

use crate::diff::{FileChange, FileMetadata};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::collections::HashMap;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sha2::Digest;
use serde_json;
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;

/// Errors that can occur during snapshot operations
#[derive(Error, Debug)]
pub enum SnapshotError {
    /// IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Invalid snapshot format
    #[error("Invalid snapshot format: {0}")]
    InvalidFormat(String),
    
    /// Snapshot not found
    #[error("Snapshot not found: {0}")]
    NotFound(String),
    
    /// Corrupted snapshot
    #[error("Corrupted snapshot: {0}")]
    Corrupted(String),
    
    /// Version mismatch
    #[error("Snapshot version {found} not supported, expected {expected}")]
    VersionMismatch {
        expected: String,
        found: String,
    },
}

/// Snapshot format version
const SNAPSHOT_VERSION: &str = "1.0";

/// Snapshot of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique snapshot ID
    pub id: String,
    
    /// Device ID this snapshot belongs to
    pub device_id: String,
    
    /// Timestamp when snapshot was created
    pub created_at: DateTime<Utc>,
    
    /// Snapshot format version
    pub version: String,
    
    /// Parent snapshot ID (for incremental snapshots)
    pub parent_id: Option<String>,
    
    /// File metadata at time of snapshot
    pub files: Vec<FileMetadata>,
    
    /// Changes included in this snapshot
    pub changes: Vec<FileChange>,
    
    /// Total files in snapshot
    pub total_files: u64,
    
    /// Total size of all files
    pub total_size: u64,
    
    /// Files synced in this operation
    pub files_synced: u64,
    
    /// Bytes transferred in this operation
    pub bytes_transferred: u64,
    
    /// Snapshot metadata
    pub metadata: SnapshotMetadata,
    
    /// Chunk references for deduplication
    pub chunk_refs: Vec<ChunkReference>,
    
    /// Verification hash of snapshot content
    pub verification_hash: String,
}

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Source path that was synced
    pub source_path: PathBuf,
    
    /// Sync options used
    pub sync_options: HashMap<String, String>,
    
    /// Errors encountered during sync
    pub errors: Vec<String>,
    
    /// Warnings
    pub warnings: Vec<String>,
    
    /// Custom key-value pairs
    pub custom: HashMap<String, String>,
}

/// Reference to a chunk in the chunk store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkReference {
    /// File path this chunk belongs to
    pub file_path: PathBuf,
    
    /// Chunk index in file
    pub chunk_index: u32,
    
    /// Chunk hash for lookup
    pub chunk_hash: String,
    
    /// Size of chunk
    pub size: u64,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(
        device_id: String,
        changes: Vec<FileChange>,
        files_synced: u64,
        bytes_transferred: u64,
    ) -> Self {
        let files = Self::build_file_list(&changes);
        let total_files = files.len() as u64;
        let total_size = files.iter().map(|f| f.size).sum();
        
        let snapshot = Self {
            id: Uuid::new_v4().to_string(),
            device_id,
            created_at: Utc::now(),
            version: SNAPSHOT_VERSION.to_string(),
            parent_id: None,
            files: files.clone(),
            changes,
            total_files,
            total_size,
            files_synced,
            bytes_transferred,
            metadata: SnapshotMetadata {
                source_path: PathBuf::new(),
                sync_options: HashMap::new(),
                errors: Vec::new(),
                warnings: Vec::new(),
                custom: HashMap::new(),
            },
            chunk_refs: Vec::new(),
            verification_hash: String::new(),
        };
        
        snapshot
    }
    
    /// Create an incremental snapshot
    pub fn new_incremental(
        device_id: String,
        parent_id: String,
        changes: Vec<FileChange>,
        parent_files: &[FileMetadata],
        files_synced: u64,
        bytes_transferred: u64,
    ) -> Self {
        let mut files = parent_files.to_vec();
        
        // Apply changes to get new file list
        for change in &changes {
            match change {
                FileChange::Added { path, size, modified, hash, permissions } => {
                    files.push(FileMetadata {
                        path: path.clone(),
                        size: *size,
                        modified: *modified,
                        hash: hash.clone(),
                        permissions: *permissions,
                    });
                }
                FileChange::Modified { path, size, modified, hash, permissions, .. } => {
                    if let Some(file) = files.iter_mut().find(|f| f.path == *path) {
                        file.size = *size;
                        file.modified = *modified;
                        file.hash = hash.clone();
                        file.permissions = *permissions;
                    }
                }
                FileChange::Deleted { path, .. } => {
                    files.retain(|f| f.path != *path);
                }
            }
        }
        
        let total_files = files.len() as u64;
        let total_size = files.iter().map(|f| f.size).sum();
        
        Self {
            id: Uuid::new_v4().to_string(),
            device_id,
            created_at: Utc::now(),
            version: SNAPSHOT_VERSION.to_string(),
            parent_id: Some(parent_id),
            files,
            changes,
            total_files,
            total_size,
            files_synced,
            bytes_transferred,
            metadata: SnapshotMetadata {
                source_path: PathBuf::new(),
                sync_options: HashMap::new(),
                errors: Vec::new(),
                warnings: Vec::new(),
                custom: HashMap::new(),
            },
            chunk_refs: Vec::new(),
            verification_hash: String::new(),
        }
    }
    
    /// Build file list from changes (for initial snapshot)
    fn build_file_list(changes: &[FileChange]) -> Vec<FileMetadata> {
        changes.iter()
            .filter_map(|change| match change {
                FileChange::Added { path, size, modified, hash, permissions } |
                FileChange::Modified { path, size, modified, hash, permissions, .. } => {
                    Some(FileMetadata {
                        path: path.clone(),
                        size: *size,
                        modified: *modified,
                        hash: hash.clone(),
                        permissions: *permissions,
                    })
                }
                FileChange::Deleted { .. } => None,
            })
            .collect()
    }
    
    /// Find a file in the snapshot
    pub fn find_file(&self, path: &Path) -> Option<&FileMetadata> {
        self.files.iter().find(|f| f.path == path)
    }
    
    /// Get all files matching a pattern
    pub fn find_files_matching(&self, pattern: &str) -> Vec<&FileMetadata> {
        let glob_pattern = glob::Pattern::new(pattern).ok();
        
        self.files.iter()
            .filter(|f| {
                glob_pattern.as_ref()
                    .map(|p| p.matches(f.path.to_string_lossy().as_ref()))
                    .unwrap_or(false)
            })
            .collect()
    }
    
    /// Compute verification hash
    pub fn compute_verification_hash(&mut self) {
        use sha2::Sha256;
        let mut hasher = Sha256::new();
        
        // Hash key fields
        hasher.update(&self.id);
        hasher.update(&self.device_id);
        hasher.update(self.created_at.to_rfc3339());
        hasher.update(&self.version);
        
        // Hash file list
        for file in &self.files {
            hasher.update(file.path.to_string_lossy().as_bytes());
            hasher.update(file.size.to_le_bytes());
            hasher.update(&file.hash);
        }
        
        self.verification_hash = hex::encode(hasher.finalize());
    }
    
    /// Verify snapshot integrity
    pub fn verify(&self) -> Result<(), SnapshotError> {
        let mut temp = self.clone();
        temp.verification_hash.clear();
        temp.compute_verification_hash();
        
        if temp.verification_hash != self.verification_hash {
            return Err(SnapshotError::Corrupted(
                "Verification hash mismatch".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Snapshot manager for storing and retrieving snapshots
pub struct SnapshotManager {
    /// Base directory for snapshot storage
    storage_dir: PathBuf,
    
    /// Whether to compress snapshots
    compress: bool,
    
    /// Cache of loaded snapshots
    cache: HashMap<String, Snapshot>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(storage_dir: &Path) -> Result<Self, SnapshotError> {
        fs::create_dir_all(storage_dir)?;
        
        Ok(Self {
            storage_dir: storage_dir.to_path_buf(),
            compress: true,
            cache: HashMap::new(),
        })
    }
    
    /// Set compression
    pub fn set_compress(&mut self, compress: bool) {
        self.compress = compress;
    }
    
    /// Get snapshot file path
    fn get_snapshot_path(&self, device_id: &str, snapshot_id: &str) -> PathBuf {
        self.storage_dir
            .join(device_id)
            .join(format!("{}.snapshot{}", snapshot_id, if self.compress { ".gz" } else { "" }))
    }
    
    /// Save a snapshot
    pub fn save_snapshot(&mut self, snapshot: &Snapshot) -> Result<(), SnapshotError> {
        // Compute verification hash
        let mut snapshot = snapshot.clone();
        snapshot.compute_verification_hash();
        
        // Ensure device directory exists
        let device_dir = self.storage_dir.join(&snapshot.device_id);
        fs::create_dir_all(&device_dir)?;
        
        // Write snapshot
        let path = self.get_snapshot_path(&snapshot.device_id, &snapshot.id);
        let file = File::create(&path)?;
        
        if self.compress {
            let encoder = GzEncoder::new(file, Compression::default());
            let writer = BufWriter::new(encoder);
            serde_json::to_writer_pretty(writer, &snapshot)?;
        } else {
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, &snapshot)?;
        }
        
        // Update cache
        self.cache.insert(snapshot.id.clone(), snapshot);
        
        Ok(())
    }
    
    /// Load a snapshot
    pub fn load_snapshot(&mut self, device_id: &str, snapshot_id: &str) -> Result<Snapshot, SnapshotError> {
        // Check cache first
        if let Some(snapshot) = self.cache.get(snapshot_id) {
            return Ok(snapshot.clone());
        }
        
        // Load from disk
        let path = self.get_snapshot_path(device_id, snapshot_id);
        if !path.exists() {
            return Err(SnapshotError::NotFound(snapshot_id.to_string()));
        }
        
        let file = File::open(&path)?;
        
        let snapshot: Snapshot = if self.compress {
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            serde_json::from_reader(reader)?
        } else {
            let reader = BufReader::new(file);
            serde_json::from_reader(reader)?
        };
        
        // Verify version
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(SnapshotError::VersionMismatch {
                expected: SNAPSHOT_VERSION.to_string(),
                found: snapshot.version,
            });
        }
        
        // Verify integrity
        snapshot.verify()?;
        
        // Update cache
        self.cache.insert(snapshot.id.clone(), snapshot.clone());
        
        Ok(snapshot)
    }
    
    /// Get latest snapshot for device
    pub fn get_latest_snapshot(&mut self, device_id: &str) -> Result<Option<Snapshot>, SnapshotError> {
        let device_dir = self.storage_dir.join(device_id);
        if !device_dir.exists() {
            return Ok(None);
        }
        
        let mut latest: Option<(PathBuf, DateTime<Utc>)> = None;
        
        for entry in fs::read_dir(&device_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("snapshot")
                || path.to_string_lossy().ends_with(".snapshot.gz") {
                
                let metadata = entry.metadata()?;
                let modified = metadata.modified()?;
                let modified_dt = DateTime::from_timestamp(
                    modified.duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    0
                ).unwrap();
                
                if latest.is_none() || latest.as_ref().unwrap().1 < modified_dt {
                    latest = Some((path, modified_dt));
                }
            }
        }
        
        if let Some((path, _)) = latest {
            let filename = path.file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| SnapshotError::InvalidFormat("Invalid filename".to_string()))?;
            
            // Extract ID from filename (handle both .snapshot and .snapshot.gz)
            let snapshot_id = if filename.ends_with(".snapshot.gz") {
                &filename[..filename.len() - 12] // Remove .snapshot.gz
            } else if filename.ends_with(".snapshot") {
                &filename[..filename.len() - 9]  // Remove .snapshot
            } else {
                return Err(SnapshotError::InvalidFormat("Invalid snapshot extension".to_string()));
            };
            
            Ok(Some(self.load_snapshot(device_id, snapshot_id)?))
        } else {
            Ok(None)
        }
    }
    
    /// List all snapshots for a device
    pub fn list_snapshots(&self, device_id: &str) -> Result<Vec<SnapshotInfo>, SnapshotError> {
        let device_dir = self.storage_dir.join(device_id);
        if !device_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut snapshots = Vec::new();
        
        for entry in fs::read_dir(&device_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("snapshot")
                || path.to_string_lossy().ends_with(".snapshot.gz") {
                
                let metadata = entry.metadata()?;
                let filename = path.file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| SnapshotError::InvalidFormat("Invalid filename".to_string()))?;
                
                // Extract ID from filename (handle both .snapshot and .snapshot.gz)
                let snapshot_id = if filename.ends_with(".snapshot.gz") {
                    &filename[..filename.len() - 12] // Remove .snapshot.gz
                } else if filename.ends_with(".snapshot") {
                    &filename[..filename.len() - 9]  // Remove .snapshot
                } else {
                    return Err(SnapshotError::InvalidFormat("Invalid snapshot extension".to_string()));
                };
                
                let snapshot_id = snapshot_id.to_string();
                
                let created = DateTime::from_timestamp(
                    metadata.modified()?
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    0
                ).unwrap();
                
                snapshots.push(SnapshotInfo {
                    id: snapshot_id,
                    device_id: device_id.to_string(),
                    created_at: created,
                    size: metadata.len(),
                });
            }
        }
        
        // Sort by creation time (newest first)
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(snapshots)
    }
    
    /// Delete a snapshot
    pub fn delete_snapshot(&mut self, device_id: &str, snapshot_id: &str) -> Result<(), SnapshotError> {
        let path = self.get_snapshot_path(device_id, snapshot_id);
        
        if path.exists() {
            fs::remove_file(&path)?;
            self.cache.remove(snapshot_id);
        }
        
        Ok(())
    }
    
    /// Clean up old snapshots based on retention policy
    pub fn cleanup_old_snapshots(
        &mut self,
        device_id: &str,
        retain_count: usize,
    ) -> Result<Vec<String>, SnapshotError> {
        let snapshots = self.list_snapshots(device_id)?;
        let mut deleted = Vec::new();
        
        if snapshots.len() > retain_count {
            // Keep the newest snapshots, delete the rest
            for snapshot in &snapshots[retain_count..] {
                self.delete_snapshot(device_id, &snapshot.id)?;
                deleted.push(snapshot.id.clone());
            }
        }
        
        Ok(deleted)
    }
    
    /// Get snapshot chain (current and all parents)
    pub fn get_snapshot_chain(
        &mut self,
        device_id: &str,
        snapshot_id: &str,
    ) -> Result<Vec<Snapshot>, SnapshotError> {
        let mut chain = Vec::new();
        let mut current_id = Some(snapshot_id.to_string());
        
        while let Some(id) = current_id {
            let snapshot = self.load_snapshot(device_id, &id)?;
            current_id = snapshot.parent_id.clone();
            chain.push(snapshot);
        }
        
        chain.reverse(); // Oldest first
        Ok(chain)
    }
    
    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

/// Snapshot information summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub id: String,
    pub device_id: String,
    pub created_at: DateTime<Utc>,
    pub size: u64,
}

/// Snapshot comparison result
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub added_files: Vec<FileMetadata>,
    pub modified_files: Vec<(FileMetadata, FileMetadata)>, // (old, new)
    pub deleted_files: Vec<FileMetadata>,
}

impl SnapshotDiff {
    /// Compare two snapshots
    pub fn compare(old: &Snapshot, new: &Snapshot) -> Self {
        let mut old_files: HashMap<&Path, &FileMetadata> = HashMap::new();
        let mut new_files: HashMap<&Path, &FileMetadata> = HashMap::new();
        
        for file in &old.files {
            old_files.insert(&file.path, file);
        }
        
        for file in &new.files {
            new_files.insert(&file.path, file);
        }
        
        let mut added_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut deleted_files = Vec::new();
        
        // Find added and modified files
        for (path, new_file) in &new_files {
            if let Some(old_file) = old_files.get(path) {
                if old_file.hash != new_file.hash || old_file.size != new_file.size {
                    modified_files.push(((*old_file).clone(), (*new_file).clone()));
                }
            } else {
                added_files.push((*new_file).clone());
            }
        }
        
        // Find deleted files
        for (path, old_file) in &old_files {
            if !new_files.contains_key(path) {
                deleted_files.push((*old_file).clone());
            }
        }
        
        Self {
            added_files,
            modified_files,
            deleted_files,
        }
    }
    
    /// Get total change count
    pub fn total_changes(&self) -> usize {
        self.added_files.len() + self.modified_files.len() + self.deleted_files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_snapshot_creation() {
        let changes = vec![
            FileChange::Added {
                path: PathBuf::from("test.txt"),
                size: 100,
                modified: Utc::now(),
                hash: "hash123".to_string(),
                permissions: 0o644,
            },
        ];
        
        let snapshot = Snapshot::new(
            "device123".to_string(),
            changes,
            1,
            100,
        );
        
        assert_eq!(snapshot.device_id, "device123");
        assert_eq!(snapshot.files_synced, 1);
        assert_eq!(snapshot.bytes_transferred, 100);
        assert_eq!(snapshot.files.len(), 1);
    }
    
    #[test]
    fn test_snapshot_manager() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path()).unwrap();
        
        let snapshot = Snapshot::new(
            "device123".to_string(),
            vec![],
            0,
            0,
        );
        
        // Save snapshot
        manager.save_snapshot(&snapshot).unwrap();
        
        // Load snapshot
        let loaded = manager.load_snapshot("device123", &snapshot.id).unwrap();
        assert_eq!(loaded.id, snapshot.id);
        
        // List snapshots
        let list = manager.list_snapshots("device123").unwrap();
        assert_eq!(list.len(), 1);
        
        // Get latest
        let latest = manager.get_latest_snapshot("device123").unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, snapshot.id);
    }
    
    #[test]
    fn test_snapshot_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path()).unwrap();
        
        // Create multiple snapshots
        for i in 0..5 {
            let snapshot = Snapshot::new(
                "device123".to_string(),
                vec![],
                i,
                i * 100,
            );
            manager.save_snapshot(&snapshot).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        
        // Clean up old snapshots
        let deleted = manager.cleanup_old_snapshots("device123", 3).unwrap();
        assert_eq!(deleted.len(), 2);
        
        // Verify only 3 remain
        let remaining = manager.list_snapshots("device123").unwrap();
        assert_eq!(remaining.len(), 3);
    }
    
    #[test]
    fn test_snapshot_diff() {
        let file1 = FileMetadata {
            path: PathBuf::from("file1.txt"),
            size: 100,
            modified: Utc::now(),
            hash: "hash1".to_string(),
            permissions: 0o644,
        };
        
        let file2 = FileMetadata {
            path: PathBuf::from("file2.txt"),
            size: 200,
            modified: Utc::now(),
            hash: "hash2".to_string(),
            permissions: 0o644,
        };
        
        let file1_modified = FileMetadata {
            path: PathBuf::from("file1.txt"),
            size: 150,
            modified: Utc::now(),
            hash: "hash1_new".to_string(),
            permissions: 0o644,
        };
        
        let old_snapshot = Snapshot {
            files: vec![file1.clone()],
            ..Snapshot::new("device".to_string(), vec![], 0, 0)
        };
        
        let new_snapshot = Snapshot {
            files: vec![file1_modified.clone(), file2.clone()],
            ..Snapshot::new("device".to_string(), vec![], 0, 0)
        };
        
        let diff = SnapshotDiff::compare(&old_snapshot, &new_snapshot);
        
        assert_eq!(diff.added_files.len(), 1);
        assert_eq!(diff.modified_files.len(), 1);
        assert_eq!(diff.deleted_files.len(), 0);
        assert_eq!(diff.total_changes(), 2);
    }
}
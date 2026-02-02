//! Diff engine for detecting file changes
//!
//! This module provides functionality to detect changes in files by comparing
//! their metadata, content hashes, and modification times.

use crate::snapshot::Snapshot;
use std::path::{Path, PathBuf};
use std::fs::{self};
use std::time::SystemTime;
use std::io::{self, Read, BufReader};
use std::collections::HashMap;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use hex;
use walkdir;

/// Errors that can occur during diff operations
#[derive(Error, Debug)]
pub enum DiffError {
    /// IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    /// Path conversion error
    #[error("Path conversion error: {0}")]
    PathConversion(String),
    
    /// Invalid metadata
    #[error("Invalid metadata for path: {0}")]
    InvalidMetadata(String),
    
    /// Hash computation failed
    #[error("Failed to compute hash: {0}")]
    HashError(String),
    
    /// Walk directory error
    #[error("Walk directory error: {0}")]
    WalkError(#[from] walkdir::Error),
}

/// Type of file change detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FileChange {
    /// File was added
    Added {
        path: PathBuf,
        size: u64,
        modified: DateTime<Utc>,
        hash: String,
        permissions: u32,
    },
    
    /// File was modified
    Modified {
        path: PathBuf,
        size: u64,
        modified: DateTime<Utc>,
        hash: String,
        old_hash: String,
        permissions: u32,
    },
    
    /// File was deleted
    Deleted {
        path: PathBuf,
        old_size: u64,
        old_hash: String,
    },
}

impl FileChange {
    /// Get the path of the changed file
    pub fn path(&self) -> &Path {
        match self {
            FileChange::Added { path, .. } |
            FileChange::Modified { path, .. } |
            FileChange::Deleted { path, .. } => path,
        }
    }
    
    /// Get the size of the file (0 for deleted files)
    pub fn size(&self) -> u64 {
        match self {
            FileChange::Added { size, .. } |
            FileChange::Modified { size, .. } => *size,
            FileChange::Deleted { .. } => 0,
        }
    }
    
    /// Get the hash of the file content
    pub fn hash(&self) -> Option<&str> {
        match self {
            FileChange::Added { hash, .. } => Some(hash),
            FileChange::Modified { hash, .. } => Some(hash),
            FileChange::Deleted { .. } => None,
        }
    }
}

/// File metadata for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub hash: String,
    pub permissions: u32,
}

impl FileMetadata {
    /// Create metadata from file path
    pub fn from_path(path: &Path, base_path: &Path) -> Result<Self, DiffError> {
        let metadata = fs::metadata(path)?;
        let relative_path = path.strip_prefix(base_path)
            .map_err(|e| DiffError::PathConversion(e.to_string()))?;
        
        let modified = metadata.modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| DiffError::InvalidMetadata(format!("Invalid modified time: {}", e)))?;
        
        let modified_dt = DateTime::from_timestamp(modified.as_secs() as i64, modified.subsec_nanos())
            .ok_or_else(|| DiffError::InvalidMetadata("Invalid timestamp".to_string()))?;
        
        let hash = compute_file_hash(path)?;
        
        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        
        #[cfg(not(unix))]
        let permissions = if metadata.permissions().readonly() { 0o444 } else { 0o644 };
        
        Ok(Self {
            path: relative_path.to_path_buf(),
            size: metadata.len(),
            modified: modified_dt,
            hash,
            permissions,
        })
    }
}

/// Compute SHA-256 hash of file content
fn compute_file_hash(path: &Path) -> Result<String, DiffError> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    
    let mut buffer = [0; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(hex::encode(hasher.finalize()))
}

/// Diff engine for computing file changes
pub struct DiffEngine {
    /// Cache of file hashes to avoid recomputation
    hash_cache: HashMap<PathBuf, (SystemTime, String)>,
    
    /// Whether to use hash cache
    use_cache: bool,
    
    /// Fast mode - use size and mtime only
    fast_mode: bool,
}

impl DiffEngine {
    /// Create a new diff engine
    pub fn new() -> Self {
        Self {
            hash_cache: HashMap::new(),
            use_cache: true,
            fast_mode: false,
        }
    }
    
    /// Create a diff engine in fast mode (no content hashing)
    pub fn new_fast() -> Self {
        Self {
            hash_cache: HashMap::new(),
            use_cache: true,
            fast_mode: true,
        }
    }
    
    /// Set whether to use hash cache
    pub fn set_use_cache(&mut self, use_cache: bool) {
        self.use_cache = use_cache;
    }
    
    /// Clear the hash cache
    pub fn clear_cache(&mut self) {
        self.hash_cache.clear();
    }
    
    /// Compute file change for a single file
    pub fn compute_file_change(
        &self,
        path: &Path,
        base_path: &Path,
        snapshot: Option<&Snapshot>,
    ) -> Result<Option<FileChange>, DiffError> {
        let metadata = FileMetadata::from_path(path, base_path)?;
        
        if let Some(snapshot) = snapshot {
            // Check if file exists in snapshot
            if let Some(old_metadata) = snapshot.find_file(&metadata.path) {
                // File exists in snapshot - check if modified
                if self.is_file_modified(&metadata, old_metadata)? {
                    Ok(Some(FileChange::Modified {
                        path: metadata.path,
                        size: metadata.size,
                        modified: metadata.modified,
                        hash: metadata.hash,
                        old_hash: old_metadata.hash.clone(),
                        permissions: metadata.permissions,
                    }))
                } else {
                    // File unchanged
                    Ok(None)
                }
            } else {
                // File not in snapshot - new file
                Ok(Some(FileChange::Added {
                    path: metadata.path,
                    size: metadata.size,
                    modified: metadata.modified,
                    hash: metadata.hash,
                    permissions: metadata.permissions,
                }))
            }
        } else {
            // No snapshot - all files are new
            Ok(Some(FileChange::Added {
                path: metadata.path,
                size: metadata.size,
                modified: metadata.modified,
                hash: metadata.hash,
                permissions: metadata.permissions,
            }))
        }
    }
    
    /// Check if a file has been modified
    fn is_file_modified(
        &self,
        current: &FileMetadata,
        old: &FileMetadata,
    ) -> Result<bool, DiffError> {
        // Quick checks first
        if current.size != old.size {
            return Ok(true);
        }
        
        if current.permissions != old.permissions {
            return Ok(true);
        }
        
        if self.fast_mode {
            // In fast mode, only check size and mtime
            return Ok(current.modified > old.modified);
        }
        
        // Full content comparison
        Ok(current.hash != old.hash)
    }
    
    /// Compute full diff between current state and snapshot
    pub fn compute_full_diff(
        &mut self,
        source_path: &Path,
        snapshot: Option<&Snapshot>,
        exclude_patterns: &[String],
    ) -> Result<Vec<FileChange>, DiffError> {
        let mut changes = Vec::new();
        let mut current_files = HashMap::new();
        
        // Walk source directory
        for entry in walkdir::WalkDir::new(source_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !should_exclude(e.path(), exclude_patterns))
        {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let metadata = if self.use_cache {
                    self.get_metadata_cached(path, source_path)?
                } else {
                    FileMetadata::from_path(path, source_path)?
                };
                
                current_files.insert(metadata.path.clone(), metadata.clone());
                
                if let Some(change) = self.compute_change_for_metadata(
                    &metadata,
                    snapshot,
                )? {
                    changes.push(change);
                }
            }
        }
        
        // Check for deleted files
        if let Some(snapshot) = snapshot {
            for old_file in &snapshot.files {
                if !current_files.contains_key(&old_file.path) {
                    changes.push(FileChange::Deleted {
                        path: old_file.path.clone(),
                        old_size: old_file.size,
                        old_hash: old_file.hash.clone(),
                    });
                }
            }
        }
        
        Ok(changes)
    }
    
    /// Get file metadata with caching
    fn get_metadata_cached(
        &mut self,
        path: &Path,
        base_path: &Path,
    ) -> Result<FileMetadata, DiffError> {
        let file_metadata = fs::metadata(path)?;
        let modified = file_metadata.modified()?;
        
        // Check cache
        if let Some((cached_time, cached_hash)) = self.hash_cache.get(path) {
            if *cached_time == modified && !self.fast_mode {
                // Cache hit - reuse hash
                let relative_path = path.strip_prefix(base_path)
                    .map_err(|e| DiffError::PathConversion(e.to_string()))?;
                
                let modified_dt = DateTime::from_timestamp(
                    modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
                    0
                ).unwrap();
                
                #[cfg(unix)]
                let permissions = {
                    use std::os::unix::fs::PermissionsExt;
                    file_metadata.permissions().mode()
                };
                
                #[cfg(not(unix))]
                let permissions = if file_metadata.permissions().readonly() { 0o444 } else { 0o644 };
                
                return Ok(FileMetadata {
                    path: relative_path.to_path_buf(),
                    size: file_metadata.len(),
                    modified: modified_dt,
                    hash: cached_hash.clone(),
                    permissions,
                });
            }
        }
        
        // Cache miss - compute metadata
        let metadata = FileMetadata::from_path(path, base_path)?;
        
        // Update cache
        if !self.fast_mode {
            self.hash_cache.insert(
                path.to_path_buf(),
                (modified, metadata.hash.clone())
            );
        }
        
        Ok(metadata)
    }
    
    /// Compute change for file metadata
    fn compute_change_for_metadata(
        &self,
        metadata: &FileMetadata,
        snapshot: Option<&Snapshot>,
    ) -> Result<Option<FileChange>, DiffError> {
        if let Some(snapshot) = snapshot {
            if let Some(old_metadata) = snapshot.find_file(&metadata.path) {
                if self.is_file_modified(metadata, old_metadata)? {
                    Ok(Some(FileChange::Modified {
                        path: metadata.path.clone(),
                        size: metadata.size,
                        modified: metadata.modified,
                        hash: metadata.hash.clone(),
                        old_hash: old_metadata.hash.clone(),
                        permissions: metadata.permissions,
                    }))
                } else {
                    Ok(None)
                }
            } else {
                Ok(Some(FileChange::Added {
                    path: metadata.path.clone(),
                    size: metadata.size,
                    modified: metadata.modified,
                    hash: metadata.hash.clone(),
                    permissions: metadata.permissions,
                }))
            }
        } else {
            Ok(Some(FileChange::Added {
                path: metadata.path.clone(),
                size: metadata.size,
                modified: metadata.modified,
                hash: metadata.hash.clone(),
                permissions: metadata.permissions,
            }))
        }
    }
}

/// Check if path should be excluded
fn should_exclude(path: &Path, patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();
    patterns.iter().any(|pattern| {
        glob::Pattern::new(pattern)
            .map(|p| p.matches(&path_str))
            .unwrap_or(false)
    })
}

/// Diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_added: usize,
    pub files_modified: usize,
    pub files_deleted: usize,
    pub bytes_added: u64,
    pub bytes_modified: u64,
    pub bytes_deleted: u64,
}

impl DiffStats {
    /// Create stats from a list of file changes
    pub fn from_changes(changes: &[FileChange]) -> Self {
        let mut stats = Self {
            files_added: 0,
            files_modified: 0,
            files_deleted: 0,
            bytes_added: 0,
            bytes_modified: 0,
            bytes_deleted: 0,
        };
        
        for change in changes {
            match change {
                FileChange::Added { size, .. } => {
                    stats.files_added += 1;
                    stats.bytes_added += size;
                }
                FileChange::Modified { size, .. } => {
                    stats.files_modified += 1;
                    stats.bytes_modified += size;
                }
                FileChange::Deleted { old_size, .. } => {
                    stats.files_deleted += 1;
                    stats.bytes_deleted += old_size;
                }
            }
        }
        
        stats
    }
    
    /// Get total number of changed files
    pub fn total_files(&self) -> usize {
        self.files_added + self.files_modified + self.files_deleted
    }
    
    /// Get total bytes affected
    pub fn total_bytes(&self) -> u64 {
        self.bytes_added + self.bytes_modified + self.bytes_deleted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[test]
    fn test_compute_file_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();
        file.sync_all().unwrap();
        
        let hash = compute_file_hash(&file_path).unwrap();
        assert_eq!(hash, "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3");
    }
    
    #[test]
    fn test_file_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Test content").unwrap();
        file.sync_all().unwrap();
        
        let metadata = FileMetadata::from_path(&file_path, temp_dir.path()).unwrap();
        assert_eq!(metadata.path, Path::new("test.txt"));
        assert_eq!(metadata.size, 12);
        assert!(!metadata.hash.is_empty());
    }
    
    #[test]
    fn test_diff_stats() {
        let changes = vec![
            FileChange::Added {
                path: PathBuf::from("new.txt"),
                size: 100,
                modified: Utc::now(),
                hash: "hash1".to_string(),
                permissions: 0o644,
            },
            FileChange::Modified {
                path: PathBuf::from("changed.txt"),
                size: 200,
                modified: Utc::now(),
                hash: "hash2".to_string(),
                old_hash: "hash3".to_string(),
                permissions: 0o644,
            },
            FileChange::Deleted {
                path: PathBuf::from("deleted.txt"),
                old_size: 50,
                old_hash: "hash4".to_string(),
            },
        ];
        
        let stats = DiffStats::from_changes(&changes);
        assert_eq!(stats.files_added, 1);
        assert_eq!(stats.files_modified, 1);
        assert_eq!(stats.files_deleted, 1);
        assert_eq!(stats.bytes_added, 100);
        assert_eq!(stats.bytes_modified, 200);
        assert_eq!(stats.bytes_deleted, 50);
        assert_eq!(stats.total_files(), 3);
        assert_eq!(stats.total_bytes(), 350);
    }
}
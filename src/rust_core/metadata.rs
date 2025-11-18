//! File metadata tracking and comparison
//!
//! This module provides structures and functions for tracking file metadata
//! including permissions, timestamps, and file attributes.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

/// Metadata error types
#[derive(Debug, Error)]
pub enum MetadataError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid metadata
    #[error("Invalid metadata: {0}")]
    Invalid(String),
}

/// Result type for metadata operations
pub type Result<T> = std::result::Result<T, MetadataError>;

/// File metadata information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileMetadata {
    /// File path (relative to source root)
    pub path: PathBuf,

    /// File size in bytes
    pub size: u64,

    /// File type (file, directory, symlink)
    pub file_type: FileType,

    /// Last modified timestamp (Unix epoch seconds)
    pub modified: u64,

    /// Creation timestamp (Unix epoch seconds)
    pub created: u64,

    /// Unix permissions (octal format)
    pub permissions: u32,

    /// File hash (SHA-256)
    pub hash: Option<String>,

    /// Whether the file is executable
    pub is_executable: bool,

    /// Whether the file is hidden
    pub is_hidden: bool,
}

/// File type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    /// Regular file
    File,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink,
}

impl FileMetadata {
    /// Create metadata from a file path
    pub fn from_path<P: AsRef<Path>>(path: P, base_path: P) -> Result<Self> {
        let path = path.as_ref();
        let base_path = base_path.as_ref();

        let metadata = fs::symlink_metadata(path)?;
        let relative_path = path.strip_prefix(base_path)
            .map_err(|e| MetadataError::Invalid(format!("Invalid path: {}", e)))?
            .to_path_buf();

        let file_type = if metadata.is_symlink() {
            FileType::Symlink
        } else if metadata.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };

        let modified = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| MetadataError::Invalid(format!("Invalid timestamp: {}", e)))?
            .as_secs();

        let created = metadata
            .created()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        #[cfg(unix)]
        let (permissions, is_executable) = {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            (mode, mode & 0o111 != 0)
        };

        #[cfg(not(unix))]
        let (permissions, is_executable) = {
            (0o644, false)
        };

        let is_hidden = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false);

        Ok(Self {
            path: relative_path,
            size: metadata.len(),
            file_type,
            modified,
            created,
            permissions,
            hash: None,
            is_executable,
            is_hidden,
        })
    }

    /// Check if metadata has changed compared to another
    pub fn has_changed(&self, other: &FileMetadata) -> bool {
        self.size != other.size
            || self.modified != other.modified
            || self.permissions != other.permissions
            || self.hash != other.hash
    }

    /// Check if content needs to be synced (based on hash)
    pub fn needs_sync(&self, other: &FileMetadata) -> bool {
        match (&self.hash, &other.hash) {
            (Some(h1), Some(h2)) => h1 != h2,
            _ => self.has_changed(other),
        }
    }
}

/// Snapshot manifest containing all file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest version
    pub version: u32,

    /// Creation timestamp
    pub created: u64,

    /// Source path
    pub source_path: PathBuf,

    /// Device ID
    pub device_id: String,

    /// List of all files
    pub files: Vec<FileMetadata>,

    /// Total size in bytes
    pub total_size: u64,

    /// Number of files
    pub file_count: usize,
}

impl Manifest {
    /// Create a new manifest
    pub fn new(source_path: PathBuf, device_id: String) -> Self {
        let created = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            version: 1,
            created,
            source_path,
            device_id,
            files: Vec::new(),
            total_size: 0,
            file_count: 0,
        }
    }

    /// Add a file to the manifest
    pub fn add_file(&mut self, metadata: FileMetadata) {
        self.total_size += metadata.size;
        self.file_count += 1;
        self.files.push(metadata);
    }

    /// Find a file by path
    pub fn find_file(&self, path: &Path) -> Option<&FileMetadata> {
        self.files.iter().find(|f| f.path == path)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| MetadataError::Serialization(e.to_string()))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| MetadataError::Serialization(e.to_string()))
    }

    /// Save to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_file_metadata_from_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let metadata = FileMetadata::from_path(&file_path, &dir.path().to_path_buf()).unwrap();
        assert_eq!(metadata.path, PathBuf::from("test.txt"));
        assert_eq!(metadata.file_type, FileType::File);
        assert_eq!(metadata.size, 0);
    }

    #[test]
    fn test_manifest_operations() {
        let mut manifest = Manifest::new(PathBuf::from("/tmp/source"), "device1".to_string());

        let file_meta = FileMetadata {
            path: PathBuf::from("file1.txt"),
            size: 100,
            file_type: FileType::File,
            modified: 1234567890,
            created: 1234567890,
            permissions: 0o644,
            hash: Some("abc123".to_string()),
            is_executable: false,
            is_hidden: false,
        };

        manifest.add_file(file_meta.clone());
        assert_eq!(manifest.file_count, 1);
        assert_eq!(manifest.total_size, 100);

        let found = manifest.find_file(&PathBuf::from("file1.txt"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().size, 100);
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = Manifest::new(PathBuf::from("/tmp/source"), "device1".to_string());
        let json = manifest.to_json().unwrap();
        let restored = Manifest::from_json(&json).unwrap();
        assert_eq!(restored.device_id, "device1");
        assert_eq!(restored.version, 1);
    }
}

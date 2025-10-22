//! Snapshot manifest for tracking file states and chunk mappings
//!
//! This module implements the manifest format that tracks all files
//! in a snapshot, their metadata, and chunk mappings.

use crate::diff::FileMetadata;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Manifest error types
#[derive(Debug, Error)]
pub enum ManifestError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Manifest validation failed
    #[error("Invalid manifest: {0}")]
    ValidationError(String),

    /// File not found in manifest
    #[error("File not found in manifest: {0}")]
    FileNotFound(PathBuf),

    /// Manifest version mismatch
    #[error("Manifest version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
}

/// Result type for manifest operations
pub type Result<T> = std::result::Result<T, ManifestError>;

/// Current manifest format version
pub const MANIFEST_VERSION: u32 = 1;

/// Snapshot manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest format version
    pub version: u32,

    /// Snapshot ID (unique identifier)
    pub snapshot_id: String,

    /// Device ID this snapshot belongs to
    pub device_id: String,

    /// Timestamp when snapshot was created
    pub created_at: DateTime<Utc>,

    /// Source path that was backed up
    pub source_path: PathBuf,

    /// Total number of files in snapshot
    pub file_count: usize,

    /// Total bytes in snapshot (original, uncompressed)
    pub total_bytes: u64,

    /// Total compressed bytes stored
    pub compressed_bytes: u64,

    /// Total number of chunks
    pub chunk_count: usize,

    /// Files in this snapshot
    pub files: HashMap<PathBuf, FileEntry>,

    /// Metadata about the manifest itself
    pub metadata: ManifestMetadata,
}

/// Metadata about the manifest
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestMetadata {
    /// Encryption algorithm used
    pub encryption_algorithm: Option<String>,

    /// Compression enabled
    pub compression_enabled: bool,

    /// Chunk size used
    pub chunk_size: usize,

    /// Previous snapshot ID (for incremental backups)
    pub previous_snapshot: Option<String>,

    /// Tags for categorizing snapshots
    pub tags: Vec<String>,

    /// Custom user metadata
    pub custom: HashMap<String, String>,
}

/// Entry for a file in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File metadata
    pub metadata: FileMetadata,

    /// List of chunk IDs that make up this file
    pub chunks: Vec<String>,

    /// Total size (sum of all chunks, original)
    pub total_size: u64,

    /// File type (regular file, directory, symlink, etc.)
    pub file_type: FileType,

    /// Checksum of the complete file (for verification)
    pub checksum: Option<String>,
}

/// Type of file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// Regular file
    RegularFile,

    /// Directory
    Directory,

    /// Symbolic link
    Symlink,

    /// Other (special files, devices, etc.)
    Other,
}

impl Manifest {
    /// Create a new manifest
    pub fn new(device_id: String, source_path: PathBuf) -> Self {
        let snapshot_id = Self::generate_snapshot_id();

        Self {
            version: MANIFEST_VERSION,
            snapshot_id,
            device_id,
            created_at: Utc::now(),
            source_path,
            file_count: 0,
            total_bytes: 0,
            compressed_bytes: 0,
            chunk_count: 0,
            files: HashMap::new(),
            metadata: ManifestMetadata::default(),
        }
    }

    /// Generate a unique snapshot ID
    fn generate_snapshot_id() -> String {
        use sha2::{Digest, Sha256};

        let timestamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let random_bytes: [u8; 16] = rand::random();

        let mut hasher = Sha256::new();
        hasher.update(timestamp.to_le_bytes());
        hasher.update(random_bytes);

        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// Add a file to the manifest
    pub fn add_file(&mut self, path: PathBuf, entry: FileEntry) {
        self.total_bytes += entry.total_size;
        self.chunk_count += entry.chunks.len();
        self.file_count += 1;
        self.files.insert(path, entry);
    }

    /// Remove a file from the manifest
    pub fn remove_file(&mut self, path: &Path) -> Option<FileEntry> {
        if let Some(entry) = self.files.remove(path) {
            self.total_bytes = self.total_bytes.saturating_sub(entry.total_size);
            self.chunk_count = self.chunk_count.saturating_sub(entry.chunks.len());
            self.file_count = self.file_count.saturating_sub(1);
            Some(entry)
        } else {
            None
        }
    }

    /// Get a file entry
    pub fn get_file(&self, path: &Path) -> Option<&FileEntry> {
        self.files.get(path)
    }

    /// Check if a file exists in manifest
    pub fn has_file(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    /// Get all file paths
    pub fn file_paths(&self) -> Vec<&PathBuf> {
        self.files.keys().collect()
    }

    /// Calculate storage efficiency
    pub fn storage_efficiency(&self) -> f64 {
        if self.total_bytes > 0 {
            self.compressed_bytes as f64 / self.total_bytes as f64
        } else {
            1.0
        }
    }

    /// Validate manifest integrity
    pub fn validate(&self) -> Result<()> {
        if self.version != MANIFEST_VERSION {
            return Err(ManifestError::VersionMismatch {
                expected: MANIFEST_VERSION,
                actual: self.version,
            });
        }

        if self.snapshot_id.is_empty() {
            return Err(ManifestError::ValidationError(
                "Snapshot ID cannot be empty".to_string(),
            ));
        }

        if self.device_id.is_empty() {
            return Err(ManifestError::ValidationError(
                "Device ID cannot be empty".to_string(),
            ));
        }

        // Verify file count matches
        if self.files.len() != self.file_count {
            return Err(ManifestError::ValidationError(format!(
                "File count mismatch: manifest says {}, but found {} files",
                self.file_count,
                self.files.len()
            )));
        }

        Ok(())
    }

    /// Save manifest to a file (TOML format)
    pub fn save_toml(&self, path: &Path) -> Result<()> {
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| ManifestError::Serialization(e.to_string()))?;

        let mut file = File::create(path)?;
        file.write_all(toml_string.as_bytes())?;

        Ok(())
    }

    /// Load manifest from a TOML file
    pub fn load_toml(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let manifest: Manifest = toml::from_str(&contents)
            .map_err(|e| ManifestError::Deserialization(e.to_string()))?;

        manifest.validate()?;

        Ok(manifest)
    }

    /// Save manifest to a file (JSON format)
    pub fn save_json(&self, path: &Path) -> Result<()> {
        let json_string = serde_json::to_string_pretty(self)
            .map_err(|e| ManifestError::Serialization(e.to_string()))?;

        let mut file = File::create(path)?;
        file.write_all(json_string.as_bytes())?;

        Ok(())
    }

    /// Load manifest from a JSON file
    pub fn load_json(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let manifest: Manifest = serde_json::from_str(&contents)
            .map_err(|e| ManifestError::Deserialization(e.to_string()))?;

        manifest.validate()?;

        Ok(manifest)
    }

    /// Create a summary of the manifest
    pub fn summary(&self) -> ManifestSummary {
        ManifestSummary {
            snapshot_id: self.snapshot_id.clone(),
            device_id: self.device_id.clone(),
            created_at: self.created_at,
            file_count: self.file_count,
            total_bytes: self.total_bytes,
            compressed_bytes: self.compressed_bytes,
            chunk_count: self.chunk_count,
            compression_ratio: self.storage_efficiency(),
        }
    }
}

/// Summary information about a manifest
#[derive(Debug, Clone)]
pub struct ManifestSummary {
    /// Snapshot ID
    pub snapshot_id: String,

    /// Device ID
    pub device_id: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Number of files
    pub file_count: usize,

    /// Total original bytes
    pub total_bytes: u64,

    /// Total compressed bytes
    pub compressed_bytes: u64,

    /// Number of chunks
    pub chunk_count: usize,

    /// Compression ratio
    pub compression_ratio: f64,
}

impl FileEntry {
    /// Create a new file entry
    pub fn new(
        metadata: FileMetadata,
        chunks: Vec<String>,
        file_type: FileType,
    ) -> Self {
        let total_size = metadata.size;

        Self {
            metadata,
            chunks,
            total_size,
            file_type,
            checksum: None,
        }
    }

    /// Create entry for a regular file
    pub fn regular_file(metadata: FileMetadata, chunks: Vec<String>) -> Self {
        Self::new(metadata, chunks, FileType::RegularFile)
    }

    /// Create entry for a directory
    pub fn directory(metadata: FileMetadata) -> Self {
        Self::new(metadata, Vec::new(), FileType::Directory)
    }

    /// Set checksum for verification
    pub fn with_checksum(mut self, checksum: String) -> Self {
        self.checksum = Some(checksum);
        self
    }

    /// Check if this is a regular file
    pub fn is_regular_file(&self) -> bool {
        self.file_type == FileType::RegularFile
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::Directory
    }
}

/// Manifest index for fast lookups across multiple manifests
pub struct ManifestIndex {
    /// Snapshots indexed by ID
    snapshots: HashMap<String, Manifest>,

    /// Files indexed by path across all snapshots
    file_index: HashMap<PathBuf, Vec<String>>, // path -> [snapshot_ids]
}

impl ManifestIndex {
    /// Create a new manifest index
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            file_index: HashMap::new(),
        }
    }

    /// Add a manifest to the index
    pub fn add_manifest(&mut self, manifest: Manifest) {
        let snapshot_id = manifest.snapshot_id.clone();

        // Index files
        for file_path in manifest.files.keys() {
            self.file_index
                .entry(file_path.clone())
                .or_insert_with(Vec::new)
                .push(snapshot_id.clone());
        }

        // Store manifest
        self.snapshots.insert(snapshot_id, manifest);
    }

    /// Get a manifest by ID
    pub fn get_manifest(&self, snapshot_id: &str) -> Option<&Manifest> {
        self.snapshots.get(snapshot_id)
    }

    /// Find all snapshots containing a specific file
    pub fn find_file(&self, path: &Path) -> Vec<&Manifest> {
        if let Some(snapshot_ids) = self.file_index.get(path) {
            snapshot_ids
                .iter()
                .filter_map(|id| self.snapshots.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all snapshot IDs for a device
    pub fn snapshots_for_device(&self, device_id: &str) -> Vec<String> {
        self.snapshots
            .values()
            .filter(|m| m.device_id == device_id)
            .map(|m| m.snapshot_id.clone())
            .collect()
    }

    /// Get total number of snapshots
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

impl Default for ManifestIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_creation() {
        let manifest = Manifest::new(
            "device123".to_string(),
            PathBuf::from("/source/path"),
        );

        assert_eq!(manifest.version, MANIFEST_VERSION);
        assert_eq!(manifest.device_id, "device123");
        assert_eq!(manifest.file_count, 0);
        assert_eq!(manifest.total_bytes, 0);
    }

    #[test]
    fn test_add_file() {
        let mut manifest = Manifest::new(
            "device123".to_string(),
            PathBuf::from("/source"),
        );

        let metadata = FileMetadata {
            path: PathBuf::from("test.txt"),
            size: 100,
            modified: SystemTime::now(),
            #[cfg(unix)]
            permissions: 0o644,
            hash: None,
            is_dir: false,
        };

        let entry = FileEntry::regular_file(
            metadata.clone(),
            vec!["chunk1".to_string(), "chunk2".to_string()],
        );

        manifest.add_file(PathBuf::from("test.txt"), entry);

        assert_eq!(manifest.file_count, 1);
        assert_eq!(manifest.total_bytes, 100);
        assert_eq!(manifest.chunk_count, 2);
        assert!(manifest.has_file(&PathBuf::from("test.txt")));
    }

    #[test]
    fn test_remove_file() {
        let mut manifest = Manifest::new(
            "device123".to_string(),
            PathBuf::from("/source"),
        );

        let metadata = FileMetadata {
            path: PathBuf::from("test.txt"),
            size: 100,
            modified: SystemTime::now(),
            #[cfg(unix)]
            permissions: 0o644,
            hash: None,
            is_dir: false,
        };

        let entry = FileEntry::regular_file(metadata, vec!["chunk1".to_string()]);
        manifest.add_file(PathBuf::from("test.txt"), entry);

        let removed = manifest.remove_file(&PathBuf::from("test.txt"));
        assert!(removed.is_some());
        assert_eq!(manifest.file_count, 0);
        assert_eq!(manifest.total_bytes, 0);
    }

    #[test]
    fn test_manifest_save_load_toml() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.toml");

        let mut manifest = Manifest::new(
            "device123".to_string(),
            PathBuf::from("/source"),
        );

        let metadata = FileMetadata {
            path: PathBuf::from("test.txt"),
            size: 100,
            modified: SystemTime::now(),
            #[cfg(unix)]
            permissions: 0o644,
            hash: Some("abc123".to_string()),
            is_dir: false,
        };

        let entry = FileEntry::regular_file(metadata, vec!["chunk1".to_string()]);
        manifest.add_file(PathBuf::from("test.txt"), entry);

        // Save
        manifest.save_toml(&manifest_path).unwrap();

        // Load
        let loaded = Manifest::load_toml(&manifest_path).unwrap();

        assert_eq!(loaded.snapshot_id, manifest.snapshot_id);
        assert_eq!(loaded.device_id, manifest.device_id);
        assert_eq!(loaded.file_count, 1);
        assert!(loaded.has_file(&PathBuf::from("test.txt")));
    }

    #[test]
    fn test_manifest_save_load_json() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.json");

        let manifest = Manifest::new(
            "device123".to_string(),
            PathBuf::from("/source"),
        );

        // Save
        manifest.save_json(&manifest_path).unwrap();

        // Load
        let loaded = Manifest::load_json(&manifest_path).unwrap();

        assert_eq!(loaded.snapshot_id, manifest.snapshot_id);
        assert_eq!(loaded.device_id, manifest.device_id);
    }

    #[test]
    fn test_manifest_validation() {
        let manifest = Manifest::new(
            "device123".to_string(),
            PathBuf::from("/source"),
        );

        assert!(manifest.validate().is_ok());

        // Test invalid manifest
        let mut invalid = manifest.clone();
        invalid.version = 999;

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_manifest_index() {
        let mut index = ManifestIndex::new();

        let mut manifest1 = Manifest::new(
            "device1".to_string(),
            PathBuf::from("/source"),
        );

        let metadata = FileMetadata {
            path: PathBuf::from("test.txt"),
            size: 100,
            modified: SystemTime::now(),
            #[cfg(unix)]
            permissions: 0o644,
            hash: None,
            is_dir: false,
        };

        let entry = FileEntry::regular_file(metadata, vec!["chunk1".to_string()]);
        manifest1.add_file(PathBuf::from("test.txt"), entry);

        let snapshot_id = manifest1.snapshot_id.clone();
        index.add_manifest(manifest1);

        assert_eq!(index.snapshot_count(), 1);
        assert!(index.get_manifest(&snapshot_id).is_some());

        let found = index.find_file(&PathBuf::from("test.txt"));
        assert_eq!(found.len(), 1);
    }
}

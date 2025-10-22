//! Archive format for encrypted storage on removable media
//!
//! This module implements the archive structure used to store encrypted
//! data on removable media with integrity verification.

use crate::chunk::{Chunk, ChunkStore};
use crate::manifest::{Manifest, ManifestError};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Archive error types
#[derive(Debug, Error)]
pub enum ArchiveError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Manifest error
    #[error("Manifest error: {0}")]
    Manifest(#[from] ManifestError),

    /// Archive is corrupted or invalid
    #[error("Invalid archive: {0}")]
    InvalidArchive(String),

    /// Archive not found
    #[error("Archive not found at: {0}")]
    ArchiveNotFound(PathBuf),

    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// Snapshot not found
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),
}

/// Result type for archive operations
pub type Result<T> = std::result::Result<T, ArchiveError>;

/// Archive format version
pub const ARCHIVE_VERSION: u32 = 1;

/// Archive structure on removable media
///
/// Directory layout:
/// ```
/// /Volumes/USB001/
///   .airgapsync/
///     device-<device-id>/
///       snapshots/
///         <snapshot-id>/
///           manifest.json    # Snapshot manifest
///           chunks/          # Encrypted chunks
///             ab/            # First 2 chars of hash
///               ab123...     # Chunk file
///             cd/
///               cd456...
///       current -> snapshots/<latest-snapshot-id>  # Symlink to latest
///     index.json            # Index of all snapshots
///     version               # Archive format version
/// ```
pub struct Archive {
    /// Base path on removable media
    base_path: PathBuf,

    /// Device ID
    device_id: String,

    /// Chunk store
    chunk_store: Option<ChunkStore>,
}

impl Archive {
    /// Archive directory name
    const ARCHIVE_DIR: &'static str = ".airgapsync";

    /// Snapshots subdirectory
    const SNAPSHOTS_DIR: &'static str = "snapshots";

    /// Chunks subdirectory
    const CHUNKS_DIR: &'static str = "chunks";

    /// Manifest filename
    const MANIFEST_FILE: &'static str = "manifest.json";

    /// Index filename
    const INDEX_FILE: &'static str = "index.json";

    /// Version filename
    const VERSION_FILE: &'static str = "version";

    /// Current snapshot symlink
    const CURRENT_LINK: &'static str = "current";

    /// Create or open an archive on removable media
    pub fn open(base_path: PathBuf, device_id: String) -> Result<Self> {
        if !base_path.exists() {
            return Err(ArchiveError::ArchiveNotFound(base_path));
        }

        Ok(Self {
            base_path,
            device_id,
            chunk_store: None,
        })
    }

    /// Initialize a new archive on removable media
    pub fn initialize(base_path: PathBuf, device_id: String) -> Result<Self> {
        let archive_path = base_path.join(Self::ARCHIVE_DIR);
        let device_path = archive_path.join(format!("device-{}", device_id));

        // Create directory structure
        fs::create_dir_all(&device_path)?;
        fs::create_dir_all(device_path.join(Self::SNAPSHOTS_DIR))?;

        // Write version file
        let version_path = archive_path.join(Self::VERSION_FILE);
        fs::write(version_path, ARCHIVE_VERSION.to_string())?;

        // Create empty index
        let index_path = archive_path.join(Self::INDEX_FILE);
        let empty_index = ArchiveIndex::new(device_id.clone());
        let index_json = serde_json::to_string_pretty(&empty_index)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;
        fs::write(index_path, index_json)?;

        Ok(Self {
            base_path,
            device_id,
            chunk_store: None,
        })
    }

    /// Get the device directory path
    fn device_path(&self) -> PathBuf {
        self.base_path
            .join(Self::ARCHIVE_DIR)
            .join(format!("device-{}", self.device_id))
    }

    /// Get the snapshot directory path
    fn snapshot_path(&self, snapshot_id: &str) -> PathBuf {
        self.device_path()
            .join(Self::SNAPSHOTS_DIR)
            .join(snapshot_id)
    }

    /// Get the chunks directory for a snapshot
    fn chunks_path(&self, snapshot_id: &str) -> PathBuf {
        self.snapshot_path(snapshot_id).join(Self::CHUNKS_DIR)
    }

    /// Get the manifest path for a snapshot
    fn manifest_path(&self, snapshot_id: &str) -> PathBuf {
        self.snapshot_path(snapshot_id).join(Self::MANIFEST_FILE)
    }

    /// Get the index path
    fn index_path(&self) -> PathBuf {
        self.base_path
            .join(Self::ARCHIVE_DIR)
            .join(Self::INDEX_FILE)
    }

    /// Create a new snapshot in the archive
    pub fn create_snapshot(&mut self, manifest: &Manifest) -> Result<String> {
        let snapshot_id = manifest.snapshot_id.clone();
        let snapshot_path = self.snapshot_path(&snapshot_id);

        // Create snapshot directory
        fs::create_dir_all(&snapshot_path)?;

        // Create chunks directory
        let chunks_path = self.chunks_path(&snapshot_id);
        fs::create_dir_all(&chunks_path)?;

        // Initialize chunk store for this snapshot
        self.chunk_store = Some(
            ChunkStore::new(chunks_path)
                .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?,
        );

        // Save manifest
        let manifest_path = self.manifest_path(&snapshot_id);
        manifest.save_json(&manifest_path)?;

        // Update index
        self.update_index(&snapshot_id, manifest)?;

        // Update 'current' symlink
        self.update_current_link(&snapshot_id)?;

        Ok(snapshot_id)
    }

    /// Store a chunk in the current snapshot
    pub fn store_chunk(&mut self, chunk: &Chunk) -> Result<bool> {
        if let Some(ref mut store) = self.chunk_store {
            store
                .store(chunk)
                .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))
        } else {
            Err(ArchiveError::InvalidArchive(
                "No active snapshot".to_string(),
            ))
        }
    }

    /// Retrieve a chunk from a snapshot
    pub fn retrieve_chunk(&self, snapshot_id: &str, chunk_id: &str) -> Result<Chunk> {
        let chunks_path = self.chunks_path(snapshot_id);
        let store = ChunkStore::new(chunks_path)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;

        store
            .retrieve(chunk_id)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))
    }

    /// Load a manifest from a snapshot
    pub fn load_manifest(&self, snapshot_id: &str) -> Result<Manifest> {
        let manifest_path = self.manifest_path(snapshot_id);

        if !manifest_path.exists() {
            return Err(ArchiveError::SnapshotNotFound(snapshot_id.to_string()));
        }

        Ok(Manifest::load_json(&manifest_path)?)
    }

    /// List all snapshots for this device
    pub fn list_snapshots(&self) -> Result<Vec<String>> {
        let snapshots_path = self.device_path().join(Self::SNAPSHOTS_DIR);

        if !snapshots_path.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();

        for entry in fs::read_dir(snapshots_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    snapshots.push(name.to_string());
                }
            }
        }

        Ok(snapshots)
    }

    /// Get the latest snapshot ID
    pub fn latest_snapshot(&self) -> Result<Option<String>> {
        let current_link = self.device_path().join(Self::CURRENT_LINK);

        if !current_link.exists() {
            return Ok(None);
        }

        #[cfg(unix)]
        {
            let target = fs::read_link(current_link)?;
            if let Some(snapshot_id) = target.file_name() {
                return Ok(Some(snapshot_id.to_string_lossy().to_string()));
            }
        }

        Ok(None)
    }

    /// Update the index with a new snapshot
    fn update_index(&self, snapshot_id: &str, manifest: &Manifest) -> Result<()> {
        let index_path = self.index_path();

        let mut index = if index_path.exists() {
            let content = fs::read_to_string(&index_path)?;
            serde_json::from_str(&content)
                .unwrap_or_else(|_| ArchiveIndex::new(self.device_id.clone()))
        } else {
            ArchiveIndex::new(self.device_id.clone())
        };

        index.add_snapshot(snapshot_id.to_string(), manifest.created_at, manifest.file_count);

        let index_json = serde_json::to_string_pretty(&index)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;

        fs::write(index_path, index_json)?;

        Ok(())
    }

    /// Update the 'current' symlink to point to the latest snapshot
    #[cfg(unix)]
    fn update_current_link(&self, snapshot_id: &str) -> Result<()> {
        let current_link = self.device_path().join(Self::CURRENT_LINK);
        let target = PathBuf::from(Self::SNAPSHOTS_DIR).join(snapshot_id);

        // Remove old symlink if exists
        if current_link.exists() {
            fs::remove_file(&current_link)?;
        }

        // Create new symlink
        std::os::unix::fs::symlink(&target, &current_link)?;

        Ok(())
    }

    #[cfg(not(unix))]
    fn update_current_link(&self, _snapshot_id: &str) -> Result<()> {
        // Symlinks not supported on non-Unix systems
        Ok(())
    }

    /// Delete a snapshot from the archive
    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot_path = self.snapshot_path(snapshot_id);

        if !snapshot_path.exists() {
            return Err(ArchiveError::SnapshotNotFound(snapshot_id.to_string()));
        }

        fs::remove_dir_all(snapshot_path)?;

        Ok(())
    }

    /// Verify archive integrity
    pub fn verify(&self, snapshot_id: &str) -> Result<VerificationResult> {
        let manifest = self.load_manifest(snapshot_id)?;
        let chunks_path = self.chunks_path(snapshot_id);

        let mut result = VerificationResult::default();
        result.total_files = manifest.file_count;
        result.total_chunks = manifest.chunk_count;

        // Verify each file's chunks exist
        for (file_path, entry) in &manifest.files {
            let mut file_ok = true;

            for chunk_id in &entry.chunks {
                let chunk_path = chunks_path.join(&chunk_id[..2]).join(chunk_id);

                if chunk_path.exists() {
                    result.verified_chunks += 1;
                } else {
                    result.missing_chunks.push(chunk_id.clone());
                    file_ok = false;
                }
            }

            if file_ok {
                result.verified_files += 1;
            } else {
                result.corrupted_files.push(file_path.clone());
            }
        }

        result.is_valid = result.missing_chunks.is_empty();

        Ok(result)
    }

    /// Get archive statistics
    pub fn statistics(&self) -> Result<ArchiveStatistics> {
        let snapshots = self.list_snapshots()?;
        let mut stats = ArchiveStatistics::default();

        stats.snapshot_count = snapshots.len();

        for snapshot_id in snapshots {
            if let Ok(manifest) = self.load_manifest(&snapshot_id) {
                stats.total_files += manifest.file_count;
                stats.total_bytes += manifest.total_bytes;
                stats.compressed_bytes += manifest.compressed_bytes;
                stats.total_chunks += manifest.chunk_count;
            }
        }

        Ok(stats)
    }
}

/// Index of all snapshots in the archive
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ArchiveIndex {
    /// Device ID
    device_id: String,

    /// List of snapshots
    snapshots: Vec<SnapshotEntry>,
}

impl ArchiveIndex {
    fn new(device_id: String) -> Self {
        Self {
            device_id,
            snapshots: Vec::new(),
        }
    }

    fn add_snapshot(
        &mut self,
        snapshot_id: String,
        created_at: chrono::DateTime<chrono::Utc>,
        file_count: usize,
    ) {
        self.snapshots.push(SnapshotEntry {
            snapshot_id,
            created_at,
            file_count,
        });

        // Sort by creation time (newest first)
        self.snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    }
}

/// Entry in the archive index
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SnapshotEntry {
    snapshot_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    file_count: usize,
}

/// Result of archive verification
#[derive(Debug, Default)]
pub struct VerificationResult {
    /// Is the archive valid?
    pub is_valid: bool,

    /// Total number of files
    pub total_files: usize,

    /// Number of verified files
    pub verified_files: usize,

    /// Total number of chunks
    pub total_chunks: usize,

    /// Number of verified chunks
    pub verified_chunks: usize,

    /// List of missing chunk IDs
    pub missing_chunks: Vec<String>,

    /// List of corrupted file paths
    pub corrupted_files: Vec<PathBuf>,
}

/// Archive statistics
#[derive(Debug, Default)]
pub struct ArchiveStatistics {
    /// Number of snapshots
    pub snapshot_count: usize,

    /// Total files across all snapshots
    pub total_files: usize,

    /// Total bytes (original)
    pub total_bytes: u64,

    /// Total compressed bytes
    pub compressed_bytes: u64,

    /// Total chunks
    pub total_chunks: usize,
}

impl ArchiveStatistics {
    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.total_bytes > 0 {
            self.compressed_bytes as f64 / self.total_bytes as f64
        } else {
            1.0
        }
    }

    /// Get average snapshot size
    pub fn avg_snapshot_size(&self) -> u64 {
        if self.snapshot_count > 0 {
            self.total_bytes / self.snapshot_count as u64
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::FileMetadata;
    use crate::manifest::{FileEntry, FileType};
    use std::time::SystemTime;
    use tempfile::TempDir;

    #[test]
    fn test_archive_initialize() {
        let temp_dir = TempDir::new().unwrap();
        let archive = Archive::initialize(
            temp_dir.path().to_path_buf(),
            "test-device".to_string(),
        )
        .unwrap();

        let archive_path = temp_dir.path().join(".airgapsync");
        assert!(archive_path.exists());

        let device_path = archive_path.join("device-test-device");
        assert!(device_path.exists());

        let version_file = archive_path.join("version");
        assert!(version_file.exists());
    }

    #[test]
    fn test_create_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let mut archive = Archive::initialize(
            temp_dir.path().to_path_buf(),
            "test-device".to_string(),
        )
        .unwrap();

        let mut manifest = Manifest::new(
            "test-device".to_string(),
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

        let snapshot_id = archive.create_snapshot(&manifest).unwrap();

        // Verify snapshot was created
        let loaded = archive.load_manifest(&snapshot_id).unwrap();
        assert_eq!(loaded.snapshot_id, snapshot_id);
        assert_eq!(loaded.file_count, 1);
    }

    #[test]
    fn test_list_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let mut archive = Archive::initialize(
            temp_dir.path().to_path_buf(),
            "test-device".to_string(),
        )
        .unwrap();

        // Create multiple snapshots
        for i in 0..3 {
            let manifest = Manifest::new(
                "test-device".to_string(),
                PathBuf::from(format!("/source{}", i)),
            );
            archive.create_snapshot(&manifest).unwrap();
        }

        let snapshots = archive.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 3);
    }

    #[test]
    fn test_store_retrieve_chunk() {
        let temp_dir = TempDir::new().unwrap();
        let mut archive = Archive::initialize(
            temp_dir.path().to_path_buf(),
            "test-device".to_string(),
        )
        .unwrap();

        let manifest = Manifest::new(
            "test-device".to_string(),
            PathBuf::from("/source"),
        );

        let snapshot_id = archive.create_snapshot(&manifest).unwrap();

        // Store a chunk
        let chunk = Chunk::new(b"test chunk data".to_vec(), 0);
        let chunk_id = chunk.id().to_string();

        let stored = archive.store_chunk(&chunk).unwrap();
        assert!(stored); // Newly stored

        // Retrieve the chunk
        let retrieved = archive.retrieve_chunk(&snapshot_id, &chunk_id).unwrap();
        assert_eq!(retrieved.data, chunk.data);
    }

    #[test]
    fn test_archive_statistics() {
        let temp_dir = TempDir::new().unwrap();
        let mut archive = Archive::initialize(
            temp_dir.path().to_path_buf(),
            "test-device".to_string(),
        )
        .unwrap();

        let mut manifest = Manifest::new(
            "test-device".to_string(),
            PathBuf::from("/source"),
        );

        manifest.total_bytes = 1000;
        manifest.compressed_bytes = 800;
        manifest.file_count = 5;
        manifest.chunk_count = 10;

        archive.create_snapshot(&manifest).unwrap();

        let stats = archive.statistics().unwrap();
        assert_eq!(stats.snapshot_count, 1);
        assert_eq!(stats.total_files, 5);
        assert_eq!(stats.total_bytes, 1000);
        assert_eq!(stats.compressed_bytes, 800);
    }
}

//! Change detection for file synchronization
//!
//! This module implements efficient file comparison and change detection
//! to identify which files need to be synced.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use walkdir::WalkDir;

/// Diff error types
#[derive(Debug, Error)]
pub enum DiffError {
    /// I/O error during file operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to compute file hash
    #[error("Failed to compute hash: {0}")]
    HashError(String),

    /// Path does not exist
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),
}

/// Result type for diff operations
pub type Result<T> = std::result::Result<T, DiffError>;

/// File metadata for change detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    /// File path relative to source root
    pub path: PathBuf,

    /// File size in bytes
    pub size: u64,

    /// Last modified time
    pub modified: SystemTime,

    /// File permissions (Unix mode)
    #[cfg(unix)]
    pub permissions: u32,

    /// Content hash (optional, computed on demand)
    pub hash: Option<String>,

    /// Is this a directory?
    pub is_dir: bool,
}

impl FileMetadata {
    /// Create metadata from a file path
    pub fn from_path(path: &Path, base_path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;
        let relative_path = path.strip_prefix(base_path)
            .unwrap_or(path)
            .to_path_buf();

        Ok(Self {
            path: relative_path,
            size: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(unix)]
            permissions: {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode()
            },
            hash: None,
            is_dir: metadata.is_dir(),
        })
    }

    /// Compute content hash for this file
    pub fn compute_hash(&mut self, full_path: &Path) -> Result<()> {
        if self.is_dir {
            return Ok(());
        }

        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        let data = fs::read(full_path)?;
        hasher.update(&data);
        self.hash = Some(format!("{:x}", hasher.finalize()));
        Ok(())
    }

    /// Quick check if files might be different (without hash)
    pub fn quick_diff(&self, other: &FileMetadata) -> bool {
        self.size != other.size ||
        self.modified != other.modified ||
        self.is_dir != other.is_dir
    }
}

/// Type of change detected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    /// File was added (new file)
    Added,

    /// File was modified
    Modified,

    /// File was deleted
    Deleted,

    /// File is unchanged
    Unchanged,
}

/// A detected change between source and destination
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Path of the changed file
    pub path: PathBuf,

    /// Type of change
    pub change_type: ChangeType,

    /// Source metadata (None for deleted files)
    pub source_meta: Option<FileMetadata>,

    /// Destination metadata (None for new files)
    pub dest_meta: Option<FileMetadata>,
}

impl FileChange {
    /// Create a new file change
    pub fn new(
        path: PathBuf,
        change_type: ChangeType,
        source_meta: Option<FileMetadata>,
        dest_meta: Option<FileMetadata>,
    ) -> Self {
        Self {
            path,
            change_type,
            source_meta,
            dest_meta,
        }
    }

    /// Get the file size (from source or dest)
    pub fn size(&self) -> u64 {
        self.source_meta.as_ref()
            .or(self.dest_meta.as_ref())
            .map(|m| m.size)
            .unwrap_or(0)
    }
}

/// Options for diff operation
#[derive(Debug, Clone)]
pub struct DiffOptions {
    /// Use content hashing for comparison (slower but more accurate)
    pub use_content_hash: bool,

    /// Exclude patterns (gitignore syntax)
    pub exclude_patterns: Vec<String>,

    /// Follow symbolic links
    pub follow_symlinks: bool,

    /// Include hidden files
    pub include_hidden: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            use_content_hash: false,
            exclude_patterns: Vec::new(),
            follow_symlinks: false,
            include_hidden: false,
        }
    }
}

/// Diff engine for comparing source and destination
pub struct DiffEngine {
    options: DiffOptions,
}

impl DiffEngine {
    /// Create a new diff engine with options
    pub fn new(options: DiffOptions) -> Self {
        Self { options }
    }

    /// Create with default options
    pub fn default() -> Self {
        Self::new(DiffOptions::default())
    }

    /// Scan directory and collect file metadata
    pub fn scan_directory(&self, path: &Path) -> Result<HashMap<PathBuf, FileMetadata>> {
        if !path.exists() {
            return Err(DiffError::PathNotFound(path.to_path_buf()));
        }

        let mut files = HashMap::new();
        let walker = WalkDir::new(path)
            .follow_links(self.options.follow_symlinks)
            .into_iter()
            .filter_entry(|e| self.should_include(e));

        for entry in walker {
            let entry = entry?;
            let file_path = entry.path();

            // Skip the root directory itself
            if file_path == path {
                continue;
            }

            let mut metadata = FileMetadata::from_path(file_path, path)?;

            // Compute hash if requested and it's a file
            if self.options.use_content_hash && !metadata.is_dir {
                metadata.compute_hash(file_path)?;
            }

            files.insert(metadata.path.clone(), metadata);
        }

        Ok(files)
    }

    /// Check if an entry should be included based on filters
    fn should_include(&self, entry: &walkdir::DirEntry) -> bool {
        let file_name = entry.file_name().to_string_lossy();

        // Filter hidden files
        if !self.options.include_hidden && file_name.starts_with('.') {
            return false;
        }

        // TODO: Implement gitignore-style pattern matching
        // For now, simple exclusion patterns
        for pattern in &self.options.exclude_patterns {
            if file_name.contains(pattern) {
                return false;
            }
        }

        true
    }

    /// Compare source and destination directories
    pub fn diff(
        &self,
        source_path: &Path,
        dest_manifest: &HashMap<PathBuf, FileMetadata>,
    ) -> Result<Vec<FileChange>> {
        let source_files = self.scan_directory(source_path)?;
        let mut changes = Vec::new();

        let source_keys: HashSet<_> = source_files.keys().cloned().collect();
        let dest_keys: HashSet<_> = dest_manifest.keys().cloned().collect();

        // Find new and modified files
        for (path, source_meta) in &source_files {
            if let Some(dest_meta) = dest_manifest.get(path) {
                // File exists in both - check if modified
                let is_modified = if self.options.use_content_hash {
                    // Compare hashes if available
                    source_meta.hash != dest_meta.hash
                } else {
                    // Quick metadata comparison
                    source_meta.quick_diff(dest_meta)
                };

                let change_type = if is_modified {
                    ChangeType::Modified
                } else {
                    ChangeType::Unchanged
                };

                changes.push(FileChange::new(
                    path.clone(),
                    change_type,
                    Some(source_meta.clone()),
                    Some(dest_meta.clone()),
                ));
            } else {
                // File is new
                changes.push(FileChange::new(
                    path.clone(),
                    ChangeType::Added,
                    Some(source_meta.clone()),
                    None,
                ));
            }
        }

        // Find deleted files
        for path in dest_keys.difference(&source_keys) {
            if let Some(dest_meta) = dest_manifest.get(path) {
                changes.push(FileChange::new(
                    path.clone(),
                    ChangeType::Deleted,
                    None,
                    Some(dest_meta.clone()),
                ));
            }
        }

        Ok(changes)
    }

    /// Get statistics about changes
    pub fn summarize_changes(changes: &[FileChange]) -> ChangeSummary {
        let mut summary = ChangeSummary::default();

        for change in changes {
            match change.change_type {
                ChangeType::Added => {
                    summary.added += 1;
                    summary.added_bytes += change.size();
                }
                ChangeType::Modified => {
                    summary.modified += 1;
                    summary.modified_bytes += change.size();
                }
                ChangeType::Deleted => {
                    summary.deleted += 1;
                    summary.deleted_bytes += change.size();
                }
                ChangeType::Unchanged => {
                    summary.unchanged += 1;
                    summary.unchanged_bytes += change.size();
                }
            }
        }

        summary
    }
}

/// Summary of detected changes
#[derive(Debug, Default, Clone)]
pub struct ChangeSummary {
    /// Number of added files
    pub added: usize,

    /// Bytes in added files
    pub added_bytes: u64,

    /// Number of modified files
    pub modified: usize,

    /// Bytes in modified files
    pub modified_bytes: u64,

    /// Number of deleted files
    pub deleted: usize,

    /// Bytes in deleted files
    pub deleted_bytes: u64,

    /// Number of unchanged files
    pub unchanged: usize,

    /// Bytes in unchanged files
    pub unchanged_bytes: u64,
}

impl ChangeSummary {
    /// Total number of files that need to be synced
    pub fn files_to_sync(&self) -> usize {
        self.added + self.modified
    }

    /// Total bytes that need to be transferred
    pub fn bytes_to_sync(&self) -> u64 {
        self.added_bytes + self.modified_bytes
    }

    /// Check if any changes detected
    pub fn has_changes(&self) -> bool {
        self.added > 0 || self.modified > 0 || self.deleted > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_metadata_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test content").unwrap();

        let metadata = FileMetadata::from_path(&file_path, temp_dir.path()).unwrap();
        assert_eq!(metadata.path, PathBuf::from("test.txt"));
        assert_eq!(metadata.size, 12);
        assert!(!metadata.is_dir);
    }

    #[test]
    fn test_compute_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test content").unwrap();

        let mut metadata = FileMetadata::from_path(&file_path, temp_dir.path()).unwrap();
        metadata.compute_hash(&file_path).unwrap();
        assert!(metadata.hash.is_some());

        // Hash should be consistent
        let hash1 = metadata.hash.clone();
        metadata.compute_hash(&file_path).unwrap();
        assert_eq!(hash1, metadata.hash);
    }

    #[test]
    fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), b"content1").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), b"content2").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir/file3.txt"), b"content3").unwrap();

        let engine = DiffEngine::default();
        let files = engine.scan_directory(temp_dir.path()).unwrap();

        // Should find 2 files + 1 directory + 1 file in subdir = 4 entries
        assert_eq!(files.len(), 4);
        assert!(files.contains_key(&PathBuf::from("file1.txt")));
        assert!(files.contains_key(&PathBuf::from("file2.txt")));
        assert!(files.contains_key(&PathBuf::from("subdir")));
        assert!(files.contains_key(&PathBuf::from("subdir/file3.txt")));
    }

    #[test]
    fn test_diff_new_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("new.txt"), b"new content").unwrap();

        let engine = DiffEngine::default();
        let dest_manifest = HashMap::new(); // Empty destination
        let changes = engine.diff(temp_dir.path(), &dest_manifest).unwrap();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Added);
    }

    #[test]
    fn test_diff_modified_files() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("modified.txt");
        fs::write(&file_path, b"original").unwrap();

        let engine = DiffEngine::new(DiffOptions {
            use_content_hash: false,
            ..Default::default()
        });

        // Create destination manifest with old version
        let mut dest_manifest = HashMap::new();
        let mut old_meta = FileMetadata::from_path(&file_path, temp_dir.path()).unwrap();

        // Wait a bit and modify file
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&file_path, b"modified content").unwrap();

        dest_manifest.insert(old_meta.path.clone(), old_meta);

        let changes = engine.diff(temp_dir.path(), &dest_manifest).unwrap();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn test_change_summary() {
        let changes = vec![
            FileChange::new(
                PathBuf::from("new.txt"),
                ChangeType::Added,
                Some(FileMetadata {
                    path: PathBuf::from("new.txt"),
                    size: 100,
                    modified: SystemTime::now(),
                    #[cfg(unix)]
                    permissions: 0o644,
                    hash: None,
                    is_dir: false,
                }),
                None,
            ),
            FileChange::new(
                PathBuf::from("modified.txt"),
                ChangeType::Modified,
                Some(FileMetadata {
                    path: PathBuf::from("modified.txt"),
                    size: 200,
                    modified: SystemTime::now(),
                    #[cfg(unix)]
                    permissions: 0o644,
                    hash: None,
                    is_dir: false,
                }),
                None,
            ),
        ];

        let summary = DiffEngine::summarize_changes(&changes);
        assert_eq!(summary.added, 1);
        assert_eq!(summary.modified, 1);
        assert_eq!(summary.files_to_sync(), 2);
        assert_eq!(summary.bytes_to_sync(), 300);
        assert!(summary.has_changes());
    }
}

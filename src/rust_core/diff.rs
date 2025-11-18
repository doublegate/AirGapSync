//! File difference detection
//!
//! This module provides functionality for detecting changes between source
//! and destination snapshots for efficient incremental synchronization.

use crate::metadata::{FileMetadata, FileType, Manifest};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Diff error types
#[derive(Debug, Error)]
pub enum DiffError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid manifest
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
}

/// Result type for diff operations
pub type Result<T> = std::result::Result<T, DiffError>;

/// Type of change detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    /// File was added
    Added,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was moved/renamed
    Moved,
    /// Metadata changed (permissions, etc.)
    MetadataChanged,
}

/// Represents a detected change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Type of change
    pub change_type: ChangeType,

    /// File path (relative)
    pub path: PathBuf,

    /// Old path (for moves)
    pub old_path: Option<PathBuf>,

    /// File size
    pub size: u64,

    /// Old metadata (if available)
    pub old_metadata: Option<FileMetadata>,

    /// New metadata
    pub new_metadata: Option<FileMetadata>,
}

impl Change {
    /// Create a new change
    pub fn new(change_type: ChangeType, path: PathBuf, size: u64) -> Self {
        Self {
            change_type,
            path,
            old_path: None,
            size,
            old_metadata: None,
            new_metadata: None,
        }
    }

    /// Create an added file change
    pub fn added(metadata: FileMetadata) -> Self {
        Self {
            change_type: ChangeType::Added,
            path: metadata.path.clone(),
            old_path: None,
            size: metadata.size,
            old_metadata: None,
            new_metadata: Some(metadata),
        }
    }

    /// Create a modified file change
    pub fn modified(old: FileMetadata, new: FileMetadata) -> Self {
        Self {
            change_type: ChangeType::Modified,
            path: new.path.clone(),
            old_path: None,
            size: new.size,
            old_metadata: Some(old),
            new_metadata: Some(new),
        }
    }

    /// Create a deleted file change
    pub fn deleted(metadata: FileMetadata) -> Self {
        Self {
            change_type: ChangeType::Deleted,
            path: metadata.path.clone(),
            old_path: None,
            size: metadata.size,
            old_metadata: Some(metadata),
            new_metadata: None,
        }
    }

    /// Create a moved file change
    pub fn moved(old: FileMetadata, new: FileMetadata) -> Self {
        Self {
            change_type: ChangeType::Moved,
            path: new.path.clone(),
            old_path: Some(old.path.clone()),
            size: new.size,
            old_metadata: Some(old),
            new_metadata: Some(new),
        }
    }

    /// Create a metadata-only change
    pub fn metadata_changed(old: FileMetadata, new: FileMetadata) -> Self {
        Self {
            change_type: ChangeType::MetadataChanged,
            path: new.path.clone(),
            old_path: None,
            size: new.size,
            old_metadata: Some(old),
            new_metadata: Some(new),
        }
    }
}

/// Changeset representing all detected changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSet {
    /// All detected changes
    pub changes: Vec<Change>,

    /// Statistics
    pub stats: ChangeStats,
}

/// Change statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeStats {
    /// Number of added files
    pub added: usize,

    /// Number of modified files
    pub modified: usize,

    /// Number of deleted files
    pub deleted: usize,

    /// Number of moved files
    pub moved: usize,

    /// Number of metadata-only changes
    pub metadata_changed: usize,

    /// Total bytes to transfer
    pub total_bytes: u64,
}

impl ChangeSet {
    /// Create a new empty changeset
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            stats: ChangeStats::default(),
        }
    }

    /// Add a change to the set
    pub fn add_change(&mut self, change: Change) {
        match change.change_type {
            ChangeType::Added => {
                self.stats.added += 1;
                self.stats.total_bytes += change.size;
            }
            ChangeType::Modified => {
                self.stats.modified += 1;
                self.stats.total_bytes += change.size;
            }
            ChangeType::Deleted => {
                self.stats.deleted += 1;
            }
            ChangeType::Moved => {
                self.stats.moved += 1;
            }
            ChangeType::MetadataChanged => {
                self.stats.metadata_changed += 1;
            }
        }
        self.changes.push(change);
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Get total number of changes
    pub fn total_changes(&self) -> usize {
        self.changes.len()
    }
}

impl Default for ChangeSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Diff engine for detecting changes
pub struct DiffEngine {
    /// Enable move detection
    detect_moves: bool,

    /// Minimum file size for move detection (bytes)
    move_detection_threshold: u64,
}

impl DiffEngine {
    /// Create a new diff engine
    pub fn new() -> Self {
        Self {
            detect_moves: true,
            move_detection_threshold: 1024, // 1KB
        }
    }

    /// Disable move detection
    pub fn without_move_detection(mut self) -> Self {
        self.detect_moves = false;
        self
    }

    /// Compare two manifests and produce a changeset
    pub fn diff_manifests(&self, old: &Manifest, new: &Manifest) -> Result<ChangeSet> {
        let mut changeset = ChangeSet::new();

        // Build lookup maps
        let old_files: HashMap<&Path, &FileMetadata> = old
            .files
            .iter()
            .map(|f| (f.path.as_path(), f))
            .collect();

        let new_files: HashMap<&Path, &FileMetadata> = new
            .files
            .iter()
            .map(|f| (f.path.as_path(), f))
            .collect();

        let old_paths: HashSet<&Path> = old_files.keys().copied().collect();
        let new_paths: HashSet<&Path> = new_files.keys().copied().collect();

        // Detect additions and modifications
        for path in &new_paths {
            let new_meta = new_files[path];

            if let Some(old_meta) = old_files.get(path) {
                // File exists in both - check for changes
                if new_meta.needs_sync(old_meta) {
                    changeset.add_change(Change::modified((*old_meta).clone(), new_meta.clone()));
                } else if new_meta.has_changed(old_meta) {
                    changeset.add_change(Change::metadata_changed(
                        (*old_meta).clone(),
                        new_meta.clone(),
                    ));
                }
            } else {
                // New file
                changeset.add_change(Change::added(new_meta.clone()));
            }
        }

        // Detect deletions
        for path in old_paths.difference(&new_paths) {
            let old_meta = old_files[path];
            changeset.add_change(Change::deleted(old_meta.clone()));
        }

        // Detect moves (if enabled)
        if self.detect_moves {
            self.detect_moves_in_changeset(&mut changeset);
        }

        Ok(changeset)
    }

    /// Scan a directory and create a manifest
    pub fn scan_directory<P: AsRef<Path>>(
        &self,
        source_path: P,
        device_id: String,
    ) -> Result<Manifest> {
        let source_path = source_path.as_ref();
        let mut manifest = Manifest::new(source_path.to_path_buf(), device_id);

        // Use walkdir to traverse directory
        for entry in walkdir::WalkDir::new(source_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip the source directory itself
            if path == source_path {
                continue;
            }

            // Create metadata
            if let Ok(metadata) = FileMetadata::from_path(path, source_path) {
                // Skip directories for now (we only sync files)
                if metadata.file_type == FileType::File {
                    manifest.add_file(metadata);
                }
            }
        }

        Ok(manifest)
    }

    /// Detect moved files in a changeset
    fn detect_moves_in_changeset(&self, changeset: &mut ChangeSet) {
        // Build hash map of deleted files by hash
        let mut deleted_by_hash: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, change) in changeset.changes.iter().enumerate() {
            if change.change_type == ChangeType::Deleted {
                if let Some(ref old_meta) = change.old_metadata {
                    if old_meta.size >= self.move_detection_threshold {
                        if let Some(ref hash) = old_meta.hash {
                            deleted_by_hash
                                .entry(hash.clone())
                                .or_default()
                                .push(idx);
                        }
                    }
                }
            }
        }

        // Check added files for potential moves
        let mut moves_to_create: Vec<(usize, usize)> = Vec::new();

        for (add_idx, change) in changeset.changes.iter().enumerate() {
            if change.change_type == ChangeType::Added {
                if let Some(ref new_meta) = change.new_metadata {
                    if new_meta.size >= self.move_detection_threshold {
                        if let Some(ref hash) = new_meta.hash {
                            if let Some(del_indices) = deleted_by_hash.get(hash) {
                                if let Some(&del_idx) = del_indices.first() {
                                    moves_to_create.push((del_idx, add_idx));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convert detected moves (mark indices for later removal)
        let mut to_remove: HashSet<usize> = HashSet::new();

        for (del_idx, add_idx) in moves_to_create {
            let old_meta = changeset.changes[del_idx]
                .old_metadata
                .as_ref()
                .unwrap()
                .clone();
            let new_meta = changeset.changes[add_idx]
                .new_metadata
                .as_ref()
                .unwrap()
                .clone();

            // Add move change
            changeset.changes.push(Change::moved(old_meta, new_meta));

            // Mark original changes for removal
            to_remove.insert(del_idx);
            to_remove.insert(add_idx);
        }

        // Remove converted changes
        if !to_remove.is_empty() {
            changeset.changes = changeset
                .changes
                .iter()
                .enumerate()
                .filter(|(idx, _)| !to_remove.contains(idx))
                .map(|(_, c)| c.clone())
                .collect();

            // Recalculate stats
            changeset.stats = ChangeStats::default();
            for change in &changeset.changes {
                match change.change_type {
                    ChangeType::Added => {
                        changeset.stats.added += 1;
                        changeset.stats.total_bytes += change.size;
                    }
                    ChangeType::Modified => {
                        changeset.stats.modified += 1;
                        changeset.stats.total_bytes += change.size;
                    }
                    ChangeType::Deleted => {
                        changeset.stats.deleted += 1;
                    }
                    ChangeType::Moved => {
                        changeset.stats.moved += 1;
                    }
                    ChangeType::MetadataChanged => {
                        changeset.stats.metadata_changed += 1;
                    }
                }
            }
        }
    }
}

impl Default for DiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata(path: &str, size: u64, hash: Option<&str>) -> FileMetadata {
        FileMetadata {
            path: PathBuf::from(path),
            size,
            file_type: FileType::File,
            modified: 1234567890,
            created: 1234567890,
            permissions: 0o644,
            hash: hash.map(|s| s.to_string()),
            is_executable: false,
            is_hidden: false,
        }
    }

    #[test]
    fn test_diff_engine_creation() {
        let engine = DiffEngine::new();
        assert!(engine.detect_moves);
    }

    #[test]
    fn test_detect_additions() {
        let engine = DiffEngine::new();

        let mut old_manifest = Manifest::new(PathBuf::from("/src"), "dev1".to_string());
        let mut new_manifest = Manifest::new(PathBuf::from("/src"), "dev1".to_string());

        new_manifest.add_file(create_test_metadata("file1.txt", 100, Some("abc123")));

        let changeset = engine.diff_manifests(&old_manifest, &new_manifest).unwrap();

        assert_eq!(changeset.stats.added, 1);
        assert_eq!(changeset.total_changes(), 1);
    }

    #[test]
    fn test_detect_modifications() {
        let engine = DiffEngine::new();

        let mut old_manifest = Manifest::new(PathBuf::from("/src"), "dev1".to_string());
        old_manifest.add_file(create_test_metadata("file1.txt", 100, Some("abc123")));

        let mut new_manifest = Manifest::new(PathBuf::from("/src"), "dev1".to_string());
        new_manifest.add_file(create_test_metadata("file1.txt", 100, Some("def456")));

        let changeset = engine.diff_manifests(&old_manifest, &new_manifest).unwrap();

        assert_eq!(changeset.stats.modified, 1);
    }

    #[test]
    fn test_detect_deletions() {
        let engine = DiffEngine::new();

        let mut old_manifest = Manifest::new(PathBuf::from("/src"), "dev1".to_string());
        old_manifest.add_file(create_test_metadata("file1.txt", 100, Some("abc123")));

        let new_manifest = Manifest::new(PathBuf::from("/src"), "dev1".to_string());

        let changeset = engine.diff_manifests(&old_manifest, &new_manifest).unwrap();

        assert_eq!(changeset.stats.deleted, 1);
    }

    #[test]
    fn test_changeset_operations() {
        let mut changeset = ChangeSet::new();
        let meta = create_test_metadata("file1.txt", 100, Some("abc123"));

        changeset.add_change(Change::added(meta));

        assert_eq!(changeset.total_changes(), 1);
        assert!(changeset.has_changes());
        assert_eq!(changeset.stats.total_bytes, 100);
    }
}

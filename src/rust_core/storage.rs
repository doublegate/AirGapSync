//! Storage backend for encrypted chunks
//!
//! This module manages the physical storage of encrypted, compressed chunks
//! on removable media with deduplication and integrity verification.

use crate::chunk::Chunker;
use crate::crypto::{self, CryptoKey};
use crate::metadata::{FileMetadata, Manifest};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Storage error types
#[derive(Debug, Error)]
pub enum StorageError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Chunk not found
    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),

    /// Invalid chunk
    #[error("Invalid chunk: {0}")]
    InvalidChunk(String),

    /// Encryption error
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// Manifest error
    #[error("Manifest error: {0}")]
    Manifest(String),
}

/// Result type for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;

/// Storage backend configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Base path for storage
    pub base_path: PathBuf,

    /// Chunk size
    pub chunk_size: usize,

    /// Enable deduplication
    pub enable_dedup: bool,

    /// Enable compression
    pub enable_compression: bool,
}

impl StorageConfig {
    /// Create default configuration
    pub fn new<P: Into<PathBuf>>(base_path: P) -> Self {
        Self {
            base_path: base_path.into(),
            chunk_size: 1024 * 1024, // 1MB
            enable_dedup: true,
            enable_compression: true,
        }
    }
}

/// Chunk storage layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMap {
    /// Chunk hash to file path mapping
    pub chunks: HashMap<String, ChunkLocation>,

    /// Total number of chunks
    pub total_chunks: usize,

    /// Total storage size (bytes)
    pub total_size: u64,

    /// Number of deduplicated chunks
    pub dedup_count: usize,
}

/// Location of a stored chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLocation {
    /// Relative path to chunk file
    pub path: PathBuf,

    /// Original size
    pub size: usize,

    /// Compressed size
    pub compressed_size: usize,

    /// Reference count (for deduplication)
    pub ref_count: usize,
}

impl ChunkMap {
    /// Create a new chunk map
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            total_chunks: 0,
            total_size: 0,
            dedup_count: 0,
        }
    }

    /// Add a chunk
    pub fn add_chunk(&mut self, hash: String, location: ChunkLocation) {
        self.total_chunks += 1;
        self.total_size += location.compressed_size as u64;

        if let Some(existing) = self.chunks.get_mut(&hash) {
            existing.ref_count += 1;
            self.dedup_count += 1;
        } else {
            self.chunks.insert(hash, location);
        }
    }

    /// Get chunk location
    pub fn get_chunk(&self, hash: &str) -> Option<&ChunkLocation> {
        self.chunks.get(hash)
    }

    /// Remove a chunk reference
    pub fn remove_chunk_ref(&mut self, hash: &str) -> Option<ChunkLocation> {
        if let Some(location) = self.chunks.get_mut(hash) {
            if location.ref_count > 0 {
                location.ref_count -= 1;
                if location.ref_count == 0 {
                    return self.chunks.remove(hash);
                }
            }
        }
        None
    }
}

impl Default for ChunkMap {
    fn default() -> Self {
        Self::new()
    }
}

/// File entry in storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFile {
    /// File metadata
    pub metadata: FileMetadata,

    /// List of chunk hashes
    pub chunks: Vec<String>,

    /// Total size
    pub total_size: u64,
}

/// Storage backend
pub struct StorageBackend {
    config: StorageConfig,
    chunk_map: ChunkMap,
    chunker: Chunker,
}

impl StorageBackend {
    /// Create a new storage backend
    pub fn new(config: StorageConfig) -> Result<Self> {
        // Create base directories
        let chunks_dir = config.base_path.join("chunks");
        let manifests_dir = config.base_path.join("manifests");
        let snapshots_dir = config.base_path.join("snapshots");

        fs::create_dir_all(&chunks_dir)?;
        fs::create_dir_all(&manifests_dir)?;
        fs::create_dir_all(&snapshots_dir)?;

        let chunker = Chunker::with_chunk_size(config.chunk_size);

        // Try to load existing chunk map
        let chunk_map_path = config.base_path.join("chunk_map.json");
        let chunk_map = if chunk_map_path.exists() {
            let data = fs::read_to_string(&chunk_map_path)?;
            serde_json::from_str(&data)
                .map_err(|e| StorageError::Manifest(e.to_string()))?
        } else {
            ChunkMap::new()
        };

        Ok(Self {
            config,
            chunk_map,
            chunker,
        })
    }

    /// Store a file
    pub fn store_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        metadata: FileMetadata,
        crypto_key: &CryptoKey,
    ) -> Result<StoredFile> {
        let file_path = file_path.as_ref();

        // Chunk the file
        let chunk_infos = self.chunker.chunk_file(file_path)
            .map_err(|e| StorageError::InvalidChunk(e.to_string()))?;

        let mut chunk_hashes = Vec::new();
        let mut total_size = 0u64;

        for chunk_info in &chunk_infos {
            // Read the chunk
            let chunk_data = self.chunker.read_chunk(file_path, chunk_info)
                .map_err(|e| StorageError::InvalidChunk(e.to_string()))?;

            // Compress if enabled
            let data_to_encrypt = if self.config.enable_compression {
                self.chunker.compress(&chunk_data)
                    .map_err(|e| StorageError::InvalidChunk(e.to_string()))?
            } else {
                chunk_data
            };

            // Encrypt the chunk
            let encrypted = crypto::encrypt(crypto_key, &data_to_encrypt, b"chunk")
                .map_err(|e| StorageError::Encryption(e.to_string()))?;

            // Calculate storage hash
            let storage_hash = Self::hash_data(&encrypted);

            // Check for deduplication
            if !self.config.enable_dedup || !self.chunk_map.chunks.contains_key(&storage_hash) {
                // Store the encrypted chunk
                let chunk_path = self.get_chunk_path(&storage_hash);
                fs::write(&chunk_path, &encrypted)?;

                let location = ChunkLocation {
                    path: chunk_path.strip_prefix(&self.config.base_path)
                        .unwrap()
                        .to_path_buf(),
                    size: chunk_info.original_size,
                    compressed_size: encrypted.len(),
                    ref_count: 1,
                };

                self.chunk_map.add_chunk(storage_hash.clone(), location);
            } else {
                // Increment reference count
                self.chunk_map.add_chunk(storage_hash.clone(),
                    self.chunk_map.get_chunk(&storage_hash).unwrap().clone());
            }

            chunk_hashes.push(storage_hash);
            total_size += chunk_info.original_size as u64;
        }

        // Save chunk map
        self.save_chunk_map()?;

        Ok(StoredFile {
            metadata,
            chunks: chunk_hashes,
            total_size,
        })
    }

    /// Retrieve a file
    pub fn retrieve_file<P: AsRef<Path>>(
        &self,
        stored_file: &StoredFile,
        output_path: P,
        crypto_key: &CryptoKey,
    ) -> Result<()> {
        let output_path = output_path.as_ref();

        // Create output file
        let mut output = fs::File::create(output_path)?;

        // Process each chunk
        for chunk_hash in &stored_file.chunks {
            // Find chunk location
            let location = self.chunk_map.get_chunk(chunk_hash)
                .ok_or_else(|| StorageError::ChunkNotFound(chunk_hash.clone()))?;

            // Read encrypted chunk
            let chunk_path = self.config.base_path.join(&location.path);
            let encrypted = fs::read(&chunk_path)?;

            // Decrypt
            let decrypted = crypto::decrypt(crypto_key, &encrypted, b"chunk")
                .map_err(|e| StorageError::Encryption(e.to_string()))?;

            // Decompress if needed
            let data = if self.config.enable_compression {
                self.chunker.decompress(&decrypted)
                    .map_err(|e| StorageError::InvalidChunk(e.to_string()))?
            } else {
                decrypted
            };

            // Write to output
            output.write_all(&data)?;
        }

        // Restore metadata (permissions, timestamps)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(stored_file.metadata.permissions);
            fs::set_permissions(output_path, perms)?;
        }

        Ok(())
    }

    /// Save manifest
    pub fn save_manifest(&self, manifest: &Manifest, snapshot_id: &str) -> Result<()> {
        let manifest_path = self.config.base_path
            .join("manifests")
            .join(format!("{}.json", snapshot_id));

        manifest.save(&manifest_path)
            .map_err(|e| StorageError::Manifest(e.to_string()))?;

        Ok(())
    }

    /// Load manifest
    pub fn load_manifest(&self, snapshot_id: &str) -> Result<Manifest> {
        let manifest_path = self.config.base_path
            .join("manifests")
            .join(format!("{}.json", snapshot_id));

        Manifest::load(&manifest_path)
            .map_err(|e| StorageError::Manifest(e.to_string()))
    }

    /// List available snapshots
    pub fn list_snapshots(&self) -> Result<Vec<String>> {
        let manifests_dir = self.config.base_path.join("manifests");
        let mut snapshots = Vec::new();

        for entry in fs::read_dir(&manifests_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    snapshots.push(name.trim_end_matches(".json").to_string());
                }
            }
        }

        snapshots.sort();
        Ok(snapshots)
    }

    /// Get chunk path from hash
    fn get_chunk_path(&self, hash: &str) -> PathBuf {
        // Use first 2 characters for subdirectory (like git)
        let subdir = &hash[..2];
        let filename = &hash[2..];

        let chunk_dir = self.config.base_path
            .join("chunks")
            .join(subdir);

        fs::create_dir_all(&chunk_dir).ok();

        chunk_dir.join(filename)
    }

    /// Save chunk map
    fn save_chunk_map(&self) -> Result<()> {
        let chunk_map_path = self.config.base_path.join("chunk_map.json");
        let json = serde_json::to_string_pretty(&self.chunk_map)
            .map_err(|e| StorageError::Manifest(e.to_string()))?;
        fs::write(chunk_map_path, json)?;
        Ok(())
    }

    /// Calculate SHA-256 hash
    fn hash_data(data: &[u8]) -> String {
        let mut context = Context::new(&SHA256);
        context.update(data);
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        StorageStats {
            total_chunks: self.chunk_map.total_chunks,
            unique_chunks: self.chunk_map.chunks.len(),
            total_size: self.chunk_map.total_size,
            dedup_count: self.chunk_map.dedup_count,
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of chunk references
    pub total_chunks: usize,

    /// Number of unique chunks
    pub unique_chunks: usize,

    /// Total storage size
    pub total_size: u64,

    /// Number of deduplicated chunks
    pub dedup_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_config() {
        let config = StorageConfig::new("/tmp/storage");
        assert_eq!(config.chunk_size, 1024 * 1024);
        assert!(config.enable_dedup);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_chunk_map() {
        let mut map = ChunkMap::new();

        let location = ChunkLocation {
            path: PathBuf::from("chunks/ab/cdef"),
            size: 1024,
            compressed_size: 512,
            ref_count: 1,
        };

        map.add_chunk("abcdef".to_string(), location);

        assert_eq!(map.total_chunks, 1);
        assert_eq!(map.chunks.len(), 1);
        assert!(map.get_chunk("abcdef").is_some());
    }

    #[test]
    fn test_storage_backend_creation() {
        let dir = tempdir().unwrap();
        let config = StorageConfig::new(dir.path());

        let backend = StorageBackend::new(config);
        assert!(backend.is_ok());

        // Check that directories were created
        assert!(dir.path().join("chunks").exists());
        assert!(dir.path().join("manifests").exists());
        assert!(dir.path().join("snapshots").exists());
    }
}

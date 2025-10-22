//! Chunk-based file processing with deduplication and compression
//!
//! This module implements chunking for large files, content-addressed
//! storage, and optional compression.

use crate::crypto::{Algorithm, CryptoError, CryptoKey};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Chunk error types
#[derive(Debug, Error)]
pub enum ChunkError {
    /// I/O error during chunk operations
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Encryption error
    #[error("Encryption error: {0}")]
    Crypto(#[from] CryptoError),

    /// Hash computation failed
    #[error("Hash error: {0}")]
    HashError(String),

    /// Chunk size invalid
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(usize),

    /// Chunk not found
    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),
}

/// Result type for chunk operations
pub type Result<T> = std::result::Result<T, ChunkError>;

/// Default chunk size (1MB)
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum chunk size (16MB)
pub const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;

/// Minimum chunk size (64KB)
pub const MIN_CHUNK_SIZE: usize = 64 * 1024;

/// A chunk of file data
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Content hash (SHA-256) - used as chunk ID
    pub hash: String,

    /// Chunk size in bytes (original, before compression)
    pub size: usize,

    /// Compressed size in bytes (after compression)
    pub compressed_size: usize,

    /// Is this chunk compressed?
    pub compressed: bool,

    /// Is this chunk encrypted?
    pub encrypted: bool,

    /// Offset in the original file
    pub offset: u64,

    /// Data (may be compressed/encrypted)
    pub data: Vec<u8>,
}

impl Chunk {
    /// Create a new chunk from data
    pub fn new(data: Vec<u8>, offset: u64) -> Self {
        let hash = Self::compute_hash(&data);
        let size = data.len();

        Self {
            hash,
            size,
            compressed_size: size,
            compressed: false,
            encrypted: false,
            offset,
            data,
        }
    }

    /// Compute SHA-256 hash of data
    fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Compress the chunk data
    pub fn compress(&mut self) -> Result<()> {
        if self.compressed {
            return Ok(());
        }

        // Simple compression using flate2 (already available via ring dependencies)
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&self.data)?;
        let compressed = encoder.finish()?;

        // Only use compression if it actually reduces size
        if compressed.len() < self.data.len() {
            self.compressed_size = compressed.len();
            self.data = compressed;
            self.compressed = true;
        }

        Ok(())
    }

    /// Decompress the chunk data
    pub fn decompress(&mut self) -> Result<()> {
        if !self.compressed {
            return Ok(());
        }

        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(&self.data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        self.data = decompressed;
        self.compressed_size = self.size;
        self.compressed = false;

        Ok(())
    }

    /// Encrypt the chunk using the provided key
    pub fn encrypt(&mut self, key: &CryptoKey) -> Result<()> {
        if self.encrypted {
            return Ok(());
        }

        let encrypted = key.encrypt(&self.data, None)?;
        self.data = encrypted;
        self.encrypted = true;

        Ok(())
    }

    /// Decrypt the chunk using the provided key
    pub fn decrypt(&mut self, key: &CryptoKey) -> Result<()> {
        if !self.encrypted {
            return Ok(());
        }

        let decrypted = key.decrypt(&self.data, None)?;
        self.data = decrypted;
        self.encrypted = false;

        Ok(())
    }

    /// Get the chunk ID (hash)
    pub fn id(&self) -> &str {
        &self.hash
    }

    /// Get the current data size (accounting for compression/encryption)
    pub fn current_size(&self) -> usize {
        self.data.len()
    }
}

/// Options for chunk processor
#[derive(Debug, Clone)]
pub struct ChunkOptions {
    /// Chunk size in bytes
    pub chunk_size: usize,

    /// Enable compression
    pub compress: bool,

    /// Enable encryption
    pub encrypt: bool,

    /// Number of parallel chunks to process
    pub parallel: usize,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            compress: true,
            encrypt: true,
            parallel: 4,
        }
    }
}

impl ChunkOptions {
    /// Validate chunk options
    pub fn validate(&self) -> Result<()> {
        if self.chunk_size < MIN_CHUNK_SIZE || self.chunk_size > MAX_CHUNK_SIZE {
            return Err(ChunkError::InvalidChunkSize(self.chunk_size));
        }
        Ok(())
    }
}

/// Chunk processor for splitting files into chunks
pub struct ChunkProcessor {
    options: ChunkOptions,
}

impl ChunkProcessor {
    /// Create a new chunk processor
    pub fn new(options: ChunkOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self { options })
    }

    /// Create with default options
    pub fn default() -> Self {
        Self::new(ChunkOptions::default()).unwrap()
    }

    /// Split a file into chunks
    pub fn chunk_file(&self, path: &Path) -> Result<Vec<Chunk>> {
        let mut file = File::open(path)?;
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut buffer = vec![0u8; self.options.chunk_size];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let chunk_data = buffer[..bytes_read].to_vec();
            let chunk = Chunk::new(chunk_data, offset);
            chunks.push(chunk);
            offset += bytes_read as u64;
        }

        Ok(chunks)
    }

    /// Process chunks (compress and/or encrypt)
    pub fn process_chunks(
        &self,
        chunks: &mut [Chunk],
        key: Option<&CryptoKey>,
    ) -> Result<()> {
        for chunk in chunks.iter_mut() {
            if self.options.compress {
                chunk.compress()?;
            }

            if self.options.encrypt {
                if let Some(k) = key {
                    chunk.encrypt(k)?;
                }
            }
        }

        Ok(())
    }

    /// Reassemble chunks into a file
    pub fn reassemble_chunks(
        &self,
        chunks: &mut [Chunk],
        output_path: &Path,
        key: Option<&CryptoKey>,
    ) -> Result<()> {
        let mut output = File::create(output_path)?;

        // Sort chunks by offset
        let mut sorted_chunks: Vec<_> = chunks.iter_mut().collect();
        sorted_chunks.sort_by_key(|c| c.offset);

        for chunk in sorted_chunks {
            // Decrypt if needed
            if chunk.encrypted {
                if let Some(k) = key {
                    chunk.decrypt(k)?;
                } else {
                    return Err(ChunkError::Crypto(CryptoError::DecryptionFailed));
                }
            }

            // Decompress if needed
            if chunk.compressed {
                chunk.decompress()?;
            }

            output.write_all(&chunk.data)?;
        }

        Ok(())
    }
}

/// Chunk store for managing chunks with deduplication
pub struct ChunkStore {
    /// Base directory for chunk storage
    base_path: PathBuf,

    /// In-memory index of known chunks
    index: std::collections::HashMap<String, ChunkMetadata>,
}

/// Metadata for a stored chunk
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    /// Chunk hash/ID
    pub hash: String,

    /// Original size
    pub size: usize,

    /// Stored size (compressed/encrypted)
    pub stored_size: usize,

    /// Number of files referencing this chunk
    pub ref_count: usize,
}

impl ChunkStore {
    /// Create a new chunk store
    pub fn new(base_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_path)?;

        Ok(Self {
            base_path,
            index: std::collections::HashMap::new(),
        })
    }

    /// Get the path for a chunk file
    fn chunk_path(&self, hash: &str) -> PathBuf {
        // Use first 2 chars as subdirectory for better filesystem performance
        let subdir = &hash[..2.min(hash.len())];
        self.base_path.join(subdir).join(hash)
    }

    /// Store a chunk (with deduplication)
    pub fn store(&mut self, chunk: &Chunk) -> Result<bool> {
        let hash = chunk.id();

        // Check if chunk already exists
        if let Some(meta) = self.index.get_mut(hash) {
            meta.ref_count += 1;
            return Ok(false); // Already exists, deduplicated
        }

        // Store the chunk
        let chunk_path = self.chunk_path(hash);
        std::fs::create_dir_all(chunk_path.parent().unwrap())?;

        let mut file = File::create(&chunk_path)?;
        file.write_all(&chunk.data)?;

        // Update index
        self.index.insert(
            hash.to_string(),
            ChunkMetadata {
                hash: hash.to_string(),
                size: chunk.size,
                stored_size: chunk.current_size(),
                ref_count: 1,
            },
        );

        Ok(true) // Newly stored
    }

    /// Retrieve a chunk
    pub fn retrieve(&self, hash: &str) -> Result<Chunk> {
        let chunk_path = self.chunk_path(hash);

        if !chunk_path.exists() {
            return Err(ChunkError::ChunkNotFound(hash.to_string()));
        }

        let data = std::fs::read(&chunk_path)?;
        let meta = self.index.get(hash)
            .ok_or_else(|| ChunkError::ChunkNotFound(hash.to_string()))?;

        Ok(Chunk {
            hash: hash.to_string(),
            size: meta.size,
            compressed_size: meta.stored_size,
            compressed: meta.stored_size != meta.size,
            encrypted: false, // Caller should know encryption state
            offset: 0,        // Offset not stored in chunk store
            data,
        })
    }

    /// Delete a chunk (decrements ref count)
    pub fn delete(&mut self, hash: &str) -> Result<bool> {
        if let Some(meta) = self.index.get_mut(hash) {
            meta.ref_count -= 1;

            if meta.ref_count == 0 {
                // Actually delete the chunk file
                let chunk_path = self.chunk_path(hash);
                if chunk_path.exists() {
                    std::fs::remove_file(&chunk_path)?;
                }
                self.index.remove(hash);
                return Ok(true); // Deleted
            }
        }

        Ok(false) // Still has references
    }

    /// Get total number of chunks
    pub fn chunk_count(&self) -> usize {
        self.index.len()
    }

    /// Get total stored bytes
    pub fn total_stored_bytes(&self) -> u64 {
        self.index.values()
            .map(|m| m.stored_size as u64)
            .sum()
    }

    /// Get total original bytes (before compression)
    pub fn total_original_bytes(&self) -> u64 {
        self.index.values()
            .map(|m| m.size as u64)
            .sum()
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        let original = self.total_original_bytes() as f64;
        let stored = self.total_stored_bytes() as f64;

        if original > 0.0 {
            stored / original
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_chunk_creation() {
        let data = b"test data".to_vec();
        let chunk = Chunk::new(data.clone(), 0);

        assert_eq!(chunk.size, 9);
        assert!(!chunk.compressed);
        assert!(!chunk.encrypted);
        assert_eq!(chunk.offset, 0);
        assert_eq!(chunk.data, data);
    }

    #[test]
    fn test_chunk_compression() {
        let data = vec![b'A'; 1000]; // Highly compressible data
        let mut chunk = Chunk::new(data, 0);

        chunk.compress().unwrap();
        assert!(chunk.compressed);
        assert!(chunk.compressed_size < chunk.size);

        chunk.decompress().unwrap();
        assert!(!chunk.compressed);
        assert_eq!(chunk.data.len(), 1000);
    }

    #[test]
    fn test_chunk_hash() {
        let data1 = b"test data".to_vec();
        let data2 = b"test data".to_vec();
        let data3 = b"different".to_vec();

        let chunk1 = Chunk::new(data1, 0);
        let chunk2 = Chunk::new(data2, 0);
        let chunk3 = Chunk::new(data3, 0);

        assert_eq!(chunk1.hash, chunk2.hash); // Same data = same hash
        assert_ne!(chunk1.hash, chunk3.hash); // Different data = different hash
    }

    #[test]
    fn test_chunk_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");

        // Create a file with 2.5MB of data (should create 3 chunks with 1MB chunks)
        let data = vec![b'X'; 2_500_000];
        std::fs::write(&file_path, &data).unwrap();

        let processor = ChunkProcessor::new(ChunkOptions {
            chunk_size: 1024 * 1024, // 1MB chunks
            compress: false,
            encrypt: false,
            parallel: 1,
        }).unwrap();

        let chunks = processor.chunk_file(&file_path).unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].size, 1024 * 1024);
        assert_eq!(chunks[1].size, 1024 * 1024);
        assert_eq!(chunks[2].size, 2_500_000 - 2 * 1024 * 1024);
    }

    #[test]
    fn test_reassemble_chunks() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("input.txt");
        let output_path = temp_dir.path().join("output.txt");

        let original_data = b"Hello, World! This is test data.";
        std::fs::write(&input_path, original_data).unwrap();

        let processor = ChunkProcessor::new(ChunkOptions {
            chunk_size: 10, // Small chunks for testing
            compress: false,
            encrypt: false,
            parallel: 1,
        }).unwrap();

        let mut chunks = processor.chunk_file(&input_path).unwrap();
        processor.reassemble_chunks(&mut chunks, &output_path, None).unwrap();

        let reassembled = std::fs::read(&output_path).unwrap();
        assert_eq!(reassembled, original_data);
    }

    #[test]
    fn test_chunk_store() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = ChunkStore::new(temp_dir.path().to_path_buf()).unwrap();

        let chunk1 = Chunk::new(b"test data 1".to_vec(), 0);
        let chunk2 = Chunk::new(b"test data 1".to_vec(), 0); // Duplicate
        let chunk3 = Chunk::new(b"test data 2".to_vec(), 0); // Different

        // Store chunks
        let stored1 = store.store(&chunk1).unwrap();
        assert!(stored1); // Newly stored

        let stored2 = store.store(&chunk2).unwrap();
        assert!(!stored2); // Deduplicated

        let stored3 = store.store(&chunk3).unwrap();
        assert!(stored3); // Newly stored

        assert_eq!(store.chunk_count(), 2); // Only 2 unique chunks

        // Retrieve chunk
        let retrieved = store.retrieve(chunk1.id()).unwrap();
        assert_eq!(retrieved.hash, chunk1.hash);
        assert_eq!(retrieved.data, chunk1.data);
    }

    #[test]
    fn test_chunk_options_validation() {
        let invalid_options = ChunkOptions {
            chunk_size: 100, // Too small
            compress: false,
            encrypt: false,
            parallel: 1,
        };

        let result = ChunkProcessor::new(invalid_options);
        assert!(result.is_err());
    }
}

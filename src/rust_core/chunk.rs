//! File chunking and compression
//!
//! This module handles breaking files into chunks for efficient incremental
//! updates and applying compression.

use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::path::Path;
use thiserror::Error;

/// Default chunk size (1MB)
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum chunk size (10MB)
pub const MAX_CHUNK_SIZE: usize = 10 * 1024 * 1024;

/// Minimum chunk size (64KB)
pub const MIN_CHUNK_SIZE: usize = 64 * 1024;

/// Chunk error types
#[derive(Debug, Error)]
pub enum ChunkError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Invalid chunk size
    #[error("Invalid chunk size: {0}")]
    InvalidSize(usize),

    /// Hash mismatch
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}

/// Result type for chunk operations
pub type Result<T> = std::result::Result<T, ChunkError>;

/// Chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkInfo {
    /// Chunk index
    pub index: u32,

    /// Chunk offset in the original file
    pub offset: u64,

    /// Original size before compression
    pub original_size: usize,

    /// Compressed size
    pub compressed_size: usize,

    /// SHA-256 hash of original data
    pub hash: String,

    /// SHA-256 hash of compressed data
    pub compressed_hash: String,
}

/// File chunker
pub struct Chunker {
    chunk_size: usize,
    compression_level: i32,
}

impl Chunker {
    /// Create a new chunker with default settings
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// Create a chunker with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        let chunk_size = chunk_size.clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE);
        Self {
            chunk_size,
            compression_level: 3, // Default zstd compression level
        }
    }

    /// Set compression level (0-22 for zstd)
    pub fn set_compression_level(&mut self, level: i32) {
        self.compression_level = level.clamp(0, 22);
    }

    /// Split a file into chunks
    pub fn chunk_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<ChunkInfo>> {
        let mut file = std::fs::File::open(path)?;
        let mut chunks = Vec::new();
        let mut index = 0u32;
        let mut offset = 0u64;

        loop {
            let mut buffer = vec![0u8; self.chunk_size];
            let bytes_read = file.read(&mut buffer)?;

            if bytes_read == 0 {
                break;
            }

            buffer.truncate(bytes_read);

            // Calculate original hash
            let hash = Self::hash_data(&buffer);

            // Compress the chunk
            let compressed = self.compress(&buffer)?;
            let compressed_hash = Self::hash_data(&compressed);

            chunks.push(ChunkInfo {
                index,
                offset,
                original_size: bytes_read,
                compressed_size: compressed.len(),
                hash,
                compressed_hash,
            });

            index += 1;
            offset += bytes_read as u64;
        }

        Ok(chunks)
    }

    /// Read a chunk from a file
    pub fn read_chunk<P: AsRef<Path>>(
        &self,
        path: P,
        chunk_info: &ChunkInfo,
    ) -> Result<Vec<u8>> {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = vec![0u8; chunk_info.original_size];

        use std::io::Seek;
        file.seek(io::SeekFrom::Start(chunk_info.offset))?;
        file.read_exact(&mut buffer)?;

        // Verify hash
        let actual_hash = Self::hash_data(&buffer);
        if actual_hash != chunk_info.hash {
            return Err(ChunkError::HashMismatch {
                expected: chunk_info.hash.clone(),
                actual: actual_hash,
            });
        }

        Ok(buffer)
    }

    /// Compress data using zstd
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Simple compression implementation using flate2 as a placeholder
        // In production, you'd use zstd crate
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::new(self.compression_level as u32),
        );
        encoder.write_all(data)?;
        encoder.finish().map_err(|e| ChunkError::Compression(e.to_string()))
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut result = Vec::new();
        decoder.read_to_end(&mut result)?;
        Ok(result)
    }

    /// Calculate SHA-256 hash of data
    fn hash_data(data: &[u8]) -> String {
        let mut context = Context::new(&SHA256);
        context.update(data);
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunk writer for streaming chunk output
pub struct ChunkWriter<W: Write> {
    writer: W,
    chunker: Chunker,
    chunks_written: Vec<ChunkInfo>,
    current_offset: u64,
    current_index: u32,
}

impl<W: Write> ChunkWriter<W> {
    /// Create a new chunk writer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            chunker: Chunker::new(),
            chunks_written: Vec::new(),
            current_offset: 0,
            current_index: 0,
        }
    }

    /// Write a chunk of data
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<ChunkInfo> {
        let hash = Chunker::hash_data(data);
        let compressed = self.chunker.compress(data)?;
        let compressed_hash = Chunker::hash_data(&compressed);

        // Write compressed data
        self.writer.write_all(&compressed)?;

        let chunk_info = ChunkInfo {
            index: self.current_index,
            offset: self.current_offset,
            original_size: data.len(),
            compressed_size: compressed.len(),
            hash,
            compressed_hash,
        };

        self.chunks_written.push(chunk_info.clone());
        self.current_offset += data.len() as u64;
        self.current_index += 1;

        Ok(chunk_info)
    }

    /// Get all written chunks
    pub fn chunks(&self) -> &[ChunkInfo] {
        &self.chunks_written
    }

    /// Finish writing and return the inner writer
    pub fn finish(self) -> W {
        self.writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_chunker_creation() {
        let chunker = Chunker::new();
        assert_eq!(chunker.chunk_size, DEFAULT_CHUNK_SIZE);
    }

    #[test]
    fn test_chunker_with_custom_size() {
        let chunker = Chunker::with_chunk_size(512 * 1024);
        assert_eq!(chunker.chunk_size, 512 * 1024);
    }

    #[test]
    fn test_compression() {
        let chunker = Chunker::new();
        let data = b"Hello, World! ".repeat(1000);
        let compressed = chunker.compress(&data).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = chunker.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_hash_data() {
        let data = b"test data";
        let hash = Chunker::hash_data(data);
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_chunk_file() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data = vec![0u8; 2 * 1024 * 1024]; // 2MB
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let chunker = Chunker::with_chunk_size(1024 * 1024);
        let chunks = chunker.chunk_file(tmpfile.path()).unwrap();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[1].index, 1);
    }

    #[test]
    fn test_chunk_writer() {
        let mut buffer = Vec::new();
        let mut writer = ChunkWriter::new(&mut buffer);

        let data1 = b"First chunk";
        let data2 = b"Second chunk";

        let chunk1 = writer.write_chunk(data1).unwrap();
        let chunk2 = writer.write_chunk(data2).unwrap();

        assert_eq!(chunk1.index, 0);
        assert_eq!(chunk2.index, 1);
        assert_eq!(writer.chunks().len(), 2);
    }
}

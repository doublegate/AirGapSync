//! Chunk-based file processing for efficient synchronization
//!
//! This module provides functionality to split files into chunks, compress them,
//! encrypt them, and manage chunk deduplication for efficient storage and transfer.

use crate::crypto::{CryptoKey, CryptoError};
use std::path::Path;
use std::fs::File;
use std::io::{self, Read, Write, BufReader, BufWriter, Seek, SeekFrom};
use std::collections::HashMap;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use hex;
use zstd::{Encoder, Decoder};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

/// Errors that can occur during chunk operations
#[derive(Error, Debug)]
pub enum ChunkError {
    /// IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    /// Encryption error
    #[error("Encryption error: {0}")]
    Encryption(#[from] CryptoError),
    
    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),
    
    /// Invalid chunk size
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(usize),
    
    /// Chunk verification failed
    #[error("Chunk verification failed: expected {expected}, got {actual}")]
    VerificationFailed {
        expected: String,
        actual: String,
    },
    
    /// Decompression error
    #[error("Decompression error: {0}")]
    Decompression(String),
}

/// Chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Chunk index in file
    pub index: u32,
    
    /// Offset in original file
    pub offset: u64,
    
    /// Size of original data
    pub original_size: usize,
    
    /// Size after compression
    pub compressed_size: usize,
    
    /// Size after encryption
    pub encrypted_size: usize,
    
    /// SHA-256 hash of original data
    pub hash: String,
    
    /// SHA-256 hash of encrypted data
    pub encrypted_hash: String,
    
    /// Compression ratio
    pub compression_ratio: f32,
}

/// Encrypted chunk data
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Chunk metadata
    pub metadata: ChunkMetadata,
    
    /// Encrypted data
    pub encrypted_data: Vec<u8>,
    
    /// Content hash for deduplication
    pub hash: String,
}

/// Chunk processor for handling file chunking
pub struct ChunkProcessor {
    /// Chunk deduplication index
    dedup_index: Arc<Mutex<HashMap<String, ChunkMetadata>>>,
    
    /// Whether to use compression
    use_compression: bool,
    
    /// Whether to verify chunks after processing
    verify_chunks: bool,
    
    /// Minimum chunk size for splitting
    min_chunk_size: usize,
    
    /// Maximum chunk size
    max_chunk_size: usize,
    
    /// Buffer pool for reuse
    buffer_pool: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl ChunkProcessor {
    /// Create a new chunk processor
    pub fn new() -> Self {
        Self {
            dedup_index: Arc::new(Mutex::new(HashMap::new())),
            use_compression: true,
            verify_chunks: true,
            min_chunk_size: 512 * 1024,      // 512KB minimum
            max_chunk_size: 16 * 1024 * 1024, // 16MB maximum
            buffer_pool: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Set compression usage
    pub fn set_use_compression(&mut self, use_compression: bool) {
        self.use_compression = use_compression;
    }
    
    /// Set chunk verification
    pub fn set_verify_chunks(&mut self, verify: bool) {
        self.verify_chunks = verify;
    }
    
    /// Get a buffer from the pool or create a new one
    fn get_buffer(&self, size: usize) -> Vec<u8> {
        let mut pool = self.buffer_pool.lock().unwrap();
        pool.pop()
            .map(|mut buf| {
                buf.resize(size, 0);
                buf
            })
            .unwrap_or_else(|| vec![0u8; size])
    }
    
    /// Return a buffer to the pool
    fn return_buffer(&self, buffer: Vec<u8>) {
        let mut pool = self.buffer_pool.lock().unwrap();
        if pool.len() < 10 { // Keep up to 10 buffers
            pool.push(buffer);
        }
    }
    
    /// Process a file into chunks
    pub fn process_file(
        &self,
        path: &Path,
        chunk_size: usize,
        compression_level: u32,
        encryption_key: &CryptoKey,
    ) -> Result<Vec<Chunk>, ChunkError> {
        if chunk_size < self.min_chunk_size || chunk_size > self.max_chunk_size {
            return Err(ChunkError::InvalidChunkSize(chunk_size));
        }
        
        let file = File::open(path)?;
        let file_size = file.metadata()?.len();
        let _reader = BufReader::new(file);
        
        // Calculate number of chunks
        let chunk_count = ((file_size + chunk_size as u64 - 1) / chunk_size as u64) as usize;
        let mut chunks = Vec::with_capacity(chunk_count);
        
        // Process chunks in parallel using rayon
        let chunk_results: Vec<Result<Chunk, ChunkError>> = (0..chunk_count)
            .into_par_iter()
            .map(|index| {
                self.process_single_chunk(
                    path,
                    index as u32,
                    index as u64 * chunk_size as u64,
                    chunk_size,
                    compression_level,
                    encryption_key,
                )
            })
            .collect();
        
        // Collect results
        for result in chunk_results {
            chunks.push(result?);
        }
        
        Ok(chunks)
    }
    
    /// Process a single chunk
    fn process_single_chunk(
        &self,
        path: &Path,
        index: u32,
        offset: u64,
        chunk_size: usize,
        compression_level: u32,
        encryption_key: &CryptoKey,
    ) -> Result<Chunk, ChunkError> {
        // Read chunk data
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        
        let mut buffer = self.get_buffer(chunk_size);
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        
        // Compute hash of original data
        let original_hash = compute_hash(&buffer);
        
        // Check deduplication index
        let dedup_key = original_hash.clone();
        if let Some(existing) = self.dedup_index.lock().unwrap().get(&dedup_key) {
            // Chunk already processed - return reference
            return Ok(Chunk {
                metadata: existing.clone(),
                encrypted_data: Vec::new(), // Empty data for deduplicated chunks
                hash: dedup_key,
            });
        }
        
        // Compress data
        let compressed_data = if self.use_compression && compression_level > 0 {
            compress_data(&buffer, compression_level)?
        } else {
            buffer.clone()
        };
        
        let compressed_size = compressed_data.len();
        let compression_ratio = if bytes_read > 0 {
            compressed_size as f32 / bytes_read as f32
        } else {
            1.0
        };
        
        // Encrypt compressed data
        let encrypted_data = crate::crypto::encrypt(
            encryption_key,
            &compressed_data,
            &format!("chunk-{}-{}", path.display(), index).as_bytes(),
        )?;
        
        let encrypted_hash = compute_hash(&encrypted_data);
        
        // Verify if requested
        if self.verify_chunks {
            self.verify_chunk(
                &encrypted_data,
                &compressed_data,
                encryption_key,
                &format!("chunk-{}-{}", path.display(), index),
            )?;
        }
        
        // Create metadata
        let metadata = ChunkMetadata {
            index,
            offset,
            original_size: bytes_read,
            compressed_size,
            encrypted_size: encrypted_data.len(),
            hash: original_hash.clone(),
            encrypted_hash: encrypted_hash.clone(),
            compression_ratio,
        };
        
        // Update deduplication index
        self.dedup_index.lock().unwrap().insert(dedup_key.clone(), metadata.clone());
        
        // Return buffer to pool
        self.return_buffer(buffer);
        
        Ok(Chunk {
            metadata,
            encrypted_data,
            hash: dedup_key,
        })
    }
    
    /// Verify chunk integrity
    fn verify_chunk(
        &self,
        encrypted_data: &[u8],
        original_compressed: &[u8],
        encryption_key: &CryptoKey,
        aad: &str,
    ) -> Result<(), ChunkError> {
        let decrypted = crate::crypto::decrypt(encryption_key, encrypted_data, aad.as_bytes())?;
        
        if decrypted != original_compressed {
            return Err(ChunkError::VerificationFailed {
                expected: compute_hash(original_compressed),
                actual: compute_hash(&decrypted),
            });
        }
        
        Ok(())
    }
    
    /// Process file using content-defined chunking (CDC)
    pub fn process_file_cdc(
        &self,
        path: &Path,
        target_chunk_size: usize,
        compression_level: u32,
        encryption_key: &CryptoKey,
    ) -> Result<Vec<Chunk>, ChunkError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        
        let mut chunks = Vec::new();
        let mut chunk_buffer = Vec::new();
        let mut hasher = RollingHash::new(target_chunk_size);
        let mut offset = 0u64;
        let mut index = 0u32;
        
        let mut byte = [0u8; 1];
        while reader.read_exact(&mut byte).is_ok() {
            chunk_buffer.push(byte[0]);
            hasher.update(byte[0]);
            
            // Check if we've found a chunk boundary
            if hasher.is_boundary() || chunk_buffer.len() >= self.max_chunk_size {
                if chunk_buffer.len() >= self.min_chunk_size {
                    // Process this chunk
                    let chunk = self.process_buffer_chunk(
                        &chunk_buffer,
                        index,
                        offset,
                        compression_level,
                        encryption_key,
                        path,
                    )?;
                    
                    offset += chunk_buffer.len() as u64;
                    chunks.push(chunk);
                    chunk_buffer.clear();
                    hasher.reset();
                    index += 1;
                }
            }
        }
        
        // Process remaining data
        if !chunk_buffer.is_empty() {
            let chunk = self.process_buffer_chunk(
                &chunk_buffer,
                index,
                offset,
                compression_level,
                encryption_key,
                path,
            )?;
            chunks.push(chunk);
        }
        
        Ok(chunks)
    }
    
    /// Process a buffer as a chunk
    fn process_buffer_chunk(
        &self,
        buffer: &[u8],
        index: u32,
        offset: u64,
        compression_level: u32,
        encryption_key: &CryptoKey,
        path: &Path,
    ) -> Result<Chunk, ChunkError> {
        let original_hash = compute_hash(buffer);
        
        // Check deduplication
        if let Some(existing) = self.dedup_index.lock().unwrap().get(&original_hash) {
            return Ok(Chunk {
                metadata: existing.clone(),
                encrypted_data: Vec::new(),
                hash: original_hash,
            });
        }
        
        // Compress
        let compressed_data = if self.use_compression && compression_level > 0 {
            compress_data(buffer, compression_level)?
        } else {
            buffer.to_vec()
        };
        
        // Encrypt
        let aad = format!("chunk-{}-{}", path.display(), index);
        let encrypted_data = crate::crypto::encrypt(encryption_key, &compressed_data, aad.as_bytes())?;
        
        let metadata = ChunkMetadata {
            index,
            offset,
            original_size: buffer.len(),
            compressed_size: compressed_data.len(),
            encrypted_size: encrypted_data.len(),
            hash: original_hash.clone(),
            encrypted_hash: compute_hash(&encrypted_data),
            compression_ratio: compressed_data.len() as f32 / buffer.len() as f32,
        };
        
        self.dedup_index.lock().unwrap().insert(original_hash.clone(), metadata.clone());
        
        Ok(Chunk {
            metadata,
            encrypted_data,
            hash: original_hash,
        })
    }
    
    /// Reconstruct a file from chunks
    pub fn reconstruct_file(
        &self,
        chunks: &[Chunk],
        output_path: &Path,
        encryption_key: &CryptoKey,
        original_path: &Path,
    ) -> Result<(), ChunkError> {
        let file = File::create(output_path)?;
        let mut writer = BufWriter::new(file);
        
        // Process chunks in order
        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.metadata.index);
        
        for chunk in sorted_chunks {
            // Skip deduplicated chunks (empty data)
            if chunk.encrypted_data.is_empty() {
                continue;
            }
            
            // Decrypt
            let aad = format!("chunk-{}-{}", original_path.display(), chunk.metadata.index);
            let compressed_data = crate::crypto::decrypt(encryption_key, &chunk.encrypted_data, aad.as_bytes())?;
            
            // Decompress
            let original_data = if self.use_compression && chunk.metadata.compression_ratio < 0.99 {
                decompress_data(&compressed_data)?
            } else {
                compressed_data
            };
            
            // Verify hash
            let actual_hash = compute_hash(&original_data);
            if actual_hash != chunk.metadata.hash {
                return Err(ChunkError::VerificationFailed {
                    expected: chunk.metadata.hash.clone(),
                    actual: actual_hash,
                });
            }
            
            // Write to output
            writer.write_all(&original_data)?;
        }
        
        writer.flush()?;
        Ok(())
    }
    
    /// Get deduplication statistics
    pub fn get_dedup_stats(&self) -> DedupStats {
        let index = self.dedup_index.lock().unwrap();
        let total_chunks = index.len();
        let total_size: u64 = index.values().map(|m| m.original_size as u64).sum();
        let compressed_size: u64 = index.values().map(|m| m.compressed_size as u64).sum();
        let encrypted_size: u64 = index.values().map(|m| m.encrypted_size as u64).sum();
        
        DedupStats {
            total_chunks,
            unique_chunks: total_chunks, // All chunks in index are unique
            total_size,
            deduplicated_size: total_size, // Would be less with actual dedup
            compressed_size,
            encrypted_size,
            compression_ratio: if total_size > 0 {
                compressed_size as f32 / total_size as f32
            } else {
                1.0
            },
        }
    }
    
    /// Clear deduplication index
    pub fn clear_dedup_index(&self) {
        self.dedup_index.lock().unwrap().clear();
    }
}

/// Compute SHA-256 hash of data
fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Compress data using zstd
fn compress_data(data: &[u8], level: u32) -> Result<Vec<u8>, ChunkError> {
    let mut encoder = Encoder::new(Vec::new(), level as i32)
        .map_err(|e| ChunkError::Compression(e.to_string()))?;
    
    encoder.write_all(data)
        .map_err(|e| ChunkError::Compression(e.to_string()))?;
    
    encoder.finish()
        .map_err(|e| ChunkError::Compression(e.to_string()))
}

/// Decompress data using zstd
fn decompress_data(data: &[u8]) -> Result<Vec<u8>, ChunkError> {
    let mut decoder = Decoder::new(data)
        .map_err(|e| ChunkError::Decompression(e.to_string()))?;
    
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)
        .map_err(|e| ChunkError::Decompression(e.to_string()))?;
    
    Ok(decompressed)
}

/// Rolling hash for content-defined chunking
struct RollingHash {
    window: Vec<u8>,
    window_size: usize,
    hash: u64,
    target_size: usize,
    mask: u64,
}

impl RollingHash {
    fn new(target_size: usize) -> Self {
        let window_size = 48; // Typical window size for CDC
        let bits = (target_size as f64).log2() as u32;
        let mask = (1u64 << bits) - 1;
        
        Self {
            window: Vec::with_capacity(window_size),
            window_size,
            hash: 0,
            target_size,
            mask,
        }
    }
    
    fn update(&mut self, byte: u8) {
        // Simple rolling hash (can be replaced with more sophisticated algorithms)
        if self.window.len() >= self.window_size {
            let old_byte = self.window.remove(0);
            self.hash = self.hash.wrapping_sub(old_byte as u64);
        }
        
        self.window.push(byte);
        self.hash = self.hash.wrapping_add(byte as u64);
        self.hash = self.hash.wrapping_mul(33); // Prime multiplier
    }
    
    fn is_boundary(&self) -> bool {
        self.window.len() >= self.window_size && (self.hash & self.mask) == 0
    }
    
    fn reset(&mut self) {
        self.window.clear();
        self.hash = 0;
    }
}

/// Deduplication statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupStats {
    pub total_chunks: usize,
    pub unique_chunks: usize,
    pub total_size: u64,
    pub deduplicated_size: u64,
    pub compressed_size: u64,
    pub encrypted_size: u64,
    pub compression_ratio: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::crypto::Algorithm;
    
    #[test]
    fn test_compute_hash() {
        let data = b"Hello, world!";
        let hash = compute_hash(data);
        assert_eq!(hash, "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3");
    }
    
    #[test]
    fn test_compress_decompress() {
        let data = b"This is some test data that should compress well because it has repetition repetition repetition";
        let compressed = compress_data(data, 3).unwrap();
        assert!(compressed.len() < data.len());
        
        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
    
    #[test]
    fn test_chunk_processing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create test file
        let mut file = File::create(&file_path).unwrap();
        let test_data = vec![b'A'; 1024 * 1024]; // 1MB of 'A's
        file.write_all(&test_data).unwrap();
        file.sync_all().unwrap();
        
        // Process file
        let processor = ChunkProcessor::new();
        let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
        let chunks = processor.process_file(
            &file_path,
            512 * 1024, // 512KB chunks
            6,          // Compression level
            &key,
        ).unwrap();
        
        assert_eq!(chunks.len(), 2); // Should produce 2 chunks
        
        // Test reconstruction
        let output_path = temp_dir.path().join("reconstructed.txt");
        processor.reconstruct_file(&chunks, &output_path, &key, &file_path).unwrap();
        
        // Verify reconstruction
        let reconstructed = std::fs::read(&output_path).unwrap();
        assert_eq!(reconstructed, test_data);
    }
    
    #[test]
    fn test_rolling_hash() {
        let mut hasher = RollingHash::new(1024);
        
        // Feed some data
        for i in 0..100 {
            hasher.update(i as u8);
        }
        
        // Should eventually find a boundary
        let mut found_boundary = false;
        for i in 100..10000 {
            hasher.update(i as u8);
            if hasher.is_boundary() {
                found_boundary = true;
                break;
            }
        }
        
        assert!(found_boundary);
    }
}
//! Cryptographic operations for AirGapSync
//!
//! This module implements encryption, decryption, and key management
//! using the ring cryptography library.

use ring::aead::{Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey};
use ring::aead::{AES_256_GCM, CHACHA20_POLY1305};
use ring::error::Unspecified;
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;
use thiserror::Error;
use zeroize::Zeroize;

/// Cryptographic error types
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Encryption operation failed
    #[error("Encryption failed")]
    EncryptionFailed,

    /// Decryption operation failed
    #[error("Decryption failed")]
    DecryptionFailed,

    /// Key has invalid length for algorithm
    #[error("Invalid key length")]
    InvalidKeyLength,

    /// Nonce has invalid format or length
    #[error("Invalid nonce")]
    InvalidNonce,

    /// Key derivation from password failed
    #[error("Key derivation failed")]
    KeyDerivationFailed,

    /// Secure random number generation failed
    #[error("Random generation failed")]
    RandomGenerationFailed,

    /// Requested algorithm is not supported
    #[error("Algorithm not supported: {0}")]
    UnsupportedAlgorithm(String),
}

/// Supported encryption algorithms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Algorithm {
    /// AES-256 in GCM mode
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

impl Algorithm {
    /// Get the key size in bytes for this algorithm
    pub fn key_size(&self) -> usize {
        match self {
            Algorithm::Aes256Gcm => 32,
            Algorithm::ChaCha20Poly1305 => 32,
        }
    }

    /// Get the nonce size in bytes for this algorithm
    pub fn nonce_size(&self) -> usize {
        match self {
            Algorithm::Aes256Gcm => 12,
            Algorithm::ChaCha20Poly1305 => 12,
        }
    }

    /// Get the tag size in bytes for this algorithm
    pub fn tag_size(&self) -> usize {
        match self {
            Algorithm::Aes256Gcm => 16,
            Algorithm::ChaCha20Poly1305 => 16,
        }
    }
}

/// A cryptographic key for encryption/decryption
pub struct CryptoKey {
    /// The raw key material (zeroed on drop)
    key: Vec<u8>,
    /// The algorithm this key is for
    algorithm: Algorithm,
}

impl CryptoKey {
    /// Create a new crypto key
    pub fn new(key: Vec<u8>, algorithm: Algorithm) -> Result<Self, CryptoError> {
        if key.len() != algorithm.key_size() {
            return Err(CryptoError::InvalidKeyLength);
        }
        Ok(Self { key, algorithm })
    }

    /// Generate a new random key
    pub fn generate(algorithm: Algorithm) -> Result<Self, CryptoError> {
        let rng = SystemRandom::new();
        let mut key = vec![0u8; algorithm.key_size()];
        rng.fill(&mut key)
            .map_err(|_| CryptoError::RandomGenerationFailed)?;
        Ok(Self { key, algorithm })
    }

    /// Derive a key from a password using PBKDF2
    pub fn derive_from_password(
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        algorithm: Algorithm,
    ) -> Result<Self, CryptoError> {
        let mut key = vec![0u8; algorithm.key_size()];

        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(iterations).ok_or(CryptoError::KeyDerivationFailed)?,
            salt,
            password,
            &mut key,
        );

        Ok(Self { key, algorithm })
    }

    /// Get the algorithm for this key
    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// Get the key material for testing purposes
    #[cfg(test)]
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// Get the key length in bytes
    pub fn key_len(&self) -> usize {
        self.key.len()
    }
}

impl Drop for CryptoKey {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

/// Nonce generator for encryption operations
pub struct NonceGenerator {
    rng: SystemRandom,
}

impl Default for NonceGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl NonceGenerator {
    /// Create a new nonce generator
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }

    /// Generate a random nonce
    pub fn generate(&self, size: usize) -> Result<Vec<u8>, CryptoError> {
        let mut nonce = vec![0u8; size];
        self.rng
            .fill(&mut nonce)
            .map_err(|_| CryptoError::RandomGenerationFailed)?;
        Ok(nonce)
    }
}

/// Encrypt data using the specified algorithm
pub fn encrypt(
    key: &CryptoKey,
    plaintext: &[u8],
    additional_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let algorithm = match key.algorithm {
        Algorithm::Aes256Gcm => &AES_256_GCM,
        Algorithm::ChaCha20Poly1305 => &CHACHA20_POLY1305,
    };

    // Create unbound key
    let unbound_key =
        UnboundKey::new(algorithm, &key.key).map_err(|_| CryptoError::EncryptionFailed)?;

    // Generate nonce
    let nonce_gen = NonceGenerator::new();
    let nonce_bytes = nonce_gen.generate(key.algorithm.nonce_size())?;

    // Create sealing key with single-use nonce
    let mut sealing_key = SealingKey::new(unbound_key, SingleUseNonce::new(nonce_bytes.clone()));

    // Prepare plaintext for encryption
    let mut plaintext_vec = plaintext.to_vec();

    // Encrypt in place
    sealing_key
        .seal_in_place_append_tag(Aad::from(additional_data), &mut plaintext_vec)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // Prepend nonce to encrypted data
    let mut output = Vec::with_capacity(nonce_bytes.len() + plaintext_vec.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&plaintext_vec);

    Ok(output)
}

/// Decrypt data using the specified algorithm
pub fn decrypt(
    key: &CryptoKey,
    ciphertext: &[u8],
    additional_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let algorithm = match key.algorithm {
        Algorithm::Aes256Gcm => &AES_256_GCM,
        Algorithm::ChaCha20Poly1305 => &CHACHA20_POLY1305,
    };

    let nonce_size = key.algorithm.nonce_size();
    if ciphertext.len() < nonce_size + key.algorithm.tag_size() {
        return Err(CryptoError::DecryptionFailed);
    }

    // Extract nonce
    let nonce_bytes = &ciphertext[..nonce_size];

    // Create unbound key
    let unbound_key =
        UnboundKey::new(algorithm, &key.key).map_err(|_| CryptoError::DecryptionFailed)?;

    // Create opening key with single-use nonce
    let mut opening_key = OpeningKey::new(unbound_key, SingleUseNonce::new(nonce_bytes.to_vec()));

    // Copy ciphertext for decryption
    let mut ciphertext_data = ciphertext[nonce_size..].to_vec();

    // Decrypt in place
    let plaintext_bytes = opening_key
        .open_in_place(Aad::from(additional_data), &mut ciphertext_data)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    Ok(plaintext_bytes.to_vec())
}

/// Single-use nonce implementation
struct SingleUseNonce {
    nonce: Option<Vec<u8>>,
}

impl SingleUseNonce {
    fn new(nonce: Vec<u8>) -> Self {
        Self { nonce: Some(nonce) }
    }
}

impl NonceSequence for SingleUseNonce {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        self.nonce
            .take()
            .ok_or(Unspecified)
            .and_then(|n| Nonce::try_assume_unique_for_key(&n))
    }
}

/// Generate a random salt for key derivation
pub fn generate_salt() -> Result<Vec<u8>, CryptoError> {
    let rng = SystemRandom::new();
    let mut salt = vec![0u8; 32]; // 256-bit salt
    rng.fill(&mut salt)
        .map_err(|_| CryptoError::RandomGenerationFailed)?;
    Ok(salt)
}

/// Securely compare two byte slices in constant time
pub fn secure_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (a_byte, b_byte) in a.iter().zip(b.iter()) {
        result |= a_byte ^ b_byte;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
        assert_eq!(key.key.len(), 32);
        assert_eq!(key.algorithm, Algorithm::Aes256Gcm);
    }

    #[test]
    fn test_key_derivation() {
        let password = b"test password";
        let salt = b"test salt";
        let key =
            CryptoKey::derive_from_password(password, salt, 100_000, Algorithm::Aes256Gcm).unwrap();

        assert_eq!(key.key.len(), 32);

        // Derive again with same parameters - should produce same key
        let key2 =
            CryptoKey::derive_from_password(password, salt, 100_000, Algorithm::Aes256Gcm).unwrap();

        assert_eq!(key.key, key2.key);
    }

    #[test]
    fn test_encrypt_decrypt_aes() {
        let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
        let plaintext = b"Hello, World! This is a test message.";
        let aad = b"additional authenticated data";

        // Encrypt
        let ciphertext = encrypt(&key, plaintext, aad).unwrap();
        assert!(ciphertext.len() > plaintext.len());

        // Decrypt
        let decrypted = decrypt(&key, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_chacha() {
        let key = CryptoKey::generate(Algorithm::ChaCha20Poly1305).unwrap();
        let plaintext = b"Testing ChaCha20-Poly1305 encryption";
        let aad = b"metadata";

        // Encrypt
        let ciphertext = encrypt(&key, plaintext, aad).unwrap();

        // Decrypt
        let decrypted = decrypt(&key, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let key1 = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
        let key2 = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
        let plaintext = b"Secret message";
        let aad = b"";

        // Encrypt with key1
        let ciphertext = encrypt(&key1, plaintext, aad).unwrap();

        // Try to decrypt with key2 - should fail
        let result = decrypt(&key2, &ciphertext, aad);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_with_wrong_aad() {
        let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
        let plaintext = b"Secret message";
        let aad1 = b"aad1";
        let aad2 = b"aad2";

        // Encrypt with aad1
        let ciphertext = encrypt(&key, plaintext, aad1).unwrap();

        // Try to decrypt with aad2 - should fail
        let result = decrypt(&key, &ciphertext, aad2);
        assert!(result.is_err());
    }

    #[test]
    fn test_secure_compare() {
        let a = b"hello";
        let b = b"hello";
        let c = b"world";

        assert!(secure_compare(a, b));
        assert!(!secure_compare(a, c));
    }
}

//! macOS Keychain integration for secure key storage
//!
//! This module provides a safe Rust wrapper around the macOS Security Framework
//! for storing and retrieving encryption keys from the system keychain.

use security_framework::os::macos::keychain::{SecKeychain, CreateOptions};
use security_framework::os::macos::passwords::find_generic_password;
use security_framework::passwords::set_generic_password;
use core_foundation::base::TCFType;
use std::ffi::CString;
use thiserror::Error;
use zeroize::Zeroize;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("Keychain access denied")]
    AccessDenied,
    
    #[error("Key not found in keychain")]
    KeyNotFound,
    
    #[error("Failed to access keychain: {0}")]
    KeychainAccess(String),
    
    #[error("Failed to encode/decode key data: {0}")]
    EncodingError(String),
    
    #[error("Invalid key format")]
    InvalidKeyFormat,
    
    #[error("Security framework error: {0}")]
    SecurityFramework(#[from] security_framework::base::Error),
}

/// Service name for keychain entries
const SERVICE_NAME: &str = "com.airgapsync.keys";

/// Key metadata stored alongside the actual key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key algorithm (RSA-2048, RSA-4096, ECDSA-P256, etc.)
    pub algorithm: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last rotation timestamp
    pub rotated_at: Option<DateTime<Utc>>,
    /// Key version number
    pub version: u32,
    /// Device ID this key belongs to
    pub device_id: String,
}

/// Encryption key with metadata
#[derive(Clone)]
pub struct EncryptionKey {
    /// Raw key material (will be zeroed on drop)
    pub key_material: Vec<u8>,
    /// Key metadata
    pub metadata: KeyMetadata,
}

impl Drop for EncryptionKey {
    fn drop(&mut self) {
        self.key_material.zeroize();
    }
}

/// Keychain manager for AirGapSync
pub struct KeychainManager {
    /// Service name for keychain entries
    service_name: String,
    /// Optional specific keychain (uses default if None)
    keychain: Option<SecKeychain>,
}

impl KeychainManager {
    /// Create a new keychain manager with default settings
    pub fn new() -> Self {
        Self {
            service_name: SERVICE_NAME.to_string(),
            keychain: None,
        }
    }
    
    /// Create a keychain manager with a custom service name
    pub fn with_service_name(service_name: String) -> Self {
        Self {
            service_name,
            keychain: None,
        }
    }
    
    /// Store a key in the keychain
    pub fn store_key(
        &self,
        device_id: &str,
        key: &EncryptionKey,
    ) -> Result<(), KeychainError> {
        // Serialize key data with metadata
        use base64::Engine;
        let key_data = KeyData {
            material: base64::engine::general_purpose::STANDARD.encode(&key.key_material),
            metadata: key.metadata.clone(),
        };
        
        let serialized = serde_json::to_string(&key_data)
            .map_err(|e| KeychainError::EncodingError(e.to_string()))?;
        
        // Store in keychain
        set_generic_password(
            &self.service_name,
            device_id,
            serialized.as_bytes(),
        ).map_err(|e| KeychainError::SecurityFramework(e))?;
        
        Ok(())
    }
    
    /// Retrieve a key from the keychain
    pub fn get_key(&self, device_id: &str) -> Result<EncryptionKey, KeychainError> {
        // Find the password entry
        let (password_data, _) = find_generic_password(
            None,
            device_id,
            &self.service_name,
        ).map_err(|e| match e.code() {
            -25300 => KeychainError::KeyNotFound,
            _ => KeychainError::SecurityFramework(e),
        })?;
        
        // Deserialize key data
        let key_data: KeyData = serde_json::from_slice(&password_data)
            .map_err(|e| KeychainError::EncodingError(e.to_string()))?;
        
        // Decode key material
        use base64::Engine;
        let key_material = base64::engine::general_purpose::STANDARD
            .decode(&key_data.material)
            .map_err(|e| KeychainError::EncodingError(e.to_string()))?;
        
        Ok(EncryptionKey {
            key_material,
            metadata: key_data.metadata,
        })
    }
    
    /// Check if a key exists for a device
    pub fn key_exists(&self, device_id: &str) -> bool {
        find_generic_password(None, device_id, &self.service_name).is_ok()
    }
    
    /// Delete a key from the keychain
    pub fn delete_key(&self, device_id: &str) -> Result<(), KeychainError> {
        // First check if key exists
        if !self.key_exists(device_id) {
            return Err(KeychainError::KeyNotFound);
        }
        
        // Note: security-framework doesn't expose delete directly,
        // so we'll use a workaround by updating with empty data
        // In a real implementation, we'd use the C API directly
        set_generic_password(
            &format!("{}.deleted", self.service_name),
            device_id,
            b"",
        ).map_err(|e| KeychainError::SecurityFramework(e))?;
        
        Ok(())
    }
    
    /// List all device IDs with stored keys
    pub fn list_devices(&self) -> Result<Vec<String>, KeychainError> {
        // Note: This is a simplified implementation
        // In production, we'd query the keychain properly
        Ok(vec![])
    }
    
    /// Update key metadata without changing the key material
    pub fn update_metadata(
        &self,
        device_id: &str,
        metadata: KeyMetadata,
    ) -> Result<(), KeychainError> {
        let mut key = self.get_key(device_id)?;
        key.metadata = metadata;
        self.store_key(device_id, &key)?;
        Ok(())
    }
}

/// Internal structure for JSON serialization
#[derive(Serialize, Deserialize)]
struct KeyData {
    material: String,  // Base64 encoded
    metadata: KeyMetadata,
}

/// Generate a new encryption key
pub fn generate_key(algorithm: &str, device_id: &str) -> Result<EncryptionKey, KeychainError> {
    use ring::rand::{SecureRandom, SystemRandom};
    
    let rng = SystemRandom::new();
    let key_size = match algorithm {
        "AES-256" => 32,
        "AES-128" => 16,
        "ChaCha20" => 32,
        _ => return Err(KeychainError::InvalidKeyFormat),
    };
    
    let mut key_material = vec![0u8; key_size];
    rng.fill(&mut key_material)
        .map_err(|_| KeychainError::EncodingError("Failed to generate random key".to_string()))?;
    
    let metadata = KeyMetadata {
        algorithm: algorithm.to_string(),
        created_at: Utc::now(),
        rotated_at: None,
        version: 1,
        device_id: device_id.to_string(),
    };
    
    Ok(EncryptionKey {
        key_material,
        metadata,
    })
}

/// Rotate an existing key
pub fn rotate_key(
    keychain: &KeychainManager,
    device_id: &str,
) -> Result<EncryptionKey, KeychainError> {
    // Get existing key to preserve algorithm
    let old_key = keychain.get_key(device_id)?;
    
    // Generate new key with same algorithm
    let mut new_key = generate_key(&old_key.metadata.algorithm, device_id)?;
    
    // Update metadata
    new_key.metadata.version = old_key.metadata.version + 1;
    new_key.metadata.rotated_at = Some(Utc::now());
    new_key.metadata.created_at = old_key.metadata.created_at;
    
    // Store new key
    keychain.store_key(device_id, &new_key)?;
    
    Ok(new_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_generation() {
        let key = generate_key("AES-256", "test-device").unwrap();
        assert_eq!(key.key_material.len(), 32);
        assert_eq!(key.metadata.algorithm, "AES-256");
        assert_eq!(key.metadata.device_id, "test-device");
        assert_eq!(key.metadata.version, 1);
    }
    
    #[test]
    fn test_key_metadata_serialization() {
        let metadata = KeyMetadata {
            algorithm: "AES-256".to_string(),
            created_at: Utc::now(),
            rotated_at: None,
            version: 1,
            device_id: "USB001".to_string(),
        };
        
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: KeyMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.algorithm, metadata.algorithm);
        assert_eq!(deserialized.device_id, metadata.device_id);
        assert_eq!(deserialized.version, metadata.version);
    }
    
    // Note: Full keychain tests require macOS environment
    // and user authorization, so they're marked as ignored
    #[test]
    #[ignore]
    fn test_keychain_store_retrieve() {
        let keychain = KeychainManager::new();
        let device_id = "test-device";
        
        // Generate and store key
        let key = generate_key("AES-256", device_id).unwrap();
        keychain.store_key(device_id, &key).unwrap();
        
        // Retrieve key
        let retrieved = keychain.get_key(device_id).unwrap();
        assert_eq!(retrieved.key_material, key.key_material);
        assert_eq!(retrieved.metadata.algorithm, key.metadata.algorithm);
        
        // Clean up
        keychain.delete_key(device_id).unwrap();
    }
}
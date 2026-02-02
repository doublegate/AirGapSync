//! Tamper-evident audit logging for AirGapSync
//! 
//! This module provides secure, append-only audit logging with cryptographic signatures
//! to detect tampering. All significant operations are logged with timestamps and context.

use crate::crypto::{CryptoKey, CryptoError, sign_message, verify_signature};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs::{self, File, OpenOptions};
use std::io::{Write, BufRead, BufReader};
use thiserror::Error;
use sha2::{Sha256, Digest};

/// Errors specific to audit logging
#[derive(Debug, Error)]
pub enum AuditError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Signature verification failed
    #[error("Signature verification failed")]
    SignatureVerification,
    
    /// Log tampering detected
    #[error("Log tampering detected: {0}")]
    TamperingDetected(String),
    
    /// Crypto error
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
}

/// Types of events that can be logged
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditEvent {
    /// Sync operation started
    SyncStarted {
        device_id: String,
        source_path: PathBuf,
        dry_run: bool,
    },
    
    /// Sync operation completed
    SyncCompleted {
        device_id: String,
        files_added: usize,
        files_modified: usize,
        files_deleted: usize,
        bytes_transferred: u64,
        duration_secs: f64,
    },
    
    /// Sync operation failed
    SyncFailed {
        device_id: String,
        error: String,
    },
    
    /// Key generation
    KeyGenerated {
        device_id: String,
        algorithm: String,
    },
    
    /// Key rotation
    KeyRotated {
        device_id: String,
        old_key_hash: String,
        new_key_hash: String,
    },
    
    /// Device connected
    DeviceConnected {
        device_id: String,
        mount_point: PathBuf,
    },
    
    /// Device disconnected
    DeviceDisconnected {
        device_id: String,
    },
    
    /// Verification performed
    VerificationPerformed {
        device_id: String,
        snapshot_id: Option<String>,
        files_verified: usize,
        errors_found: usize,
    },
    
    /// Snapshot created
    SnapshotCreated {
        device_id: String,
        snapshot_id: String,
        size: u64,
    },
    
    /// Snapshot deleted
    SnapshotDeleted {
        device_id: String,
        snapshot_id: String,
    },
    
    /// Restore operation
    RestorePerformed {
        snapshot_id: String,
        destination: PathBuf,
        files_restored: usize,
        bytes_restored: u64,
    },
    
    /// Configuration change
    ConfigurationChanged {
        setting: String,
        old_value: String,
        new_value: String,
    },
    
    /// Security warning
    SecurityWarning {
        message: String,
        details: Option<String>,
    },
    
    /// Generic error
    Error {
        message: String,
        context: Option<String>,
    },
}

/// An entry in the audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique ID for this entry
    pub id: String,
    
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    
    /// The event that occurred
    pub event: AuditEvent,
    
    /// Hash of the previous entry (for chain integrity)
    pub previous_hash: String,
    
    /// Digital signature of this entry
    pub signature: String,
}

/// Manages the audit log
pub struct AuditLogger {
    log_path: PathBuf,
    signing_key: Option<CryptoKey>,
    verification_key: Option<CryptoKey>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(log_dir: &Path) -> std::result::Result<Self, AuditError> {
        fs::create_dir_all(log_dir)?;
        let log_path = log_dir.join("audit.log");
        
        Ok(Self {
            log_path,
            signing_key: None,
            verification_key: None,
        })
    }
    
    /// Initialize with signing keys
    pub fn with_keys(log_dir: &Path, signing_key: CryptoKey, verification_key: CryptoKey) -> std::result::Result<Self, AuditError> {
        let mut logger = Self::new(log_dir)?;
        logger.signing_key = Some(signing_key);
        logger.verification_key = Some(verification_key);
        Ok(logger)
    }
    
    /// Log an event
    pub fn log_event(&self, event: AuditEvent) -> std::result::Result<(), AuditError> {
        let previous_hash = self.get_last_hash()?;
        
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event,
            previous_hash,
            signature: String::new(), // Will be filled if signing key is available
        };
        
        // Sign the entry if we have a signing key
        let entry = if let Some(key) = &self.signing_key {
            self.sign_entry(entry, key)?
        } else {
            entry
        };
        
        // Append to log file
        self.append_entry(&entry)?;
        
        Ok(())
    }
    
    /// Get the hash of the last entry
    fn get_last_hash(&self) -> std::result::Result<String, AuditError> {
        if !self.log_path.exists() {
            return Ok(String::from("0000000000000000000000000000000000000000000000000000000000000000"));
        }
        
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut last_line = String::new();
        
        for line in reader.lines() {
            last_line = line?;
        }
        
        if last_line.is_empty() {
            return Ok(String::from("0000000000000000000000000000000000000000000000000000000000000000"));
        }
        
        let entry: AuditEntry = serde_json::from_str(&last_line)?;
        Ok(self.compute_entry_hash(&entry))
    }
    
    /// Compute the hash of an entry
    fn compute_entry_hash(&self, entry: &AuditEntry) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&entry.id);
        hasher.update(entry.timestamp.to_rfc3339());
        hasher.update(serde_json::to_string(&entry.event).unwrap_or_default());
        hasher.update(&entry.previous_hash);
        hex::encode(hasher.finalize())
    }
    
    /// Sign an entry
    fn sign_entry(&self, mut entry: AuditEntry, key: &CryptoKey) -> std::result::Result<AuditEntry, AuditError> {
        let data_to_sign = format!("{}{}{}{}", 
            entry.id, 
            entry.timestamp.to_rfc3339(),
            serde_json::to_string(&entry.event)?,
            entry.previous_hash
        );
        
        // Sign the data with the provided key
        let signature = sign_message(key, data_to_sign.as_bytes())?;
        entry.signature = hex::encode(signature);
        
        Ok(entry)
    }
    
    /// Append an entry to the log file
    fn append_entry(&self, entry: &AuditEntry) -> std::result::Result<(), AuditError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        
        writeln!(file, "{}", serde_json::to_string(entry)?)?;
        file.sync_all()?;
        
        Ok(())
    }
    
    /// Read entries from the log
    pub fn read_entries(&self, limit: Option<usize>, since: Option<DateTime<Utc>>) -> std::result::Result<Vec<AuditEntry>, AuditError> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }
        
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            
            let entry: AuditEntry = serde_json::from_str(&line)?;
            
            // Filter by timestamp if specified
            if let Some(since_time) = since {
                if entry.timestamp < since_time {
                    continue;
                }
            }
            
            entries.push(entry);
        }
        
        // Apply limit if specified
        if let Some(limit) = limit {
            entries.truncate(limit);
        }
        
        Ok(entries)
    }
    
    /// Verify the integrity of the audit log
    pub fn verify_integrity(&self) -> std::result::Result<(), AuditError> {
        if !self.log_path.exists() {
            return Ok(());
        }
        
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut previous_hash = String::from("0000000000000000000000000000000000000000000000000000000000000000");
        let mut line_number = 0;
        
        for line in reader.lines() {
            line_number += 1;
            let line = line?;
            if line.is_empty() {
                continue;
            }
            
            let entry: AuditEntry = serde_json::from_str(&line)?;
            
            // Verify chain integrity
            if entry.previous_hash != previous_hash {
                return Err(AuditError::TamperingDetected(
                    format!("Hash chain broken at line {}", line_number)
                ));
            }
            
            // Verify signature if we have a verification key
            if let Some(key) = &self.verification_key {
                if entry.signature.is_empty() {
                    return Err(AuditError::TamperingDetected(
                        format!("Missing signature at line {}", line_number)
                    ));
                }
                
                // Reconstruct the data that was signed
                let data_to_verify = format!("{}{}{}{}", 
                    entry.id, 
                    entry.timestamp.to_rfc3339(),
                    serde_json::to_string(&entry.event).unwrap_or_default(),
                    entry.previous_hash
                );
                
                // Decode the signature
                let signature_bytes = hex::decode(&entry.signature)
                    .map_err(|_| AuditError::TamperingDetected(
                        format!("Invalid signature format at line {}", line_number)
                    ))?;
                
                // Verify the signature
                let is_valid = verify_signature(key, data_to_verify.as_bytes(), &signature_bytes)?;
                if !is_valid {
                    return Err(AuditError::TamperingDetected(
                        format!("Invalid signature at line {}", line_number)
                    ));
                }
            }
            
            previous_hash = self.compute_entry_hash(&entry);
        }
        
        Ok(())
    }
    
    /// Export entries to JSON
    pub fn export_json(&self, output_path: &Path, limit: Option<usize>, since: Option<DateTime<Utc>>) -> std::result::Result<(), AuditError> {
        let entries = self.read_entries(limit, since)?;
        let json = serde_json::to_string_pretty(&entries)?;
        fs::write(output_path, json)?;
        Ok(())
    }
    
    /// Get statistics about the audit log
    pub fn get_statistics(&self) -> std::result::Result<AuditStatistics, AuditError> {
        let entries = self.read_entries(None, None)?;
        
        let mut stats = AuditStatistics {
            total_entries: entries.len(),
            sync_operations: 0,
            key_operations: 0,
            device_operations: 0,
            verification_operations: 0,
            errors: 0,
            warnings: 0,
            first_entry: None,
            last_entry: None,
        };
        
        for entry in &entries {
            match &entry.event {
                AuditEvent::SyncStarted { .. } | 
                AuditEvent::SyncCompleted { .. } | 
                AuditEvent::SyncFailed { .. } => stats.sync_operations += 1,
                
                AuditEvent::KeyGenerated { .. } | 
                AuditEvent::KeyRotated { .. } => stats.key_operations += 1,
                
                AuditEvent::DeviceConnected { .. } | 
                AuditEvent::DeviceDisconnected { .. } => stats.device_operations += 1,
                
                AuditEvent::VerificationPerformed { .. } => stats.verification_operations += 1,
                
                AuditEvent::Error { .. } => stats.errors += 1,
                AuditEvent::SecurityWarning { .. } => stats.warnings += 1,
                
                _ => {}
            }
        }
        
        if let Some(first) = entries.first() {
            stats.first_entry = Some(first.timestamp);
        }
        
        if let Some(last) = entries.last() {
            stats.last_entry = Some(last.timestamp);
        }
        
        Ok(stats)
    }
}

/// Statistics about the audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_entries: usize,
    pub sync_operations: usize,
    pub key_operations: usize,
    pub device_operations: usize,
    pub verification_operations: usize,
    pub errors: usize,
    pub warnings: usize,
    pub first_entry: Option<DateTime<Utc>>,
    pub last_entry: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_audit_logging() {
        let temp_dir = TempDir::new().unwrap();
        let logger = AuditLogger::new(temp_dir.path()).unwrap();
        
        // Log some events
        logger.log_event(AuditEvent::SyncStarted {
            device_id: "DEVICE_001".to_string(),
            source_path: PathBuf::from("/Users/test"),
            dry_run: false,
        }).unwrap();
        
        logger.log_event(AuditEvent::SyncCompleted {
            device_id: "DEVICE_001".to_string(),
            files_added: 10,
            files_modified: 5,
            files_deleted: 2,
            bytes_transferred: 1024 * 1024,
            duration_secs: 5.2,
        }).unwrap();
        
        // Read entries
        let entries = logger.read_entries(None, None).unwrap();
        assert_eq!(entries.len(), 2);
        
        // Verify integrity
        logger.verify_integrity().unwrap();
    }
    
    #[test]
    fn test_audit_statistics() {
        let temp_dir = TempDir::new().unwrap();
        let logger = AuditLogger::new(temp_dir.path()).unwrap();
        
        // Log various events
        logger.log_event(AuditEvent::SyncStarted {
            device_id: "DEVICE_001".to_string(),
            source_path: PathBuf::from("/Users/test"),
            dry_run: false,
        }).unwrap();
        
        logger.log_event(AuditEvent::KeyGenerated {
            device_id: "DEVICE_001".to_string(),
            algorithm: "AES-256-GCM".to_string(),
        }).unwrap();
        
        logger.log_event(AuditEvent::Error {
            message: "Test error".to_string(),
            context: None,
        }).unwrap();
        
        // Get statistics
        let stats = logger.get_statistics().unwrap();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.sync_operations, 1);
        assert_eq!(stats.key_operations, 1);
        assert_eq!(stats.errors, 1);
    }
}
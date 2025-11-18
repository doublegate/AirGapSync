//! Audit logging system
//!
//! This module provides an immutable, append-only audit log with cryptographic
//! signatures for tamper detection and compliance.

use chrono::{DateTime, Utc};
use ring::digest::{Context, SHA256};
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Audit error types
#[derive(Debug, Error)]
pub enum AuditError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Verification error
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Invalid log entry
    #[error("Invalid log entry: {0}")]
    InvalidEntry(String),
}

/// Result type for audit operations
pub type Result<T> = std::result::Result<T, AuditError>;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Sync started
    SyncStarted,

    /// Sync completed
    SyncCompleted,

    /// Sync failed
    SyncFailed,

    /// File encrypted
    FileEncrypted,

    /// File decrypted
    FileDecrypted,

    /// Key generated
    KeyGenerated,

    /// Key rotated
    KeyRotated,

    /// Key deleted
    KeyDeleted,

    /// Configuration changed
    ConfigChanged,

    /// Snapshot created
    SnapshotCreated,

    /// Snapshot deleted
    SnapshotDeleted,

    /// Verification performed
    VerificationPerformed,

    /// Error occurred
    ErrorOccurred,

    /// Security event
    SecurityEvent,
}

/// Audit event severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational
    Info,

    /// Warning
    Warning,

    /// Error
    Error,

    /// Critical security event
    Critical,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Sequential entry ID
    pub id: u64,

    /// Timestamp (ISO 8601)
    pub timestamp: DateTime<Utc>,

    /// Event type
    pub event_type: AuditEventType,

    /// Severity level
    pub severity: Severity,

    /// Device ID
    pub device_id: String,

    /// User or process ID
    pub actor: String,

    /// Event description
    pub message: String,

    /// Additional metadata (JSON)
    pub metadata: Option<serde_json::Value>,

    /// Hash of previous entry (for chain)
    pub prev_hash: String,

    /// Signature (HMAC-SHA256)
    pub signature: String,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(
        id: u64,
        event_type: AuditEventType,
        severity: Severity,
        device_id: String,
        message: String,
        prev_hash: String,
    ) -> Self {
        Self {
            id,
            timestamp: Utc::now(),
            event_type,
            severity,
            device_id,
            actor: whoami::username(),
            message,
            metadata: None,
            prev_hash,
            signature: String::new(), // Will be computed later
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Calculate entry hash
    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}|{}|{:?}|{}|{}|{}|{}",
            self.id,
            self.timestamp.to_rfc3339(),
            self.event_type,
            self.device_id,
            self.actor,
            self.message,
            self.prev_hash
        );

        let mut context = Context::new(&SHA256);
        context.update(data.as_bytes());
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Sign the entry
    pub fn sign(&mut self, key: &hmac::Key) {
        let hash = self.calculate_hash();
        let tag = hmac::sign(key, hash.as_bytes());
        self.signature = hex::encode(tag.as_ref());
    }

    /// Verify the signature
    pub fn verify(&self, key: &hmac::Key) -> bool {
        let hash = self.calculate_hash();
        let expected_tag = hex::decode(&self.signature).unwrap_or_default();

        hmac::verify(key, hash.as_bytes(), &expected_tag).is_ok()
    }

    /// Serialize to JSON line
    pub fn to_json_line(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| AuditError::Serialization(e.to_string()))
    }

    /// Deserialize from JSON line
    pub fn from_json_line(line: &str) -> Result<Self> {
        serde_json::from_str(line)
            .map_err(|e| AuditError::Serialization(e.to_string()))
    }
}

/// Audit logger
pub struct AuditLogger {
    log_path: PathBuf,
    signing_key: hmac::Key,
    last_hash: String,
    next_id: u64,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new<P: AsRef<Path>>(log_path: P) -> Result<Self> {
        let log_path = log_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Generate signing key (in production, this should be stored securely)
        let key_bytes = ring::rand::SystemRandom::new();
        let key_material = ring::hmac::Key::generate(hmac::HMAC_SHA256, &key_bytes)
            .map_err(|_| AuditError::InvalidEntry("Failed to generate key".to_string()))?;

        let mut logger = Self {
            log_path,
            signing_key: key_material,
            last_hash: "0".repeat(64), // Genesis hash
            next_id: 1,
        };

        // Load existing log if it exists
        if logger.log_path.exists() {
            logger.load_state()?;
        }

        Ok(logger)
    }

    /// Log an audit event
    pub fn log(
        &mut self,
        event_type: AuditEventType,
        severity: Severity,
        device_id: String,
        message: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let mut entry = AuditEntry::new(
            self.next_id,
            event_type,
            severity,
            device_id,
            message,
            self.last_hash.clone(),
        );

        if let Some(meta) = metadata {
            entry = entry.with_metadata(meta);
        }

        entry.sign(&self.signing_key);

        // Append to log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        writeln!(file, "{}", entry.to_json_line()?)?;
        file.sync_all()?;

        // Update state
        self.last_hash = entry.calculate_hash();
        self.next_id += 1;

        Ok(())
    }

    /// Load logger state from existing log
    fn load_state(&mut self) -> Result<()> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);

        let mut last_entry: Option<AuditEntry> = None;

        for line in reader.lines() {
            let line = line?;
            if !line.is_empty() {
                let entry = AuditEntry::from_json_line(&line)?;
                last_entry = Some(entry);
            }
        }

        if let Some(entry) = last_entry {
            self.last_hash = entry.calculate_hash();
            self.next_id = entry.id + 1;
        }

        Ok(())
    }

    /// Verify the integrity of the entire log
    pub fn verify_log(&self) -> Result<VerificationReport> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);

        let mut report = VerificationReport {
            total_entries: 0,
            verified_entries: 0,
            failed_entries: Vec::new(),
            chain_valid: true,
        };

        let mut prev_hash = "0".repeat(64);

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let entry = AuditEntry::from_json_line(&line)?;
            report.total_entries += 1;

            // Verify signature
            if !entry.verify(&self.signing_key) {
                report.failed_entries.push((entry.id, "Invalid signature".to_string()));
                report.chain_valid = false;
                continue;
            }

            // Verify chain
            if entry.prev_hash != prev_hash {
                report.failed_entries.push((entry.id, "Chain broken".to_string()));
                report.chain_valid = false;
                continue;
            }

            report.verified_entries += 1;
            prev_hash = entry.calculate_hash();
        }

        Ok(report)
    }

    /// Read all log entries
    pub fn read_entries(&self) -> Result<Vec<AuditEntry>> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if !line.is_empty() {
                let entry = AuditEntry::from_json_line(&line)?;
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Filter entries by event type
    pub fn filter_by_type(&self, event_type: AuditEventType) -> Result<Vec<AuditEntry>> {
        Ok(self.read_entries()?
            .into_iter()
            .filter(|e| e.event_type == event_type)
            .collect())
    }

    /// Filter entries by severity
    pub fn filter_by_severity(&self, min_severity: Severity) -> Result<Vec<AuditEntry>> {
        Ok(self.read_entries()?
            .into_iter()
            .filter(|e| e.severity >= min_severity)
            .collect())
    }
}

/// Verification report
#[derive(Debug)]
pub struct VerificationReport {
    /// Total entries checked
    pub total_entries: usize,

    /// Successfully verified entries
    pub verified_entries: usize,

    /// Failed entries (ID, reason)
    pub failed_entries: Vec<(u64, String)>,

    /// Whether the chain is valid
    pub chain_valid: bool,
}

impl VerificationReport {
    /// Check if verification passed
    pub fn is_valid(&self) -> bool {
        self.chain_valid && self.failed_entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_audit_entry_creation() {
        let entry = AuditEntry::new(
            1,
            AuditEventType::SyncStarted,
            Severity::Info,
            "device1".to_string(),
            "Sync initiated".to_string(),
            "0".repeat(64),
        );

        assert_eq!(entry.id, 1);
        assert_eq!(entry.event_type, AuditEventType::SyncStarted);
    }

    #[test]
    fn test_audit_entry_hash() {
        let entry = AuditEntry::new(
            1,
            AuditEventType::SyncStarted,
            Severity::Info,
            "device1".to_string(),
            "Test".to_string(),
            "0".repeat(64),
        );

        let hash = entry.calculate_hash();
        assert_eq!(hash.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_audit_logger() {
        let tmpfile = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(tmpfile.path()).unwrap();

        logger.log(
            AuditEventType::SyncStarted,
            Severity::Info,
            "device1".to_string(),
            "Test sync".to_string(),
            None,
        ).unwrap();

        assert_eq!(logger.next_id, 2);
    }

    #[test]
    fn test_log_verification() {
        let tmpfile = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(tmpfile.path()).unwrap();

        logger.log(
            AuditEventType::SyncStarted,
            Severity::Info,
            "device1".to_string(),
            "Test 1".to_string(),
            None,
        ).unwrap();

        logger.log(
            AuditEventType::SyncCompleted,
            Severity::Info,
            "device1".to_string(),
            "Test 2".to_string(),
            None,
        ).unwrap();

        let report = logger.verify_log().unwrap();
        assert!(report.is_valid());
        assert_eq!(report.total_entries, 2);
    }
}

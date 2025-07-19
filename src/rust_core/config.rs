//! Configuration management for AirGapSync
//! 
//! This module defines the TOML configuration schema and provides
//! serialization/deserialization support with validation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to parse TOML: {0}")]
    ParseError(#[from] toml::de::Error),
    
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    
    #[error("Failed to serialize configuration: {0}")]
    SerializationError(#[from] toml::ser::Error),
}

/// Main configuration structure for AirGapSync
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    /// General settings
    #[serde(default)]
    pub general: GeneralConfig,
    
    /// Source directory configuration
    pub source: SourceConfig,
    
    /// Device configurations (can have multiple)
    pub device: Vec<DeviceConfig>,
    
    /// Retention and cleanup policies
    #[serde(default)]
    pub policy: PolicyConfig,
    
    /// Security settings
    #[serde(default)]
    pub security: SecurityConfig,
    
    /// Schedule settings (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<ScheduleConfig>,
    
    /// Notification preferences
    #[serde(default)]
    pub notifications: NotificationConfig,
    
    /// Advanced settings
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeneralConfig {
    /// Enable verbose logging
    #[serde(default = "default_false")]
    pub verbose: bool,
    
    /// Log file location (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,
    
    /// Number of worker threads (0 = auto-detect)
    #[serde(default)]
    pub threads: usize,
}

/// Source directory configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceConfig {
    /// Source directory path
    pub path: PathBuf,
    
    /// Exclude patterns (gitignore syntax)
    #[serde(default)]
    pub exclude: Vec<String>,
    
    /// Follow symbolic links
    #[serde(default = "default_false")]
    pub follow_symlinks: bool,
    
    /// Include hidden files
    #[serde(default = "default_false")]
    pub include_hidden: bool,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeviceConfig {
    /// Unique device identifier
    pub id: String,
    
    /// Human-readable device name
    pub name: String,
    
    /// Mount point path
    pub mount_point: PathBuf,
    
    /// Device-specific encryption settings
    #[serde(default)]
    pub encryption: EncryptionConfig,
}

/// Encryption configuration for a device
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EncryptionConfig {
    /// Encryption algorithm
    #[serde(default = "default_encryption_algorithm")]
    pub algorithm: EncryptionAlgorithm,
    
    /// Key derivation function
    #[serde(default = "default_key_derivation")]
    pub key_derivation: KeyDerivation,
    
    /// PBKDF2 iterations (if using PBKDF2)
    #[serde(default = "default_pbkdf2_iterations")]
    pub iterations: u32,
}

/// Supported encryption algorithms
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EncryptionAlgorithm {
    /// AES-256 in GCM mode
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

/// Key derivation functions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KeyDerivation {
    /// PBKDF2 with SHA-256
    Pbkdf2,
    /// Argon2id
    Argon2,
}

/// Retention and cleanup policies
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PolicyConfig {
    /// Number of snapshots to retain
    #[serde(default = "default_retain_snapshots")]
    pub retain_snapshots: u32,
    
    /// Keep snapshots for N days
    #[serde(default = "default_retain_days")]
    pub retain_days: u32,
    
    /// Run garbage collection every N hours
    #[serde(default = "default_gc_interval_hours")]
    pub gc_interval_hours: u32,
    
    /// Verify data after writing
    #[serde(default = "default_true")]
    pub verify_after_write: bool,
    
    /// Compression level (0-9, 0=none)
    #[serde(default = "default_compression_level")]
    pub compression_level: u8,
    
    /// Chunk size in MB
    #[serde(default = "default_chunk_size_mb")]
    pub chunk_size_mb: u32,
    
    /// Number of files to process in parallel
    #[serde(default = "default_parallel_files")]
    pub parallel_files: u32,
    
    /// I/O buffer size in KB
    #[serde(default = "default_buffer_size_kb")]
    pub buffer_size_kb: u32,
}

/// Security settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecurityConfig {
    /// Key rotation interval in days
    #[serde(default = "default_key_rotation_days")]
    pub key_rotation_days: u32,
    
    /// Require macOS authentication for operations
    #[serde(default = "default_true")]
    pub require_authentication: bool,
    
    /// Audit logging level
    #[serde(default = "default_audit_level")]
    pub audit_level: AuditLevel,
    
    /// Audit log retention in days
    #[serde(default = "default_audit_retention_days")]
    pub audit_retention_days: u32,
}

/// Audit logging levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
    /// No audit logging
    None,
    /// Basic operation logging
    Basic,
    /// Full audit trail
    Full,
}

/// Schedule configuration for automatic syncs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScheduleConfig {
    /// Cron expression for scheduling
    pub schedule: String,
    
    /// Only sync when on AC power
    #[serde(default = "default_true")]
    pub require_ac_power: bool,
    
    /// Prevent system sleep during sync
    #[serde(default = "default_true")]
    pub prevent_sleep: bool,
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotificationConfig {
    /// Notify on sync start
    #[serde(default = "default_false")]
    pub notify_on_start: bool,
    
    /// Notify on sync completion
    #[serde(default = "default_true")]
    pub notify_on_complete: bool,
    
    /// Notify on errors
    #[serde(default = "default_true")]
    pub notify_on_error: bool,
    
    /// Play sound on completion
    #[serde(default = "default_true")]
    pub sound_on_complete: bool,
    
    /// Play sound on error
    #[serde(default = "default_true")]
    pub sound_on_error: bool,
}

/// Advanced settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AdvancedConfig {
    /// Snapshot format version
    #[serde(default = "default_snapshot_version")]
    pub snapshot_version: u32,
    
    /// Enable experimental deduplication
    #[serde(default = "default_false")]
    pub experimental_dedup: bool,
    
    /// Enable experimental delta sync
    #[serde(default = "default_false")]
    pub experimental_delta_sync: bool,
    
    /// Enable debug encryption output
    #[serde(default = "default_false")]
    pub debug_encryption: bool,
    
    /// Enable performance debugging
    #[serde(default = "default_false")]
    pub debug_performance: bool,
    
    /// Save sync report after each operation
    #[serde(default = "default_true")]
    pub save_sync_report: bool,
}

// Default value functions
fn default_false() -> bool { false }
fn default_true() -> bool { true }
fn default_encryption_algorithm() -> EncryptionAlgorithm { EncryptionAlgorithm::Aes256Gcm }
fn default_key_derivation() -> KeyDerivation { KeyDerivation::Pbkdf2 }
fn default_pbkdf2_iterations() -> u32 { 100_000 }
fn default_retain_snapshots() -> u32 { 7 }
fn default_retain_days() -> u32 { 30 }
fn default_gc_interval_hours() -> u32 { 24 }
fn default_compression_level() -> u8 { 3 }
fn default_chunk_size_mb() -> u32 { 1 }
fn default_parallel_files() -> u32 { 4 }
fn default_buffer_size_kb() -> u32 { 1024 }
fn default_key_rotation_days() -> u32 { 90 }
fn default_audit_level() -> AuditLevel { AuditLevel::Full }
fn default_audit_retention_days() -> u32 { 365 }
fn default_snapshot_version() -> u32 { 1 }

// Default trait implementations
impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            log_file: None,
            threads: 0,
        }
    }
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            algorithm: default_encryption_algorithm(),
            key_derivation: default_key_derivation(),
            iterations: default_pbkdf2_iterations(),
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            retain_snapshots: default_retain_snapshots(),
            retain_days: default_retain_days(),
            gc_interval_hours: default_gc_interval_hours(),
            verify_after_write: true,
            compression_level: default_compression_level(),
            chunk_size_mb: default_chunk_size_mb(),
            parallel_files: default_parallel_files(),
            buffer_size_kb: default_buffer_size_kb(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            key_rotation_days: default_key_rotation_days(),
            require_authentication: true,
            audit_level: default_audit_level(),
            audit_retention_days: default_audit_retention_days(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            notify_on_start: false,
            notify_on_complete: true,
            notify_on_error: true,
            sound_on_complete: true,
            sound_on_error: true,
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            snapshot_version: default_snapshot_version(),
            experimental_dedup: false,
            experimental_delta_sync: false,
            debug_encryption: false,
            debug_performance: false,
            save_sync_report: true,
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }
    
    /// Save configuration to a file
    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Ensure at least one device is configured
        if self.device.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one device must be configured".to_string()
            ));
        }
        
        // Validate source path exists
        if !self.source.path.exists() {
            return Err(ConfigError::ValidationError(
                format!("Source path does not exist: {:?}", self.source.path)
            ));
        }
        
        // Validate device IDs are unique
        let mut device_ids = std::collections::HashSet::new();
        for device in &self.device {
            if !device_ids.insert(&device.id) {
                return Err(ConfigError::ValidationError(
                    format!("Duplicate device ID: {}", device.id)
                ));
            }
        }
        
        // Validate compression level
        if self.policy.compression_level > 9 {
            return Err(ConfigError::ValidationError(
                "Compression level must be between 0-9".to_string()
            ));
        }
        
        // Validate chunk size
        if self.policy.chunk_size_mb == 0 {
            return Err(ConfigError::ValidationError(
                "Chunk size must be greater than 0".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Get default configuration path
    pub fn default_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| ConfigError::ValidationError(
                "Could not determine config directory".to_string()
            ))?;
        Ok(config_dir.join("airgapsync").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_config_serialization() {
        let config = Config {
            general: GeneralConfig::default(),
            source: SourceConfig {
                path: PathBuf::from("/Users/test/Documents"),
                exclude: vec!["*.tmp".to_string()],
                follow_symlinks: false,
                include_hidden: false,
            },
            device: vec![DeviceConfig {
                id: "USB001".to_string(),
                name: "Test USB".to_string(),
                mount_point: PathBuf::from("/Volumes/USB001"),
                encryption: EncryptionConfig::default(),
            }],
            policy: PolicyConfig::default(),
            security: SecurityConfig::default(),
            schedule: None,
            notifications: NotificationConfig::default(),
            advanced: AdvancedConfig::default(),
        };
        
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(parsed.source.path, config.source.path);
        assert_eq!(parsed.device.len(), 1);
        assert_eq!(parsed.device[0].id, "USB001");
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config {
            general: GeneralConfig::default(),
            source: SourceConfig {
                path: PathBuf::from("/nonexistent/path"),
                exclude: vec![],
                follow_symlinks: false,
                include_hidden: false,
            },
            device: vec![],
            policy: PolicyConfig::default(),
            security: SecurityConfig::default(),
            schedule: None,
            notifications: NotificationConfig::default(),
            advanced: AdvancedConfig::default(),
        };
        
        // Should fail with no devices
        assert!(config.validate().is_err());
        
        // Add a device
        config.device.push(DeviceConfig {
            id: "USB001".to_string(),
            name: "Test USB".to_string(),
            mount_point: PathBuf::from("/Volumes/USB001"),
            encryption: EncryptionConfig::default(),
        });
        
        // Should still fail with nonexistent source path
        assert!(config.validate().is_err());
    }
}
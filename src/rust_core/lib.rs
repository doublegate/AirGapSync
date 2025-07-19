//! AirGapSync Rust Core Library
//!
//! This library provides the core functionality for secure file synchronization
//! to removable media with encryption and air-gap security principles.

#![warn(missing_docs)]
#![deny(unsafe_code)]

// Feature gates
#[cfg(not(target_os = "macos"))]
compile_error!("AirGapSync currently only supports macOS");

// Module declarations
pub mod config;
pub mod crypto;
#[cfg(target_os = "macos")]
pub mod keychain;
pub mod keys;
pub mod schema;

// Re-exports for convenience
pub use config::{Config, ConfigError};
pub use crypto::{Algorithm as EncryptionAlgorithm, CryptoError, CryptoKey};
#[cfg(target_os = "macos")]
pub use keychain::{EncryptionKey, KeychainError, KeychainManager};
pub use keys::{AsymmetricAlgorithm, AsymmetricKey, KeyAgreement};

use thiserror::Error;

/// Main error type for the AirGapSync library
#[derive(Debug, Error)]
pub enum AirGapError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Cryptography error
    #[error("Cryptography error: {0}")]
    Crypto(#[from] CryptoError),

    /// Keychain error
    #[cfg(target_os = "macos")]
    #[error("Keychain error: {0}")]
    Keychain(#[from] KeychainError),

    /// Key error
    #[error("Key error: {0}")]
    Key(#[from] keys::KeyError),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// Sync operation error
    #[error("Sync error: {0}")]
    SyncError(String),
}

/// Result type alias for AirGapSync operations
pub type Result<T> = std::result::Result<T, AirGapError>;

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Package authors information
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// Initialize the AirGapSync library
///
/// This should be called once at application startup to:
/// - Initialize logging
/// - Verify system requirements
/// - Set up any global state
pub fn initialize() -> Result<()> {
    // Initialize logger if not already done
    let _ = env_logger::try_init();

    log::info!("Initializing AirGapSync v{VERSION}");

    // Verify we're on macOS
    #[cfg(not(target_os = "macos"))]
    {
        return Err(AirGapError::SyncError(
            "AirGapSync requires macOS for Keychain integration".to_string(),
        ));
    }

    // Check for required system capabilities
    verify_system_requirements()?;

    log::info!("AirGapSync initialized successfully");
    Ok(())
}

/// Verify system requirements
fn verify_system_requirements() -> Result<()> {
    // Check macOS version (10.15+ required)
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let output = Command::new("sw_vers").arg("-productVersion").output()?;

        let version = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = version.trim().split('.').collect();

        if parts.len() >= 2 {
            let major: u32 = parts[0].parse().unwrap_or(0);
            let minor: u32 = parts[1].parse().unwrap_or(0);

            if major < 10 || (major == 10 && minor < 15) {
                return Err(AirGapError::SyncError(
                    "macOS 10.15 or later required".to_string(),
                ));
            }
        }
    }

    Ok(())
}

/// Get library information
pub fn get_info() -> String {
    format!(
        "AirGapSync v{VERSION} by {AUTHORS}\nEncrypted Removable-Media Sync Manager"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize() {
        let result = initialize();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_info() {
        let info = get_info();
        assert!(info.contains("AirGapSync"));
        assert!(info.contains(VERSION));
    }
}

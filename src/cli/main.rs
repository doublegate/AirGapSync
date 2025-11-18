//! AirGapSync CLI - Phase 1 Implementation
//!
//! This CLI demonstrates Phase 1 functionality including configuration
//! management, key generation, and basic encryption operations.

use airgap_sync::*;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[clap(
    name = "AirGapSync",
    version = "0.1.0",
    author = "DoubleGate <parobek@gmail.com>",
    about = "Encrypted Removable-Media Sync Manager"
)]
struct Cli {
    /// Enable verbose output
    #[clap(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[clap(short, long, global = true)]
    config: Option<PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration
    Init {
        /// Output path for configuration file
        #[clap(short, long, default_value = "~/.airgapsync/config.toml")]
        output: String,
    },

    /// Generate encryption keys
    Keygen {
        /// Device ID
        device_id: String,

        /// Algorithm (aes-256, rsa-2048, ecdsa-p256)
        #[clap(short, long, default_value = "aes-256")]
        algorithm: String,
    },

    /// List stored keys
    Keys,

    /// Rotate encryption key
    Rotate {
        /// Device ID
        device_id: String,
    },

    /// Encrypt a file (demonstration)
    Encrypt {
        /// Input file
        input: PathBuf,

        /// Output file
        output: PathBuf,

        /// Device ID for key
        device_id: String,
    },

    /// Decrypt a file (demonstration)
    Decrypt {
        /// Input file
        input: PathBuf,

        /// Output file
        output: PathBuf,

        /// Device ID for key
        device_id: String,
    },

    /// Validate configuration
    Validate {
        /// Configuration file path
        #[clap(short, long)]
        config: Option<PathBuf>,
    },

    /// Generate JSON schema
    Schema {
        /// Output file path
        #[clap(short, long, default_value = "config-schema.json")]
        output: PathBuf,
    },

    /// Show system information
    Info,

    /// Synchronize source to device
    Sync {
        /// Source directory
        #[clap(long)]
        src: PathBuf,

        /// Destination device or path
        #[clap(long)]
        dest: PathBuf,

        /// Device ID
        #[clap(long)]
        device_id: String,

        /// Dry run (don't make changes)
        #[clap(long)]
        dry_run: bool,

        /// Full sync (ignore previous state)
        #[clap(long)]
        full: bool,
    },

    /// List snapshots on device
    Snapshots {
        /// Destination device or path
        #[clap(long)]
        dest: PathBuf,
    },

    /// Verify snapshot integrity
    Verify {
        /// Destination device or path
        #[clap(long)]
        dest: PathBuf,

        /// Snapshot ID to verify
        snapshot_id: String,
    },

    /// Restore from snapshot
    Restore {
        /// Destination device or path
        #[clap(long)]
        dest: PathBuf,

        /// Snapshot ID to restore
        snapshot_id: String,

        /// Restore target path
        #[clap(long)]
        target: PathBuf,

        /// Device ID for decryption
        #[clap(long)]
        device_id: String,
    },

    /// View audit log
    Audit {
        /// Destination device or path
        #[clap(long)]
        dest: PathBuf,

        /// Filter by event type
        #[clap(long)]
        filter: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Initialize library
    airgap_sync::initialize().context("Failed to initialize AirGapSync")?;

    match cli.command {
        Commands::Init { output } => cmd_init(&output),
        Commands::Keygen {
            device_id,
            algorithm,
        } => cmd_keygen(&device_id, &algorithm),
        Commands::Keys => cmd_list_keys(),
        Commands::Rotate { device_id } => cmd_rotate(&device_id),
        Commands::Encrypt {
            input,
            output,
            device_id,
        } => cmd_encrypt(&input, &output, &device_id),
        Commands::Decrypt {
            input,
            output,
            device_id,
        } => cmd_decrypt(&input, &output, &device_id),
        Commands::Validate { config } => cmd_validate(config),
        Commands::Schema { output } => cmd_schema(&output),
        Commands::Info => cmd_info(),
        Commands::Sync {
            src,
            dest,
            device_id,
            dry_run,
            full,
        } => cmd_sync(&src, &dest, &device_id, dry_run, full),
        Commands::Snapshots { dest } => cmd_snapshots(&dest),
        Commands::Verify { dest, snapshot_id } => cmd_verify(&dest, &snapshot_id),
        Commands::Restore {
            dest,
            snapshot_id,
            target,
            device_id,
        } => cmd_restore(&dest, &snapshot_id, &target, &device_id),
        Commands::Audit { dest, filter } => cmd_audit(&dest, filter.as_deref()),
    }
}

fn cmd_init(output: &str) -> Result<()> {
    use airgap_sync::config::*;

    println!("Initializing AirGapSync configuration...");

    // Expand tilde in path
    let output_path = shellexpand::tilde(output);
    let path = PathBuf::from(output_path.as_ref());

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create example configuration
    let config = Config {
        general: GeneralConfig {
            verbose: false,
            log_file: Some(PathBuf::from("~/.airgapsync/sync.log")),
            threads: 0,
        },
        source: SourceConfig {
            path: PathBuf::from("~/Documents"),
            exclude: vec![
                "*.tmp".to_string(),
                ".DS_Store".to_string(),
                "node_modules/".to_string(),
            ],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "USB001".to_string(),
            name: "Secure Backup USB".to_string(),
            mount_point: PathBuf::from("/Volumes/SecureUSB"),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    // Write configuration
    config.save(&path)?;

    println!("✓ Configuration initialized at: {}", path.display());
    println!("\nNext steps:");
    println!("1. Edit {} to customize settings", path.display());
    println!("2. Run 'airgapsync keygen USB001' to generate encryption keys");
    println!("3. Connect your removable device and update the mount_point");

    Ok(())
}

fn cmd_keygen(device_id: &str, algorithm: &str) -> Result<()> {
    println!("Generating {algorithm} key for device: {device_id}");

    #[cfg(target_os = "macos")]
    {
        use airgap_sync::keychain::*;

        let keychain = KeychainManager::new();

        // Check if key already exists
        if keychain.key_exists(device_id) {
            anyhow::bail!(
                "Key already exists for device: {}. Use 'rotate' to generate a new key.",
                device_id
            );
        }

        // Generate key based on algorithm
        let key = match algorithm {
            "aes-256" => generate_key("AES-256", device_id)?,
            "aes-128" => generate_key("AES-128", device_id)?,
            "chacha20" => generate_key("ChaCha20", device_id)?,
            _ => {
                // Try asymmetric keys
                use airgap_sync::keys::*;
                let asym_alg = match algorithm {
                    "rsa-2048" => AsymmetricAlgorithm::Rsa2048,
                    "rsa-4096" => AsymmetricAlgorithm::Rsa4096,
                    "ecdsa-p256" => AsymmetricAlgorithm::EcdsaP256,
                    "ecdsa-p384" => AsymmetricAlgorithm::EcdsaP384,
                    _ => anyhow::bail!("Unsupported algorithm: {}", algorithm),
                };

                let asym_key = AsymmetricKey::generate(asym_alg)?;
                println!("Generated {} key pair", asym_alg.as_str());
                println!("Public key:\n{}", asym_key.public_key_pem());

                // Display key information
                return Ok(());
            }
        };

        // Store in keychain
        keychain.store_key(device_id, &key)?;

        println!("✓ {algorithm} key generated and stored in keychain");
        println!("  Device ID: {device_id}");
        println!("  Algorithm: {}", key.metadata.algorithm);
        println!(
            "  Created: {}",
            key.metadata.created_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Keychain integration requires macOS");
    }

    Ok(())
}

fn cmd_list_keys() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use airgap_sync::keychain::*;

        let keychain = KeychainManager::new();

        println!("Stored encryption keys:");
        println!(
            "{:<20} {:<15} {:<10} {:<20}",
            "Device ID", "Algorithm", "Version", "Created"
        );
        println!("{}", "-".repeat(70));

        // Check stored device keys in keychain
        // Note: Using common device ID patterns for demo
        for device_id in &[
            "USB001",
            "USB002",
            "SSD001",
            "TEST001",
            "BACKUP001",
            "EXTERNAL001",
        ] {
            if keychain.key_exists(device_id) {
                if let Ok(key) = keychain.get_key(device_id) {
                    println!(
                        "{:<20} {:<15} {:<10} {:<20}",
                        device_id,
                        key.metadata.algorithm,
                        key.metadata.version,
                        key.metadata.created_at.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Keychain integration requires macOS");
    }

    Ok(())
}

fn cmd_rotate(device_id: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use airgap_sync::keychain::*;

        println!("Rotating key for device: {device_id}");

        let keychain = KeychainManager::new();
        let new_key = rotate_key(&keychain, device_id)?;

        println!("✓ Key rotated successfully");
        println!("  New version: {}", new_key.metadata.version);
        println!(
            "  Rotated at: {}",
            new_key
                .metadata
                .rotated_at
                .unwrap()
                .format("%Y-%m-%d %H:%M:%S")
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Keychain integration requires macOS");
    }

    Ok(())
}

fn cmd_encrypt(input: &PathBuf, output: &PathBuf, device_id: &str) -> Result<()> {
    use airgap_sync::crypto::*;

    println!("Encrypting {} -> {}", input.display(), output.display());

    #[cfg(target_os = "macos")]
    {
        use airgap_sync::keychain::*;

        // Get key from keychain
        let keychain = KeychainManager::new();
        let key_data = keychain.get_key(device_id)?;

        // Create crypto key
        let algorithm = match key_data.metadata.algorithm.as_str() {
            "AES-256" => EncryptionAlgorithm::Aes256Gcm,
            "ChaCha20" => EncryptionAlgorithm::ChaCha20Poly1305,
            _ => anyhow::bail!("Unsupported algorithm for encryption"),
        };

        let key = CryptoKey::new(key_data.key_material.clone(), algorithm)?;

        // Read input file
        let plaintext = std::fs::read(input)?;
        let metadata = format!("file:{}", input.file_name().unwrap().to_string_lossy());

        // Encrypt
        let ciphertext = encrypt(&key, &plaintext, metadata.as_bytes())?;

        // Write output
        std::fs::write(output, &ciphertext)?;

        println!("✓ File encrypted successfully");
        println!("  Input size: {} bytes", plaintext.len());
        println!("  Output size: {} bytes", ciphertext.len());
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Keychain integration requires macOS");
    }

    Ok(())
}

fn cmd_decrypt(input: &PathBuf, output: &PathBuf, device_id: &str) -> Result<()> {
    use airgap_sync::crypto::*;

    println!("Decrypting {} -> {}", input.display(), output.display());

    #[cfg(target_os = "macos")]
    {
        use airgap_sync::keychain::*;

        // Get key from keychain
        let keychain = KeychainManager::new();
        let key_data = keychain.get_key(device_id)?;

        // Create crypto key
        let algorithm = match key_data.metadata.algorithm.as_str() {
            "AES-256" => EncryptionAlgorithm::Aes256Gcm,
            "ChaCha20" => EncryptionAlgorithm::ChaCha20Poly1305,
            _ => anyhow::bail!("Unsupported algorithm for decryption"),
        };

        let key = CryptoKey::new(key_data.key_material.clone(), algorithm)?;

        // Read input file
        let ciphertext = std::fs::read(input)?;
        let metadata = format!("file:{}", output.file_name().unwrap().to_string_lossy());

        // Decrypt
        let plaintext = decrypt(&key, &ciphertext, metadata.as_bytes())?;

        // Write output
        std::fs::write(output, &plaintext)?;

        println!("✓ File decrypted successfully");
        println!("  Output size: {} bytes", plaintext.len());
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Keychain integration requires macOS");
    }

    Ok(())
}

fn cmd_validate(config_path: Option<PathBuf>) -> Result<()> {
    use airgap_sync::config::*;

    let path = config_path
        .unwrap_or_else(|| Config::default_path().expect("Failed to get default config path"));

    println!("Validating configuration: {}", path.display());

    match Config::from_file(&path) {
        Ok(config) => {
            println!("✓ Configuration is valid");
            println!("\nConfiguration summary:");
            println!("  Source: {}", config.source.path.display());
            println!("  Devices: {}", config.device.len());
            for device in &config.device {
                println!("    - {} ({})", device.name, device.id);
            }
            println!(
                "  Retention: {} snapshots / {} days",
                config.policy.retain_snapshots, config.policy.retain_days
            );
        }
        Err(e) => {
            println!("✗ Configuration validation failed:");
            println!("  {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn cmd_schema(output: &Path) -> Result<()> {
    use airgap_sync::schema::*;

    println!("Generating JSON schema...");

    write_schema_to_file(output)?;

    println!("✓ Schema written to: {}", output.display());
    println!("\nYou can use this schema to:");
    println!("- Validate configuration files");
    println!("- Generate documentation");
    println!("- Enable IDE auto-completion");

    Ok(())
}

fn cmd_info() -> Result<()> {
    println!("{}", airgap_sync::get_info());
    println!();
    println!("System Information:");

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let version_output = Command::new("sw_vers").arg("-productVersion").output()?;
        let version = String::from_utf8_lossy(&version_output.stdout);
        println!("  macOS Version: {}", version.trim());
    }

    // Get Rust version at runtime
    if let Ok(rustc_output) = std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        let rustc_version = String::from_utf8_lossy(&rustc_output.stdout);
        println!("  Rust Version: {}", rustc_version.trim());
    } else {
        println!("  Rust Version: unknown");
    }
    println!();
    println!("Phase 1 Features:");
    println!("  ✓ TOML configuration with validation");
    println!("  ✓ JSON schema generation");
    println!("  ✓ macOS Keychain integration");
    println!("  ✓ AES-256-GCM encryption");
    println!("  ✓ ChaCha20-Poly1305 encryption");
    println!("  ✓ RSA key generation (2048/4096)");
    println!("  ✓ ECDSA key generation (P-256/P-384)");
    println!("  ✓ Key rotation support");

    Ok(())
}

fn cmd_sync(
    src: &Path,
    dest: &Path,
    device_id: &str,
    dry_run: bool,
    full: bool,
) -> Result<()> {
    println!("Starting sync operation...");
    println!("  Source: {}", src.display());
    println!("  Destination: {}", dest.display());
    println!("  Device ID: {}", device_id);

    #[cfg(target_os = "macos")]
    {
        use airgap_sync::crypto::*;
        use airgap_sync::keychain::*;
        use airgap_sync::sync::*;

        // Get encryption key from keychain
        let keychain = KeychainManager::new();
        let key_data = keychain.get_key(device_id)?;

        let algorithm = match key_data.metadata.algorithm.as_str() {
            "AES-256" => EncryptionAlgorithm::Aes256Gcm,
            "ChaCha20" => EncryptionAlgorithm::ChaCha20Poly1305,
            _ => anyhow::bail!("Unsupported algorithm for sync"),
        };

        let crypto_key = CryptoKey::new(key_data.key_material.clone(), algorithm)?;

        // Create sync configuration
        let mode = if dry_run {
            SyncMode::DryRun
        } else if full {
            SyncMode::Full
        } else {
            SyncMode::Incremental
        };

        let config = SyncConfig::new(src, dest, device_id.to_string()).with_mode(mode);

        // Create sync engine
        let mut engine = SyncEngine::new(config, crypto_key.clone())?;

        // Set up progress callback
        engine.set_progress_callback(|progress| {
            let percent = progress.percentage();
            println!(
                "[{:>3.0}%] {}: {} / {} files ({} / {} bytes)",
                percent,
                progress.current_operation,
                progress.processed_files,
                progress.total_files,
                progress.transferred_bytes,
                progress.total_bytes
            );
        });

        // Perform sync
        let result = engine.sync(&crypto_key)?;

        println!("\n✓ Sync completed successfully");
        println!("  Snapshot ID: {}", result.snapshot_id);
        println!("  Files synced: {}", result.files_synced);
        println!("  Bytes transferred: {}", result.bytes_transferred);
        println!("  Duration: {} seconds", result.duration_seconds);
        println!("\nChangeset:");
        println!("  Added: {}", result.changeset.stats.added);
        println!("  Modified: {}", result.changeset.stats.modified);
        println!("  Deleted: {}", result.changeset.stats.deleted);
        println!("  Moved: {}", result.changeset.stats.moved);

        if !result.errors.is_empty() {
            println!("\nErrors encountered:");
            for error in &result.errors {
                println!("  - {}", error);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Sync requires macOS for Keychain integration");
    }

    Ok(())
}

fn cmd_snapshots(dest: &Path) -> Result<()> {
    use airgap_sync::sync::*;

    println!("Listing snapshots on: {}", dest.display());

    let config = SyncConfig::new("/tmp", dest, "temp".to_string());
    let dummy_key = CryptoKey::new(vec![0u8; 32], EncryptionAlgorithm::Aes256Gcm)?;
    let engine = SyncEngine::new(config, dummy_key)?;

    let snapshots = engine.list_snapshots()?;

    if snapshots.is_empty() {
        println!("No snapshots found");
    } else {
        println!("\nAvailable snapshots:");
        println!("{:<40} {}", "Snapshot ID", "Date");
        println!("{}", "-".repeat(60));
        for snapshot in snapshots {
            println!("{}", snapshot);
        }
    }

    Ok(())
}

fn cmd_verify(dest: &Path, snapshot_id: &str) -> Result<()> {
    use airgap_sync::sync::*;

    println!("Verifying snapshot: {}", snapshot_id);
    println!("  Location: {}", dest.display());

    let config = SyncConfig::new("/tmp", dest, "temp".to_string());
    let dummy_key = CryptoKey::new(vec![0u8; 32], EncryptionAlgorithm::Aes256Gcm)?;
    let engine = SyncEngine::new(config, dummy_key)?;

    let is_valid = engine.verify_snapshot(snapshot_id)?;

    if is_valid {
        println!("✓ Snapshot is valid");
    } else {
        println!("✗ Snapshot verification failed");
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_restore(dest: &Path, snapshot_id: &str, target: &Path, device_id: &str) -> Result<()> {
    println!("Restoring snapshot: {}", snapshot_id);
    println!("  From: {}", dest.display());
    println!("  To: {}", target.display());

    #[cfg(target_os = "macos")]
    {
        use airgap_sync::crypto::*;
        use airgap_sync::keychain::*;
        use airgap_sync::sync::*;

        // Get encryption key
        let keychain = KeychainManager::new();
        let key_data = keychain.get_key(device_id)?;

        let algorithm = match key_data.metadata.algorithm.as_str() {
            "AES-256" => EncryptionAlgorithm::Aes256Gcm,
            "ChaCha20" => EncryptionAlgorithm::ChaCha20Poly1305,
            _ => anyhow::bail!("Unsupported algorithm"),
        };

        let crypto_key = CryptoKey::new(key_data.key_material.clone(), algorithm)?;

        let config = SyncConfig::new("/tmp", dest, device_id.to_string());
        let engine = SyncEngine::new(config, crypto_key.clone())?;

        engine.restore(snapshot_id, target, &crypto_key)?;

        println!("✓ Restore completed successfully");
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Restore requires macOS for Keychain integration");
    }

    Ok(())
}

fn cmd_audit(dest: &Path, filter: Option<&str>) -> Result<()> {
    use airgap_sync::audit::*;

    let audit_path = dest.join(".airgapsync").join("audit.log");

    if !audit_path.exists() {
        println!("No audit log found at: {}", audit_path.display());
        return Ok(());
    }

    println!("Audit log: {}", audit_path.display());

    let logger = AuditLogger::new(&audit_path)?;

    // Verify log integrity
    let report = logger.verify_log()?;
    println!("\nIntegrity check:");
    println!("  Total entries: {}", report.total_entries);
    println!("  Verified: {}", report.verified_entries);
    println!("  Chain valid: {}", report.chain_valid);

    if !report.is_valid() {
        println!("\n⚠ Warning: Audit log integrity issues detected!");
    }

    // Read and display entries
    let entries = if let Some(filter_type) = filter {
        let event_type = match filter_type {
            "sync" => AuditEventType::SyncCompleted,
            "error" => AuditEventType::ErrorOccurred,
            "key" => AuditEventType::KeyGenerated,
            _ => {
                println!("Unknown filter: {}", filter_type);
                return Ok(());
            }
        };
        logger.filter_by_type(event_type)?
    } else {
        logger.read_entries()?
    };

    println!("\nAudit entries:");
    println!("{:<6} {:<20} {:<15} {:<10} {}", "ID", "Timestamp", "Event", "Severity", "Message");
    println!("{}", "-".repeat(100));

    for entry in entries.iter().rev().take(20) {
        println!(
            "{:<6} {:<20} {:<15?} {:<10?} {}",
            entry.id,
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.event_type,
            entry.severity,
            entry.message
        );
    }

    Ok(())
}

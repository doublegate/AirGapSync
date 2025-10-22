//! AirGapSync CLI - Phase 2 Implementation
//!
//! This CLI implements full sync functionality including file synchronization,
//! restore, verification, and snapshot management.

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

    /// Sync files to removable media
    Sync {
        /// Source directory
        #[clap(short, long)]
        source: PathBuf,

        /// Destination device path
        #[clap(short, long)]
        dest: PathBuf,

        /// Device ID for encryption
        #[clap(short = 'i', long)]
        device_id: String,

        /// Dry run (don't actually sync)
        #[clap(long)]
        dry_run: bool,

        /// Chunk size in KB (default: 1024)
        #[clap(long)]
        chunk_size: Option<usize>,

        /// Disable compression
        #[clap(long)]
        no_compress: bool,

        /// Disable encryption
        #[clap(long)]
        no_encrypt: bool,
    },

    /// Verify backup integrity
    Verify {
        /// Archive path
        #[clap(short, long)]
        archive: PathBuf,

        /// Device ID
        #[clap(short = 'i', long)]
        device_id: String,

        /// Snapshot ID (latest if not specified)
        #[clap(short, long)]
        snapshot: Option<String>,
    },

    /// List snapshots
    ListSnapshots {
        /// Archive path
        #[clap(short, long)]
        archive: PathBuf,

        /// Device ID
        #[clap(short = 'i', long)]
        device_id: String,
    },

    /// Restore from backup
    Restore {
        /// Archive path
        #[clap(short, long)]
        archive: PathBuf,

        /// Device ID
        #[clap(short = 'i', long)]
        device_id: String,

        /// Snapshot ID
        #[clap(short, long)]
        snapshot: String,

        /// Destination directory
        #[clap(short, long)]
        dest: PathBuf,
    },

    /// Show archive statistics
    Stats {
        /// Archive path
        #[clap(short, long)]
        archive: PathBuf,

        /// Device ID
        #[clap(short = 'i', long)]
        device_id: String,
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
            source,
            dest,
            device_id,
            dry_run,
            chunk_size,
            no_compress,
            no_encrypt,
        } => cmd_sync(&source, &dest, &device_id, dry_run, chunk_size, no_compress, no_encrypt),
        Commands::Verify {
            archive,
            device_id,
            snapshot,
        } => cmd_verify(&archive, &device_id, snapshot.as_deref()),
        Commands::ListSnapshots { archive, device_id } => cmd_list_snapshots(&archive, &device_id),
        Commands::Restore {
            archive,
            device_id,
            snapshot,
            dest,
        } => cmd_restore(&archive, &device_id, &snapshot, &dest),
        Commands::Stats { archive, device_id } => cmd_stats(&archive, &device_id),
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
    source: &Path,
    dest: &Path,
    device_id: &str,
    dry_run: bool,
    chunk_size: Option<usize>,
    no_compress: bool,
    no_encrypt: bool,
) -> Result<()> {
    use airgap_sync::sync::*;
    use std::sync::Arc;

    println!("Starting sync...");
    println!("  Source: {}", source.display());
    println!("  Destination: {}", dest.display());
    println!("  Device ID: {}", device_id);

    if dry_run {
        println!("  Mode: DRY RUN (no changes will be made)");
    }

    // Get encryption key from keychain
    let encryption_key = if !no_encrypt {
        #[cfg(target_os = "macos")]
        {
            use airgap_sync::keychain::*;
            let keychain = KeychainManager::new();
            let key_data = keychain.get_key(device_id)?;

            let algorithm = match key_data.metadata.algorithm.as_str() {
                "AES-256" => EncryptionAlgorithm::Aes256Gcm,
                "ChaCha20" => EncryptionAlgorithm::ChaCha20Poly1305,
                _ => anyhow::bail!("Unsupported algorithm for encryption"),
            };

            Some(CryptoKey::new(key_data.key_material.clone(), algorithm)?)
        }

        #[cfg(not(target_os = "macos"))]
        {
            anyhow::bail!("Keychain integration requires macOS");
        }
    } else {
        None
    };

    // Create sync options
    let mut options = SyncOptions {
        device_id: device_id.to_string(),
        dry_run,
        ..Default::default()
    };

    if let Some(size_kb) = chunk_size {
        options.chunk_options.chunk_size = size_kb * 1024;
    }

    options.chunk_options.compress = !no_compress;
    options.chunk_options.encrypt = !no_encrypt;

    // Progress callback
    let progress_callback: ProgressCallback = Arc::new(|progress| {
        if let Some(ref file) = progress.current_file {
            println!(
                "  [{}/{}] {}",
                progress.files_processed, progress.total_files, file.display()
            );
        }
    });

    // Create and run sync engine
    let mut engine = SyncEngine::new(options);

    let result = engine.sync(
        source,
        dest,
        encryption_key.as_ref(),
        Some(progress_callback),
    )?;

    println!("\n✓ Sync completed successfully");
    println!("  Snapshot ID: {}", result.snapshot_id);
    println!("  Files synced: {}", result.files_synced);
    println!("  Bytes synced: {} ({:.2} MB)", result.bytes_synced, result.bytes_synced as f64 / 1_000_000.0);
    println!("  Bytes written: {} ({:.2} MB)", result.bytes_written, result.bytes_written as f64 / 1_000_000.0);
    println!("  Compression ratio: {:.1}%", result.compression_ratio() * 100.0);
    println!("  Chunks created: {}", result.chunks_created);
    println!("  Chunks deduplicated: {}", result.chunks_deduplicated);
    println!("  Duration: {:.2}s", result.duration.as_secs_f64());

    Ok(())
}

fn cmd_verify(archive: &Path, device_id: &str, snapshot_id: Option<&str>) -> Result<()> {
    use airgap_sync::archive::*;

    println!("Verifying backup integrity...");
    println!("  Archive: {}", archive.display());

    let archive = Archive::open(archive.to_path_buf(), device_id.to_string())?;

    let snapshot = if let Some(id) = snapshot_id {
        id.to_string()
    } else {
        archive.latest_snapshot()?
            .ok_or_else(|| anyhow::anyhow!("No snapshots found"))?
    };

    println!("  Snapshot: {}", snapshot);

    let result = archive.verify(&snapshot)?;

    if result.is_valid {
        println!("\n✓ Backup is valid");
        println!("  Files verified: {}/{}", result.verified_files, result.total_files);
        println!("  Chunks verified: {}/{}", result.verified_chunks, result.total_chunks);
    } else {
        println!("\n✗ Backup verification failed");
        println!("  Missing chunks: {}", result.missing_chunks.len());
        println!("  Corrupted files: {}", result.corrupted_files.len());

        if !result.missing_chunks.is_empty() {
            println!("\nMissing chunks:");
            for chunk_id in result.missing_chunks.iter().take(10) {
                println!("  - {}", chunk_id);
            }
        }

        if !result.corrupted_files.is_empty() {
            println!("\nCorrupted files:");
            for file_path in result.corrupted_files.iter().take(10) {
                println!("  - {}", file_path.display());
            }
        }
    }

    Ok(())
}

fn cmd_list_snapshots(archive: &Path, device_id: &str) -> Result<()> {
    use airgap_sync::archive::*;

    let archive = Archive::open(archive.to_path_buf(), device_id.to_string())?;
    let snapshots = archive.list_snapshots()?;

    if snapshots.is_empty() {
        println!("No snapshots found");
        return Ok(());
    }

    println!("Snapshots for device {}:", device_id);
    println!("{:<20} {:<10} {:<20}", "Snapshot ID", "Files", "Created");
    println!("{}", "-".repeat(60));

    for snapshot_id in &snapshots {
        if let Ok(manifest) = archive.load_manifest(snapshot_id) {
            println!(
                "{:<20} {:<10} {:<20}",
                &snapshot_id[..16],
                manifest.file_count,
                manifest.created_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }

    Ok(())
}

fn cmd_restore(archive: &Path, device_id: &str, snapshot_id: &str, dest: &Path) -> Result<()> {
    use airgap_sync::sync::*;
    use std::sync::Arc;

    println!("Restoring from backup...");
    println!("  Archive: {}", archive.display());
    println!("  Snapshot: {}", snapshot_id);
    println!("  Destination: {}", dest.display());

    // Get encryption key from keychain
    #[cfg(target_os = "macos")]
    let encryption_key = {
        use airgap_sync::keychain::*;
        let keychain = KeychainManager::new();
        let key_data = keychain.get_key(device_id)?;

        let algorithm = match key_data.metadata.algorithm.as_str() {
            "AES-256" => EncryptionAlgorithm::Aes256Gcm,
            "ChaCha20" => EncryptionAlgorithm::ChaCha20Poly1305,
            _ => anyhow::bail!("Unsupported algorithm for decryption"),
        };

        Some(CryptoKey::new(key_data.key_material.clone(), algorithm)?)
    };

    #[cfg(not(target_os = "macos"))]
    let encryption_key: Option<CryptoKey> = None;

    // Progress callback
    let progress_callback: ProgressCallback = Arc::new(|progress| {
        if let Some(ref file) = progress.current_file {
            println!(
                "  [{}/{}] {}",
                progress.files_processed, progress.total_files, file.display()
            );
        }
    });

    // Create sync engine and restore
    let options = SyncOptions {
        device_id: device_id.to_string(),
        ..Default::default()
    };

    let mut engine = SyncEngine::new(options);

    let result = engine.restore(
        archive,
        snapshot_id,
        dest,
        encryption_key.as_ref(),
        Some(progress_callback),
    )?;

    println!("\n✓ Restore completed successfully");
    println!("  Files restored: {}", result.files_restored);
    println!("  Bytes restored: {} ({:.2} MB)", result.bytes_restored, result.bytes_restored as f64 / 1_000_000.0);
    println!("  Duration: {:.2}s", result.duration.as_secs_f64());

    Ok(())
}

fn cmd_stats(archive: &Path, device_id: &str) -> Result<()> {
    use airgap_sync::archive::*;

    let archive = Archive::open(archive.to_path_buf(), device_id.to_string())?;
    let stats = archive.statistics()?;

    println!("Archive Statistics:");
    println!("  Snapshots: {}", stats.snapshot_count);
    println!("  Total files: {}", stats.total_files);
    println!("  Total bytes: {} ({:.2} GB)", stats.total_bytes, stats.total_bytes as f64 / 1_000_000_000.0);
    println!("  Compressed bytes: {} ({:.2} GB)", stats.compressed_bytes, stats.compressed_bytes as f64 / 1_000_000_000.0);
    println!("  Total chunks: {}", stats.total_chunks);
    println!("  Compression ratio: {:.1}%", stats.compression_ratio() * 100.0);
    println!("  Average snapshot size: {:.2} MB", stats.avg_snapshot_size() as f64 / 1_000_000.0);

    Ok(())
}

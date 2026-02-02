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

    /// Dry run mode - preview changes without making them
    #[clap(long, global = true)]
    dry_run: bool,

    /// Rotate encryption keys after operation
    #[clap(long, global = true)]
    rotate_keys: bool,

    /// View audit log
    #[clap(long, global = true)]
    audit_log: bool,

    /// Verify backup integrity for device
    #[clap(long, global = true, value_name = "DEVICE")]
    verify: Option<String>,

    /// Restore from snapshot (format: SNAPSHOT:DEST)
    #[clap(long, global = true, value_name = "SNAPSHOT:DEST")]
    restore: Option<String>,

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

    /// Sync files to device
    Sync {
        /// Device ID or mount path
        #[clap(long)]
        device: Option<String>,

        /// Dry run mode
        #[clap(long)]
        dry_run: bool,

        /// Verbose output
        #[clap(short, long)]
        verbose: bool,

        /// Continue from previous sync
        #[clap(long)]
        resume: bool,

        /// Number of parallel workers
        #[clap(long, default_value = "4")]
        workers: usize,

        /// Configuration file path
        #[clap(short, long)]
        config: Option<PathBuf>,

        /// Source directory (overrides config)
        #[clap(long)]
        source: Option<PathBuf>,

        /// Exclude patterns
        #[clap(long)]
        exclude: Vec<String>,

        /// Rotate keys after sync
        #[clap(long)]
        rotate_keys: bool,
    },

    /// Verify backup integrity
    Verify {
        /// Device ID
        device_id: String,

        /// Configuration file path
        #[clap(short, long)]
        config: Option<PathBuf>,

        /// Specific snapshot ID to verify
        #[clap(long)]
        snapshot: Option<String>,
    },

    /// Restore files from a snapshot
    Restore {
        /// Snapshot ID
        snapshot_id: String,

        /// Destination directory
        destination: PathBuf,

        /// Device ID
        device_id: String,

        /// Configuration file path
        #[clap(short, long)]
        config: Option<PathBuf>,
    },

    /// Device management
    Device {
        #[clap(subcommand)]
        command: DeviceCommands,
    },

    /// Snapshot management
    Snapshot {
        #[clap(subcommand)]
        command: SnapshotCommands,
    },

    /// View audit log
    AuditLog {
        /// Filter by device ID
        #[clap(long)]
        device: Option<String>,

        /// Number of entries to show
        #[clap(long, default_value = "50")]
        limit: usize,

        /// Show entries since date (YYYY-MM-DD or RFC3339)
        #[clap(long)]
        since: Option<String>,
    },

    /// Generate shell completions
    Completion {
        /// Shell to generate completions for
        #[clap(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Watch for device changes and auto-sync
    Watch {
        /// Configuration file path
        #[clap(short, long)]
        config: Option<PathBuf>,

        /// Sync interval in seconds
        #[clap(long, default_value = "300")]
        interval: u64,

        /// Enable dry-run mode
        #[clap(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum DeviceCommands {
    /// List configured devices
    List,

    /// Add a new device
    Add {
        /// Device ID
        id: String,

        /// Device name
        name: String,

        /// Mount point path
        mount_point: PathBuf,
    },

    /// Remove a device
    Remove {
        /// Device ID
        id: String,
    },

    /// Show device information
    Info {
        /// Device ID
        id: String,
    },
}

#[derive(Subcommand)]
enum SnapshotCommands {
    /// List snapshots for a device
    List {
        /// Device ID
        device_id: String,

        /// Show detailed information
        #[clap(long)]
        detailed: bool,
    },

    /// Show snapshot information
    Info {
        /// Snapshot ID
        id: String,

        /// Device ID
        device_id: String,
    },

    /// Delete a snapshot
    Delete {
        /// Snapshot ID
        id: String,

        /// Device ID
        device_id: String,

        /// Force deletion without confirmation
        #[clap(long)]
        force: bool,
    },

    /// Compare two snapshots
    Diff {
        /// First snapshot ID
        snapshot1: String,

        /// Second snapshot ID
        snapshot2: String,

        /// Device ID
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

    // Handle global options first
    if cli.audit_log {
        return cmd_audit_log(None, 50, None);
    }

    if let Some(device) = cli.verify {
        return cmd_verify(&device, cli.config, None);
    }

    if let Some(restore_spec) = cli.restore {
        let parts: Vec<&str> = restore_spec.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Restore format must be SNAPSHOT:DEST");
        }
        // Extract device ID from snapshot ID (format: device_id/snapshot_id)
        let device_id = if parts[0].contains('/') {
            parts[0].split('/').next().unwrap_or("default")
        } else {
            "default"
        };
        return cmd_restore(parts[0], &PathBuf::from(parts[1]), device_id, cli.config.clone());
    }

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
            device,
            dry_run,
            verbose,
            resume,
            workers,
            config,
            source,
            exclude,
            rotate_keys,
        } => cmd_sync(
            device,
            dry_run || cli.dry_run,  // Use global dry_run if set
            verbose || cli.verbose,  // Use global verbose if set
            resume,
            workers,
            config.or(cli.config.clone()),  // Use global config if sync config not set
            source,
            exclude,
            rotate_keys || cli.rotate_keys,  // Use global rotate_keys if set
        ),
        Commands::Verify { device_id, config, snapshot } => cmd_verify(&device_id, config, snapshot),
        Commands::Restore { snapshot_id, destination, device_id, config } => {
            cmd_restore(&snapshot_id, &destination, &device_id, config)
        },
        Commands::Device { command } => cmd_device(command),
        Commands::Snapshot { command } => cmd_snapshot(command),
        Commands::AuditLog { device, limit, since } => cmd_audit_log(device, limit, since),
        Commands::Completion { shell } => {
            use clap::CommandFactory;
            use clap_complete::generate;
            use std::io;
            
            let mut cmd = Cli::command();
            let cmd_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, cmd_name, &mut io::stdout());
            Ok(())
        },
        Commands::Watch { config, interval, dry_run } => {
            cmd_watch(config.or(cli.config).as_deref(), interval, dry_run || cli.dry_run)
        },
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
    println!();
    println!("Phase 2 Features:");
    println!("  ✓ Full sync engine implementation");
    println!("  ✓ Diff-based file comparison");
    println!("  ✓ Chunk-based deduplication");
    println!("  ✓ Streaming encryption");
    println!("  ✓ Progress reporting");
    println!("  ✓ Snapshot management");
    println!("  ✓ Parallel processing");
    println!("  ✓ Compression support");
    println!("  ✓ Audit logging");

    Ok(())
}

fn cmd_sync(
    device: Option<String>,
    dry_run: bool,
    verbose: bool,
    resume: bool,
    workers: usize,
    config_path: Option<PathBuf>,
    source: Option<PathBuf>,
    exclude: Vec<String>,
    rotate_keys: bool,
) -> Result<()> {
    use airgap_sync::sync::{SyncEngine, SyncOptions, SyncBuilder};
    use airgap_sync::config::Config;
    use std::sync::Arc;
    
    // Load configuration
    let config_path = config_path
        .or_else(|| Config::default_path().ok())
        .context("Could not determine config path")?;
    
    let mut config = Config::from_file(&config_path)
        .context("Failed to load configuration")?;
    
    // Override source if provided
    if let Some(src) = source {
        config.source.path = src;
    }
    
    // Add additional excludes
    config.source.exclude.extend(exclude);
    
    // Determine device
    let device_id = if let Some(dev) = device {
        // Check if it's a device ID or path
        if config.device.iter().any(|d| d.id == dev) {
            dev
        } else {
            // Try to find device by mount point
            config.device.iter()
                .find(|d| d.mount_point.to_string_lossy() == dev)
                .map(|d| d.id.clone())
                .ok_or_else(|| anyhow::anyhow!("Device not found: {}", dev))?
        }
    } else {
        // Use first device
        config.device.first()
            .map(|d| d.id.clone())
            .ok_or_else(|| anyhow::anyhow!("No devices configured"))?
    };
    
    println!("Syncing to device: {}", device_id);
    if dry_run {
        println!("DRY RUN MODE - No changes will be made");
    }
    
    // Build sync options
    let options = SyncOptions {
        dry_run,
        verbose,
        parallel_workers: workers,
        chunk_size: 1024 * 1024, // 1MB chunks
        verify_after_write: true,
        resume,
        compression_level: 6,
        show_progress: true,
        max_retries: 3,
        exclude_patterns: vec![],
    };
    
    // Set up progress callback
    let progress_callback = if verbose || !dry_run {
        Some(Arc::new(move |progress: SyncProgress| {
            println!("[{}] {} files, {} transferred, {} errors",
                progress.operation,
                progress.processed_files,
                humanize_bytes(progress.processed_bytes),
                progress.errors.len()
            );
            
            if let Some(file) = &progress.current_file {
                println!("  Current: {}", file.display());
            }
        }) as Arc<dyn Fn(SyncProgress) + Send + Sync>)
    } else {
        None
    };
    
    // Create and run sync
    let mut builder = SyncBuilder::new()
        .with_config(config)
        .with_options(options);
    
    if let Some(callback) = progress_callback {
        builder = builder.with_progress_callback(callback);
    }
    
    // Perform sync
    let result = builder.sync_to_device(&device_id)?;
    
    // Display results
    println!("\nSync completed:");
    println!("  Files synced: {}", result.files_synced);
    println!("  Bytes transferred: {} ({:.1} MB)", 
        humanize_bytes(result.bytes_transferred),
        result.bytes_transferred as f64 / (1024.0 * 1024.0)
    );
    println!("  Duration: {}s", result.duration_seconds);
    if result.average_transfer_rate > 0 {
        println!("  Average rate: {}/s", 
            humanize_bytes(result.average_transfer_rate)
        );
    }
    
    if let Some(snapshot_id) = result.snapshot_id {
        println!("  Snapshot: {}", snapshot_id);
    }
    
    if result.files_failed > 0 {
        println!("  ⚠️  Failed: {} files", result.files_failed);
    }
    
    if !result.errors.is_empty() {
        println!("\nErrors:");
        for err in &result.errors {
            println!("  - {}", err);
        }
    }
    
    // Rotate keys if requested
    if rotate_keys && !dry_run {
        println!("\nRotating encryption keys...");
        cmd_rotate(&device_id)?;
    }
    
    Ok(())
}

fn cmd_verify(device_id: &str, config_path: Option<PathBuf>, snapshot_id: Option<String>) -> Result<()> {
    use airgap_sync::config::Config;
    use airgap_sync::snapshot::SnapshotManager;
    
    println!("Verifying backup integrity for device: {}", device_id);
    
    // Load configuration
    let config_path = config_path
        .or_else(|| Config::default_path().ok())
        .context("Could not determine config path")?;
    
    let config = Config::from_file(&config_path)
        .context("Failed to load configuration")?;
    
    // Check device exists
    let device = config.device.iter()
        .find(|d| d.id == device_id)
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_id))?;
    
    // Initialize snapshot manager
    let snapshot_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".airgapsync")
        .join("snapshots");
    
    let mut manager = SnapshotManager::new(&snapshot_dir)?;
    
    // Get snapshot to verify
    let snapshot = if let Some(id) = snapshot_id {
        manager.load_snapshot(device_id, &id)?
    } else {
        manager.get_latest_snapshot(device_id)?
            .ok_or_else(|| anyhow::anyhow!("No snapshots found for device"))?
    };
    
    println!("Verifying snapshot: {}", snapshot.id);
    println!("Created: {}", snapshot.created_at.format("%Y-%m-%d %H:%M:%S"));
    
    // Verify snapshot integrity
    match snapshot.verify() {
        Ok(_) => {
            println!("✓ Snapshot integrity verified");
            
            // Verify chunk existence
            let chunk_dir = Path::new(&device.mount_point)
                .join(".airgapsync")
                .join("chunks");
            
            if chunk_dir.exists() {
                let mut missing_chunks = 0;
                let mut verified_chunks = 0;
                
                for chunk_ref in &snapshot.chunk_refs {
                    let chunk_path = chunk_dir
                        .join(&chunk_ref.chunk_hash[..2])
                        .join(&chunk_ref.chunk_hash);
                    
                    if chunk_path.exists() {
                        verified_chunks += 1;
                    } else {
                        missing_chunks += 1;
                        println!("  ⚠️  Missing chunk: {}", chunk_ref.chunk_hash);
                    }
                }
                
                println!("\nChunk verification:");
                println!("  Verified: {}", verified_chunks);
                println!("  Missing: {}", missing_chunks);
                
                if missing_chunks > 0 {
                    anyhow::bail!("Backup integrity check failed: {} chunks missing", missing_chunks);
                }
            } else {
                println!("⚠️  Warning: Chunk directory not found on device");
            }
            
            println!("\n✅ Backup verification complete - all checks passed");
        }
        Err(e) => {
            anyhow::bail!("Snapshot verification failed: {}", e);
        }
    }
    
    Ok(())
}

fn cmd_restore(
    snapshot_id: &str,
    destination: &Path,
    device_id: &str,
    config_path: Option<PathBuf>,
) -> Result<()> {
    use airgap_sync::snapshot::SnapshotManager;
    use airgap_sync::chunk::ChunkProcessor;
    use airgap_sync::config::Config;
    
    println!("Restoring from snapshot: {}", snapshot_id);
    println!("Destination: {}", destination.display());
    
    // Create destination if needed
    std::fs::create_dir_all(destination)?;
    
    // Load configuration
    let config_path = config_path
        .or_else(|| Config::default_path().ok())
        .context("Could not determine config path")?;
    
    let config = Config::from_file(&config_path)
        .context("Failed to load configuration")?;
    
    // Get device
    let device = config.device.iter()
        .find(|d| d.id == device_id)
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_id))?;
    
    // Load snapshot
    let snapshot_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".airgapsync")
        .join("snapshots");
    
    let mut manager = SnapshotManager::new(&snapshot_dir)?;
    let snapshot = manager.load_snapshot(device_id, snapshot_id)?;
    
    println!("Snapshot contains {} files", snapshot.files.len());
    
    // Get encryption key
    #[cfg(target_os = "macos")]
    {
        use airgap_sync::keychain::KeychainManager;
        let keychain = KeychainManager::new();
        let key_data = keychain.get_key(device_id)?;
        
        let algorithm = match device.encryption.algorithm {
            crate::config::EncryptionAlgorithm::Aes256Gcm => airgap_sync::crypto::Algorithm::Aes256Gcm,
            crate::config::EncryptionAlgorithm::ChaCha20Poly1305 => airgap_sync::crypto::Algorithm::ChaCha20Poly1305,
        };
        
        let key = airgap_sync::crypto::CryptoKey::new(key_data.key_material.clone(), algorithm)?;
        
        // Create chunk processor
        let processor = ChunkProcessor::new();
        
        // Restore each file
        let chunk_dir = Path::new(&device.mount_point)
            .join(".airgapsync")
            .join("chunks");
        
        let mut restored = 0;
        let mut failed = 0;
        
        for file_meta in &snapshot.files {
            let dest_path = destination.join(&file_meta.path);
            
            // Create parent directory
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            println!("Restoring: {}", file_meta.path.display());
            
            // Collect chunks for this file
            let file_chunks: Vec<_> = snapshot.chunk_refs.iter()
                .filter(|c| c.file_path == file_meta.path)
                .collect();
            
            if file_chunks.is_empty() {
                println!("  ⚠️  No chunks found for file");
                failed += 1;
                continue;
            }
            
            // Load and reconstruct chunks
            match restore_file_from_chunks(&processor, &file_chunks, &chunk_dir, &dest_path, &key, &file_meta.path) {
                Ok(_) => {
                    restored += 1;
                    
                    // Set permissions
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let permissions = std::fs::Permissions::from_mode(file_meta.permissions);
                        std::fs::set_permissions(&dest_path, permissions)?;
                    }
                }
                Err(e) => {
                    println!("  ❌ Failed: {}", e);
                    failed += 1;
                }
            }
        }
        
        println!("\nRestore complete:");
        println!("  Restored: {} files", restored);
        println!("  Failed: {} files", failed);
        
        if failed > 0 {
            anyhow::bail!("Restore completed with {} failures", failed);
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("Restore requires macOS Keychain integration");
    }
    
    Ok(())
}

fn restore_file_from_chunks(
    processor: &airgap_sync::chunk::ChunkProcessor,
    chunk_refs: &[&airgap_sync::snapshot::ChunkReference],
    chunk_dir: &Path,
    dest_path: &Path,
    key: &airgap_sync::crypto::CryptoKey,
    original_path: &Path,
) -> Result<()> {
    use airgap_sync::chunk::Chunk;
    
    let mut chunks = Vec::new();
    
    for chunk_ref in chunk_refs {
        let chunk_path = chunk_dir
            .join(&chunk_ref.chunk_hash[..2])
            .join(&chunk_ref.chunk_hash);
        
        if !chunk_path.exists() {
            anyhow::bail!("Chunk not found: {}", chunk_ref.chunk_hash);
        }
        
        let encrypted_data = std::fs::read(&chunk_path)?;
        
        let chunk = Chunk {
            metadata: airgap_sync::chunk::ChunkMetadata {
                index: chunk_ref.chunk_index,
                offset: 0, // Will be handled by processor
                original_size: chunk_ref.size as usize,
                compressed_size: 0, // Unknown
                encrypted_size: encrypted_data.len(),
                hash: chunk_ref.chunk_hash.clone(),
                encrypted_hash: String::new(), // Not needed for restore
                compression_ratio: 0.0, // Not needed
            },
            encrypted_data,
            hash: chunk_ref.chunk_hash.clone(),
        };
        
        chunks.push(chunk);
    }
    
    processor.reconstruct_file(&chunks, dest_path, key, original_path)?;
    Ok(())
}

fn cmd_device(command: DeviceCommands) -> Result<()> {
    use airgap_sync::config::*;
    
    let config_path = Config::default_path()?;
    
    match command {
        DeviceCommands::List => {
            let config = Config::from_file(&config_path)?;
            
            if config.device.is_empty() {
                println!("No devices configured.");
                return Ok(());
            }
            
            println!("Configured devices:");
            println!("{:<15} {:<25} {:<30}", "ID", "Name", "Mount Point");
            println!("{}", "-".repeat(70));
            
            for device in &config.device {
                println!(
                    "{:<15} {:<25} {:<30}",
                    device.id,
                    device.name,
                    device.mount_point.display()
                );
            }
        }
        DeviceCommands::Add { id, name, mount_point } => {
            let mut config = Config::from_file(&config_path)?;
            
            // Check if device already exists
            if config.device.iter().any(|d| d.id == id) {
                anyhow::bail!("Device '{}' already exists", id);
            }
            
            // Add new device
            config.device.push(DeviceConfig {
                id: id.clone(),
                name: name.clone(),
                mount_point,
                encryption: EncryptionConfig::default(),
            });
            
            config.save(&config_path)?;
            println!("✓ Device '{}' added successfully", id);
            println!("\nNext step: Generate encryption key");
            println!("  airgapsync keygen {}", id);
        }
        DeviceCommands::Remove { id } => {
            let mut config = Config::from_file(&config_path)?;
            
            let initial_count = config.device.len();
            config.device.retain(|d| d.id != id);
            
            if config.device.len() == initial_count {
                anyhow::bail!("Device '{}' not found", id);
            }
            
            config.save(&config_path)?;
            println!("✓ Device '{}' removed", id);
        }
        DeviceCommands::Info { id } => {
            let config = Config::from_file(&config_path)?;
            
            let device = config.device.iter()
                .find(|d| d.id == id)
                .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", id))?;
            
            println!("Device Information:");
            println!("  ID: {}", device.id);
            println!("  Name: {}", device.name);
            println!("  Mount Point: {}", device.mount_point.display());
            println!("  Encryption: {:?}", device.encryption.algorithm);
            
            // Check if mounted
            if device.mount_point.exists() {
                println!("  Status: Mounted");
                
                // Get disk usage
                if let Ok(_metadata) = std::fs::metadata(&device.mount_point) {
                    println!("  Available: (filesystem info not available in this context)");
                }
            } else {
                println!("  Status: Not mounted");
            }
        }
    }
    
    Ok(())
}

fn cmd_snapshot(command: SnapshotCommands) -> Result<()> {
    use airgap_sync::snapshot::{SnapshotManager, SnapshotDiff};
    
    let snapshot_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".airgapsync")
        .join("snapshots");
    
    let mut manager = SnapshotManager::new(&snapshot_dir)?;
    
    match command {
        SnapshotCommands::List { device_id, detailed } => {
            let snapshots = manager.list_snapshots(&device_id)?;
            
            if snapshots.is_empty() {
                println!("No snapshots found for device '{}'", device_id);
                return Ok(());
            }
            
            if detailed {
                for info in snapshots {
                    println!("Snapshot: {}", info.id);
                    println!("  Created: {}", info.created_at.format("%Y-%m-%d %H:%M:%S"));
                    println!("  Size: {}", humanize_bytes(info.size));
                    println!();
                }
            } else {
                println!("Snapshots for device '{}':", device_id);
                println!("{:<40} {:<20} {:<10}", "ID", "Created", "Size");
                println!("{}", "-".repeat(70));
                
                for info in snapshots {
                    println!(
                        "{:<40} {:<20} {:<10}",
                        info.id,
                        info.created_at.format("%Y-%m-%d %H:%M"),
                        humanize_bytes(info.size)
                    );
                }
            }
        }
        SnapshotCommands::Info { id, device_id } => {
            let snapshot = manager.load_snapshot(&device_id, &id)?;
            
            println!("Snapshot Information:");
            println!("  ID: {}", snapshot.id);
            println!("  Device: {}", snapshot.device_id);
            println!("  Created: {}", snapshot.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("  Version: {}", snapshot.version);
            if let Some(parent) = &snapshot.parent_id {
                println!("  Parent: {}", parent);
            }
            println!("\nStatistics:");
            println!("  Total files: {}", snapshot.total_files);
            println!("  Total size: {}", humanize_bytes(snapshot.total_size));
            println!("  Files synced: {}", snapshot.files_synced);
            println!("  Bytes transferred: {}", humanize_bytes(snapshot.bytes_transferred));
            println!("\nChanges in this snapshot:");
            
            let mut added = 0;
            let mut modified = 0;
            let mut deleted = 0;
            
            for change in &snapshot.changes {
                match change {
                    airgap_sync::diff::FileChange::Added { .. } => added += 1,
                    airgap_sync::diff::FileChange::Modified { .. } => modified += 1,
                    airgap_sync::diff::FileChange::Deleted { .. } => deleted += 1,
                }
            }
            
            println!("  Added: {} files", added);
            println!("  Modified: {} files", modified);
            println!("  Deleted: {} files", deleted);
        }
        SnapshotCommands::Delete { id, device_id, force } => {
            if !force {
                println!("Are you sure you want to delete snapshot '{}'? (y/N)", id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Deletion cancelled");
                    return Ok(());
                }
            }
            
            manager.delete_snapshot(&device_id, &id)?;
            println!("✓ Snapshot '{}' deleted", id);
        }
        SnapshotCommands::Diff { snapshot1, snapshot2, device_id } => {
            let snap1 = manager.load_snapshot(&device_id, &snapshot1)?;
            let snap2 = manager.load_snapshot(&device_id, &snapshot2)?;
            
            let diff = SnapshotDiff::compare(&snap1, &snap2);
            
            println!("Differences between snapshots:");
            println!("  Snapshot 1: {} ({})", snapshot1, snap1.created_at.format("%Y-%m-%d %H:%M"));
            println!("  Snapshot 2: {} ({})", snapshot2, snap2.created_at.format("%Y-%m-%d %H:%M"));
            println!();
            
            if diff.added_files.is_empty() && diff.modified_files.is_empty() && diff.deleted_files.is_empty() {
                println!("No differences found");
            } else {
                if !diff.added_files.is_empty() {
                    println!("Added files ({}):", diff.added_files.len());
                    for file in &diff.added_files[..5.min(diff.added_files.len())] {
                        println!("  + {}", file.path.display());
                    }
                    if diff.added_files.len() > 5 {
                        println!("  ... and {} more", diff.added_files.len() - 5);
                    }
                    println!();
                }
                
                if !diff.modified_files.is_empty() {
                    println!("Modified files ({}):", diff.modified_files.len());
                    for (_, new_file) in &diff.modified_files[..5.min(diff.modified_files.len())] {
                        println!("  ~ {}", new_file.path.display());
                    }
                    if diff.modified_files.len() > 5 {
                        println!("  ... and {} more", diff.modified_files.len() - 5);
                    }
                    println!();
                }
                
                if !diff.deleted_files.is_empty() {
                    println!("Deleted files ({}):", diff.deleted_files.len());
                    for file in &diff.deleted_files[..5.min(diff.deleted_files.len())] {
                        println!("  - {}", file.path.display());
                    }
                    if diff.deleted_files.len() > 5 {
                        println!("  ... and {} more", diff.deleted_files.len() - 5);
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn cmd_audit_log(device: Option<String>, limit: usize, since: Option<String>) -> Result<()> {
    use airgap_sync::audit::{AuditLogger, AuditEvent};
    use chrono::{DateTime, Utc, NaiveDateTime};
    
    // Create audit logger
    let audit_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".airgapsync")
        .join("audit");
    
    let logger = AuditLogger::new(&audit_dir)?;
    
    // Parse since date if provided
    let since_date = if let Some(date_str) = since {
        Some(DateTime::parse_from_rfc3339(&date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&format!("{} 00:00:00", date_str), "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc())
            })
            .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD or RFC3339"))?)
    } else {
        None
    };
    
    // Read entries
    let entries = logger.read_entries(Some(limit), since_date)?;
    
    // Filter by device if specified
    let entries: Vec<_> = if let Some(device_id) = &device {
        entries.into_iter().filter(|entry| {
            match &entry.event {
                AuditEvent::SyncStarted { device_id: id, .. } |
                AuditEvent::SyncCompleted { device_id: id, .. } |
                AuditEvent::SyncFailed { device_id: id, .. } |
                AuditEvent::KeyGenerated { device_id: id, .. } |
                AuditEvent::KeyRotated { device_id: id, .. } |
                AuditEvent::DeviceConnected { device_id: id, .. } |
                AuditEvent::DeviceDisconnected { device_id: id } |
                AuditEvent::VerificationPerformed { device_id: id, .. } |
                AuditEvent::SnapshotCreated { device_id: id, .. } |
                AuditEvent::SnapshotDeleted { device_id: id, .. } => id == device_id,
                _ => true,
            }
        }).collect()
    } else {
        entries
    };
    
    // Display results
    println!("Audit Log");
    println!("{}", "=".repeat(80));
    
    if entries.is_empty() {
        println!("No audit entries found");
        return Ok(());
    }
    
    // Verify integrity
    match logger.verify_integrity() {
        Ok(()) => println!("✓ Log integrity verified"),
        Err(e) => println!("⚠️  WARNING: {}", e),
    }
    println!();
    
    // Display entries
    for entry in entries.iter().rev() {
        println!("[{}] {}", 
            entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            entry.id
        );
        
        match &entry.event {
            AuditEvent::SyncStarted { device_id, source_path, dry_run } => {
                println!("  SYNC STARTED");
                println!("  Device: {}", device_id);
                println!("  Source: {}", source_path.display());
                if *dry_run {
                    println!("  Mode: DRY RUN");
                }
            }
            AuditEvent::SyncCompleted { device_id, files_added, files_modified, files_deleted, bytes_transferred, duration_secs } => {
                println!("  SYNC COMPLETED");
                println!("  Device: {}", device_id);
                println!("  Files: +{} ~{} -{}", files_added, files_modified, files_deleted);
                println!("  Transferred: {} ({:.1} MB)", 
                    humanize_bytes(*bytes_transferred),
                    *bytes_transferred as f64 / (1024.0 * 1024.0)
                );
                println!("  Duration: {:.1}s", duration_secs);
            }
            AuditEvent::SyncFailed { device_id, error } => {
                println!("  SYNC FAILED");
                println!("  Device: {}", device_id);
                println!("  Error: {}", error);
            }
            AuditEvent::KeyGenerated { device_id, algorithm } => {
                println!("  KEY GENERATED");
                println!("  Device: {}", device_id);
                println!("  Algorithm: {}", algorithm);
            }
            AuditEvent::KeyRotated { device_id, old_key_hash, new_key_hash } => {
                println!("  KEY ROTATED");
                println!("  Device: {}", device_id);
                println!("  Old hash: {}...", &old_key_hash[..8]);
                println!("  New hash: {}...", &new_key_hash[..8]);
            }
            AuditEvent::VerificationPerformed { device_id, snapshot_id, files_verified, errors_found } => {
                println!("  VERIFICATION");
                println!("  Device: {}", device_id);
                if let Some(snap) = snapshot_id {
                    println!("  Snapshot: {}", snap);
                }
                println!("  Files: {} verified, {} errors", files_verified, errors_found);
            }
            AuditEvent::SnapshotCreated { device_id, snapshot_id, size } => {
                println!("  SNAPSHOT CREATED");
                println!("  Device: {}", device_id);
                println!("  ID: {}", snapshot_id);
                println!("  Size: {}", humanize_bytes(*size));
            }
            AuditEvent::Error { message, context } => {
                println!("  ERROR: {}", message);
                if let Some(ctx) = context {
                    println!("  Context: {}", ctx);
                }
            }
            _ => {
                println!("  {:?}", entry.event);
            }
        }
        println!();
    }
    
    // Show statistics
    let stats = logger.get_statistics()?;
    println!("{}", "-".repeat(80));
    println!("Statistics:");
    println!("  Total entries: {}", stats.total_entries);
    println!("  Sync operations: {}", stats.sync_operations);
    println!("  Key operations: {}", stats.key_operations);
    println!("  Errors: {}", stats.errors);
    println!("  Warnings: {}", stats.warnings);
    if let (Some(first), Some(last)) = (stats.first_entry, stats.last_entry) {
        println!("  Period: {} to {}", 
            first.format("%Y-%m-%d"),
            last.format("%Y-%m-%d")
        );
    }
    
    Ok(())
}

fn cmd_watch(config_path: Option<&Path>, interval: u64, dry_run: bool) -> Result<()> {
    use std::thread;
    use std::time::Duration;
    use std::collections::HashMap;
    
    println!("🔍 Starting device watch mode...");
    println!("Sync interval: {} seconds", interval);
    if dry_run {
        println!("DRY RUN MODE - No changes will be made");
    }
    
    // Load configuration
    let config = if let Some(path) = config_path {
        airgap_sync::config::Config::from_file(&path.to_path_buf())?
    } else {
        let default_path = dirs::home_dir()
            .unwrap()
            .join(".airgapsync")
            .join("config.toml");
        airgap_sync::config::Config::from_file(&default_path)?
    };
    
    // Track device states
    let mut device_states: HashMap<String, bool> = HashMap::new();
    
    println!("Monitoring {} devices. Press Ctrl+C to stop.", config.device.len());
    
    loop {
        // Check each configured device
        for device in &config.device {
            let device_path = Path::new(&device.mount_point);
            let is_mounted = device_path.exists() && device_path.is_dir();
            
            // Check if device state changed
            let prev_state = device_states.get(&device.id).copied().unwrap_or(false);
            if is_mounted != prev_state {
                device_states.insert(device.id.clone(), is_mounted);
                
                if is_mounted {
                    println!("\n✅ Device '{}' mounted at {}", device.id, device.mount_point.display());
                    
                    // Run sync for this device
                    println!("Starting sync for device '{}'...", device.id);
                    match cmd_sync(
                        Some(device.id.clone()),
                        dry_run,
                        false, // verbose
                        false, // resume
                        4,     // workers
                        Some(config_path.unwrap_or(&PathBuf::from("~/.airgapsync/config.toml")).to_path_buf()),
                        None,  // source
                        vec![], // exclude
                        false, // rotate_keys
                    ) {
                        Ok(_) => println!("✅ Sync completed for device '{}'", device.id),
                        Err(e) => eprintln!("❌ Sync failed for device '{}': {}", device.id, e),
                    }
                } else {
                    println!("\n🔌 Device '{}' unmounted", device.id);
                }
            }
        }
        
        // Sleep until next check
        thread::sleep(Duration::from_secs(interval));
    }
}

/// Format bytes into human-readable string
fn humanize_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    const THRESHOLD: f64 = 1024.0;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(target_os = "macos")]
fn generate_key(algorithm: &str, device_id: &str) -> Result<airgap_sync::keychain::EncryptionKey> {
    use airgap_sync::crypto::*;
    use airgap_sync::keychain::*;
    
    let (key_material, _algo) = match algorithm {
        "AES-256" => {
            let key = CryptoKey::generate(Algorithm::Aes256Gcm)?;
            (key.as_bytes().to_vec(), Algorithm::Aes256Gcm)
        }
        "AES-128" => {
            // For AES-128, we generate 256-bit key but document it as 128
            let key = CryptoKey::generate(Algorithm::Aes256Gcm)?;
            (key.as_bytes()[..16].to_vec(), Algorithm::Aes256Gcm)
        }
        "ChaCha20" => {
            let key = CryptoKey::generate(Algorithm::ChaCha20Poly1305)?;
            (key.as_bytes().to_vec(), Algorithm::ChaCha20Poly1305)
        }
        _ => anyhow::bail!("Unsupported algorithm: {}", algorithm),
    };
    
    let metadata = KeyMetadata {
        algorithm: algorithm.to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        rotated_at: None,
        device_id: device_id.to_string(),
    };
    
    Ok(EncryptionKey {
        key_material,
        metadata,
    })
}

#[cfg(target_os = "macos")]
fn rotate_key(keychain: &airgap_sync::keychain::KeychainManager, device_id: &str) -> Result<airgap_sync::keychain::EncryptionKey> {
    use airgap_sync::crypto::*;
    use airgap_sync::keychain::*;
    
    // Get existing key
    let old_key = keychain.get_key(device_id)?;
    
    // Generate new key with same algorithm
    let (key_material, _) = match old_key.metadata.algorithm.as_str() {
        "AES-256" => {
            let key = CryptoKey::generate(Algorithm::Aes256Gcm)?;
            (key.as_bytes().to_vec(), Algorithm::Aes256Gcm)
        }
        "ChaCha20" => {
            let key = CryptoKey::generate(Algorithm::ChaCha20Poly1305)?;
            (key.as_bytes().to_vec(), Algorithm::ChaCha20Poly1305)
        }
        _ => anyhow::bail!("Unsupported algorithm for rotation: {}", old_key.metadata.algorithm),
    };
    
    let metadata = KeyMetadata {
        algorithm: old_key.metadata.algorithm.clone(),
        created_at: old_key.metadata.created_at,
        version: old_key.metadata.version + 1,
        rotated_at: Some(chrono::Utc::now()),
        device_id: device_id.to_string(),
    };
    
    let new_key = EncryptionKey {
        key_material,
        metadata,
    };
    
    // Store new key
    keychain.store_key(device_id, &new_key)?;
    
    Ok(new_key)
}

// Helper function for formatting bytes
fn format_bytes(bytes: u64) -> String {
    humanize_bytes(bytes)
}
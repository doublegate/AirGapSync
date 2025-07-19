# API Documentation

## Rust Core Library API

### Overview

The AirGapSync core library provides a comprehensive API for secure file synchronization. All public APIs are exposed through the `airgap_sync` crate.

### Core Types

```rust
use airgap_sync::{SyncEngine, Config, Device, SyncResult};
```

### Configuration

#### `Config`

```rust
pub struct Config {
    pub source_path: PathBuf,
    pub exclude_patterns: Vec<String>,
    pub encryption_algorithm: EncryptionAlgorithm,
    pub compression_level: CompressionLevel,
    pub retention_policy: RetentionPolicy,
}

impl Config {
    /// Load configuration from TOML file
    pub fn from_file(path: &Path) -> Result<Self, ConfigError>;
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), ValidationError>;
    
    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<(), IoError>;
}
```

#### `RetentionPolicy`

```rust
pub struct RetentionPolicy {
    pub max_snapshots: u32,
    pub max_age_days: u32,
    pub gc_interval_hours: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_snapshots: 7,
            max_age_days: 30,
            gc_interval_hours: 24,
        }
    }
}
```

### Device Management

#### `Device`

```rust
pub struct Device {
    pub id: String,
    pub name: String,
    pub mount_point: PathBuf,
    pub total_space: u64,
    pub available_space: u64,
    pub is_encrypted: bool,
}

impl Device {
    /// Detect all connected removable devices
    pub fn detect_all() -> Result<Vec<Device>, DeviceError>;
    
    /// Find device by ID
    pub fn find_by_id(id: &str) -> Result<Option<Device>, DeviceError>;
    
    /// Check if device is mounted
    pub fn is_mounted(&self) -> bool;
    
    /// Get device health status
    pub fn health_check(&self) -> Result<DeviceHealth, DeviceError>;
}
```

### Sync Engine

#### `SyncEngine`

```rust
pub struct SyncEngine {
    config: Config,
    device: Device,
    keychain: KeychainManager,
}

impl SyncEngine {
    /// Create new sync engine instance
    pub fn new(config: Config, device: Device) -> Result<Self, SyncError>;
    
    /// Perform synchronization
    pub async fn sync(&mut self) -> Result<SyncResult, SyncError>;
    
    /// Perform dry run (no changes)
    pub async fn dry_run(&self) -> Result<SyncPlan, SyncError>;
    
    /// Resume interrupted sync
    pub async fn resume(&mut self, state: SyncState) -> Result<SyncResult, SyncError>;
    
    /// Cancel ongoing sync
    pub fn cancel(&mut self);
    
    /// Set progress callback
    pub fn on_progress<F>(&mut self, callback: F)
    where
        F: Fn(SyncProgress) + Send + 'static;
}
```

#### `SyncResult`

```rust
pub struct SyncResult {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_files: u64,
    pub total_bytes: u64,
    pub files_added: u64,
    pub files_updated: u64,
    pub files_deleted: u64,
    pub bytes_transferred: u64,
    pub errors: Vec<SyncError>,
}
```

#### `SyncProgress`

```rust
pub struct SyncProgress {
    pub phase: SyncPhase,
    pub current_file: Option<PathBuf>,
    pub total_files: u64,
    pub processed_files: u64,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub speed_bps: u64,
    pub eta_seconds: Option<u64>,
}

pub enum SyncPhase {
    Scanning,
    Analyzing,
    Transferring,
    Verifying,
    Finalizing,
}
```

### Encryption

#### `KeychainManager`

```rust
pub struct KeychainManager {
    service_name: String,
}

impl KeychainManager {
    /// Create or load key for device
    pub fn get_or_create_key(&self, device_id: &str) -> Result<EncryptionKey, KeyError>;
    
    /// Rotate encryption key
    pub fn rotate_key(&self, device_id: &str) -> Result<EncryptionKey, KeyError>;
    
    /// Export key (requires user authentication)
    pub fn export_key(&self, device_id: &str) -> Result<ExportedKey, KeyError>;
    
    /// Import key
    pub fn import_key(&self, key: ExportedKey) -> Result<(), KeyError>;
}
```

#### `EncryptionKey`

```rust
pub struct EncryptionKey {
    pub algorithm: EncryptionAlgorithm,
    pub created_at: DateTime<Utc>,
    pub fingerprint: String,
}

pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}
```

### Audit Logging

#### `AuditLogger`

```rust
pub struct AuditLogger {
    log_path: PathBuf,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(log_path: PathBuf) -> Result<Self, AuditError>;
    
    /// Log sync event
    pub fn log_sync(&mut self, event: SyncEvent) -> Result<(), AuditError>;
    
    /// Log security event
    pub fn log_security(&mut self, event: SecurityEvent) -> Result<(), AuditError>;
    
    /// Query audit logs
    pub fn query(&self, filter: AuditFilter) -> Result<Vec<AuditEntry>, AuditError>;
    
    /// Verify log integrity
    pub fn verify_integrity(&self) -> Result<IntegrityReport, AuditError>;
}
```

### Snapshot Management

#### `SnapshotManager`

```rust
pub struct SnapshotManager {
    device: Device,
}

impl SnapshotManager {
    /// List all snapshots
    pub fn list(&self) -> Result<Vec<Snapshot>, SnapshotError>;
    
    /// Get snapshot by ID
    pub fn get(&self, id: &str) -> Result<Option<Snapshot>, SnapshotError>;
    
    /// Delete snapshot
    pub fn delete(&self, id: &str) -> Result<(), SnapshotError>;
    
    /// Restore from snapshot
    pub async fn restore(&self, id: &str, target: &Path) -> Result<(), SnapshotError>;
    
    /// Run garbage collection
    pub fn garbage_collect(&self, policy: &RetentionPolicy) -> Result<GcResult, SnapshotError>;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Device error: {0}")]
    Device(#[from] DeviceError),
    
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Sync cancelled by user")]
    Cancelled,
}
```

### FFI Bindings

For Swift integration, the following C-compatible API is provided:

```rust
#[repr(C)]
pub struct CSyncProgress {
    pub phase: c_int,
    pub current_file: *const c_char,
    pub total_files: u64,
    pub processed_files: u64,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
}

#[no_mangle]
pub extern "C" fn airgap_sync_new(
    config_path: *const c_char,
    device_id: *const c_char,
) -> *mut c_void;

#[no_mangle]
pub extern "C" fn airgap_sync_start(
    engine: *mut c_void,
    callback: extern "C" fn(*const CSyncProgress),
) -> c_int;

#[no_mangle]
pub extern "C" fn airgap_sync_cancel(engine: *mut c_void);

#[no_mangle]
pub extern "C" fn airgap_sync_free(engine: *mut c_void);
```

### Example Usage

#### Basic Sync

```rust
use airgap_sync::{Config, Device, SyncEngine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::from_file("~/.airgapsync/config.toml")?;
    
    // Detect device
    let devices = Device::detect_all()?;
    let device = devices
        .into_iter()
        .find(|d| d.id == "USB001")
        .ok_or("Device not found")?;
    
    // Create sync engine
    let mut engine = SyncEngine::new(config, device)?;
    
    // Set progress callback
    engine.on_progress(|progress| {
        println!("Sync progress: {}/{} files", 
            progress.processed_files, 
            progress.total_files
        );
    });
    
    // Perform sync
    let result = engine.sync().await?;
    
    println!("Sync completed: {} files transferred", result.files_added);
    
    Ok(())
}
```

#### Key Rotation

```rust
use airgap_sync::KeychainManager;

fn rotate_device_key(device_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let keychain = KeychainManager::new("com.airgapsync.keys");
    
    // Rotate key
    let new_key = keychain.rotate_key(device_id)?;
    
    println!("Key rotated successfully");
    println!("New fingerprint: {}", new_key.fingerprint);
    
    Ok(())
}
```

### Thread Safety

All types in the public API are designed to be thread-safe:

- `SyncEngine`: Send + Sync (use Arc<Mutex<>> for shared access)
- `Config`: Send + Sync (immutable after creation)
- `Device`: Send + Sync
- `KeychainManager`: Send + Sync

### Performance Notes

1. **Chunking**: Files are processed in 1MB chunks by default
2. **Parallelism**: Up to 4 files processed concurrently
3. **Memory**: Approximately 50MB baseline + 10MB per concurrent file
4. **Compression**: zstd level 3 by default (configurable)

### Versioning

The API follows semantic versioning:
- Major version: Breaking changes
- Minor version: New features (backwards compatible)
- Patch version: Bug fixes

Current version: 0.1.0 (pre-release)
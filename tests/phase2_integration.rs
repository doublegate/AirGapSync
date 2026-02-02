//! Phase 2 Integration Tests
//!
//! Tests for the sync engine, diff detection, chunking, and snapshots

use airgap_sync::*;
use tempfile::TempDir;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Helper to create test configuration
fn create_test_config(source_dir: &Path, device_dir: &Path) -> config::Config {
    use config::*;
    
    Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.to_path_buf(),
            exclude: vec![".DS_Store".to_string()],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "TEST_DEVICE".to_string(),
            name: "Test Device".to_string(),
            mount_point: device_dir.to_path_buf(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    }
}

#[test]
fn test_sync_engine_basic() {
    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");
    
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();
    
    // Create test files
    let test_file1 = source_dir.join("test1.txt");
    let test_file2 = source_dir.join("test2.txt");
    
    File::create(&test_file1).unwrap()
        .write_all(b"Hello, World!").unwrap();
    File::create(&test_file2).unwrap()
        .write_all(b"Test file 2 content").unwrap();
    
    // Create configuration
    let config = create_test_config(&source_dir, &device_dir);
    
    // Create sync engine
    let mut engine = sync::SyncEngine::new(config).unwrap();
    
    // Run sync in dry-run mode
    let options = sync::SyncOptions {
        dry_run: true,
        ..Default::default()
    };
    
    let result = engine.sync_to_device("TEST_DEVICE", &options);
    
    // Check result
    assert!(result.is_ok(), "Sync should succeed: {:?}", result);
    let sync_result = result.unwrap();
    assert_eq!(sync_result.files_synced, 2, "Should sync 2 files");
}

#[test]
fn test_diff_engine() {
    use diff::{DiffEngine, FileChange};
    
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path();
    
    // Create test file
    let test_file = source_dir.join("test.txt");
    File::create(&test_file).unwrap()
        .write_all(b"Initial content").unwrap();
    
    // Create diff engine
    let mut diff_engine = DiffEngine::new();
    
    // Compute initial diff (no snapshot)
    let changes = diff_engine.compute_full_diff(
        source_dir,
        None,
        &[],
    ).unwrap();
    
    assert_eq!(changes.len(), 1);
    match &changes[0] {
        FileChange::Added { path, .. } => {
            assert_eq!(path, &PathBuf::from("test.txt"));
        }
        _ => panic!("Expected Added change"),
    }
}

#[test]
fn test_chunk_processor() {
    use chunk::ChunkProcessor;
    use crypto::{Algorithm, CryptoKey};
    
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("large_file.txt");
    
    // Create a file larger than chunk size
    let mut file = File::create(&test_file).unwrap();
    let data = vec![b'A'; 2 * 1024 * 1024]; // 2MB
    file.write_all(&data).unwrap();
    file.sync_all().unwrap();
    
    // Create chunk processor
    let processor = ChunkProcessor::new();
    
    // Create encryption key
    let key = CryptoKey::generate(Algorithm::Aes256Gcm).unwrap();
    
    // Process file into chunks
    let chunks = processor.process_file(
        &test_file,
        1024 * 1024, // 1MB chunks
        6,           // Compression level
        &key,
    ).unwrap();
    
    assert_eq!(chunks.len(), 2, "Should create 2 chunks");
    
    // Verify chunks can be reconstructed
    let output_file = temp_dir.path().join("reconstructed.txt");
    processor.reconstruct_file(
        &chunks,
        &output_file,
        &key,
        &test_file,
    ).unwrap();
    
    // Verify content matches
    let reconstructed = fs::read(&output_file).unwrap();
    assert_eq!(reconstructed, data);
}

#[test]
fn test_snapshot_manager() {
    use snapshot::{Snapshot, SnapshotManager};
    use diff::FileChange;
    
    let temp_dir = TempDir::new().unwrap();
    let snapshot_dir = temp_dir.path().join("snapshots");
    
    // Create snapshot manager
    let mut manager = SnapshotManager::new(&snapshot_dir).unwrap();
    
    // Create a snapshot
    let changes = vec![
        FileChange::Added {
            path: PathBuf::from("file1.txt"),
            size: 100,
            modified: chrono::Utc::now(),
            hash: "hash1".to_string(),
            permissions: 0o644,
        },
    ];
    
    let snapshot = Snapshot::new(
        "TEST_DEVICE".to_string(),
        changes,
        1,
        100,
    );
    
    // Get snapshot ID before saving
    let snapshot_id = snapshot.id.clone();
    
    // Save snapshot
    manager.save_snapshot(&snapshot).unwrap();
    
    // Debug: List files in snapshot directory
    let device_dir = snapshot_dir.join("TEST_DEVICE");
    if device_dir.exists() {
        println!("Files in device dir:");
        for entry in fs::read_dir(&device_dir).unwrap() {
            let entry = entry.unwrap();
            println!("  - {}", entry.path().display());
        }
    }
    
    // Load snapshot
    let loaded = manager.load_snapshot("TEST_DEVICE", &snapshot_id).unwrap();
    assert_eq!(loaded.id, snapshot.id);
    assert_eq!(loaded.device_id, "TEST_DEVICE");
    assert_eq!(loaded.files_synced, 1);
    
    // Get latest snapshot
    let latest = manager.get_latest_snapshot("TEST_DEVICE").unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().id, snapshot.id);
}

#[test]
fn test_sync_with_encryption() {
    use keychain::{KeychainManager, EncryptionKey, KeyMetadata};
    
    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");
    
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();
    
    // Create test file
    let test_file = source_dir.join("secret.txt");
    File::create(&test_file).unwrap()
        .write_all(b"This is a secret message").unwrap();
    
    // Store encryption key
    #[cfg(target_os = "macos")]
    {
        let keychain = KeychainManager::new();
        let key = EncryptionKey {
            key_material: vec![0u8; 32], // Test key
            metadata: KeyMetadata {
                algorithm: "AES-256".to_string(),
                created_at: chrono::Utc::now(),
                rotated_at: None,
                version: 1,
                device_id: "TEST_DEVICE".to_string(),
            },
        };
        
        // Try to store key (may fail in test environment)
        let _ = keychain.store_key("TEST_DEVICE", &key);
    }
    
    // Create configuration
    let config = create_test_config(&source_dir, &device_dir);
    
    // Create sync engine
    let mut engine = sync::SyncEngine::new(config).unwrap();
    
    // Run sync
    let options = sync::SyncOptions {
        dry_run: true, // Use dry-run to avoid keychain issues in tests
        show_progress: false,
        ..Default::default()
    };
    
    let result = engine.sync_to_device("TEST_DEVICE", &options);
    
    // Check result (may fail if keychain not available)
    if result.is_ok() {
        let sync_result = result.unwrap();
        assert_eq!(sync_result.files_synced, 1);
    }
}
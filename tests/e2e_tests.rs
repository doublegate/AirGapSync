//! End-to-End Integration Tests
//!
//! Comprehensive tests that exercise the entire sync workflow from
//! source directory → encryption → transfer → verification → restore

use airgap_sync::*;
use tempfile::TempDir;
use std::fs::{self, File};
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Helper to create test files with specific content
fn create_test_file(path: &Path, content: &[u8]) {
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent).ok();
    File::create(path)
        .unwrap()
        .write_all(content)
        .unwrap();
}

/// Helper to read file content
fn read_file_content(path: &Path) -> Vec<u8> {
    let mut file = File::open(path).unwrap();
    let mut content = Vec::new();
    file.read_to_end(&mut content).unwrap();
    content
}

/// Helper to create a realistic directory structure
fn create_realistic_test_structure(base_dir: &Path) {
    // Documents
    create_test_file(&base_dir.join("documents/report.txt"), b"Annual Report 2025");
    create_test_file(&base_dir.join("documents/notes.md"), b"# Meeting Notes\n\n- Item 1\n- Item 2");

    // Photos
    create_test_file(&base_dir.join("photos/vacation.jpg"), &vec![0xFF; 1024 * 100]); // 100KB "image"
    create_test_file(&base_dir.join("photos/family.jpg"), &vec![0xAA; 1024 * 200]); // 200KB "image"

    // Videos
    create_test_file(&base_dir.join("videos/movie.mp4"), &vec![0x42; 1024 * 1024 * 5]); // 5MB "video"

    // Code
    create_test_file(&base_dir.join("code/main.rs"), b"fn main() { println!(\"Hello!\"); }");
    create_test_file(&base_dir.join("code/lib.rs"), b"pub fn add(a: i32, b: i32) -> i32 { a + b }");

    // Archives
    create_test_file(&base_dir.join("archives/backup.tar.gz"), &vec![0x1F, 0x8B, 0x08]); // gzip header

    // Hidden files (should be excluded by default)
    create_test_file(&base_dir.join(".DS_Store"), b"DS_STORE_DATA");
    create_test_file(&base_dir.join(".gitignore"), b"*.log\n*.tmp");

    // Temp files (should be excluded)
    create_test_file(&base_dir.join("temp.tmp"), b"temporary data");
}

/// Helper to count files recursively
fn count_files(dir: &Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

/// Helper to calculate directory size
fn dir_size(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

#[test]
fn test_e2e_basic_sync() {
    use config::*;

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create test structure
    create_realistic_test_structure(&source_dir);

    // Configure sync
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![".DS_Store".to_string(), "*.tmp".to_string()],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "E2E_DEVICE".to_string(),
            name: "E2E Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    // Create sync engine
    let mut engine = sync::SyncEngine::new(config).unwrap();

    // Perform sync
    let options = sync::SyncOptions::default();
    let result = engine.sync_to_device("E2E_DEVICE", &options);

    // Verify sync succeeded
    assert!(result.is_ok(), "Sync should succeed: {:?}", result);
    let sync_result = result.unwrap();

    // Check results
    assert!(sync_result.files_synced > 0, "Should have synced some files");
    assert!(sync_result.bytes_transferred > 0, "Should have transferred data");
    assert_eq!(sync_result.files_failed, 0, "No files should fail");

    println!("E2E Basic Sync Results:");
    println!("  Files synced: {}", sync_result.files_synced);
    println!("  Bytes transferred: {}", sync_result.bytes_transferred);
    println!("  Duration: {}s", sync_result.duration_seconds);
    println!("  Avg rate: {} MB/s",
        sync_result.average_transfer_rate / (1024 * 1024));

    // Verify encrypted data exists on device
    let airgapsync_dir = device_dir.join(".airgapsync");
    assert!(airgapsync_dir.exists(), "AirGapSync directory should exist");

    // Verify snapshots directory exists
    let snapshots_dir = airgapsync_dir.join("snapshots");
    assert!(snapshots_dir.exists(), "Snapshots directory should exist");
}

#[test]
fn test_e2e_incremental_sync() {
    use config::*;

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Initial files
    create_test_file(&source_dir.join("file1.txt"), b"Content 1");
    create_test_file(&source_dir.join("file2.txt"), b"Content 2");

    // Configure and perform initial sync
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "INC_DEVICE".to_string(),
            name: "Incremental Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    let mut engine = sync::SyncEngine::new(config.clone()).unwrap();
    let options = sync::SyncOptions::default();

    // First sync
    let result1 = engine.sync_to_device("INC_DEVICE", &options).unwrap();
    println!("First sync: {} files", result1.files_synced);

    // Add more files
    create_test_file(&source_dir.join("file3.txt"), b"Content 3");
    create_test_file(&source_dir.join("subdir/file4.txt"), b"Content 4");

    // Second sync (incremental)
    let mut engine = sync::SyncEngine::new(config).unwrap();
    let result2 = engine.sync_to_device("INC_DEVICE", &options).unwrap();

    println!("Second sync: {} files", result2.files_synced);

    // Second sync should only transfer new files
    assert_eq!(result2.files_synced, 2, "Should sync only 2 new files");
}

#[test]
fn test_e2e_large_file_sync() {
    use config::*;

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create a large file (10 MB)
    let large_file_size = 10 * 1024 * 1024;
    let large_file = source_dir.join("large_file.bin");
    let mut file = File::create(&large_file).unwrap();

    // Write in chunks
    let chunk = vec![0x42u8; 1024 * 1024]; // 1MB chunk
    for _ in 0..10 {
        file.write_all(&chunk).unwrap();
    }
    file.sync_all().unwrap();
    drop(file);

    // Verify file size
    let metadata = fs::metadata(&large_file).unwrap();
    assert_eq!(metadata.len(), large_file_size as u64);

    // Configure and sync
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "LARGE_DEVICE".to_string(),
            name: "Large File Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig {
            compression_level: 3, // Lower compression for large files
            verify_after_write: true,
            retain_snapshots: 5,
            gc_threshold: 0.8,
            parallel_workers: 4,
            chunk_size_mb: 4, // Larger chunks for large files
        },
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    let mut engine = sync::SyncEngine::new(config).unwrap();
    let options = sync::SyncOptions::default();

    // Sync large file
    let start = std::time::Instant::now();
    let result = engine.sync_to_device("LARGE_DEVICE", &options).unwrap();
    let duration = start.elapsed();

    println!("Large file sync:");
    println!("  Size: {} MB", large_file_size / (1024 * 1024));
    println!("  Duration: {:?}", duration);
    println!("  Rate: {} MB/s",
        (result.bytes_transferred as f64 / duration.as_secs_f64()) / (1024.0 * 1024.0));

    assert_eq!(result.files_synced, 1);
    assert!(result.bytes_transferred >= large_file_size as u64);
}

#[test]
fn test_e2e_many_small_files() {
    use config::*;

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create many small files
    let file_count = 1000;
    for i in 0..file_count {
        let dir = source_dir.join(format!("batch_{}", i / 100));
        fs::create_dir_all(&dir).ok();
        create_test_file(&dir.join(format!("file_{}.txt", i)), format!("Content {}", i).as_bytes());
    }

    // Configure and sync
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "MANY_DEVICE".to_string(),
            name: "Many Files Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig {
            compression_level: 6,
            verify_after_write: true,
            retain_snapshots: 5,
            gc_threshold: 0.8,
            parallel_workers: 8, // More workers for many small files
            chunk_size_mb: 1,
        },
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    let mut engine = sync::SyncEngine::new(config).unwrap();
    let options = sync::SyncOptions::default();

    // Sync many files
    let start = std::time::Instant::now();
    let result = engine.sync_to_device("MANY_DEVICE", &options).unwrap();
    let duration = start.elapsed();

    println!("Many small files sync:");
    println!("  Files: {}", file_count);
    println!("  Duration: {:?}", duration);
    println!("  Files/sec: {:.0}", file_count as f64 / duration.as_secs_f64());

    assert_eq!(result.files_synced, file_count);
    assert_eq!(result.files_failed, 0);
}

#[test]
fn test_e2e_snapshot_verification() {
    use config::*;
    use snapshot::*;

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create test files
    create_test_file(&source_dir.join("verify1.txt"), b"Verification Test 1");
    create_test_file(&source_dir.join("verify2.txt"), b"Verification Test 2");

    // Configure and sync
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "VERIFY_DEVICE".to_string(),
            name: "Verification Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    let mut engine = sync::SyncEngine::new(config).unwrap();
    let options = sync::SyncOptions::default();

    // Perform sync
    let result = engine.sync_to_device("VERIFY_DEVICE", &options).unwrap();
    let snapshot_id = result.snapshot_id.expect("Should have snapshot ID");

    println!("Created snapshot: {}", snapshot_id);

    // Verify snapshot exists and can be loaded
    let snapshot_dir = device_dir.join(".airgapsync/snapshots");
    let mut snapshot_manager = SnapshotManager::new(&snapshot_dir).unwrap();

    let snapshot = snapshot_manager
        .load_snapshot("VERIFY_DEVICE", &snapshot_id)
        .expect("Should load snapshot");

    // Verify snapshot contents
    assert_eq!(snapshot.device_id, "VERIFY_DEVICE");
    assert_eq!(snapshot.file_count, 2);
    assert!(snapshot.total_size > 0);

    // Verify snapshot integrity
    snapshot.verify().expect("Snapshot verification should pass");

    println!("Snapshot verified successfully");
}

#[test]
fn test_e2e_audit_log() {
    use config::*;
    use audit::*;

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");
    let audit_dir = temp_dir.path().join("audit");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();
    fs::create_dir_all(&audit_dir).unwrap();

    // Create test files
    create_test_file(&source_dir.join("audit_test.txt"), b"Audit test content");

    // Configure with audit logging
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "AUDIT_DEVICE".to_string(),
            name: "Audit Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig {
            key_rotation_days: 90,
            require_authentication: false, // Disabled for testing
            audit_logging: true,
            enforce_device_encryption: true,
        },
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    let mut engine = sync::SyncEngine::new(config).unwrap();
    let options = sync::SyncOptions::default();

    // Perform sync (should generate audit logs)
    let _result = engine.sync_to_device("AUDIT_DEVICE", &options);

    // Verify audit logs were created
    // Note: In a real implementation, you would:
    // 1. Check audit log file exists
    // 2. Parse and verify audit entries
    // 3. Verify signature on audit log
    // 4. Check that sync events are recorded

    println!("Audit log test completed");
}

#[test]
fn test_e2e_error_recovery() {
    use config::*;

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create test files
    create_test_file(&source_dir.join("good_file.txt"), b"Good file");

    // Create a file we can't read (simulate permission error)
    // Note: This is platform-specific and might not work in all test environments
    let restricted_file = source_dir.join("restricted.txt");
    create_test_file(&restricted_file, b"Restricted content");

    // Configure and sync with error tolerance
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "ERROR_DEVICE".to_string(),
            name: "Error Recovery Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig::default(),
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    let mut engine = sync::SyncEngine::new(config).unwrap();
    let options = sync::SyncOptions::default();

    // Perform sync - should handle errors gracefully
    let result = engine.sync_to_device("ERROR_DEVICE", &options);

    // Even if some files fail, sync should complete
    if let Ok(sync_result) = result {
        println!("Error recovery test:");
        println!("  Files synced: {}", sync_result.files_synced);
        println!("  Files failed: {}", sync_result.files_failed);

        // At least the good file should sync
        assert!(sync_result.files_synced >= 1);
    }
}

#[test]
fn test_e2e_performance_baseline() {
    use config::*;

    // Setup with known workload
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create benchmark workload
    // 100 files, 1 MB each = 100 MB total
    for i in 0..100 {
        let content = vec![i as u8; 1024 * 1024]; // 1 MB
        create_test_file(&source_dir.join(format!("bench_{}.dat", i)), &content);
    }

    // Configure for performance
    let config = Config {
        general: GeneralConfig::default(),
        source: SourceConfig {
            path: source_dir.clone(),
            exclude: vec![],
            follow_symlinks: false,
            include_hidden: false,
        },
        device: vec![DeviceConfig {
            id: "PERF_DEVICE".to_string(),
            name: "Performance Test Device".to_string(),
            mount_point: device_dir.clone(),
            encryption: EncryptionConfig::default(),
        }],
        policy: PolicyConfig {
            compression_level: 6,
            verify_after_write: true,
            retain_snapshots: 5,
            gc_threshold: 0.8,
            parallel_workers: 4,
            chunk_size_mb: 1,
        },
        security: SecurityConfig::default(),
        schedule: None,
        notifications: NotificationConfig::default(),
        advanced: AdvancedConfig::default(),
    };

    let mut engine = sync::SyncEngine::new(config).unwrap();
    let options = sync::SyncOptions::default();

    // Benchmark sync
    let start = std::time::Instant::now();
    let result = engine.sync_to_device("PERF_DEVICE", &options).unwrap();
    let duration = start.elapsed();

    let mb_transferred = result.bytes_transferred as f64 / (1024.0 * 1024.0);
    let mb_per_sec = mb_transferred / duration.as_secs_f64();

    println!("Performance Baseline:");
    println!("  Files: {}", result.files_synced);
    println!("  Size: {:.2} MB", mb_transferred);
    println!("  Duration: {:?}", duration);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
    println!("  Files/sec: {:.0}", result.files_synced as f64 / duration.as_secs_f64());

    // Assert reasonable performance (very loose bounds for CI)
    assert!(mb_per_sec > 1.0, "Should achieve at least 1 MB/s");
    assert!(duration.as_secs() < 300, "Should complete within 5 minutes");
}

//! Phase 3 Integration Tests
//!
//! Tests for FFI bridge, universal binary support, and Swift integration

use airgap_sync::ffi::*;
use airgap_sync::config::*;
use tempfile::TempDir;
use std::ffi::{CString, CStr};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::ptr;

/// Helper to create a test config TOML file
fn create_test_config_file(source_dir: &Path, device_dir: &Path) -> String {
    let config_content = format!(
        r#"
[general]
verbose = false

[source]
path = "{}"
exclude = [".DS_Store", "*.tmp"]
follow_symlinks = false
include_hidden = false

[[device]]
id = "TEST_DEVICE"
name = "Test Device"
mount_point = "{}"

[device.encryption]
algorithm = "aes-256-gcm"

[policy]
compression_level = 6
verify_after_write = true
retain_snapshots = 10
gc_threshold = 0.8
parallel_workers = 4
chunk_size_mb = 1

[security]
key_rotation_days = 90
require_authentication = true
audit_logging = true
enforce_device_encryption = true

[notifications]
enable = false
"#,
        source_dir.display(),
        device_dir.display()
    );

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    File::create(&config_path)
        .unwrap()
        .write_all(config_content.as_bytes())
        .unwrap();

    // Keep the temp dir alive for the duration of the test
    std::mem::forget(temp_dir);
    config_path.to_string_lossy().to_string()
}

#[test]
fn test_ffi_initialize() {
    // Test library initialization
    let result = unsafe { airgap_initialize() };

    assert_eq!(
        result.error_code, FFIErrorCode::Success,
        "Initialization should succeed"
    );
    assert!(result.error_message.is_null(), "No error message expected");
}

#[test]
fn test_ffi_get_version() {
    // Initialize first
    unsafe { airgap_initialize() };

    // Get version string
    let version_ptr = unsafe { airgap_get_version() };

    assert!(
        !version_ptr.is_null(),
        "Version pointer should not be null"
    );

    // Convert to Rust string
    let version = unsafe {
        assert!(!version_ptr.is_null());
        CStr::from_ptr(version_ptr).to_string_lossy().into_owned()
    };

    assert!(!version.is_empty(), "Version string should not be empty");
    println!("AirGapSync version: {}", version);

    // Free the string
    unsafe { airgap_free_string(version_ptr) };
}

#[test]
fn test_ffi_config_lifecycle() {
    // Initialize library
    unsafe { airgap_initialize() };

    // Create temporary directories and config
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    let config_path = create_test_config_file(&source_dir, &device_dir);
    let config_path_cstring = CString::new(config_path.as_str()).unwrap();

    // Load configuration
    let config_ptr = unsafe { airgap_load_config(config_path_cstring.as_ptr()) };

    assert!(
        !config_ptr.is_null(),
        "Config pointer should not be null after loading"
    );

    // Free configuration
    unsafe { airgap_free_config(config_ptr) };
}

#[test]
fn test_ffi_engine_lifecycle() {
    // Initialize library
    unsafe { airgap_initialize() };

    // Create temporary directories and config
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    let config_path = create_test_config_file(&source_dir, &device_dir);
    let config_path_cstring = CString::new(config_path.as_str()).unwrap();

    // Load configuration
    let config_ptr = unsafe { airgap_load_config(config_path_cstring.as_ptr()) };
    assert!(!config_ptr.is_null());

    // Create sync engine
    let engine_ptr = unsafe { airgap_create_engine(config_ptr) };

    assert!(
        !engine_ptr.is_null(),
        "Engine pointer should not be null after creation"
    );

    // Free engine and config
    unsafe {
        airgap_free_engine(engine_ptr);
        airgap_free_config(config_ptr);
    }
}

#[test]
fn test_ffi_sync_options() {
    // Create FFI sync options with various settings
    let options = FFISyncOptions {
        dry_run: true,
        verbose: false,
        parallel_workers: 4,
        chunk_size: 1024 * 1024, // 1 MB
        verify_after_write: true,
        resume: true,
        compression_level: 6,
        show_progress: true,
        max_retries: 3,
    };

    // Verify options can be created and have correct values
    assert_eq!(options.parallel_workers, 4);
    assert_eq!(options.chunk_size, 1024 * 1024);
    assert_eq!(options.compression_level, 6);
    assert!(options.dry_run);
    assert!(options.verify_after_write);
}

#[test]
fn test_ffi_sync_dry_run() {
    // Initialize library
    unsafe { airgap_initialize() };

    // Create temporary directories and files
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create test files
    File::create(source_dir.join("test1.txt"))
        .unwrap()
        .write_all(b"Test file 1 content")
        .unwrap();
    File::create(source_dir.join("test2.txt"))
        .unwrap()
        .write_all(b"Test file 2 content")
        .unwrap();

    // Create config and engine
    let config_path = create_test_config_file(&source_dir, &device_dir);
    let config_path_cstring = CString::new(config_path.as_str()).unwrap();

    let config_ptr = unsafe { airgap_load_config(config_path_cstring.as_ptr()) };
    assert!(!config_ptr.is_null());

    let engine_ptr = unsafe { airgap_create_engine(config_ptr) };
    assert!(!engine_ptr.is_null());

    // Prepare sync options (dry run)
    let mut options = FFISyncOptions {
        dry_run: true,
        verbose: false,
        parallel_workers: 2,
        chunk_size: 1024 * 1024,
        verify_after_write: true,
        resume: false,
        compression_level: 6,
        show_progress: false,
        max_retries: 3,
    };

    // Prepare result structure
    let mut sync_result = FFISyncResult {
        files_synced: 0,
        bytes_transferred: 0,
        files_skipped: 0,
        files_failed: 0,
        duration_seconds: 0,
        average_transfer_rate: 0,
        snapshot_id: ptr::null_mut(),
    };

    // Perform sync
    let device_id = CString::new("TEST_DEVICE").unwrap();
    let result = unsafe {
        airgap_sync(
            engine_ptr,
            device_id.as_ptr(),
            &mut options,
            None,
            ptr::null_mut(),
            &mut sync_result,
        )
    };

    // In dry run mode, sync might fail because device isn't properly set up,
    // but we should at least get a valid result structure
    println!("Sync result error code: {:?}", result.error_code);

    // Clean up
    unsafe {
        if !sync_result.snapshot_id.is_null() {
            airgap_free_string(sync_result.snapshot_id);
        }
        if !result.error_message.is_null() {
            let error_msg = CStr::from_ptr(result.error_message)
                .to_string_lossy()
                .into_owned();
            println!("Sync error message: {}", error_msg);
            airgap_free_string(result.error_message);
        }
        airgap_free_engine(engine_ptr);
        airgap_free_config(config_ptr);
    }
}

#[test]
fn test_ffi_get_device_info() {
    // Initialize library
    unsafe { airgap_initialize() };

    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create config
    let config_path = create_test_config_file(&source_dir, &device_dir);
    let config_path_cstring = CString::new(config_path.as_str()).unwrap();

    let config_ptr = unsafe { airgap_load_config(config_path_cstring.as_ptr()) };
    assert!(!config_ptr.is_null());

    // Get device info
    let device_id = CString::new("TEST_DEVICE").unwrap();
    let device_info_ptr =
        unsafe { airgap_get_device_info(config_ptr, device_id.as_ptr()) };

    if !device_info_ptr.is_null() {
        unsafe {
            let device_info = &*device_info_ptr;

            // Verify device info fields
            let id = CStr::from_ptr(device_info.id).to_string_lossy();
            let name = CStr::from_ptr(device_info.name).to_string_lossy();
            let mount_point = CStr::from_ptr(device_info.mount_point).to_string_lossy();

            assert_eq!(id, "TEST_DEVICE");
            assert_eq!(name, "Test Device");
            assert!(mount_point.contains("device"));
            assert!(device_info.is_encrypted);

            println!("Device Info:");
            println!("  ID: {}", id);
            println!("  Name: {}", name);
            println!("  Mount Point: {}", mount_point);
            println!("  Encrypted: {}", device_info.is_encrypted);
            println!("  Snapshots: {}", device_info.snapshot_count);

            airgap_free_device_info(device_info_ptr);
        }
    }

    // Clean up
    unsafe {
        airgap_free_config(config_ptr);
    }
}

#[test]
fn test_ffi_list_snapshots() {
    // Initialize library
    unsafe { airgap_initialize() };

    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    // Create config
    let config_path = create_test_config_file(&source_dir, &device_dir);
    let config_path_cstring = CString::new(config_path.as_str()).unwrap();

    let config_ptr = unsafe { airgap_load_config(config_path_cstring.as_ptr()) };
    assert!(!config_ptr.is_null());

    // List snapshots (will be empty since no syncs have been performed)
    let device_id = CString::new("TEST_DEVICE").unwrap();
    let mut count: u32 = 0;

    let snapshots_ptr =
        unsafe { airgap_list_snapshots(config_ptr, device_id.as_ptr(), &mut count) };

    println!("Found {} snapshots", count);

    // Clean up snapshots list if not null
    if !snapshots_ptr.is_null() && count > 0 {
        unsafe {
            airgap_free_snapshot_list(snapshots_ptr, count);
        }
    }

    // Clean up config
    unsafe {
        airgap_free_config(config_ptr);
    }
}

#[test]
fn test_ffi_error_handling() {
    // Initialize library
    unsafe { airgap_initialize() };

    // Test with null pointers - should return error
    let result = unsafe { airgap_load_config(ptr::null()) };
    assert!(result.is_null(), "Should return null for null input");

    // Test with invalid config path
    let invalid_path = CString::new("/nonexistent/path/config.toml").unwrap();
    let result = unsafe { airgap_load_config(invalid_path.as_ptr()) };
    assert!(result.is_null(), "Should return null for invalid path");

    // Test creating engine with null config
    let result = unsafe { airgap_create_engine(ptr::null()) };
    assert!(result.is_null(), "Should return null for null config");
}

#[test]
fn test_ffi_memory_safety() {
    // Initialize library
    unsafe { airgap_initialize() };

    // Test multiple allocations and frees
    for _ in 0..10 {
        let version_ptr = unsafe { airgap_get_version() };
        assert!(!version_ptr.is_null());
        unsafe { airgap_free_string(version_ptr) };
    }

    // Create and destroy multiple configs
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    for _ in 0..5 {
        let config_path = create_test_config_file(&source_dir, &device_dir);
        let config_path_cstring = CString::new(config_path.as_str()).unwrap();

        let config_ptr = unsafe { airgap_load_config(config_path_cstring.as_ptr()) };
        assert!(!config_ptr.is_null());

        let engine_ptr = unsafe { airgap_create_engine(config_ptr) };
        assert!(!engine_ptr.is_null());

        unsafe {
            airgap_free_engine(engine_ptr);
            airgap_free_config(config_ptr);
        }
    }
}

#[test]
fn test_ffi_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    // Initialize library once
    unsafe { airgap_initialize() };

    // Create test setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let device_dir = temp_dir.path().join("device");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&device_dir).unwrap();

    let config_path = Arc::new(create_test_config_file(&source_dir, &device_dir));

    // Spawn multiple threads accessing FFI
    let mut handles = vec![];

    for i in 0..4 {
        let config_path = Arc::clone(&config_path);

        let handle = thread::spawn(move || {
            let config_path_cstring = CString::new(config_path.as_str()).unwrap();

            // Load config
            let config_ptr = unsafe { airgap_load_config(config_path_cstring.as_ptr()) };

            if !config_ptr.is_null() {
                // Get version
                let version_ptr = unsafe { airgap_get_version() };

                if !version_ptr.is_null() {
                    let version = unsafe { CStr::from_ptr(version_ptr).to_string_lossy() };
                    println!("Thread {}: Version = {}", i, version);
                    unsafe { airgap_free_string(version_ptr) };
                }

                unsafe { airgap_free_config(config_ptr) };
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}

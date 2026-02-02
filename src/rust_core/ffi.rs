//! FFI (Foreign Function Interface) bridge for Swift integration
//!
//! This module provides C-compatible functions that can be called from Swift.
//! It handles conversion between Rust types and C types, memory management,
//! and error handling across the FFI boundary.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong};
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::config::Config;
use crate::sync::{SyncEngine, SyncOptions, SyncProgress, SyncResult, SyncBuilder};
use crate::snapshot::{SnapshotManager, SnapshotMetadata};
use crate::audit::{AuditLogger, AuditEntry};
use crate::keychain::KeychainManager;
use crate::{initialize, AirGapError};

/// FFI error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FFIErrorCode {
    /// No error
    Success = 0,
    /// Null pointer error
    NullPointer = 1,
    /// Invalid UTF-8 string
    InvalidUtf8 = 2,
    /// Configuration error
    ConfigError = 3,
    /// Crypto error
    CryptoError = 4,
    /// Keychain error
    KeychainError = 5,
    /// IO error
    IoError = 6,
    /// Device not found
    DeviceNotFound = 7,
    /// Sync error
    SyncError = 8,
    /// Unknown error
    Unknown = 99,
}

/// FFI result with error code and message
#[repr(C)]
pub struct FFIResult {
    pub error_code: FFIErrorCode,
    pub error_message: *mut c_char,
}

impl FFIResult {
    fn success() -> Self {
        Self {
            error_code: FFIErrorCode::Success,
            error_message: ptr::null_mut(),
        }
    }

    fn error(code: FFIErrorCode, message: &str) -> Self {
        let msg = CString::new(message).unwrap_or_else(|_| CString::new("Invalid UTF-8").unwrap());
        Self {
            error_code: code,
            error_message: msg.into_raw(),
        }
    }

    fn from_rust_error(error: AirGapError) -> Self {
        let (code, message) = match error {
            AirGapError::Config(e) => (FFIErrorCode::ConfigError, e.to_string()),
            AirGapError::Crypto(e) => (FFIErrorCode::CryptoError, e.to_string()),
            AirGapError::Keychain(e) => (FFIErrorCode::KeychainError, e.to_string()),
            AirGapError::Io(e) => (FFIErrorCode::IoError, e.to_string()),
            AirGapError::DeviceNotFound(id) => (FFIErrorCode::DeviceNotFound, format!("Device not found: {}", id)),
            AirGapError::SyncError(msg) => (FFIErrorCode::SyncError, msg),
            _ => (FFIErrorCode::Unknown, error.to_string()),
        };
        Self::error(code, &message)
    }
}

/// FFI sync progress callback
pub type FFIProgressCallback = extern "C" fn(
    context: *mut std::ffi::c_void,
    operation: *const c_char,
    current_file: *const c_char,
    total_files: c_ulonglong,
    processed_files: c_ulonglong,
    total_bytes: c_ulonglong,
    processed_bytes: c_ulonglong,
    transfer_rate: c_ulonglong,
);

/// FFI sync options
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FFISyncOptions {
    pub dry_run: bool,
    pub verbose: bool,
    pub parallel_workers: c_uint,
    pub chunk_size: c_ulonglong,
    pub verify_after_write: bool,
    pub resume: bool,
    pub compression_level: c_uint,
    pub show_progress: bool,
    pub max_retries: c_uint,
}

impl From<FFISyncOptions> for SyncOptions {
    fn from(ffi_opts: FFISyncOptions) -> Self {
        Self {
            dry_run: ffi_opts.dry_run,
            verbose: ffi_opts.verbose,
            parallel_workers: ffi_opts.parallel_workers as usize,
            chunk_size: ffi_opts.chunk_size as usize,
            verify_after_write: ffi_opts.verify_after_write,
            resume: ffi_opts.resume,
            compression_level: ffi_opts.compression_level,
            show_progress: ffi_opts.show_progress,
            max_retries: ffi_opts.max_retries,
            exclude_patterns: vec![],
        }
    }
}

/// FFI sync result
#[repr(C)]
pub struct FFISyncResult {
    pub files_synced: c_ulonglong,
    pub bytes_transferred: c_ulonglong,
    pub files_skipped: c_ulonglong,
    pub files_failed: c_ulonglong,
    pub duration_seconds: c_ulonglong,
    pub average_transfer_rate: c_ulonglong,
    pub snapshot_id: *mut c_char,
}

impl From<SyncResult> for FFISyncResult {
    fn from(result: SyncResult) -> Self {
        let snapshot_id = result.snapshot_id
            .map(|id| CString::new(id).unwrap().into_raw())
            .unwrap_or(ptr::null_mut());

        Self {
            files_synced: result.files_synced,
            bytes_transferred: result.bytes_transferred,
            files_skipped: result.files_skipped,
            files_failed: result.files_failed,
            duration_seconds: result.duration_seconds,
            average_transfer_rate: result.average_transfer_rate,
            snapshot_id,
        }
    }
}

/// FFI device info
#[repr(C)]
pub struct FFIDeviceInfo {
    pub id: *mut c_char,
    pub name: *mut c_char,
    pub mount_point: *mut c_char,
    pub is_encrypted: bool,
    pub total_capacity: c_ulonglong,
    pub available_capacity: c_ulonglong,
    pub last_sync_timestamp: c_ulonglong,
    pub snapshot_count: c_uint,
}

/// Opaque handle to sync engine
pub struct SyncEngineHandle {
    engine: SyncEngine,
}

/// Initialize the AirGapSync library
///
/// # Safety
///
/// This function is safe to call multiple times. It must be called before any other FFI functions.
#[no_mangle]
pub extern "C" fn airgap_initialize() -> FFIResult {
    match initialize() {
        Ok(_) => FFIResult::success(),
        Err(e) => FFIResult::from_rust_error(e),
    }
}

/// Get library version string
///
/// # Safety
///
/// Caller must free the returned string using `airgap_free_string`.
#[no_mangle]
pub extern "C" fn airgap_get_version() -> *mut c_char {
    CString::new(crate::VERSION)
        .unwrap_or_else(|_| CString::new("unknown").unwrap())
        .into_raw()
}

/// Load configuration from file
///
/// # Safety
///
/// `config_path` must be a valid null-terminated UTF-8 string.
/// Caller must free the returned config using `airgap_free_config`.
#[no_mangle]
pub unsafe extern "C" fn airgap_load_config(
    config_path: *const c_char,
) -> *mut Config {
    if config_path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(config_path).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let path = PathBuf::from(path_str);
    match Config::from_file(&path) {
        Ok(config) => Box::into_raw(Box::new(config)),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a configuration object
///
/// # Safety
///
/// `config` must be a valid pointer returned by `airgap_load_config`.
/// After calling this function, the pointer is no longer valid.
#[no_mangle]
pub unsafe extern "C" fn airgap_free_config(config: *mut Config) {
    if !config.is_null() {
        drop(Box::from_raw(config));
    }
}

/// Create a new sync engine
///
/// # Safety
///
/// `config` must be a valid pointer returned by `airgap_load_config`.
/// Caller must free the returned engine using `airgap_free_engine`.
#[no_mangle]
pub unsafe extern "C" fn airgap_create_engine(
    config: *const Config,
) -> *mut SyncEngineHandle {
    if config.is_null() {
        return ptr::null_mut();
    }

    let config_ref = &*config;
    match SyncEngine::new(config_ref.clone()) {
        Ok(engine) => {
            let handle = SyncEngineHandle { engine };
            Box::into_raw(Box::new(handle))
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Free a sync engine
///
/// # Safety
///
/// `engine` must be a valid pointer returned by `airgap_create_engine`.
/// After calling this function, the pointer is no longer valid.
#[no_mangle]
pub unsafe extern "C" fn airgap_free_engine(engine: *mut SyncEngineHandle) {
    if !engine.is_null() {
        drop(Box::from_raw(engine));
    }
}

/// Perform sync operation
///
/// # Safety
///
/// - `engine` must be a valid pointer returned by `airgap_create_engine`
/// - `device_id` must be a valid null-terminated UTF-8 string
/// - `options` must be a valid pointer to FFISyncOptions
/// - `callback` is optional and can be null
/// - `callback_context` is passed to the callback function
/// - `result` must be a valid pointer to store the sync result
#[no_mangle]
pub unsafe extern "C" fn airgap_sync(
    engine: *mut SyncEngineHandle,
    device_id: *const c_char,
    options: *const FFISyncOptions,
    _callback: Option<FFIProgressCallback>,
    _callback_context: *mut std::ffi::c_void,
    result: *mut FFISyncResult,
) -> FFIResult {
    if engine.is_null() || device_id.is_null() || options.is_null() || result.is_null() {
        return FFIResult::error(FFIErrorCode::NullPointer, "Null pointer argument");
    }

    let engine_ref = &mut *engine;
    let device_id_str = match CStr::from_ptr(device_id).to_str() {
        Ok(s) => s,
        Err(_) => return FFIResult::error(FFIErrorCode::InvalidUtf8, "Invalid device ID"),
    };

    let sync_opts: SyncOptions = (*options).into();

    // Create progress callback wrapper if provided
    // Note: We need to handle the callback carefully for thread safety
    // For now, we'll skip progress callbacks in FFI and let the caller poll for progress
    // In a production implementation, you'd want a more sophisticated progress reporting mechanism

    // Perform sync
    match engine_ref.engine.sync_to_device(device_id_str, &sync_opts) {
        Ok(sync_result) => {
            *result = sync_result.into();
            FFIResult::success()
        }
        Err(e) => FFIResult::from_rust_error(AirGapError::SyncError(e.to_string())),
    }
}

/// Get device information
///
/// # Safety
///
/// - `config` must be a valid pointer returned by `airgap_load_config`
/// - `device_id` must be a valid null-terminated UTF-8 string
/// - Caller must free the returned device info using `airgap_free_device_info`
#[no_mangle]
pub unsafe extern "C" fn airgap_get_device_info(
    config: *const Config,
    device_id: *const c_char,
) -> *mut FFIDeviceInfo {
    if config.is_null() || device_id.is_null() {
        return ptr::null_mut();
    }

    let config_ref = &*config;
    let device_id_str = match CStr::from_ptr(device_id).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let device = match config_ref.device.iter().find(|d| d.id == device_id_str) {
        Some(d) => d,
        None => return ptr::null_mut(),
    };

    // Get additional device info from snapshot manager
    let snapshot_manager = match SnapshotManager::new(
        &PathBuf::from(&device.mount_point).join(".airgapsync/snapshots")
    ) {
        Ok(sm) => sm,
        Err(_) => return ptr::null_mut(),
    };

    let snapshots = snapshot_manager.list_snapshots(device_id_str).unwrap_or_default();
    let last_sync = snapshots.first()
        .map(|s| s.created_at.timestamp() as u64)
        .unwrap_or(0);

    let device_info = FFIDeviceInfo {
        id: CString::new(device.id.clone()).unwrap().into_raw(),
        name: CString::new(device.name.clone()).unwrap().into_raw(),
        mount_point: CString::new(device.mount_point.to_string_lossy().to_string()).unwrap().into_raw(),
        is_encrypted: true, // Always encrypted in AirGapSync
        total_capacity: 0, // Would need to query filesystem
        available_capacity: 0, // Would need to query filesystem
        last_sync_timestamp: last_sync,
        snapshot_count: snapshots.len() as u32,
    };

    Box::into_raw(Box::new(device_info))
}

/// Free device info
///
/// # Safety
///
/// `device_info` must be a valid pointer returned by `airgap_get_device_info`.
/// After calling this function, the pointer is no longer valid.
#[no_mangle]
pub unsafe extern "C" fn airgap_free_device_info(device_info: *mut FFIDeviceInfo) {
    if !device_info.is_null() {
        let info = Box::from_raw(device_info);
        if !info.id.is_null() {
            drop(CString::from_raw(info.id));
        }
        if !info.name.is_null() {
            drop(CString::from_raw(info.name));
        }
        if !info.mount_point.is_null() {
            drop(CString::from_raw(info.mount_point));
        }
    }
}

/// Verify snapshot integrity
///
/// # Safety
///
/// - `config` must be a valid pointer returned by `airgap_load_config`
/// - `device_id` must be a valid null-terminated UTF-8 string
/// - `snapshot_id` must be a valid null-terminated UTF-8 string
#[no_mangle]
pub unsafe extern "C" fn airgap_verify_snapshot(
    config: *const Config,
    device_id: *const c_char,
    snapshot_id: *const c_char,
) -> FFIResult {
    if config.is_null() || device_id.is_null() || snapshot_id.is_null() {
        return FFIResult::error(FFIErrorCode::NullPointer, "Null pointer argument");
    }

    let config_ref = &*config;
    let device_id_str = match CStr::from_ptr(device_id).to_str() {
        Ok(s) => s,
        Err(_) => return FFIResult::error(FFIErrorCode::InvalidUtf8, "Invalid device ID"),
    };

    let snapshot_id_str = match CStr::from_ptr(snapshot_id).to_str() {
        Ok(s) => s,
        Err(_) => return FFIResult::error(FFIErrorCode::InvalidUtf8, "Invalid snapshot ID"),
    };

    let device = match config_ref.device.iter().find(|d| d.id == device_id_str) {
        Some(d) => d,
        None => return FFIResult::error(FFIErrorCode::DeviceNotFound, "Device not found"),
    };

    let mut snapshot_manager = match SnapshotManager::new(
        &PathBuf::from(&device.mount_point).join(".airgapsync/snapshots")
    ) {
        Ok(sm) => sm,
        Err(e) => return FFIResult::error(FFIErrorCode::SyncError, &format!("Failed to create snapshot manager: {}", e)),
    };

    match snapshot_manager.load_snapshot(device_id_str, snapshot_id_str) {
        Ok(snapshot) => {
            match snapshot.verify() {
                Ok(_) => FFIResult::success(),
                Err(e) => FFIResult::error(FFIErrorCode::SyncError, &format!("Verification failed: {}", e)),
            }
        }
        Err(e) => FFIResult::error(FFIErrorCode::SyncError, &format!("Failed to load snapshot: {}", e)),
    }
}

/// Get snapshot info
///
/// # Safety
///
/// - `config` must be a valid pointer returned by `airgap_load_config`
/// - `device_id` must be a valid null-terminated UTF-8 string
/// - `snapshot_id` must be a valid null-terminated UTF-8 string
/// - Caller must free the returned string using `airgap_free_string`
#[no_mangle]
pub unsafe extern "C" fn airgap_get_snapshot_info(
    config: *const Config,
    device_id: *const c_char,
    snapshot_id: *const c_char,
) -> *mut c_char {
    if config.is_null() || device_id.is_null() || snapshot_id.is_null() {
        return ptr::null_mut();
    }

    let config_ref = &*config;
    let device_id_str = match CStr::from_ptr(device_id).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let snapshot_id_str = match CStr::from_ptr(snapshot_id).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let device = match config_ref.device.iter().find(|d| d.id == device_id_str) {
        Some(d) => d,
        None => return ptr::null_mut(),
    };

    let mut snapshot_manager = match SnapshotManager::new(
        &PathBuf::from(&device.mount_point).join(".airgapsync/snapshots")
    ) {
        Ok(sm) => sm,
        Err(_) => return ptr::null_mut(),
    };

    match snapshot_manager.load_snapshot(device_id_str, snapshot_id_str) {
        Ok(snapshot) => {
            let info = format!(
                "Snapshot: {}\nFiles: {}\nSize: {} bytes\nTimestamp: {}",
                snapshot.id,
                snapshot.files.len(),
                snapshot.total_size,
                snapshot.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            CString::new(info).unwrap_or_default().into_raw()
        }
        Err(_) => ptr::null_mut(),
    }
}

/// List snapshots for a device
///
/// # Safety
///
/// - `config` must be a valid pointer returned by `airgap_load_config`
/// - `device_id` must be a valid null-terminated UTF-8 string
/// - `out_count` must be a valid pointer to store the snapshot count
/// - Caller must free the returned array using `airgap_free_snapshot_list`
#[no_mangle]
pub unsafe extern "C" fn airgap_list_snapshots(
    config: *const Config,
    device_id: *const c_char,
    out_count: *mut c_uint,
) -> *mut *mut c_char {
    if config.is_null() || device_id.is_null() || out_count.is_null() {
        return ptr::null_mut();
    }

    let config_ref = &*config;
    let device_id_str = match CStr::from_ptr(device_id).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let device = match config_ref.device.iter().find(|d| d.id == device_id_str) {
        Some(d) => d,
        None => return ptr::null_mut(),
    };

    let snapshot_manager = match SnapshotManager::new(
        &PathBuf::from(&device.mount_point).join(".airgapsync/snapshots")
    ) {
        Ok(sm) => sm,
        Err(_) => return ptr::null_mut(),
    };

    let snapshots = match snapshot_manager.list_snapshots(device_id_str) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    *out_count = snapshots.len() as u32;

    if snapshots.is_empty() {
        return ptr::null_mut();
    }

    let snapshot_ids: Vec<*mut c_char> = snapshots
        .iter()
        .map(|s| CString::new(s.id.clone()).unwrap().into_raw())
        .collect();

    let mut boxed_slice = snapshot_ids.into_boxed_slice();
    let ptr = boxed_slice.as_mut_ptr();
    std::mem::forget(boxed_slice);
    ptr
}

/// Free snapshot list
///
/// # Safety
///
/// `snapshots` must be a valid pointer returned by `airgap_list_snapshots`.
/// `count` must match the count returned by `airgap_list_snapshots`.
#[no_mangle]
pub unsafe extern "C" fn airgap_free_snapshot_list(
    snapshots: *mut *mut c_char,
    count: c_uint,
) {
    if !snapshots.is_null() && count > 0 {
        let slice = slice::from_raw_parts_mut(snapshots, count as usize);
        for snapshot_ptr in slice {
            if !snapshot_ptr.is_null() {
                drop(CString::from_raw(*snapshot_ptr));
            }
        }
        drop(Vec::from_raw_parts(snapshots, count as usize, count as usize));
    }
}

/// Free a C string allocated by this library
///
/// # Safety
///
/// `s` must be a valid pointer to a C string allocated by this library.
#[no_mangle]
pub unsafe extern "C" fn airgap_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Free a sync result
///
/// # Safety
///
/// `result` must be a valid pointer to FFISyncResult.
#[no_mangle]
pub unsafe extern "C" fn airgap_free_sync_result(result: *mut FFISyncResult) {
    if !result.is_null() {
        let result_ref = &*result;
        if !result_ref.snapshot_id.is_null() {
            drop(CString::from_raw(result_ref.snapshot_id));
        }
    }
}

/// Free an FFI result error message
///
/// # Safety
///
/// `result` must be a valid pointer to FFIResult.
#[no_mangle]
pub unsafe extern "C" fn airgap_free_result(result: *mut FFIResult) {
    if !result.is_null() {
        let result_ref = &*result;
        if !result_ref.error_message.is_null() {
            drop(CString::from_raw(result_ref.error_message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_result() {
        let success = FFIResult::success();
        assert_eq!(success.error_code, FFIErrorCode::Success);
        assert!(success.error_message.is_null());

        let error = FFIResult::error(FFIErrorCode::ConfigError, "Test error");
        assert_eq!(error.error_code, FFIErrorCode::ConfigError);
        assert!(!error.error_message.is_null());
        unsafe {
            let msg = CStr::from_ptr(error.error_message).to_str().unwrap();
            assert_eq!(msg, "Test error");
            airgap_free_string(error.error_message);
        }
    }

    #[test]
    fn test_initialize() {
        let result = airgap_initialize();
        assert_eq!(result.error_code, FFIErrorCode::Success);
    }

    #[test]
    fn test_get_version() {
        let version_ptr = airgap_get_version();
        assert!(!version_ptr.is_null());
        unsafe {
            let version = CStr::from_ptr(version_ptr).to_str().unwrap();
            assert!(!version.is_empty());
            airgap_free_string(version_ptr);
        }
    }
}

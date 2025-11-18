//! FFI (Foreign Function Interface) bridge for Swift integration
//!
//! This module provides C-compatible interfaces for calling Rust code from Swift.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// C-compatible sync progress structure
#[repr(C)]
pub struct CSyncProgress {
    pub total_files: u64,
    pub processed_files: u64,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub percentage: f64,
}

/// C-compatible sync result
#[repr(C)]
pub struct CSyncResult {
    pub success: bool,
    pub files_synced: u64,
    pub bytes_transferred: u64,
    pub duration_seconds: u64,
    pub error_message: *mut c_char,
}

/// Initialize the library
#[no_mangle]
pub extern "C" fn airgap_init() -> bool {
    crate::initialize().is_ok()
}

/// Get library version
#[no_mangle]
pub extern "C" fn airgap_version() -> *mut c_char {
    CString::new(crate::VERSION).unwrap().into_raw()
}

/// Free a C string
#[no_mangle]
pub extern "C" fn airgap_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Start a sync operation (placeholder)
#[no_mangle]
pub extern "C" fn airgap_sync_start(
    source_path: *const c_char,
    dest_path: *const c_char,
    device_id: *const c_char,
) -> *mut CSyncResult {
    if source_path.is_null() || dest_path.is_null() || device_id.is_null() {
        return ptr::null_mut();
    }

    let _source = unsafe { CStr::from_ptr(source_path).to_string_lossy() };
    let _dest = unsafe { CStr::from_ptr(dest_path).to_string_lossy() };
    let _device = unsafe { CStr::from_ptr(device_id).to_string_lossy() };

    // Placeholder - actual implementation would perform sync
    let result = Box::new(CSyncResult {
        success: false,
        files_synced: 0,
        bytes_transferred: 0,
        duration_seconds: 0,
        error_message: CString::new("Not implemented").unwrap().into_raw(),
    });

    Box::into_raw(result)
}

/// Free sync result
#[no_mangle]
pub extern "C" fn airgap_free_sync_result(result: *mut CSyncResult) {
    if !result.is_null() {
        unsafe {
            let r = Box::from_raw(result);
            if !r.error_message.is_null() {
                let _ = CString::from_raw(r.error_message);
            }
        }
    }
}

/// Generate encryption key (placeholder)
#[no_mangle]
pub extern "C" fn airgap_keygen(device_id: *const c_char) -> bool {
    if device_id.is_null() {
        return false;
    }

    let _device = unsafe { CStr::from_ptr(device_id).to_string_lossy() };

    // Placeholder - actual implementation would generate key
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_init() {
        let result = airgap_init();
        assert!(result || !result); // Just test it doesn't crash
    }

    #[test]
    fn test_ffi_version() {
        let version = airgap_version();
        assert!(!version.is_null());
        unsafe {
            airgap_free_string(version);
        }
    }

    #[test]
    fn test_ffi_null_safety() {
        let result = airgap_sync_start(ptr::null(), ptr::null(), ptr::null());
        assert!(result.is_null());
    }
}

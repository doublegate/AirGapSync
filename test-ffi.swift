#!/usr/bin/env swift
//
// test-ffi.swift
// Simple test to verify FFI integration works
//

import Foundation

// Import the FFI - this would normally come through a bridging header
// For testing, we'll declare the functions directly

@_silgen_name("airgap_initialize")
func airgap_initialize() -> FFIResult

@_silgen_name("airgap_get_version")
func airgap_get_version() -> UnsafeMutablePointer<CChar>?

@_silgen_name("airgap_free_string")
func airgap_free_string(_ str: UnsafeMutablePointer<CChar>?)

// FFI types
enum FFIErrorCode: UInt32 {
    case Success = 0
    case NullPointer = 1
    case InvalidUtf8 = 2
    case ConfigError = 3
    case CryptoError = 4
    case KeychainError = 5
    case IoError = 6
    case DeviceNotFound = 7
    case SyncError = 8
    case Unknown = 99
}

struct FFIResult {
    let error_code: FFIErrorCode
    let error_message: UnsafeMutablePointer<CChar>?
}

// Test the FFI
print("Testing AirGapSync FFI...")
print("=" * 50)

// Test 1: Initialize
print("\n1. Testing initialization...")
let initResult = airgap_initialize()
if initResult.error_code == .Success {
    print("   ✅ Initialization successful")
} else {
    print("   ❌ Initialization failed")
    if let msgPtr = initResult.error_message {
        let message = String(cString: msgPtr)
        print("   Error: \(message)")
        airgap_free_string(msgPtr)
    }
}

// Test 2: Get version
print("\n2. Testing get_version...")
if let versionPtr = airgap_get_version() {
    let version = String(cString: versionPtr)
    print("   ✅ Version: \(version)")
    airgap_free_string(versionPtr)
} else {
    print("   ❌ Failed to get version")
}

print("\n" + "=" * 50)
print("FFI tests complete!")

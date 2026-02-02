# AirGapSync Phase 3 Continuation Guide

This document provides everything needed to continue development of AirGapSync Phase 3 (SwiftUI & Production).

## Current Status: ~36% Complete

### ✅ Completed (4/11 tasks):
1. **FFI Bridge Implementation** - Full C-compatible FFI layer
2. **cbindgen Configuration** - Automatic C header generation
3. **Swift FFI Bridge Layer** - Type-safe Swift wrapper
4. **Universal Binary Build** - Makefile targets for x86_64 and ARM64

### 🔄 In Progress (1/11 tasks):
3. **Update Swift SyncManager** - FFI integration started, needs completion

### ⏳ Remaining (6/11 tasks):
4. Create and Configure Xcode Project
6. Set Up Code Signing & Notarization
7. Create Comprehensive Benchmarks
8. Build Comprehensive Test Suite
9. Create Complete Documentation
10. Set Up Distribution Pipeline
11. Add Final Polish & Accessibility

---

## Quick Start for Next Session

### 1. Verify Build Environment
```bash
cd /Users/parobek/Code/AirGapSync

# Test Rust library builds
cargo build --lib --release

# Verify FFI header generated
ls -l target/airgapsync.h

# Test universal library build
make universal-lib

# Should create target/universal/libairgap_sync.a
ls -l target/universal/libairgap_sync.a
```

### 2. Xcode Project Setup

The Xcode project exists at: `/Users/parobek/Code/AirGapSync/AirGapSync.xcodeproj`

**Critical Configuration Steps:**

#### A. Add Pre-Build Script
1. Open `AirGapSync.xcodeproj` in Xcode
2. Select project in navigator
3. Select "AirGapSync" target
4. Go to "Build Phases" tab
5. Click "+" → "New Run Script Phase"
6. Move it to the top (before "Compile Sources")
7. Add script:
```bash
#!/bin/bash
set -e
cd "${SRCROOT}"
echo "Building Rust library..."
make universal-lib
echo "Rust library built successfully"
```
8. Name it: "Build Rust Library"

#### B. Configure Build Settings
Search for these settings and update:

**Library Search Paths**:
- Add: `$(SRCROOT)/target/universal`
- Add: `$(SRCROOT)/target/release`
- Add: `$(SRCROOT)/target/debug` (for debug builds)

**Header Search Paths**:
- Add: `$(SRCROOT)/target`

**Objective-C Bridging Header**:
- Set to: `$(SRCROOT)/AirGapSync/AirGapSync-Bridging-Header.h`

**Other Linker Flags**:
- Add: `-lairgap_sync`
- Add: `-lresolv` (for Rust's networking)
- Add: `-framework Security` (for Keychain)

**Runpath Search Paths**:
- Add: `@executable_path/../Frameworks`

#### C. Add Swift Files to Project
If not already added:
1. Right-click on project → "Add Files to AirGapSync"
2. Add `AirGapSync/AirGapSync/FFIBridge.swift`
3. Ensure "Copy items if needed" is checked
4. Ensure target membership includes "AirGapSync"

#### D. Verify Bridging Header
1. Check that `AirGapSync/AirGapSync-Bridging-Header.h` exists
2. Content should be:
```objective-c
#import "airgapsync.h"
```

### 3. Test Basic Build
```bash
# From terminal
make app

# Or in Xcode
# Press Cmd+B to build
```

**Expected Outcomes**:
- ✅ Rust library compiles
- ✅ Header is generated
- ✅ Swift code compiles
- ✅ Linking succeeds
- ✅ App bundle is created

**Common Issues & Fixes**:

**Issue**: "airgapsync.h file not found"
- **Fix**: Run `cargo build --lib` first to generate header
- **Fix**: Check Header Search Paths includes `$(SRCROOT)/target`

**Issue**: "Undefined symbols for architecture x86_64"
- **Fix**: Ensure Library Search Paths includes `$(SRCROOT)/target/universal`
- **Fix**: Check that `libairgap_sync.a` exists in that path
- **Fix**: Add `-lairgap_sync` to Other Linker Flags

**Issue**: "Use of undeclared type 'FFIResult'"
- **Fix**: Verify bridging header path is set correctly
- **Fix**: Rebuild Rust library to regenerate header

---

## Implementation Priority Order

### IMMEDIATE (Critical Path)
1. ✅ Configure Xcode project (follow steps above)
2. Update SyncManager to use FFI
3. Test basic sync operation

### HIGH PRIORITY (Week 1)
4. Create example config generator
5. Implement progress reporting mechanism
6. Test with real USB device
7. Add error handling & recovery

### MEDIUM PRIORITY (Week 2)
8. Enhance benchmarks (crypto_bench, sync_bench)
9. Write integration tests (phase3_integration.rs)
10. Write E2E tests (e2e_tests.rs)
11. Document FFI API

### LOW PRIORITY (Week 3+)
12. Code signing setup
13. Notarization scripts
14. Distribution pipeline
15. App icon design
16. Accessibility features
17. Localization

---

## Files Created This Session

### Core FFI Layer
- `src/rust_core/ffi.rs` (724 lines)
  - Complete C-compatible FFI
  - All memory management functions
  - Error handling with FFIResult
  - Device info, sync, snapshot operations

### Build Configuration
- `build.rs` (48 lines)
  - Automatic header generation with cbindgen
  - Copies header to accessible location
- `cbindgen.toml` (36 lines)
  - cbindgen configuration
  - C language output with C++ compat

### Swift Bridge Layer
- `AirGapSync/AirGapSync-Bridging-Header.h` (14 lines)
  - Imports generated C header
- `AirGapSync/AirGapSync/FFIBridge.swift` (268 lines)
  - `AirGapError` - Swift error enum
  - `AirGapSyncOptions` - Swift-friendly options
  - `AirGapSyncResult` - Result wrapper
  - `AirGapSyncEngine` - Main API class

### Build System
- Updated `Makefile` (150+ lines)
  - `make universal-lib` - Build universal library
  - `make app` - Build SwiftUI app
  - `make run-app` - Build and run app
  - `make clean-app` - Clean Xcode artifacts

### Documentation
- `docs/PHASE3-PROGRESS.md` (comprehensive status)
- `docs/CONTINUATION-GUIDE.md` (this file)

**Total New Code**: ~1,300+ lines

---

## Key Technical Details

### FFI Memory Management Rules

**Strings**:
- FFI returns `*mut c_char` (owned)
- Swift must free with `airgap_free_string()`
- FFIBridge uses `defer` for automatic cleanup

**Structs**:
- FFI returns `*mut Type` (owned)
- Swift must free with corresponding `airgap_free_*()` function
- Examples: `airgap_free_config()`, `airgap_free_device_info()`

**Arrays**:
- FFI returns `*mut *mut c_char` with count
- Swift must free each element then array
- Use `airgap_free_snapshot_list(ptr, count)`

**Opaque Handles**:
- Config and SyncEngine are opaque pointers
- Rust retains ownership
- Must call free functions when done

### Threading Considerations

**Current Limitation**: Progress callbacks are disabled in FFI because raw C function pointers can't safely cross thread boundaries with Rust's Send+Sync requirements.

**Solutions**:
1. **Polling**: Add `airgap_get_progress()` function that Swift can call
2. **Shared State**: Use unsafe shared memory (not recommended)
3. **Channel-based**: Implement a channel system (complex)
4. **Recommendation**: Use polling for now, it's the simplest and safest

### Error Handling Flow

```
Rust Error → FFIResult → AirGapError (Swift) → throw
```

1. Rust function returns `Result<T, Error>`
2. FFI converts to `FFIResult` with code and message
3. Swift checks `error_code != Success`
4. If error, creates `AirGapError` enum case
5. Throws error to Swift caller

---

## SyncManager Integration Example

Here's how to update `SyncManager.swift` to use the FFI:

```swift
class SyncManager: ObservableObject {
    // ... existing properties ...

    private var syncEngine: AirGapSyncEngine?

    private init() {
        // ... existing init code ...

        // Initialize sync engine
        do {
            // Create config file if needed
            let configPath = createConfigFile()
            syncEngine = try AirGapSyncEngine(configPath: configPath)
        } catch {
            print("Failed to initialize sync engine: \(error)")
        }
    }

    private func createConfigFile() -> String {
        let configDir = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".airgapsync")

        try? FileManager.default.createDirectory(at: configDir,
            withIntermediateDirectories: true)

        let configPath = configDir.appendingPathComponent("config.toml").path

        // Create config if it doesn't exist
        if !FileManager.default.fileExists(atPath: configPath) {
            let config = generateConfig()
            try? config.write(toFile: configPath, atomically: true, encoding: .utf8)
        }

        return configPath
    }

    private func generateConfig() -> String {
        let sourceDir = sourceDirectory ?? NSHomeDirectory() + "/Documents"

        return """
        [general]
        verbose = false

        [source]
        path = "\(sourceDir)"
        exclude = [".DS_Store", "*.tmp", ".git/*"]

        [policy]
        compression_level = \(UserDefaults.standard.integer(forKey: "compressionLevel"))
        verify_after_write = \(UserDefaults.standard.bool(forKey: "verifyAfterWrite"))
        retain_snapshots = 10

        [[device]]
        id = "device1"
        name = "Default Device"
        mount_point = "/Volumes/USB001"

        [device.encryption]
        algorithm = "\(UserDefaults.standard.string(forKey: "encryptionAlgorithm") ?? "aes-256-gcm")"
        """
    }

    private func performSync(from source: String, to device: Device) -> (error: String?) {
        guard let syncEngine = syncEngine else {
            return (error: "Sync engine not initialized")
        }

        do {
            // Configure sync options
            var options = AirGapSyncOptions()
            options.parallelWorkers = UserDefaults.standard.integer(forKey: "parallelWorkers")
            options.chunkSize = UserDefaults.standard.integer(forKey: "chunkSizeMB") * 1024 * 1024
            options.compressionLevel = UserDefaults.standard.integer(forKey: "compressionLevel")
            options.verifyAfterWrite = UserDefaults.standard.bool(forKey: "verifyAfterWrite")

            // Perform sync
            let result = try syncEngine.sync(deviceId: device.id, options: options)

            print("Sync completed: \(result.filesSynced) files, \(result.bytesTransferred) bytes")

            return (error: nil)
        } catch {
            return (error: error.localizedDescription)
        }
    }
}
```

---

## Testing Checklist

### Unit Tests (Rust)
- [ ] FFI initialization
- [ ] Config loading/freeing
- [ ] Engine creation/destruction
- [ ] Sync operation
- [ ] Device info retrieval
- [ ] Snapshot listing
- [ ] Error handling
- [ ] Memory safety (valgrind/instruments)

### Integration Tests (Rust)
- [ ] Full sync workflow via FFI
- [ ] Config→Engine→Sync→Cleanup
- [ ] Multiple devices
- [ ] Snapshot verification
- [ ] Error recovery

### Swift Tests
- [ ] FFIBridge initialization
- [ ] Config generation
- [ ] Sync operation
- [ ] Error handling
- [ ] Memory management (Instruments)

### Manual Tests
- [ ] Sync to real USB device
- [ ] Large file sync (>1GB)
- [ ] Many files (>1000)
- [ ] Device unmount during sync
- [ ] Power loss simulation
- [ ] Corrupt snapshot recovery

### Performance Tests
- [ ] Sync speed >100MB/s on USB 3.0
- [ ] Memory usage <100MB
- [ ] Startup time <1s
- [ ] Progress reporting accuracy

---

## Common Commands Reference

```bash
# Build Rust library only
cargo build --lib --release

# Build universal library
make universal-lib

# Build SwiftUI app
make app

# Run app
make run-app

# Clean everything
make clean

# Run Rust tests
cargo test

# Run Rust tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench

# Check for warnings
cargo clippy --all -- -D warnings

# Format code
cargo fmt --all

# Generate docs
cargo doc --no-deps --open

# Security audit
cargo audit
```

### Xcode Commands
```bash
# Build from command line
xcodebuild -project AirGapSync.xcodeproj \
  -scheme AirGapSync \
  -configuration Debug \
  build

# Clean
xcodebuild -project AirGapSync.xcodeproj \
  -scheme AirGapSync \
  clean

# Archive for distribution
xcodebuild -project AirGapSync.xcodeproj \
  -scheme AirGapSync \
  -configuration Release \
  archive \
  -archivePath build/AirGapSync.xcarchive
```

---

## Troubleshooting Guide

### Build Fails: "airgapsync.h not found"
1. Run `cargo build --lib` to generate header
2. Check `target/airgapsync.h` exists
3. Verify Header Search Paths in Xcode includes `$(SRCROOT)/target`

### Build Fails: "Undefined symbol: _airgap_initialize"
1. Check Library Search Paths includes path to `libairgap_sync.a`
2. Verify library was built: `ls target/universal/libairgap_sync.a`
3. Check Other Linker Flags includes `-lairgap_sync`
4. Clean and rebuild: `make clean && make universal-lib && make app`

### Runtime Crash: EXC_BAD_ACCESS
1. Check all FFI strings are freed properly
2. Use Instruments → Zombies to find over-released objects
3. Verify bridging header is imported correctly
4. Check that opaque pointers aren't dereferenced in Swift

### Sync Fails: "Device not found"
1. Check device ID matches config
2. Verify mount point exists and is writable
3. Check device is mounted: `diskutil list`
4. Verify config file is correct: `cat ~/.airgapsync/config.toml`

### Performance Issues
1. Run benchmarks: `cargo bench`
2. Profile with Instruments
3. Check chunk size (1MB is default)
4. Verify parallel workers setting (4 is default)
5. Check USB interface version (USB 3.0 vs 2.0)

---

## Next Steps Summary

1. **Immediate**:
   - Configure Xcode project build settings
   - Test that app builds
   - Update SyncManager to use FFI

2. **This Week**:
   - Complete SyncManager integration
   - Test with real device
   - Add progress reporting
   - Handle errors gracefully

3. **Next Week**:
   - Write comprehensive tests
   - Enhance benchmarks
   - Document FFI API
   - Add example configurations

4. **Following Weeks**:
   - Code signing & notarization
   - Distribution pipeline
   - App icon & polish
   - User documentation

---

## Important Reminders

1. **User's Directive**: Only add, enhance, or fully implement - never remove or simplify
2. **CLI Must Remain**: Keep the 1747-line CLI fully functional
3. **Complete Implementation**: No placeholders or stubs
4. **Test Everything**: >80% code coverage target
5. **Document Changes**: Update docs as you go

---

## Success Criteria

Phase 3 is complete when:
- ✅ FFI layer fully functional
- ✅ Swift app builds and runs
- ✅ Can sync to real USB device
- ✅ Universal binary for Intel + ARM
- ✅ All tests pass
- ✅ Code signing configured
- ✅ Distribution package created
- ✅ Documentation complete
- ✅ App icon designed
- ✅ Performance targets met

**Current**: 4/10 criteria met (40%)

---

## Contact & Resources

**Project Location**: `/Users/parobek/Code/AirGapSync`

**Key Files**:
- FFI Implementation: `src/rust_core/ffi.rs`
- Swift Bridge: `AirGapSync/AirGapSync/FFIBridge.swift`
- Progress Doc: `docs/PHASE3-PROGRESS.md`
- This Guide: `docs/CONTINUATION-GUIDE.md`

**Generated Files**:
- C Header: `target/airgapsync.h`
- Library: `target/universal/libairgap_sync.a`
- CLI Binary: `target/release/airgapsync`

**Configuration**:
- Cargo: `Cargo.toml`
- cbindgen: `cbindgen.toml`
- Build: `build.rs`
- Make: `Makefile`

---

Last Updated: 2026-02-02
Session: claude-code (Opus 4.5)

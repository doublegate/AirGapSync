# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AirGapSync is an encrypted removable-media sync manager for macOS that provides secure data synchronization to removable media with an air-gap security model. It combines a Rust core for performance and security with a native SwiftUI interface for ease of use.

## Development Commands

### Quick Reference

```bash
# Common development tasks
make build          # Build debug version
make release        # Build release version
make test           # Run all tests
make run            # Run with example args
make lint           # Run clippy linter
make fmt            # Format code
make doc            # Generate documentation
make help           # Show all available commands
```

### Detailed Commands

```bash
# Build commands
cargo build                    # Build all components
cargo build --release          # Build optimized release
cargo build --bin airgapsync   # Build CLI only

# Running
cargo run --bin airgapsync -- --src ~/Data --dest /Volumes/USB001
cargo run --bin airgapsync -- --config ~/.airgapsync/config.toml

# Testing
cargo test                     # Run all tests
cargo test --lib              # Run library tests only
cargo test --doc              # Run documentation tests
cargo test -- --nocapture     # Show test output

# Code quality
cargo fmt                      # Format code
cargo fmt -- --check          # Check formatting
cargo clippy                   # Run linter
cargo clippy -- -D warnings   # Treat warnings as errors

# Documentation
cargo doc --no-deps --open    # Generate and open docs
cargo doc --document-private-items  # Include private API

# Security
cargo audit                    # Check for vulnerabilities
cargo outdated                # Check for outdated deps
```

### SwiftUI Development

The SwiftUI menu-bar app is located in `AirGapSync/AirGapSync/`. Use Xcode to build and run the macOS app.

```bash
# Open in Xcode
open AirGapSync/AirGapSync.xcodeproj

# Build from command line
xcodebuild -project AirGapSync/AirGapSync.xcodeproj -scheme AirGapSync build
```

## Architecture

### Core Components

1. **Rust Core Library** (`src/rust_core/lib.rs`)
   - Sync engine with diff/chunk/encrypt pipeline
   - Keychain integration for secure key storage
   - Audit logging with cryptographic signatures
   - FFI bridge for Swift integration

2. **CLI Interface** (`src/cli/main.rs`)
   - Full-featured command-line tool
   - Progress reporting and dry-run mode
   - Configuration file support
   - Scriptable for automation

3. **SwiftUI Menu Bar App** (`AirGapSync/AirGapSync/`)
   - Native macOS menu bar interface
   - Real-time sync status monitoring
   - Device detection and management
   - Settings and configuration UI

### Key Technologies

- **Encryption**: AES-256-GCM, ChaCha20-Poly1305
- **Asymmetric**: RSA-2048/4096, ECDSA P-256/P-384
- **Key Agreement**: ECDH with P-256/P-384 curves
- **Key Management**: macOS Keychain Services
- **Compression**: zstd for efficient storage
- **Serialization**: TOML for configuration
- **FFI**: cbindgen for Rust-Swift bridge

## CLI Commands

### Current Implementation
```bash
# Key management
airgapsync keygen <device-id> --algorithm <alg>
airgapsync keys
airgapsync rotate <device-id>

# Encryption operations
airgapsync encrypt <input> <output> <device-id>
airgapsync decrypt <input> <output> <device-id>

# Configuration
airgapsync init
airgapsync validate
airgapsync schema --output <path>

# System
airgapsync info

# Sync operations (Phase 2)
airgapsync sync [--device <id>] [--dry-run] [--resume]
airgapsync verify <device-id> [--snapshot <id>]
airgapsync restore <snapshot-id> <dest> [--device <id>]

# Device management
airgapsync device list
airgapsync device add <id> <name> <mount-point>
airgapsync device remove <id>
airgapsync device info <id>

# Snapshot management
airgapsync snapshot list <device-id>
airgapsync snapshot info <id> <device-id>
airgapsync snapshot delete <id> <device-id>
airgapsync snapshot diff <snap1> <snap2> <device-id>

# Audit and monitoring
airgapsync audit-log [--device <id>] [--limit <n>]
airgapsync watch [--interval <secs>] [--dry-run]

# Shell completions
airgapsync completion <shell>

# Global options
airgapsync --dry-run                    # Preview mode
airgapsync --rotate-keys                # Auto key rotation
airgapsync --audit-log                  # View audit log
airgapsync --verify <device>            # Quick verify
airgapsync --restore <snapshot:dest>    # Quick restore
```

## Configuration

Configuration files use TOML format and are stored at `~/.airgapsync/config.toml`.

Key sections:
- `[source]` - Source directory settings
- `[[device]]` - Device configurations (multiple allowed)
- `[policy]` - Retention and GC policies
- `[security]` - Encryption and key settings
- `[schedule]` - Automatic sync scheduling

See `config.example.toml` for a complete example.

## Development Workflow

### Setting Up
```bash
# Clone repository
git clone https://github.com/<org>/airgap-sync.git
cd airgap-sync

# Install dependencies
make setup

# Create example config
make example-config
```

### Making Changes
1. Create feature branch: `git checkout -b feature/your-feature`
2. Make changes and test: `make test`
3. Format code: `make fmt`
4. Run linter: `make lint`
5. Update documentation if needed
6. Commit with descriptive message

### Before Submitting PR
- Run full test suite: `make test`
- Check formatting: `make fmt-check`
- Run security audit: `make audit`
- Update CHANGELOG.md if applicable
- Ensure CI passes

## Project Structure

```
.
├── src/
│   ├── rust_core/      # Core sync engine
│   │   ├── lib.rs      # Library entry point
│   │   ├── config.rs   # Configuration handling
│   │   ├── crypto.rs   # Encryption/decryption
│   │   ├── keychain.rs # macOS Keychain integration
│   │   ├── keys.rs     # Asymmetric key management
│   │   ├── schema.rs   # Config schema validation
│   │   ├── sync.rs     # Sync orchestration
│   │   ├── diff.rs     # File comparison engine
│   │   ├── chunk.rs    # Chunk-based processing
│   │   ├── snapshot.rs # Backup snapshots
│   │   └── audit.rs    # Audit logging
│   ├── cli/            # CLI application
│   │   └── main.rs     # 1747 lines, full Phase 2 implementation
│   └── swift_ui/       # macOS GUI (placeholder)
├── AirGapSync/         # Xcode project for SwiftUI app
│   └── AirGapSync/
│       ├── MenuBarApp.swift
│       └── SyncManager.swift
├── docs/               # Documentation
│   ├── ARCHITECTURE.md # System design
│   ├── API.md          # Library API
│   ├── SECURITY.md     # Security model
│   ├── PHASE1-COMPLETE.md
│   └── PHASE2-COMPLETE.md
├── to-dos/             # Development tasks
│   ├── phase-*.md      # Phase planning
│   └── ROADMAP.md      # Project roadmap
├── tests/              # Integration tests
│   ├── phase1_integration.rs
│   └── phase2_integration.rs
├── benches/            # Performance benchmarks
│   ├── crypto_bench.rs
│   └── sync_bench.rs
├── Cargo.toml          # Rust configuration
├── Makefile            # Build automation
└── config.example.toml # Example config
```

## Key Design Decisions

1. **Rust Core**: Chosen for memory safety and performance
2. **SwiftUI**: Native macOS experience with modern UI
3. **Keychain Integration**: Leverage OS security features
4. **Chunk-based Storage**: Efficient incremental updates
5. **Immutable Logs**: Tamper-evident audit trail
6. **TOML Config**: Human-readable configuration

## Testing Strategy

- **Unit Tests**: Core library functions
- **Integration Tests**: End-to-end sync scenarios
- **Fuzz Testing**: Security-critical components
- **Performance Tests**: Benchmark sync speed
- **UI Tests**: SwiftUI component testing

## Performance Targets

- Sync speed: >100MB/s on USB 3.0
- Memory usage: <100MB for typical sync
- Startup time: <1 second
- Chunk size: 1MB (configurable)
- Parallel files: 4 (configurable)

## Security Considerations

- Never store keys on removable media
- All data encrypted before writing
- Audit logs are append-only and signed
- Keys require user authentication to access
- Support for key rotation and revocation

## Current Status: Phase 2 Complete (2025-07-20)

### What's Been Completed
- ✅ Full cryptographic implementation (AES, ChaCha20, RSA, ECDSA, ECDH)
- ✅ macOS Keychain integration with security-framework
- ✅ Comprehensive CLI with 20+ commands (1747 lines)
- ✅ TOML/JSON configuration with schema validation
- ✅ Complete sync engine with diff/chunk/snapshot
- ✅ Audit logging with tamper-evident signatures
- ✅ Device monitoring and auto-sync
- ✅ Shell completion generation
- ✅ Streaming encryption for large files
- ✅ Full test suite with 100% passing tests
- ✅ Zero compilation warnings or errors
- ✅ Full API documentation

### Key Technical Decisions Made
1. **ECDH Implementation**: Using elliptic-curve crates (p256, p384) instead of ring's limited ECDH API
2. **RSA Signing**: Implemented with hazmat traits for prehash signing (SHA-256/384)
3. **Schema Validation**: Using schemars with chrono feature for DateTime support
4. **Error Handling**: Comprehensive error types with thiserror
5. **Testing**: Integration tests in tests/phase1_integration.rs validate all core functionality
6. **Progress Reporting**: Using indicatif crate for CLI progress bars
7. **Parallel Processing**: Using rayon for concurrent file operations
8. **Compression**: Using zstd for better compression ratios
9. **Hashing**: Using blake3 for fast content-addressed storage

### Important Implementation Details
- **Keychain parameter order**: find_generic_password(None, &service_name, &account_name)
- **Enum serialization**: Using kebab-case for encryption algorithms (e.g., "aes256-gcm")
- **ECDH shared secrets**: Using diffie_hellman() from elliptic-curve crate
- **Memory safety**: All sensitive data uses zeroize for secure cleanup
- **Progress callbacks**: Type is `Arc<dyn Fn(SyncProgress) + Send + Sync>` (no reference)
- **Config loading**: Config::from_file() expects &PathBuf, not &Path
- **Device config**: Use `config.device` (singular), not `config.devices`

## Next Steps: Phase 3 (SwiftUI & Production)

When continuing development with `claude -c`, focus on:

1. **SwiftUI Menu Bar App**: Complete implementation in AirGapSync/AirGapSync/
2. **FFI Bridge**: Create Rust-Swift interop layer
3. **Device Detection**: Implement DiskArbitration framework integration
4. **Auto-sync Daemon**: Background service for continuous monitoring
5. **Performance Optimization**: Profile and optimize for large datasets
6. **Universal Binary**: Build for both Intel and Apple Silicon
7. **Code Signing**: Prepare for notarization and distribution
8. **Documentation**: User guide and API documentation

Key files to work on:
- `AirGapSync/AirGapSync/` (enhance SwiftUI app)
- `src/rust_core/ffi.rs` (create new)
- `build.rs` (add cbindgen configuration)
- Update `Makefile` with universal binary targets

## Important Notes for Continuation

1. **All tests must pass**: Run `cargo test` before any commits
2. **No warnings allowed**: Use `cargo clippy` to check
3. **Document new APIs**: All public functions need documentation
4. **Update CHANGELOG**: Track all significant changes
5. **Follow existing patterns**: Check similar code for conventions
6. **CLI is complete**: 1747 lines with all Phase 2+ features implemented
7. **Build succeeds**: Project compiles with zero errors

## Dependencies Added in Phase 2

```toml
# Progress reporting
indicatif = "0.17"

# Parallel processing
rayon = "1.8"

# Compression
zstd = "0.13"
flate2 = "1.0"

# Hashing
blake3 = "1.5"

# File patterns
glob = "0.3"

# UUID generation
uuid = { version = "1.11", features = ["v4", "serde"] }

# Shell completions
clap_complete = "4"
```

## Common Issues and Solutions

1. **Keychain tests failing**: May need manual keychain unlock on macOS
2. **ECDH test failures**: Ensure elliptic-curve crates are properly imported
3. **Schema validation**: Remember kebab-case for enum serialization
4. **Memory leaks**: Use `zeroize` for all sensitive data
5. **Progress callback types**: No reference on SyncProgress parameter
6. **Config path types**: Use PathBuf references, not Path

## Current Git Status

- Branch: main
- Recent major commits:
  - "feat: Complete Phase 1 - Full cryptographic implementation with ECDH support"
  - "Complete Phase 1: Design & Key Management Implementation"
- CLI fully implemented: 1747 lines
- All changes staged for commit
- Ready for Phase 3 development

## Critical Context from Session

**User's Primary Directive**: "ONLY add to, enhance, or fully implement features --> do NOT remove/delete or disable or simplify anything --> always make sure everything is completely developed"

The user was very explicit about never removing code. When I accidentally reduced the CLI from ~1500 lines to 1 line, they were justifiably upset. The CLI has now been fully restored and enhanced to 1747 lines with additional features.

See `docs/PHASE2-COMPLETE.md` for detailed Phase 2 summary.

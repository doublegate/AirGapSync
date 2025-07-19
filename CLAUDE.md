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

The SwiftUI menu-bar app is located in `src/swift_ui/`. Use Xcode to build and run the macOS app.

```bash
# Open in Xcode
open AirGapSync.xcodeproj

# Build from command line
xcodebuild -project AirGapSync.xcodeproj -scheme AirGapSync build
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

3. **SwiftUI Menu Bar App** (`src/swift_ui/App.swift`)
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
```

### Planned Commands (Phase 2+)
```bash
airgapsync sync                           # Run with config file
airgapsync --dry-run                      # Preview changes
airgapsync --rotate-keys                  # Rotate encryption keys
airgapsync --audit-log                    # View audit trail
airgapsync --verify <device>              # Verify backup integrity
airgapsync --restore <snapshot> <dest>    # Restore from snapshot
airgapsync config --init                  # Initialize config
airgapsync device --list                  # List devices
airgapsync snapshot --list <device>       # List snapshots
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
│   ├── cli/            # CLI application
│   └── swift_ui/       # macOS GUI
├── docs/               # Documentation
│   ├── ARCHITECTURE.md # System design
│   ├── API.md          # Library API
│   ├── SECURITY.md     # Security model
│   └── ...
├── to-dos/             # Development tasks
│   ├── phase-*.md      # Phase planning
│   └── ROADMAP.md      # Project roadmap
├── tests/              # Integration tests
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

## Current Status: Phase 1 Complete (2025-07-19)

### What's Been Completed
- ✅ Full cryptographic implementation (AES, ChaCha20, RSA, ECDSA, ECDH)
- ✅ macOS Keychain integration with security-framework
- ✅ Comprehensive CLI with 11+ commands
- ✅ TOML/JSON configuration with schema validation
- ✅ Complete test suite with 100% passing tests
- ✅ Zero compilation warnings or errors
- ✅ Full API documentation

### Key Technical Decisions Made
1. **ECDH Implementation**: Using elliptic-curve crates (p256, p384) instead of ring's limited ECDH API
2. **RSA Signing**: Implemented with hazmat traits for prehash signing (SHA-256/384)
3. **Schema Validation**: Using schemars with chrono feature for DateTime support
4. **Error Handling**: Comprehensive error types with thiserror
5. **Testing**: Integration tests in tests/phase1_integration.rs validate all core functionality

### Important Implementation Details
- **Keychain parameter order**: find_generic_password(None, &service_name, &account_name)
- **Enum serialization**: Using kebab-case for encryption algorithms (e.g., "aes256-gcm")
- **ECDH shared secrets**: Using diffie_hellman() from elliptic-curve crate
- **Memory safety**: All sensitive data uses zeroize for secure cleanup

## Next Steps: Phase 2 (Sync Engine)

When continuing development with `claude -c`, focus on:

1. **Diff Algorithm**: Implement efficient file comparison
2. **Chunk Processing**: Build the chunk-based sync engine
3. **Streaming Encryption**: Add streaming support for large files
4. **Progress Reporting**: Real-time sync progress feedback
5. **Error Recovery**: Robust handling of partial syncs

Key files to start with:
- `src/rust_core/sync.rs` (create new)
- `src/rust_core/chunk.rs` (create new)
- `src/rust_core/diff.rs` (create new)
- Update `src/cli/main.rs` with sync command

## Important Notes for Continuation

1. **All tests must pass**: Run `cargo test` before any commits
2. **No warnings allowed**: Use `cargo clippy` to check
3. **Document new APIs**: All public functions need documentation
4. **Update CHANGELOG**: Track all significant changes
5. **Follow existing patterns**: Check similar code for conventions

## Dependencies Added in Phase 1

```toml
# Elliptic curve cryptography (added for ECDH)
elliptic-curve = { version = "0.13", features = ["ecdh", "pkcs8", "sec1"] }
p256 = { version = "0.13", features = ["ecdh", "ecdsa", "pkcs8"] }
p384 = { version = "0.13", features = ["ecdh", "ecdsa", "pkcs8"] }
ecdsa = { version = "0.16", features = ["pkcs8", "pem", "signing", "verifying"] }

# Schema validation (chrono feature added)
schemars = { version = "0.8", features = ["chrono"] }
```

## Common Issues and Solutions

1. **Keychain tests failing**: May need manual keychain unlock on macOS
2. **ECDH test failures**: Ensure elliptic-curve crates are properly imported
3. **Schema validation**: Remember kebab-case for enum serialization
4. **Memory leaks**: Use `zeroize` for all sensitive data

## Current Git Status

- Branch: main
- Last commit: "feat: Complete Phase 1 - Full cryptographic implementation with ECDH support"
- All changes pushed to origin
- Ready for Phase 2 development

See `docs/PHASE1-COMPLETE.md` for detailed Phase 1 summary.
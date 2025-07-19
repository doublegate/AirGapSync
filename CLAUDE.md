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
- **Key Management**: macOS Keychain Services
- **Compression**: zstd for efficient storage
- **Serialization**: TOML for configuration
- **FFI**: cbindgen for Rust-Swift bridge

## CLI Commands

### Current Implementation
```bash
airgapsync --src <path> --dest <path>    # Basic sync
```

### Planned Commands
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

## Current Status

Phase 1 (Foundation) - In Progress:
- [x] Project structure established
- [x] Documentation framework created
- [x] Development workflow defined
- [ ] Key management design (in progress)
- [ ] Configuration schema finalization

Next: Phase 2 (Sync Engine) - Q2 2025

See `to-dos/ROADMAP.md` for detailed timeline.
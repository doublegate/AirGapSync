# AirGapSync

**Encrypted Removable-Media Sync Manager**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Swift](https://img.shields.io/badge/swift-F54A2A?style=flat&logo=swift&logoColor=white)](https://swift.org/)
[![macOS](https://img.shields.io/badge/macOS-10.15+-blue)](https://www.apple.com/macos/)

A cross-platform macOS app (SwiftUI + Rust back-end) that automates secure synchronization between local folders and removable media (USB drives, SSDs). Leverages system Keychain for key management and provides immutable audit logs. Supports policy-based retention and GC for compliance.

## 🌟 Features

- 🔐 **End-to-End Encryption**: All data encrypted before writing to removable media
- 🔑 **Secure Key Management**: Integration with macOS Keychain for RSA/ECDSA keypairs
- 📝 **Immutable Audit Logs**: Cryptographically verifiable audit trail of all sync operations
- 🗑️ **Smart Retention**: Policy-based snapshot retention with automatic garbage collection
- 🖥️ **Native macOS UI**: SwiftUI menu-bar app with real-time sync status
- ⚡ **High Performance**: Rust core for fast, memory-safe operations
- 🛡️ **Air-Gap Security**: Designed for secure data transfer to untrusted media

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/<org>/airgap-sync.git
cd airgap-sync

# Build the project
make build

# Run a sync operation
make run

# Or use cargo directly
cargo run --bin airgapsync -- --src ~/Documents --dest /Volumes/SecureUSB
```

## 📦 Installation

### Homebrew (Coming Soon)

```bash
brew tap <org>/airgapsync
brew install --cask airgapsync
```

### From Source

```bash
# Prerequisites
brew install rust

# Clone and build
git clone https://github.com/<org>/airgap-sync.git
cd airgap-sync
make install
```

### Download Binary

Pre-built binaries will be available from the [Releases](https://github.com/<org>/airgap-sync/releases) page.

## 🏗️ Project Structure

```
airgap-sync/
├── src/
│   ├── rust_core/      # Core sync engine and encryption
│   ├── cli/            # Command-line interface
│   └── swift_ui/       # macOS menu-bar application
├── docs/               # Documentation
│   ├── ARCHITECTURE.md # System design and components
│   ├── API.md          # Rust library API reference
│   ├── SECURITY.md     # Threat model and key lifecycle
│   ├── CONFIGURATION.md# Policy file schema
│   └── CLI_REFERENCE.md# Command-line documentation
├── to-dos/             # Development task tracking
│   ├── phase-*.md      # Phase-specific tasks
│   └── ROADMAP.md      # Development roadmap
├── tests/              # Integration and unit tests
├── Makefile            # Build automation
├── Cargo.toml          # Rust project configuration
└── config.example.toml # Example configuration
```

## 📖 Documentation

- [Configuration Guide](docs/CONFIGURATION.md) - Policy file schema and examples
- [Security Model](docs/SECURITY.md) - Threat model and key lifecycle
- [CLI Reference](docs/CLI_REFERENCE.md) - Command-line interface documentation
- [Architecture](docs/ARCHITECTURE.md) - System design and components
- [API Documentation](docs/API.md) - Rust library API reference
- [Development Roadmap](to-dos/ROADMAP.md) - Project milestones and timeline

## 🔧 Configuration

AirGapSync uses TOML configuration files. Copy `config.example.toml` to `~/.airgapsync/config.toml` and customize:

```toml
[source]
path = "/Users/username/Documents"
exclude = ["*.tmp", ".DS_Store", "node_modules/"]

[[device]]
id = "USB001"
name = "Secure Backup USB"
mount_point = "/Volumes/SecureUSB"

[policy]
retain_snapshots = 7
gc_interval_hours = 24

[security]
key_rotation_days = 90
audit_level = "full"
```

See [config.example.toml](config.example.toml) for all available options.

## 🎯 Use Cases

### Personal Data Backup
Securely backup sensitive documents to USB drives without worrying about device theft or loss.

### Corporate Compliance
Meet regulatory requirements for air-gapped backups with full audit trails and retention policies.

### Secure Data Transfer
Transfer confidential data between air-gapped systems using encrypted removable media.

### Disaster Recovery
Maintain offline backups that are immune to ransomware and network-based attacks.

## 🛠️ Development

### Prerequisites

- macOS 10.15 or later
- Rust 1.70 or later
- Xcode 14 or later (for SwiftUI components)

### Building from Source

```bash
# Setup development environment
make setup

# Build debug version
make build

# Run tests
make test

# Run linter
make lint

# Format code
make fmt

# Generate documentation
make doc
```

### Available Make Commands

```bash
make help              # Show all available commands
make run               # Run with example arguments
make release           # Build optimized release
make audit             # Run security audit
make bench             # Run benchmarks
make universal         # Build macOS universal binary
make package           # Create distribution package
```

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`make test`)
5. Format code (`make fmt`)
6. Commit changes (`git commit -m 'Add amazing feature'`)
7. Push to branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## 📋 Development Phases

### ✅ Phase 1: Design & Key Management
- [x] Project structure and documentation
- [x] Configuration schema design
- [ ] Keychain integration design
- [ ] Encryption architecture

### 🚧 Phase 2: Sync Engine Prototype (Current)
- [ ] Diff algorithm implementation
- [ ] Chunk-based processing
- [ ] Streaming encryption
- [ ] CLI implementation

### 📋 Phase 3: UI Implementation
- [ ] SwiftUI menu-bar app
- [ ] Device detection
- [ ] Real-time sync status
- [ ] Settings interface

### 📋 Phase 4: Audit & Resilience
- [ ] Immutable audit logs
- [ ] Fault injection testing
- [ ] Recovery mechanisms
- [ ] Verification tools

### 📋 Phase 5: Packaging & Distribution
- [ ] Code signing & notarization
- [ ] Homebrew formula
- [ ] CI/CD pipeline
- [ ] Release automation

See [Development Roadmap](to-dos/ROADMAP.md) for detailed timeline and milestones.

## 🔒 Security

AirGapSync is designed with security as the primary concern:

- **Encryption**: AES-256-GCM or ChaCha20-Poly1305
- **Key Storage**: macOS Keychain (never on removable media)
- **Audit Trail**: Cryptographically signed, append-only logs
- **Threat Model**: Protection against untrusted media and physical access

For security vulnerabilities, please email security@<org>.com instead of using the issue tracker.

See [SECURITY.md](docs/SECURITY.md) for our complete security model and vulnerability disclosure policy.

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### How to Contribute

1. Check existing [issues](https://github.com/<org>/airgap-sync/issues) or create a new one
2. Fork the repository and create your branch
3. Write tests for your changes
4. Ensure all tests pass
5. Submit a Pull Request

### Code of Conduct

This project adheres to the Contributor Covenant [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## 📊 Performance Targets

- **Sync Speed**: >100MB/s on USB 3.0
- **Memory Usage**: <100MB for typical workloads
- **Startup Time**: <1 second
- **Encryption**: Hardware-accelerated when available
- **Concurrency**: Up to 4 files processed in parallel

## 🗺️ Roadmap

### Near Term (Q1-Q2 2025)
- [ ] Core sync engine implementation
- [ ] Basic CLI functionality
- [ ] Keychain integration
- [ ] Unit test coverage

### Medium Term (Q3-Q4 2025)
- [ ] SwiftUI menu-bar app
- [ ] Audit logging system
- [ ] Beta release
- [ ] Security audit

### Long Term (2026+)
- [ ] Windows/Linux support
- [ ] Cloud backend options
- [ ] Enterprise features
- [ ] Mobile companion app

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) and [Swift](https://swift.org/)
- [clap](https://github.com/clap-rs/clap) - CLI argument parsing
- [ring](https://github.com/briansmith/ring) - Cryptography primitives
- [zstd](https://github.com/facebook/zstd) - Compression algorithm
- [serde](https://github.com/serde-rs/serde) - Serialization framework

## 📞 Support

- 📧 Email: support@<org>.com
- 💬 Discord: [Join our community](https://discord.gg/<invite>)
- 📚 Documentation: [docs.airgapsync.com](https://docs.airgapsync.com)
- 🐛 Issues: [GitHub Issues](https://github.com/<org>/airgap-sync/issues)

## 🚦 Status

This project is currently in early development (v0.1.0). The core architecture is being implemented, and we welcome early feedback and contributions.

[![GitHub issues](https://img.shields.io/github/issues/<org>/airgap-sync)](https://github.com/<org>/airgap-sync/issues)
[![GitHub forks](https://img.shields.io/github/forks/<org>/airgap-sync)](https://github.com/<org>/airgap-sync/network)
[![GitHub stars](https://img.shields.io/github/stars/<org>/airgap-sync)](https://github.com/<org>/airgap-sync/stargazers)

---

<p align="center">
  Made with ❤️ for the security-conscious
</p>
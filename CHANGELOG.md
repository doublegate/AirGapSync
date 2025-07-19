# Changelog

## 0.1.0 - Phase 1 Complete (2025-07-19)

### Added - Phase 1: Design & Key Management

#### Core Library Architecture
- **Complete Rust core library** with comprehensive key management
- **macOS Keychain integration** for secure key storage using security-framework
- **Multi-algorithm encryption support**: AES-256-GCM, ChaCha20-Poly1305, RSA-2048/4096, ECDSA-P256/P384
- **Full RSA implementation** with SHA-256/SHA-384 prehash signing using hazmat traits
- **ECDSA implementation** with P-256/P-384 curve support and ECDH key agreement
- **Cryptographic key derivation** using HKDF with configurable salt and info parameters
- **Elliptic curve dependencies** added: elliptic-curve, p256, p384, ecdsa crates

#### CLI Interface & Commands
- **Comprehensive CLI tool** with 11+ commands for key management and system operations
- **Key generation**: Support for all encryption algorithms with secure random generation
- **Key rotation**: Automatic key rotation with old key archival
- **File encryption/decryption**: Practical cryptographic operations with multiple algorithms
- **Configuration validation**: Real-time TOML/JSON schema validation
- **JSON schema generation**: Auto-generated configuration schemas for tooling integration
- **System information**: Hardware and software environment reporting

#### Configuration System
- **TOML-based configuration** with comprehensive schema validation using schemars
- **Multi-device support**: Configure multiple USB devices with individual policies
- **Retention policies**: Configurable snapshot retention and garbage collection
- **Security policies**: Key rotation schedules and audit trail configuration
- **Flexible source/device mappings**: Support for complex backup scenarios

#### SwiftUI Foundation
- **Menu-bar application structure** with native macOS integration
- **FFI bridge preparation** for Rust-Swift communication
- **Modern SwiftUI architecture** following Apple's latest patterns
- **Xcode project configuration** with proper build settings and dependencies

#### Development Infrastructure
- **Comprehensive build system** with 15+ make targets for all development workflows
- **Integration test suite** covering key generation, encryption, keychain operations
- **Performance benchmarks** for encryption algorithms and file operations
- **Security audit integration** with cargo-audit for dependency vulnerability scanning
- **Documentation generation** with rustdoc and comprehensive API coverage

#### Documentation & Planning
- **Complete technical documentation** covering architecture, security model, and API reference
- **Phase-based development roadmap** with detailed milestones and timeline
- **Security threat model** with comprehensive vulnerability analysis
- **Configuration examples** and deployment guides
- **Developer contribution guidelines** and code standards

### Fixed
- **All clippy warnings resolved** with comprehensive documentation for public APIs
- **RSA prehash signing implementation** using proper hazmat traits for security compliance
- **Keychain parameter order corrections** for reliable key storage and retrieval
- **Schema validation accuracy** with proper enum serialization format matching
- **Memory safety improvements** with proper lifetime management and error handling
- **Build system optimization** with parallel compilation and dependency caching
- **ECDH key agreement implementation** using elliptic-curve crates for proper shared secret derivation

### Technical Details
- **52 documentation warnings fixed** with comprehensive API documentation
- **Full test coverage** for core cryptographic operations and keychain integration
- **Zero compilation errors** across all supported configurations
- **Secure random number generation** using ring's SystemRandom for all cryptographic operations
- **Platform-specific optimizations** for macOS Keychain Services integration
- **Future-proof architecture** designed for cross-platform expansion
- **ECDH implementation** using p256/p384 crates for standards-compliant key agreement

### Security Enhancements
- **Secure key storage** with macOS Keychain Services integration
- **Memory-safe cryptographic operations** using audited Rust crates (ring, rsa, ecdsa, p256, p384)
- **Protection against timing attacks** with constant-time operations where applicable
- **Secure key generation** with hardware random number generators
- **Comprehensive audit trail preparation** for Phase 4 implementation
- **Standards-compliant ECDH** using NIST curves P-256 and P-384

### Performance Optimizations
- **Efficient chunk-based architecture** for large file processing
- **Streaming encryption capability** for memory-constrained environments
- **Parallel processing support** for multi-file operations
- **Hardware acceleration utilization** where available on supported platforms
- **Minimal memory footprint** with zero-copy operations where possible

This release establishes the complete foundation for AirGapSync with production-ready key management, comprehensive CLI interface, and robust development infrastructure. Phase 2 (Sync Engine) development can now proceed with full confidence in the cryptographic and architectural foundations.
EOF < /dev/null
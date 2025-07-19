# Phase 1: Design & Key Management

## Overview
Define folder→device sync policy schema (TOML/JSON) and integrate Keychain with RSA/ECDSA keypairs in Rust library.

## Tasks

### Policy Schema Design
- [ ] Define TOML configuration schema for sync policies
- [ ] Create JSON schema alternative for programmatic use
- [ ] Document all configuration options
- [ ] Add validation rules for configurations
- [ ] Create example configurations for common use cases

### Key Management Infrastructure
- [ ] Research macOS Keychain Services API
- [ ] Design key storage architecture
- [ ] Implement Rust wrapper for Keychain Services
- [ ] Support RSA-2048/4096 key generation
- [ ] Support ECDSA P-256/P-384 key generation
- [ ] Implement key rotation mechanism
- [ ] Create key backup/export functionality

### Security Architecture
- [ ] Define threat model document
- [ ] Design encryption scheme (AES-256-GCM)
- [ ] Implement secure key derivation (PBKDF2/Argon2)
- [ ] Design key lifecycle management
- [ ] Create security audit checklist

### Integration & Testing
- [ ] Unit tests for key generation
- [ ] Integration tests with Keychain
- [ ] Performance benchmarks for crypto operations
- [ ] Security review of implementation

## Deliverables
1. Configuration schema documentation
2. Rust Keychain wrapper library
3. Security architecture document
4. Test suite for key management

## Success Criteria
- All keys stored securely in macOS Keychain
- Configuration schema validated and documented
- Crypto operations pass security audit
- Performance meets requirements (<100ms for key operations)
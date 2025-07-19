# Architecture Overview

## System Design

AirGapSync is designed as a modular, secure synchronization system with clear separation between the core sync engine, user interfaces, and platform-specific integrations.

```
┌─────────────────────────────────────────────────────────────┐
│                        User Interfaces                       │
├──────────────────────┬──────────────────────────────────────┤
│    SwiftUI App       │         CLI Tool                     │
│   (Menu Bar UI)      │    (Command Line)                    │
└──────────┬───────────┴──────────────┬───────────────────────┘
           │                          │
           │         FFI Bridge       │
           └──────────┬───────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                    Rust Core Library                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │
│  │ Sync Engine │  │ Crypto Module│  │ Storage Backend │    │
│  └─────────────┘  └──────────────┘  └─────────────────┘    │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │
│  │ Diff Engine │  │ Audit Logger │  │ Config Manager  │    │
│  └─────────────┘  └──────────────┘  └─────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                   Platform Services                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐    │
│  │   Keychain  │  │ File System  │  │ USB Detection   │    │
│  └─────────────┘  └──────────────┘  └─────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### Rust Core Library (`src/rust_core/`)

The heart of AirGapSync, providing all synchronization logic, encryption, and data management.

#### Sync Engine
- **Purpose**: Orchestrates the entire synchronization process
- **Key Features**:
  - Policy-based sync execution
  - Progress tracking and reporting
  - Error recovery and retry logic
  - Concurrent file processing

#### Crypto Module
- **Purpose**: Handles all encryption and key management operations
- **Key Features**:
  - AES-256-GCM encryption for data
  - RSA/ECDSA key generation and storage
  - Keychain integration for secure key storage
  - Key rotation and versioning

#### Storage Backend
- **Purpose**: Manages the physical storage of encrypted data
- **Key Features**:
  - Chunk-based storage for efficient updates
  - Deduplication at the chunk level
  - Snapshot management
  - Garbage collection

#### Diff Engine
- **Purpose**: Detects changes between source and destination
- **Key Features**:
  - Fast hash-based comparison
  - Metadata tracking (permissions, timestamps)
  - Incremental update detection
  - Move/rename detection

#### Audit Logger
- **Purpose**: Maintains tamper-proof audit trail
- **Key Features**:
  - Append-only log structure
  - Cryptographic signatures
  - Log rotation and archival
  - Compliance reporting

### User Interfaces

#### SwiftUI App (`src/swift_ui/`)
- Menu bar application for macOS
- Real-time sync status monitoring
- Device management interface
- Settings and configuration UI

#### CLI Tool (`src/cli/`)
- Command-line interface for automation
- Full feature parity with GUI
- Scriptable operations
- Headless operation support

### FFI Bridge

Provides safe interoperability between Rust core and Swift UI:

```rust
#[repr(C)]
pub struct SyncProgress {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub total_files: u32,
    pub processed_files: u32,
}

#[no_mangle]
pub extern "C" fn airgap_sync_start(
    config: *const c_char,
    callback: extern "C" fn(progress: SyncProgress)
) -> i32 {
    // Implementation
}
```

## Data Flow

### Sync Operation Flow

```
1. User initiates sync (GUI/CLI)
       │
2. Load configuration and policies
       │
3. Detect target device availability
       │
4. Load encryption keys from Keychain
       │
5. Scan source directory for changes
       │
6. Build change set (diff)
       │
7. For each changed file:
   a. Read file in chunks
   b. Compress chunk (zstd)
   c. Encrypt chunk (AES-256-GCM)
   d. Write to destination
   e. Update progress
       │
8. Write manifest and metadata
       │
9. Update audit log
       │
10. Cleanup old snapshots (if needed)
```

### Encryption Flow

```
Original File
     │
     ▼
[Chunking: 1MB blocks]
     │
     ▼
[Compression: zstd]
     │
     ▼
[Encryption: AES-256-GCM]
     │
     ▼
[Storage: Encrypted chunks]
     │
     ▼
[Manifest: Chunk map + metadata]
```

## Security Architecture

### Threat Model
- **Untrusted Media**: Removable drives may be compromised
- **Physical Access**: Attackers may have physical access to media
- **Key Extraction**: Protect against key extraction attempts
- **Tampering**: Detect any modification of stored data

### Security Measures
1. **Encryption at Rest**: All data encrypted before writing
2. **Key Isolation**: Keys never stored on removable media
3. **Integrity Verification**: HMAC for all stored data
4. **Audit Trail**: Cryptographically signed logs
5. **Secure Erase**: Overwrite deleted data

## Performance Considerations

### Optimization Strategies
1. **Parallel Processing**: Multi-threaded chunk processing
2. **Memory Mapping**: For large file handling
3. **Incremental Updates**: Only sync changed blocks
4. **Compression**: Reduce I/O with zstd compression
5. **Caching**: In-memory cache for frequently accessed data

### Benchmarks
- Target: 100MB/s on USB 3.0 media
- Memory usage: <100MB for typical workloads
- CPU usage: <50% on modern processors

## Platform Integration

### macOS Specific
- **Keychain Services**: For secure key storage
- **FSEvents**: For real-time file change detection
- **DiskArbitration**: For device detection
- **Endpoint Security**: For enhanced monitoring

### Cross-Platform Considerations
- Abstract file system operations
- Platform-agnostic encryption
- Portable data formats
- Standard compliance (POSIX)

## Error Handling

### Recovery Strategies
1. **Transactional Updates**: Atomic operations with rollback
2. **Resume Capability**: Continue interrupted syncs
3. **Corruption Detection**: Verify data integrity
4. **Fallback Modes**: Graceful degradation
5. **User Notifications**: Clear error reporting

## Future Extensibility

### Planned Extensions
1. **Cloud Backends**: S3, Azure Blob, Google Cloud
2. **Network Sync**: Direct device-to-device
3. **Multi-Platform**: Windows and Linux support
4. **Compression Algorithms**: Support for multiple codecs
5. **Encryption Schemes**: Post-quantum cryptography ready
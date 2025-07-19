# Phase 2: Sync Engine Prototype

## Overview
Build Rust core: diff, chunk, and encrypted archive on the fly. Expose CLI and Swift bridge via FFI.

## Tasks

### Core Sync Engine
- [ ] Implement file system walker with exclusion patterns
- [ ] Create diff algorithm for detecting changes
- [ ] Design chunk-based processing for large files
- [ ] Implement streaming encryption (AES-256-GCM)
- [ ] Create compressed archive format
- [ ] Add resumable sync capability

### Data Structures
- [ ] Design snapshot manifest format
- [ ] Implement metadata tracking (permissions, timestamps)
- [ ] Create index for fast lookups
- [ ] Design deduplication system
- [ ] Implement delta compression

### CLI Implementation
- [ ] Enhance CLI with all sync options
- [ ] Add progress reporting
- [ ] Implement dry-run mode
- [ ] Add verbose logging options
- [ ] Create configuration file support
- [ ] Add interrupt handling

### Swift FFI Bridge
- [ ] Design C API for Rust functions
- [ ] Create Swift bindings
- [ ] Implement callback system for progress
- [ ] Add error handling across FFI boundary
- [ ] Create async wrapper for Swift

### Performance Optimization
- [ ] Implement parallel processing
- [ ] Add memory-mapped file support
- [ ] Optimize for SSD vs HDD
- [ ] Benchmark against rsync
- [ ] Profile memory usage

## Deliverables
1. Rust sync engine library
2. CLI binary with full features
3. Swift framework for macOS app
4. Performance benchmarks
5. Integration test suite

## Success Criteria
- Sync 1GB in <10 seconds to USB 3.0
- Memory usage <100MB for typical sync
- Resume interrupted syncs successfully
- Pass all integration tests
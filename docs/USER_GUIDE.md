# AirGapSync User Guide

Welcome to AirGapSync! This guide will help you get started with securely syncing your data to removable media.

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Configuration](#configuration)
5. [Basic Usage](#basic-usage)
6. [Advanced Features](#advanced-features)
7. [Troubleshooting](#troubleshooting)
8. [FAQ](#faq)

## Introduction

### What is AirGapSync?

AirGapSync is an encrypted removable-media sync manager designed for macOS. It provides:

- **Air-gap security**: Data is never sent over the network
- **Military-grade encryption**: AES-256-GCM and ChaCha20-Poly1305
- **Incremental syncs**: Only changed files are transferred
- **Snapshot history**: Roll back to any previous state
- **Audit logging**: Track all sync operations
- **Native macOS integration**: Menu bar app with system notifications

### Who is it for?

AirGapSync is perfect for:

- **Security-conscious users** who need offline backups
- **Developers** syncing code without cloud services
- **Professionals** with sensitive documents
- **Anyone** who wants complete control over their backups

### Key Features

✅ **Encryption** - All data encrypted before writing to device
✅ **Compression** - Automatic compression saves space
✅ **Incremental** - Only sync what changed
✅ **Snapshots** - Point-in-time backups with verification
✅ **Audit Trail** - Complete log of all operations
✅ **Resume Support** - Pick up where you left off
✅ **Multiple Devices** - Manage several backup destinations

## Installation

### Requirements

- macOS 13.0 (Ventura) or later
- Xcode Command Line Tools (for building from source)
- Rust 1.70+ (for building from source)

### Pre-built Binary

*Coming soon: Download from GitHub Releases*

```bash
# Install via Homebrew (future)
brew install --cask airgapsync
```

### Building from Source

```bash
# Clone repository
git clone https://github.com/yourusername/airgapsync.git
cd airgapsync

# Build release version
make release

# Install CLI
sudo make install

# Build macOS app
make app
```

### Verify Installation

```bash
# Check CLI version
airgapsync --version

# Check library info
airgapsync info

# Run self-test
airgapsync validate
```

## Quick Start

### 1. Initialize Configuration

```bash
# Create default configuration
airgapsync init

# Creates ~/.airgapsync/config.toml
```

### 2. Add Your First Device

```bash
# Plug in USB device

# Add device (replace /Volumes/YourUSB with your device path)
airgapsync device add \
  --id USB001 \
  --name "My Backup Drive" \
  --mount-point /Volumes/YourUSB
```

### 3. Configure Source Directory

Edit `~/.airgapsync/config.toml`:

```toml
[source]
path = "/Users/yourname/Documents"
exclude = [".DS_Store", "*.tmp", "node_modules"]
```

### 4. Perform First Sync

```bash
# Dry run (preview what would be synced)
airgapsync sync --device USB001 --dry-run

# Actual sync
airgapsync sync --device USB001

# Watch for automatic syncing
airgapsync watch
```

### 5. Verify Sync

```bash
# Check device status
airgapsync device info USB001

# List snapshots
airgapsync snapshot list USB001

# Verify integrity
airgapsync verify USB001
```

## Configuration

### Configuration File Location

Primary config: `~/.airgapsync/config.toml`

### Configuration Structure

```toml
[general]
verbose = false              # Enable detailed logging

[source]
path = "/Users/you/Documents"  # Directory to sync
exclude = [                    # Files to skip
  ".DS_Store",
  "*.tmp",
  ".git/*"
]
follow_symlinks = false        # Don't follow symbolic links
include_hidden = false         # Skip hidden files

[[device]]                     # Multiple devices supported
id = "USB001"                  # Unique identifier
name = "Backup Drive"          # Human-readable name
mount_point = "/Volumes/USB001"  # Where device is mounted

[device.encryption]
algorithm = "aes-256-gcm"      # Encryption algorithm

[policy]
compression_level = 6          # 0-9, higher = slower but smaller
verify_after_write = true      # Verify data after writing
retain_snapshots = 10          # Keep last 10 snapshots
gc_threshold = 0.8             # GC when 80% full
parallel_workers = 4           # Concurrent file processors
chunk_size_mb = 1              # Chunk size for large files

[security]
key_rotation_days = 90         # Rotate encryption keys every 90 days
require_authentication = true  # Require password for sync
audit_logging = true           # Enable audit trail
enforce_device_encryption = true  # Ensure device is encrypted

[schedule]
enabled = false                # Auto-sync on schedule
interval_hours = 24            # Run every 24 hours
time = "02:00"                 # Run at 2 AM

[notifications]
enable = true                  # Show notifications
on_sync_complete = true        # Notify when sync completes
on_error = true                # Notify on errors

[advanced]
buffer_size_kb = 64            # I/O buffer size
hash_algorithm = "blake3"      # Hash algorithm for integrity
max_file_size_gb = 100         # Skip files larger than this
```

### Environment Variables

```bash
# Override config file location
export AIRGAP_CONFIG="$HOME/my-config.toml"

# Set log level
export AIRGAP_LOG_LEVEL=debug

# Disable keychain integration (testing only)
export AIRGAP_NO_KEYCHAIN=1
```

## Basic Usage

### Syncing

#### One-time Sync

```bash
# Sync to specific device
airgapsync sync USB001

# Sync with options
airgapsync sync USB001 \
  --parallel 8 \
  --chunk-size-mb 4 \
  --no-verify  # Skip verification (faster)
```

#### Automatic Syncing

```bash
# Watch for device connections and auto-sync
airgapsync watch

# Watch with specific device
airgapsync watch --device USB001

# Watch with custom interval
airgapsync watch --interval 300  # Check every 5 minutes
```

#### Dry Run Mode

```bash
# Preview what would be synced
airgapsync sync USB001 --dry-run

# See detailed list of files
airgapsync sync USB001 --dry-run --verbose
```

### Device Management

#### List Devices

```bash
# List all configured devices
airgapsync device list

# Show detailed info
airgapsync device list --verbose
```

#### Add Device

```bash
airgapsync device add \
  --id BACKUP01 \
  --name "Home Backup" \
  --mount-point /Volumes/Backup

# With custom encryption
airgapsync device add \
  --id BACKUP02 \
  --name "Work Backup" \
  --mount-point /Volumes/WorkBackup \
  --algorithm chacha20-poly1305
```

#### Remove Device

```bash
airgapsync device remove BACKUP01

# Force removal without confirmation
airgapsync device remove BACKUP01 --force
```

#### Device Information

```bash
# Show device details
airgapsync device info BACKUP01

# Show with snapshots
airgapsync device info BACKUP01 --snapshots
```

### Snapshot Management

#### List Snapshots

```bash
# List all snapshots for device
airgapsync snapshot list USB001

# Show detailed info
airgapsync snapshot list USB001 --verbose

# Limit results
airgapsync snapshot list USB001 --limit 5
```

#### Snapshot Information

```bash
# Show snapshot details
airgapsync snapshot info SNAPSHOT_ID USB001

# Show file list
airgapsync snapshot info SNAPSHOT_ID USB001 --files
```

#### Compare Snapshots

```bash
# Compare two snapshots
airgapsync snapshot diff SNAP1 SNAP2 USB001

# Show only changes
airgapsync snapshot diff SNAP1 SNAP2 USB001 --changes-only
```

#### Delete Snapshot

```bash
# Delete specific snapshot
airgapsync snapshot delete SNAPSHOT_ID USB001

# Delete old snapshots
airgapsync snapshot delete USB001 --older-than 30d
```

### Verification

#### Verify Device

```bash
# Verify latest snapshot
airgapsync verify USB001

# Verify specific snapshot
airgapsync verify USB001 --snapshot SNAPSHOT_ID

# Full verification (slower but thorough)
airgapsync verify USB001 --full
```

### Restoration

#### Restore Files

```bash
# Restore latest snapshot to destination
airgapsync restore USB001 /path/to/restore/to

# Restore specific snapshot
airgapsync restore USB001 /path/to/restore/to \
  --snapshot SNAPSHOT_ID

# Restore specific files only
airgapsync restore USB001 /path/to/restore/to \
  --pattern "*.txt"

# Dry run
airgapsync restore USB001 /path/to/restore/to --dry-run
```

### Audit Logs

#### View Audit Log

```bash
# View recent audit entries
airgapsync audit-log

# View for specific device
airgapsync audit-log --device USB001

# Limit results
airgapsync audit-log --limit 50

# Filter by date
airgapsync audit-log --since "2025-01-01"
```

### Key Management

#### Generate Keys

```bash
# Generate new encryption key for device
airgapsync keygen DEVICE_ID

# With specific algorithm
airgapsync keygen DEVICE_ID --algorithm chacha20-poly1305

# With asymmetric key
airgapsync keygen DEVICE_ID --asymmetric rsa-4096
```

#### Rotate Keys

```bash
# Rotate encryption keys for device
airgapsync rotate DEVICE_ID

# Force rotation even if not due
airgapsync rotate DEVICE_ID --force
```

#### List Keys

```bash
# List all keys
airgapsync keys

# Show key details
airgapsync keys --verbose
```

## Advanced Features

### Shell Completion

Generate shell completion scripts:

```bash
# For Bash
airgapsync completion bash > /usr/local/etc/bash_completion.d/airgapsync

# For Zsh
airgapsync completion zsh > /usr/local/share/zsh/site-functions/_airgapsync

# For Fish
airgapsync completion fish > ~/.config/fish/completions/airgapsync.fish
```

### Encryption Algorithms

Supported algorithms:

- **AES-256-GCM**: Default, widely supported, hardware-accelerated on Intel
- **ChaCha20-Poly1305**: Fast on ARM/Apple Silicon, no hardware dependence

Choose based on your hardware:

```bash
# Check performance on your system
cargo bench crypto_bench

# Set in device configuration
airgapsync device add ... --algorithm chacha20-poly1305
```

### Performance Tuning

#### For USB 3.0

```toml
[policy]
parallel_workers = 4
chunk_size_mb = 1
compression_level = 6
```

#### For USB 3.1+

```toml
[policy]
parallel_workers = 8
chunk_size_mb = 4
compression_level = 3
```

#### For Many Small Files

```toml
[policy]
parallel_workers = 8
chunk_size_mb = 0.25  # 256 KB
```

#### For Large Files

```toml
[policy]
parallel_workers = 4
chunk_size_mb = 16
compression_level = 0  # Disable if files are already compressed
```

### Exclude Patterns

Use glob patterns to exclude files:

```toml
[source]
exclude = [
  "*.tmp",           # All .tmp files
  "*.log",           # All log files
  ".git/*",          # Git directories
  "node_modules/*",  # Node modules
  "*/cache/*",       # Any cache directory
  ".DS_Store",       # macOS metadata
  "Thumbs.db",       # Windows metadata
]
```

### Scheduled Syncs

Enable automatic syncing on a schedule:

```toml
[schedule]
enabled = true
interval_hours = 24    # Run every 24 hours
time = "02:00"         # At 2 AM
```

Or use macOS launchd:

```xml
<!-- ~/Library/LaunchAgents/com.airgapsync.sync.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.airgapsync.sync</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/airgapsync</string>
        <string>watch</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Load with:
```bash
launchctl load ~/Library/LaunchAgents/com.airgapsync.sync.plist
```

### Multiple Source Directories

To sync multiple directories, create separate configs:

```bash
# Documents sync
AIRGAP_CONFIG=~/.airgapsync/documents.toml airgapsync sync USB001

# Photos sync
AIRGAP_CONFIG=~/.airgapsync/photos.toml airgapsync sync USB002
```

### Network Shares (Advanced)

While AirGapSync is designed for local devices, you can sync to network shares:

```toml
[[device]]
id = "NAS001"
name = "Network Attached Storage"
mount_point = "/Volumes/NASBackup"
```

**Note**: This defeats the air-gap security model. Use only in trusted networks.

## Troubleshooting

### Common Issues

#### "Device not found"

**Cause**: Device is not mounted or config has wrong path

**Solution**:
```bash
# Check if device is mounted
diskutil list

# Verify mount point
ls /Volumes/

# Update device config
airgapsync device info DEVICE_ID
```

#### "Keychain access denied"

**Cause**: macOS Keychain permissions not granted

**Solution**:
1. Open Keychain Access app
2. Find "AirGapSync" entries
3. Right-click → Get Info → Access Control
4. Add airgapsync to allowed applications

#### "Insufficient space"

**Cause**: Device is full

**Solution**:
```bash
# Check device space
df -h /Volumes/YourDevice

# Delete old snapshots
airgapsync snapshot delete DEVICE_ID --older-than 60d

# Or reduce retention
# Edit config.toml: retain_snapshots = 5
```

#### "Sync is slow"

**Cause**: Suboptimal settings for hardware

**Solution**:
```bash
# Check current settings
airgapsync info

# Tune for your hardware (see Performance Tuning section)
# Edit config.toml and adjust:
# - parallel_workers
# - chunk_size_mb
# - compression_level
```

#### "Permission denied"

**Cause**: Insufficient file permissions

**Solution**:
```bash
# Check source directory permissions
ls -la /path/to/source

# Fix permissions
chmod -R u+r /path/to/source

# Or run as superuser (not recommended)
sudo airgapsync sync DEVICE_ID
```

### Debug Mode

Enable verbose logging:

```bash
# Environment variable
export AIRGAP_LOG_LEVEL=debug
airgapsync sync USB001

# Command line flag
airgapsync sync USB001 --verbose

# Very verbose
airgapsync sync USB001 -vv
```

### Getting Help

```bash
# CLI help
airgapsync --help

# Command-specific help
airgapsync sync --help

# Show version and build info
airgapsync info
```

### Log Files

Logs are stored in:
- `~/.airgapsync/logs/airgapsync.log`
- `~/.airgapsync/audit/audit.log` (audit trail)

```bash
# View recent logs
tail -f ~/.airgapsync/logs/airgapsync.log

# Search logs
grep ERROR ~/.airgapsync/logs/airgapsync.log
```

## FAQ

### General

**Q: Is my data safe?**
A: Yes. All data is encrypted using industry-standard AES-256-GCM before writing to the device. Encryption keys are stored securely in macOS Keychain.

**Q: Can I use multiple USB devices?**
A: Yes. Add each device with a unique ID and sync to them separately or all at once.

**Q: What happens if sync is interrupted?**
A: AirGapSync has resume support. Simply run sync again and it will continue from where it left off.

**Q: How much space do I need?**
A: Initial sync requires space for all source files plus ~10% for metadata and compression overhead. Incremental syncs only need space for changed files.

### Security

**Q: Where are encryption keys stored?**
A: Keys are stored in macOS Keychain, which is encrypted and protected by your login password.

**Q: Can I use a password instead of Keychain?**
A: Yes. Set `require_authentication = true` in config and you'll be prompted for a password on each sync.

**Q: What encryption algorithms are supported?**
A: AES-256-GCM (default) and ChaCha20-Poly1305. Both are military-grade and suitable for sensitive data.

**Q: Can encrypted data be read on other systems?**
A: Only with the correct encryption key, which is stored in macOS Keychain. You would need to export the key and have compatible decryption software.

### Performance

**Q: How fast is syncing?**
A: Depends on hardware. Typical speeds:
- USB 2.0: ~30 MB/s
- USB 3.0: ~100 MB/s
- USB 3.1: ~200 MB/s

**Q: Does compression slow down syncing?**
A: Slightly, but it usually saves time overall due to less data being written. You can disable compression with `compression_level = 0`.

**Q: How many snapshots should I keep?**
A: Default is 10. Increase if you need longer history, decrease to save space.

### Compatibility

**Q: Does it work on Windows or Linux?**
A: Currently macOS only. Linux support is planned. Windows support is unlikely due to Keychain dependency.

**Q: What macOS versions are supported?**
A: macOS 13.0 (Ventura) and later.

**Q: Can I use it with Time Machine?**
A: Yes, but they serve different purposes. Use both for redundant backups.

### Usage

**Q: How do I backup my entire home directory?**
A: Set `source.path = "/Users/yourname"` in config. Consider excluding cache directories to save space.

**Q: Can I sync to cloud storage?**
A: Technically yes (mount cloud storage as volume), but this defeats the air-gap security model.

**Q: How do I recover from a failed drive?**
A: Connect a working backup device and use `airgapsync restore` to recover files.

## Next Steps

- Read [PERFORMANCE.md](PERFORMANCE.md) for tuning guidance
- See [FFI_REFERENCE.md](FFI_REFERENCE.md) for integration details
- Check [CONTRIBUTING.md](CONTRIBUTING.md) to contribute
- Visit [GitHub Issues](https://github.com/yourusername/airgapsync/issues) for support

## Support

- **Issues**: https://github.com/yourusername/airgapsync/issues
- **Discussions**: https://github.com/yourusername/airgapsync/discussions
- **Email**: support@airgapsync.com (coming soon)

---

**Version**: 1.0.0
**Last Updated**: 2026-02-02
**License**: MIT

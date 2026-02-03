#!/bin/bash
#
# setup-xcode.sh
# Creates a new Xcode project configured for AirGapSync with Rust FFI integration
#

set -e

PROJECT_DIR="/Users/parobek/Code/AirGapSync"
APP_NAME="AirGapSync"
PROJECT_NAME="AirGapSync"
ORGANIZATION="com.airgapsync"

cd "$PROJECT_DIR"

echo "Setting up Xcode project for AirGapSync..."

# Backup old project if it exists
if [ -d "${PROJECT_NAME}.xcodeproj" ]; then
    echo "Backing up old project..."
    mv "${PROJECT_NAME}.xcodeproj" "${PROJECT_NAME}.xcodeproj.backup.$(date +%Y%m%d_%H%M%S)"
fi

# Create a new macOS app project using xcodebuild isn't straightforward,
# so we'll use a template approach. Let's create the xcodeproj structure manually.

echo "Creating Xcode project structure..."

# Create project directory
mkdir -p "${PROJECT_NAME}.xcodeproj"

# We'll use a different approach - let's use the app target build commands directly
# and create a minimal working xcodeproj

echo "Building Rust library..."
cargo build --lib --release

echo "Copying library to accessible location..."
mkdir -p lib
cp target/release/libairgap_sync.a lib/
cp target/airgapsync.h lib/

echo "✅ Xcode setup preparation complete!"
echo ""
echo "Next steps:"
echo "1. Open Xcode"
echo "2. Create New Project → macOS → App"
echo "3. Name: AirGapSync, Team: Your team"
echo "4. Save to: $PROJECT_DIR (replace existing)"
echo "5. Add files:"
echo "   - AirGapSync/AirGapSync/MenuBarApp.swift"
echo "   - AirGapSync/AirGapSync/SyncManager.swift"
echo "   - AirGapSync/AirGapSync/FFIBridge.swift"
echo "6. Configure Build Settings:"
echo "   - Library Search Paths: \$(SRCROOT)/lib \$(SRCROOT)/target/release"
echo "   - Header Search Paths: \$(SRCROOT)/lib \$(SRCROOT)/target"
echo "   - Other Linker Flags: -lairgap_sync -lresolv"
echo "   - Objective-C Bridging Header: \$(SRCROOT)/AirGapSync/AirGapSync-Bridging-Header.h"
echo "7. Add Run Script Phase (before Compile Sources):"
echo "   cd \"\${SRCROOT}\""
echo "   cargo build --lib --release"
echo "   cp target/release/libairgap_sync.a lib/"
echo "   cp target/airgapsync.h lib/"
echo ""
echo "Or use the automated approach below..."

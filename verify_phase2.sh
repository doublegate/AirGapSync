#!/bin/bash
# Phase 2 Verification Script
# Run this script to validate the Phase 2 implementation

set -e

echo "========================================="
echo "Phase 2 Implementation Verification"
echo "========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Step 1: Check dependencies
echo "Step 1: Installing dependencies..."
cargo fetch
echo -e "${GREEN}✓${NC} Dependencies fetched"
echo ""

# Step 2: Build
echo "Step 2: Building project..."
cargo build
echo -e "${GREEN}✓${NC} Build successful"
echo ""

# Step 3: Run unit tests
echo "Step 3: Running unit tests..."
cargo test --lib
echo -e "${GREEN}✓${NC} Unit tests passed"
echo ""

# Step 4: Run integration tests
echo "Step 4: Running integration tests..."
cargo test --test phase2_integration
echo -e "${GREEN}✓${NC} Integration tests passed"
echo ""

# Step 5: Run clippy
echo "Step 5: Running clippy..."
cargo clippy -- -D warnings
echo -e "${GREEN}✓${NC} No clippy warnings"
echo ""

# Step 6: Run benchmarks
echo "Step 6: Running benchmarks (this may take a while)..."
cargo test --release --test phase2_benchmarks -- --nocapture
echo -e "${GREEN}✓${NC} Benchmarks completed"
echo ""

# Step 7: Format check
echo "Step 7: Checking code formatting..."
cargo fmt -- --check
echo -e "${GREEN}✓${NC} Code properly formatted"
echo ""

# Step 8: Documentation
echo "Step 8: Generating documentation..."
cargo doc --no-deps
echo -e "${GREEN}✓${NC} Documentation generated"
echo ""

echo "========================================="
echo -e "${GREEN}All verification steps passed!${NC}"
echo "========================================="
echo ""
echo "Phase 2 is complete and ready for use."
echo ""
echo "Next steps:"
echo "  1. Review docs/SYNC_ENGINE.md for usage"
echo "  2. Try: ./target/debug/airgapsync sync --help"
echo "  3. Begin Phase 3 (SwiftUI integration)"
echo ""

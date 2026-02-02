# AirGapSync Makefile

.PHONY: all build test clean install run lint fmt doc release help

# Default target
all: build

# Build the project
build:
	@echo "Building AirGapSync..."
	cargo build --all

# Build release version
release:
	@echo "Building release version..."
	cargo build --release --all

# Run tests
test:
	@echo "Running tests..."
	cargo test --all

# Run with example arguments
run:
	@echo "Running AirGapSync..."
	cargo run --bin airgapsync -- --src ~/Documents --dest /Volumes/USB001

# Clean build artifacts
clean: clean-app
	@echo "Cleaning..."
	cargo clean
	rm -rf target/
	find . -name "*.log" -delete

# Install locally
install: release
	@echo "Installing AirGapSync..."
	cargo install --path . --force

# Run linter
lint:
	@echo "Running clippy..."
	cargo clippy --all -- -D warnings

# Format code
fmt:
	@echo "Formatting code..."
	cargo fmt --all

# Check formatting
fmt-check:
	@echo "Checking code formatting..."
	cargo fmt --all -- --check

# Generate documentation
doc:
	@echo "Generating documentation..."
	cargo doc --no-deps --open

# Run security audit
audit:
	@echo "Running security audit..."
	cargo audit

# Setup development environment
setup:
	@echo "Setting up development environment..."
	rustup component add clippy rustfmt
	cargo install cargo-audit
	@echo "Development environment ready!"

# Run benchmarks
bench:
	@echo "Running benchmarks..."
	cargo bench

# Create example configuration
example-config:
	@echo "Creating example configuration..."
	@mkdir -p ~/.airgapsync
	@cp config.example.toml ~/.airgapsync/config.toml
	@echo "Example config created at ~/.airgapsync/config.toml"

# Build static library for a specific target
build-lib-x86_64:
	@echo "Building library for x86_64..."
	cargo build --lib --release --target x86_64-apple-darwin

build-lib-aarch64:
	@echo "Building library for aarch64..."
	cargo build --lib --release --target aarch64-apple-darwin

# Build universal static library
universal-lib: build-lib-x86_64 build-lib-aarch64
	@echo "Creating universal library..."
	@mkdir -p target/universal
	lipo -create \
		target/x86_64-apple-darwin/release/libairgap_sync.a \
		target/aarch64-apple-darwin/release/libairgap_sync.a \
		-output target/universal/libairgap_sync.a
	@echo "Universal library created at target/universal/libairgap_sync.a"

# Build for macOS universal binary (CLI)
universal: build-lib-x86_64 build-lib-aarch64
	@echo "Building universal binary..."
	cargo build --release --target x86_64-apple-darwin
	cargo build --release --target aarch64-apple-darwin
	lipo -create \
		target/x86_64-apple-darwin/release/airgapsync \
		target/aarch64-apple-darwin/release/airgapsync \
		-output target/release/airgapsync-universal

# Build the SwiftUI app
app: universal-lib
	@echo "Building SwiftUI app..."
	xcodebuild -project AirGapSync.xcodeproj \
		-scheme AirGapSync \
		-configuration Release \
		-derivedDataPath build \
		build

# Build and run the app
run-app: app
	@echo "Launching app..."
	open build/Build/Products/Release/AirGapSync.app

# Clean Xcode build artifacts
clean-app:
	@echo "Cleaning Xcode build artifacts..."
	rm -rf build/
	xcodebuild -project AirGapSync.xcodeproj -scheme AirGapSync clean

# Package for distribution
package: universal
	@echo "Creating distribution package..."
	@mkdir -p dist
	@cp target/release/airgapsync-universal dist/airgapsync
	@cp LICENSE README.md dist/
	@tar -czf dist/airgapsync-macos.tar.gz -C dist airgapsync LICENSE README.md
	@echo "Package created at dist/airgapsync-macos.tar.gz"

# Help
help:
	@echo "AirGapSync Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make build              - Build the project"
	@echo "  make release            - Build release version"
	@echo "  make test               - Run tests"
	@echo "  make run                - Run CLI with example arguments"
	@echo "  make clean              - Clean all build artifacts"
	@echo "  make install            - Install CLI locally"
	@echo "  make lint               - Run linter (clippy)"
	@echo "  make fmt                - Format code"
	@echo "  make fmt-check          - Check code formatting"
	@echo "  make doc                - Generate documentation"
	@echo "  make audit              - Run security audit"
	@echo "  make setup              - Setup development environment"
	@echo "  make bench              - Run benchmarks"
	@echo "  make build-lib-x86_64   - Build library for Intel"
	@echo "  make build-lib-aarch64  - Build library for Apple Silicon"
	@echo "  make universal-lib      - Build universal static library"
	@echo "  make universal          - Build universal CLI binary"
	@echo "  make app                - Build SwiftUI app"
	@echo "  make run-app            - Build and run SwiftUI app"
	@echo "  make clean-app          - Clean Xcode build artifacts"
	@echo "  make package            - Create distribution package"
	@echo "  make help               - Show this help"
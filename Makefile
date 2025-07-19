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
clean:
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

# Build for macOS universal binary
universal:
	@echo "Building universal binary..."
	cargo build --release --target x86_64-apple-darwin
	cargo build --release --target aarch64-apple-darwin
	lipo -create \
		target/x86_64-apple-darwin/release/airgapsync \
		target/aarch64-apple-darwin/release/airgapsync \
		-output target/release/airgapsync-universal

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
	@echo "  make build        - Build the project"
	@echo "  make release      - Build release version"
	@echo "  make test         - Run tests"
	@echo "  make run          - Run with example arguments"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make install      - Install locally"
	@echo "  make lint         - Run linter (clippy)"
	@echo "  make fmt          - Format code"
	@echo "  make fmt-check    - Check code formatting"
	@echo "  make doc          - Generate documentation"
	@echo "  make audit        - Run security audit"
	@echo "  make setup        - Setup development environment"
	@echo "  make bench        - Run benchmarks"
	@echo "  make universal    - Build macOS universal binary"
	@echo "  make package      - Create distribution package"
	@echo "  make help         - Show this help"
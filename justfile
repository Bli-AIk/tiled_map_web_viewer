# Justfile for Tiled Map Web Viewer

# Install prerequisites:
# cargo install trunk
# rustup target add wasm32-unknown-unknown

# Default target
default: run

# Build for desktop (native)
build:
    cargo build

# Build for desktop in release mode
build-release:
    cargo build --release

# Run desktop version
run:
    cargo run

# Serve WASM version locally (development, http://127.0.0.1:8080)
serve:
    trunk serve

# Build WASM version (release, output in dist/)
build-wasm:
    trunk build --release

# Check native + WASM compilation
check:
    cargo check
    cargo check --target wasm32-unknown-unknown

# Run clippy lints
lint:
    cargo clippy --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting (CI-style)
fmt-check:
    cargo fmt -- --check

# Run cargo deny checks
deny:
    cargo deny check

# Run all CI checks locally
ci: fmt-check lint check deny

# Clean build artifacts
clean:
    cargo clean
    rm -rf dist

# Update dependencies
update:
    cargo update

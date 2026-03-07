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

# Serve WASM version (development)
serve:
    trunk serve

# Build WASM version (release)
build-wasm:
    trunk build --release

# Clean build artifacts
clean:
    cargo clean
    rm -rf dist

# Check code without building
check:
    cargo check

# Update dependencies
update:
    cargo update

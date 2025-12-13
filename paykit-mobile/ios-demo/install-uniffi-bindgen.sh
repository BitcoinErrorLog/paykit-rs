#!/bin/bash
# Install uniffi-bindgen CLI tool
# Note: uniffi 0.25 doesn't provide a standalone binary
# This script provides workarounds and alternatives

set -e

echo "=========================================="
echo "uniffi-bindgen Installation Guide"
echo "=========================================="
echo ""

# Check if already installed
if command -v uniffi-bindgen &> /dev/null; then
    echo "✓ uniffi-bindgen is already installed"
    uniffi-bindgen --version
    echo ""
    exit 0
fi

echo "Building uniffi-bindgen CLI from uniffi-rs examples..."
echo ""

# Create temporary directory
TMPDIR=$(mktemp -d)
cd "$TMPDIR"

echo "Cloning uniffi-rs repository..."
git clone --depth 1 --branch v0.26.0 https://github.com/mozilla/uniffi-rs.git
cd uniffi-rs

echo "Building uniffi-bindgen CLI from examples..."
cd examples/app/uniffi-bindgen-cli
cargo build --release

if [ -f "../../target/release/uniffi-bindgen" ]; then
    echo ""
    echo "=========================================="
    echo "✓ Build successful!"
    echo "=========================================="
    echo ""
    echo "The binary is at: $TMPDIR/uniffi-rs/target/release/uniffi-bindgen"
    echo ""
    echo "To use it, either:"
    echo "1. Copy it to your PATH:"
    echo "   cp $TMPDIR/uniffi-rs/target/release/uniffi-bindgen ~/.cargo/bin/"
    echo ""
    echo "2. Or use it directly:"
    echo "   $TMPDIR/uniffi-rs/target/release/uniffi-bindgen generate --library <path> -l swift -o <out>"
    echo ""
    echo "Note: The binary will remain in $TMPDIR until you delete it."
    echo "You can now run:"
    echo "  cd paykit-mobile"
    echo "  ./generate-bindings.sh"
    echo ""
    
    # Copy to cargo bin for global access
    mkdir -p ~/.cargo/bin
    if cp ../../target/release/uniffi-bindgen ~/.cargo/bin/ 2>/dev/null; then
        echo "✓ Installed to ~/.cargo/bin/uniffi-bindgen"
        echo "  You can now use 'uniffi-bindgen' from anywhere"
    else
        echo "⚠️  Could not install to ~/.cargo/bin/"
        echo "  You can manually copy: cp $TMPDIR/uniffi-rs/target/release/uniffi-bindgen ~/.cargo/bin/"
    fi
    
    exit 0
else
    echo ""
    echo "=========================================="
    echo "Build failed"
    echo "=========================================="
    echo ""
    echo "You can still generate bindings using:"
    echo "  cd paykit-mobile"
    echo "  cargo run --bin generate-bindings --features bindgen-cli -- --library <path> -l swift -o <out>"
    echo ""
    exit 1
fi


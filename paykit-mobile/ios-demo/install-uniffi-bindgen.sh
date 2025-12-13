#!/bin/bash
# Install uniffi-bindgen CLI tool for uniffi 0.25
# This script builds and installs uniffi-bindgen from source

set -e

echo "=========================================="
echo "Installing uniffi-bindgen CLI tool"
echo "=========================================="
echo ""

# Check if already installed
if command -v uniffi-bindgen &> /dev/null; then
    echo "✓ uniffi-bindgen is already installed"
    uniffi-bindgen --version
    echo ""
    echo "If you want to reinstall, uninstall first:"
    echo "  cargo uninstall uniffi_bindgen"
    exit 0
fi

# Create temporary directory
TMPDIR=$(mktemp -d)
cd "$TMPDIR"

echo "Cloning uniffi-rs repository..."
git clone https://github.com/mozilla/uniffi-rs.git
cd uniffi-rs

echo "Checking out v0.25.0..."
git checkout v0.25.0

echo "Building and installing uniffi-bindgen..."
cd uniffi_bindgen
cargo install --path . --force

echo ""
echo "=========================================="
echo "Installation complete!"
echo "=========================================="
echo ""
echo "Verifying installation..."
uniffi-bindgen --version

echo ""
echo "You can now run:"
echo "  cd paykit-mobile"
echo "  ./generate-bindings.sh"
echo ""

# Cleanup
cd /
rm -rf "$TMPDIR"


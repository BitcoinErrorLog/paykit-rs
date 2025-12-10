#!/bin/bash
#
# Generate Swift and Kotlin bindings for Paykit Mobile
#
# This script uses uniffi-bindgen to generate platform-specific bindings
# from the Rust FFI definitions.
#
# Prerequisites:
#   - cargo install uniffi-bindgen-cli@0.25
#   - Rust toolchain with uniffi 0.25
#
# Usage:
#   ./generate-bindings.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "Building paykit-mobile library..."
cargo build --release -p paykit-mobile

# Determine library extension based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    LIB_EXT="dylib"
    LIB_PREFIX="lib"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    LIB_EXT="so"
    LIB_PREFIX="lib"
else
    LIB_EXT="dll"
    LIB_PREFIX=""
fi

LIB_PATH="target/release/${LIB_PREFIX}paykit_mobile.${LIB_EXT}"

if [ ! -f "$LIB_PATH" ]; then
    echo "Error: Library not found at $LIB_PATH"
    exit 1
fi

echo ""
echo "Library built at: $LIB_PATH"

# Check if uniffi-bindgen is installed
if command -v uniffi-bindgen &> /dev/null; then
    echo ""
    echo "Generating Swift bindings..."
    mkdir -p paykit-mobile/swift
    uniffi-bindgen generate --library "$LIB_PATH" -l swift -o paykit-mobile/swift

    echo ""
    echo "Generating Kotlin bindings..."
    mkdir -p paykit-mobile/kotlin
    uniffi-bindgen generate --library "$LIB_PATH" -l kotlin -o paykit-mobile/kotlin

    echo ""
    echo "Bindings generated successfully!"
    echo "  Swift:  paykit-mobile/swift/"
    echo "  Kotlin: paykit-mobile/kotlin/"
else
    echo ""
    echo "uniffi-bindgen not found. Install it with:"
    echo "  cargo install uniffi-bindgen-cli@0.25"
    echo ""
    echo "Then run this script again, or manually:"
    echo "  uniffi-bindgen generate --library $LIB_PATH -l swift -o paykit-mobile/swift"
    echo "  uniffi-bindgen generate --library $LIB_PATH -l kotlin -o paykit-mobile/kotlin"
fi

echo ""
echo "Done!"

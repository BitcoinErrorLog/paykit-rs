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
#   - For Android: Android NDK and cross-compilation targets
#
# Usage:
#   ./generate-bindings.sh            # Build and generate bindings for host
#   ./generate-bindings.sh --android  # Build for Android targets
#   ./generate-bindings.sh --ios      # Build for iOS targets
#   ./generate-bindings.sh --all      # Build for all targets
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

BUILD_ANDROID=false
BUILD_IOS=false
BUILD_HOST=true

# Parse arguments
for arg in "$@"; do
    case $arg in
        --android)
            BUILD_ANDROID=true
            BUILD_HOST=false
            ;;
        --ios)
            BUILD_IOS=true
            BUILD_HOST=false
            ;;
        --all)
            BUILD_ANDROID=true
            BUILD_IOS=true
            BUILD_HOST=true
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Usage: $0 [--android] [--ios] [--all]"
            exit 1
            ;;
    esac
done

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

# Build for host platform
if [ "$BUILD_HOST" = true ]; then
    echo "========================================"
    echo "Building paykit-mobile for host platform"
    echo "========================================"
    cargo build --release -p paykit-mobile

    if [ ! -f "$LIB_PATH" ]; then
        echo "Error: Library not found at $LIB_PATH"
        exit 1
    fi

    echo ""
    echo "Library built at: $LIB_PATH"
fi

# Build for Android targets
if [ "$BUILD_ANDROID" = true ]; then
    echo ""
    echo "========================================"
    echo "Building paykit-mobile for Android"
    echo "========================================"
    
    ANDROID_TARGETS=(
        "aarch64-linux-android"
        "armv7-linux-androideabi"
        "x86_64-linux-android"
        "i686-linux-android"
    )
    
    for target in "${ANDROID_TARGETS[@]}"; do
        echo ""
        echo "Building for $target..."
        if cargo build --release -p paykit-mobile --target "$target" 2>/dev/null; then
            echo "  ✓ Built for $target"
        else
            echo "  ✗ Failed to build for $target (target may not be installed)"
            echo "    Install with: rustup target add $target"
        fi
    done
fi

# Build for iOS targets
if [ "$BUILD_IOS" = true ]; then
    echo ""
    echo "========================================"
    echo "Building paykit-mobile for iOS"
    echo "========================================"
    
    IOS_TARGETS=(
        "aarch64-apple-ios"
        "aarch64-apple-ios-sim"
        "x86_64-apple-ios"
    )
    
    for target in "${IOS_TARGETS[@]}"; do
        echo ""
        echo "Building for $target..."
        if cargo build --release -p paykit-mobile --target "$target" 2>/dev/null; then
            echo "  ✓ Built for $target"
        else
            echo "  ✗ Failed to build for $target (target may not be installed)"
            echo "    Install with: rustup target add $target"
        fi
    done
fi

# Generate bindings
generate_bindings() {
    echo ""
    echo "========================================"
    echo "Generating bindings"
    echo "========================================"
    
    if ! command -v uniffi-bindgen &> /dev/null; then
        echo ""
        echo "uniffi-bindgen not found. Install it with:"
        echo "  cargo install uniffi-bindgen-cli@0.25"
        echo ""
        echo "Then run this script again, or manually:"
        echo "  uniffi-bindgen generate --library $LIB_PATH -l swift -o paykit-mobile/swift"
        echo "  uniffi-bindgen generate --library $LIB_PATH -l kotlin -o paykit-mobile/kotlin"
        return 1
    fi

    if [ ! -f "$LIB_PATH" ]; then
        echo "Error: Library not found at $LIB_PATH"
        echo "Please build for host platform first: $0"
        return 1
    fi

    echo ""
    echo "Generating Swift bindings..."
    mkdir -p paykit-mobile/swift/generated
    uniffi-bindgen generate --library "$LIB_PATH" -l swift -o paykit-mobile/swift/generated
    
    echo ""
    echo "Generating Kotlin bindings..."
    mkdir -p paykit-mobile/kotlin/generated
    uniffi-bindgen generate --library "$LIB_PATH" -l kotlin -o paykit-mobile/kotlin/generated
    
    echo ""
    echo "Bindings generated successfully!"
    echo ""
    echo "Generated files:"
    echo "  Swift:"
    ls -la paykit-mobile/swift/generated/ 2>/dev/null || echo "    (none)"
    echo ""
    echo "  Kotlin:"
    ls -la paykit-mobile/kotlin/generated/ 2>/dev/null || echo "    (none)"
}

# Generate bindings if host library exists
if [ -f "$LIB_PATH" ]; then
    generate_bindings
fi

echo ""
echo "========================================"
echo "Summary"
echo "========================================"
echo ""
echo "Available APIs:"
echo "  - PaykitClient: Core client with payment methods, subscriptions, health"
echo "  - AuthenticatedTransportFFI: Write access for publishing endpoints"
echo "  - UnauthenticatedTransportFFI: Read access for discovering endpoints"
echo "  - DirectoryOperationsAsync: Async directory operations"
echo "  - PaykitMessageBuilder: Create protocol messages for Noise channels"
echo "  - ReceiptStore: In-memory receipt and endpoint storage"
echo "  - ContactCacheFFI: Local contact cache with sync support"
echo ""
echo "Directory Operations:"
echo "  - publish_payment_endpoint: Publish payment methods"
echo "  - fetch_supported_payments: Discover payment methods"
echo "  - fetch_payment_endpoint: Get specific endpoint"
echo "  - add_contact, remove_contact, list_contacts: Contact management"
echo ""
echo "For usage examples, see:"
echo "  - paykit-mobile/README.md"
echo "  - docs/mobile-integration.md"
echo ""
echo "Done!"

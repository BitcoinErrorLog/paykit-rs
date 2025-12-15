#!/bin/bash
#
# Build Paykit Mobile library for Android
#
# This script builds the Rust library for all Android targets and packages them
# into the jniLibs directory structure for Android projects.
#
# Prerequisites:
#   - Rust toolchain with Android targets installed:
#     rustup target add aarch64-linux-android
#     rustup target add armv7-linux-androideabi
#     rustup target add x86_64-linux-android
#     rustup target add i686-linux-android
#   - Android NDK (for linking, if needed)
#
# Usage:
#   ./build-android.sh              # Build for all Android targets
#   ./build-android.sh --jniLibs    # Build and package into jniLibs structure
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

PACKAGE_JNILIBS=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --jniLibs)
            PACKAGE_JNILIBS=true
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Usage: $0 [--jniLibs]"
            exit 1
            ;;
    esac
done

echo "========================================"
echo "Building Paykit Mobile for Android"
echo "========================================"
echo ""

ANDROID_TARGETS=(
    "aarch64-linux-android:arm64-v8a"
    "armv7-linux-androideabi:armeabi-v7a"
    "x86_64-linux-android:x86_64"
    "i686-linux-android:x86"
)

BUILT_TARGETS=()
FAILED_TARGETS=()

# Build for each Android target
for target_mapping in "${ANDROID_TARGETS[@]}"; do
    IFS=':' read -r target abi <<< "$target_mapping"
    echo "Building for $target ($abi)..."
    if cargo build --release -p paykit-mobile --target "$target" 2>&1 | grep -q "Finished"; then
        echo "  ✓ Built for $target"
        BUILT_TARGETS+=("$target_mapping")
    else
        echo "  ✗ Failed to build for $target"
        FAILED_TARGETS+=("$target_mapping")
        echo "    Install with: rustup target add $target"
    fi
    echo ""
done

# Summary
echo "========================================"
echo "Build Summary"
echo "========================================"
echo "Successfully built: ${#BUILT_TARGETS[@]} target(s)"
for target_mapping in "${BUILT_TARGETS[@]}"; do
    IFS=':' read -r target abi <<< "$target_mapping"
    echo "  ✓ $target ($abi)"
done

if [ ${#FAILED_TARGETS[@]} -gt 0 ]; then
    echo ""
    echo "Failed to build: ${#FAILED_TARGETS[@]} target(s)"
    for target_mapping in "${FAILED_TARGETS[@]}"; do
        IFS=':' read -r target abi <<< "$target_mapping"
        echo "  ✗ $target ($abi)"
    done
fi

# Package into jniLibs if requested
if [ "$PACKAGE_JNILIBS" = true ]; then
    echo ""
    echo "========================================"
    echo "Packaging into jniLibs structure"
    echo "========================================"
    
    JNILIBS_BASE="paykit-mobile/android-demo/app/src/main/jniLibs"
    
    # Remove existing jniLibs if it exists
    if [ -d "$JNILIBS_BASE" ]; then
        echo "Removing existing jniLibs..."
        rm -rf "$JNILIBS_BASE"
    fi
    
    # Copy libraries to jniLibs structure
    for target_mapping in "${BUILT_TARGETS[@]}"; do
        IFS=':' read -r target abi <<< "$target_mapping"
        LIB_PATH="target/$target/release/libpaykit_mobile.so"
        
        if [ -f "$LIB_PATH" ]; then
            ABI_DIR="$JNILIBS_BASE/$abi"
            mkdir -p "$ABI_DIR"
            echo "Copying $target to $abi..."
            cp "$LIB_PATH" "$ABI_DIR/libpaykit_mobile.so"
        fi
    done
    
    echo ""
    echo "✅ Libraries packaged into: $JNILIBS_BASE"
    echo ""
    echo "Directory structure:"
    find "$JNILIBS_BASE" -type f -name "*.so" | sort | while read -r lib; do
        echo "  $lib"
    done
fi

echo ""
echo "========================================"
echo "Done!"
echo "========================================"
echo ""
echo "Library files:"
for target_mapping in "${BUILT_TARGETS[@]}"; do
    IFS=':' read -r target abi <<< "$target_mapping"
    echo "  target/$target/release/libpaykit_mobile.so"
done

if [ "$PACKAGE_JNILIBS" = true ]; then
    echo ""
    echo "jniLibs directory:"
    echo "  $JNILIBS_BASE"
fi
echo ""

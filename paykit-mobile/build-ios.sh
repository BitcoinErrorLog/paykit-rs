#!/bin/bash
#
# Build Paykit Mobile library for iOS
#
# This script builds the Rust library for all iOS targets and creates an XCFramework.
#
# Prerequisites:
#   - Rust toolchain with iOS targets installed:
#     rustup target add aarch64-apple-ios
#     rustup target add aarch64-apple-ios-sim
#     rustup target add x86_64-apple-ios
#   - Xcode command line tools
#
# Usage:
#   ./build-ios.sh              # Build for all iOS targets
#   ./build-ios.sh --framework  # Build and create XCFramework
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

CREATE_FRAMEWORK=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --framework)
            CREATE_FRAMEWORK=true
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Usage: $0 [--framework]"
            exit 1
            ;;
    esac
done

echo "========================================"
echo "Building Paykit Mobile for iOS"
echo "========================================"
echo ""

IOS_TARGETS=(
    "aarch64-apple-ios"
    "aarch64-apple-ios-sim"
    "x86_64-apple-ios"
)

BUILT_TARGETS=()
FAILED_TARGETS=()

# Build for each iOS target
for target in "${IOS_TARGETS[@]}"; do
    echo "Building for $target..."
    if cargo build --release -p paykit-mobile --target "$target" 2>&1 | grep -q "Finished"; then
        echo "  ✓ Built for $target"
        BUILT_TARGETS+=("$target")
    else
        echo "  ✗ Failed to build for $target"
        FAILED_TARGETS+=("$target")
        echo "    Install with: rustup target add $target"
    fi
    echo ""
done

# Summary
echo "========================================"
echo "Build Summary"
echo "========================================"
echo "Successfully built: ${#BUILT_TARGETS[@]} target(s)"
for target in "${BUILT_TARGETS[@]}"; do
    echo "  ✓ $target"
done

if [ ${#FAILED_TARGETS[@]} -gt 0 ]; then
    echo ""
    echo "Failed to build: ${#FAILED_TARGETS[@]} target(s)"
    for target in "${FAILED_TARGETS[@]}"; do
        echo "  ✗ $target"
    done
fi

# Create XCFramework if requested
if [ "$CREATE_FRAMEWORK" = true ]; then
    echo ""
    echo "========================================"
    echo "Creating XCFramework"
    echo "========================================"
    
    FRAMEWORK_DIR="paykit-mobile/ios-demo/PaykitDemo/PaykitDemo/Frameworks"
    XCFRAMEWORK_PATH="$FRAMEWORK_DIR/PaykitMobile.xcframework"
    
    # Remove existing framework if it exists
    if [ -d "$XCFRAMEWORK_PATH" ]; then
        echo "Removing existing XCFramework..."
        rm -rf "$XCFRAMEWORK_PATH"
    fi
    
    mkdir -p "$FRAMEWORK_DIR"
    
    # Prepare libraries
    DEVICE_LIB="target/aarch64-apple-ios/release/libpaykit_mobile.a"
    SIM_ARM64_LIB="target/aarch64-apple-ios-sim/release/libpaykit_mobile.a"
    SIM_X86_64_LIB="target/x86_64-apple-ios/release/libpaykit_mobile.a"
    UNIVERSAL_SIM_LIB="$FRAMEWORK_DIR/universal-sim-libpaykit_mobile.a"

    # Create universal simulator lib if both simulator architectures were built
    if [ -f "$SIM_ARM64_LIB" ] && [ -f "$SIM_X86_64_LIB" ]; then
        echo "Creating universal simulator library (arm64 + x86_64)..."
        lipo -create "$SIM_ARM64_LIB" "$SIM_X86_64_LIB" -output "$UNIVERSAL_SIM_LIB"
    elif [ -f "$SIM_ARM64_LIB" ]; then
        echo "Only arm64 simulator library available; using arm64-only simulator lib"
        cp "$SIM_ARM64_LIB" "$UNIVERSAL_SIM_LIB"
    else
        echo "Warning: No simulator library found; simulator slice will be missing"
    fi

    # Copy headers and modulemap (prefer the lowercase module used by generated Swift bindings)
    FFI_HEADER=""
    FFI_MODULEMAP=""

    if [ -f "paykit-mobile/swift/generated/paykit_mobileFFI.h" ] && [ -f "paykit-mobile/swift/generated/paykit_mobileFFI.modulemap" ]; then
        FFI_HEADER="paykit-mobile/swift/generated/paykit_mobileFFI.h"
        FFI_MODULEMAP="paykit-mobile/swift/generated/paykit_mobileFFI.modulemap"
    elif [ -f "paykit-mobile/swift/generated/PaykitMobileFFI.h" ] && [ -f "paykit-mobile/swift/generated/PaykitMobileFFI.modulemap" ]; then
        FFI_HEADER="paykit-mobile/swift/generated/PaykitMobileFFI.h"
        FFI_MODULEMAP="paykit-mobile/swift/generated/PaykitMobileFFI.modulemap"
    fi

    # Device slice
    if [ -f "$DEVICE_LIB" ]; then
        echo "Adding device slice (ios-arm64)..."
        PLATFORM_DIR="$XCFRAMEWORK_PATH/ios-arm64"
        mkdir -p "$PLATFORM_DIR/Headers"
        cp "$DEVICE_LIB" "$PLATFORM_DIR/libpaykit_mobile.a"
        if [ -n "$FFI_HEADER" ]; then
            cp "$FFI_HEADER" "$PLATFORM_DIR/Headers/"
        fi
        if [ -n "$FFI_MODULEMAP" ]; then
            cp "$FFI_MODULEMAP" "$PLATFORM_DIR/"
        fi

        cat > "$PLATFORM_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>iPhoneOS</string>
    </array>
    <key>MinimumOSVersion</key>
    <string>13.0</string>
</dict>
</plist>
EOF
    fi

    # Simulator slice
    if [ -f "$UNIVERSAL_SIM_LIB" ]; then
        echo "Adding simulator slice (ios-arm64_x86_64-simulator)..."
        PLATFORM_DIR="$XCFRAMEWORK_PATH/ios-arm64_x86_64-simulator"
        mkdir -p "$PLATFORM_DIR/Headers"
        cp "$UNIVERSAL_SIM_LIB" "$PLATFORM_DIR/libpaykit_mobile.a"
        if [ -n "$FFI_HEADER" ]; then
            cp "$FFI_HEADER" "$PLATFORM_DIR/Headers/"
        fi
        if [ -n "$FFI_MODULEMAP" ]; then
            cp "$FFI_MODULEMAP" "$PLATFORM_DIR/"
        fi

        cat > "$PLATFORM_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>iPhoneSimulator</string>
    </array>
    <key>MinimumOSVersion</key>
    <string>13.0</string>
</dict>
</plist>
EOF
    fi
    
    # Create root Info.plist
    cat > "$XCFRAMEWORK_PATH/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AvailableLibraries</key>
    <array>
        <dict>
            <key>LibraryIdentifier</key>
            <string>ios-arm64</string>
            <key>LibraryPath</key>
            <string>libpaykit_mobile.a</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>arm64</string>
            </array>
            <key>SupportedPlatform</key>
            <string>ios</string>
        </dict>
        <dict>
            <key>LibraryIdentifier</key>
            <string>ios-arm64_x86_64-simulator</string>
            <key>LibraryPath</key>
            <string>libpaykit_mobile.a</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>arm64</string>
                <string>x86_64</string>
            </array>
            <key>SupportedPlatform</key>
            <string>ios</string>
            <key>SupportedPlatformVariant</key>
            <string>simulator</string>
        </dict>
    </array>
    <key>CFBundlePackageType</key>
    <string>XFWK</string>
    <key>XCFrameworkFormatVersion</key>
    <string>1.0</string>
</dict>
</plist>
EOF
    
    echo ""
    echo "✅ XCFramework created at: $XCFRAMEWORK_PATH"
fi

echo ""
echo "========================================"
echo "Done!"
echo "========================================"
echo ""
echo "Library files:"
for target in "${BUILT_TARGETS[@]}"; do
    echo "  target/$target/release/libpaykit_mobile.a"
done

if [ "$CREATE_FRAMEWORK" = true ]; then
    echo ""
    echo "XCFramework:"
    echo "  $XCFRAMEWORK_PATH"
fi
echo ""

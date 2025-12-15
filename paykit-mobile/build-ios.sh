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
    
    # Create framework structure for each target
    for target in "${BUILT_TARGETS[@]}"; do
        case $target in
            aarch64-apple-ios)
                PLATFORM="ios-arm64"
                ;;
            aarch64-apple-ios-sim)
                PLATFORM="ios-arm64_x86_64-simulator"
                ;;
            x86_64-apple-ios)
                # x86_64 simulator is included in aarch64-apple-ios-sim universal
                continue
                ;;
            *)
                continue
                ;;
        esac
        
        LIB_PATH="target/$target/release/libpaykit_mobile.a"
        if [ -f "$LIB_PATH" ]; then
            echo "Adding $target to XCFramework ($PLATFORM)..."
            
            PLATFORM_DIR="$XCFRAMEWORK_PATH/$PLATFORM"
            mkdir -p "$PLATFORM_DIR/Headers"
            
            # Copy library
            cp "$LIB_PATH" "$PLATFORM_DIR/libpaykit_mobile.a"
            
            # Copy headers and modulemap
            if [ -f "paykit-mobile/swift/generated/PaykitMobileFFI.h" ]; then
                cp "paykit-mobile/swift/generated/PaykitMobileFFI.h" "$PLATFORM_DIR/Headers/"
            fi
            
            if [ -f "paykit-mobile/swift/generated/PaykitMobileFFI.modulemap" ]; then
                cp "paykit-mobile/swift/generated/PaykitMobileFFI.modulemap" "$PLATFORM_DIR/"
            fi
            
            # Create Info.plist
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
    done
    
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

# Paykit Mobile Build Guide

This document describes how to build the Paykit Mobile library for iOS and Android platforms.

## Prerequisites

### Common Prerequisites
- Rust 1.70+ with Cargo
- UniFFI bindgen CLI: `cargo install uniffi-bindgen-cli@0.25`

### iOS Prerequisites
- Xcode with iOS SDK
- Rust iOS targets:
  ```bash
  rustup target add aarch64-apple-ios
  rustup target add aarch64-apple-ios-sim
  rustup target add x86_64-apple-ios
  ```

### Android Prerequisites
- Android NDK (for linking native libraries)
- Rust Android targets:
  ```bash
  rustup target add aarch64-linux-android
  rustup target add armv7-linux-androideabi
  rustup target add x86_64-linux-android
  rustup target add i686-linux-android
  ```

## Building

### Generate Bindings

The first step is to generate Swift and Kotlin bindings from the Rust FFI definitions:

```bash
cd paykit-mobile
./generate-bindings.sh --all
```

This will:
1. Build the library for the host platform
2. Generate Swift bindings in `swift/generated/`
3. Generate Kotlin bindings in `kotlin/generated/`

**Generated Files:**
- Swift: `PaykitMobile.swift`, `PaykitMobileFFI.h`, `PaykitMobileFFI.modulemap`
- Kotlin: `com/paykit/mobile/paykit_mobile.kt`

### Build for iOS

Build the Rust library for all iOS targets and create an XCFramework:

```bash
cd paykit-mobile
./build-ios.sh --framework
```

This will:
1. Build for `aarch64-apple-ios` (iOS devices)
2. Build for `aarch64-apple-ios-sim` (Apple Silicon simulators)
3. Build for `x86_64-apple-ios` (Intel simulators)
4. Create an XCFramework at `ios-demo/PaykitDemo/PaykitDemo/Frameworks/PaykitMobile.xcframework`

**Output:**
- Static libraries: `target/{target}/release/libpaykit_mobile.a`
- XCFramework: `ios-demo/PaykitDemo/PaykitDemo/Frameworks/PaykitMobile.xcframework`

### Build for Android

Build the Rust library for all Android targets and package into jniLibs:

```bash
cd paykit-mobile
./build-android.sh --jniLibs
```

This will:
1. Build for `aarch64-linux-android` (arm64-v8a)
2. Build for `armv7-linux-androideabi` (armeabi-v7a)
3. Build for `x86_64-linux-android` (x86_64)
4. Build for `i686-linux-android` (x86)
5. Package libraries into `android-demo/app/src/main/jniLibs/`

**Output:**
- Shared libraries: `target/{target}/release/libpaykit_mobile.so`
- jniLibs structure: `android-demo/app/src/main/jniLibs/{abi}/libpaykit_mobile.so`

**Note:** Android builds require proper NDK configuration. If builds fail, ensure:
- Android NDK is installed and `ANDROID_NDK_HOME` is set
- Cross-compilation toolchains are configured (see [Android NDK Setup](#android-ndk-setup))

## Integration

### iOS Integration

1. **Add XCFramework to Xcode Project:**
   - Drag `PaykitMobile.xcframework` into your Xcode project
   - Add to "Frameworks, Libraries, and Embedded Content"
   - Ensure "Embed & Sign" is selected

2. **Add Generated Swift Files:**
   - Add `swift/generated/PaykitMobile.swift` to your project
   - Add `swift/generated/PaykitMobileFFI.h` to your bridging header (if using Objective-C)

3. **Import in Swift:**
   ```swift
   import PaykitMobile
   ```

### Android Integration

1. **Add jniLibs to Gradle:**
   ```kotlin
   android {
       sourceSets {
           getByName("main") {
               jniLibs.srcDirs("path/to/jniLibs")
           }
       }
   }
   ```

2. **Add Generated Kotlin Files:**
   - Copy `kotlin/generated/com/paykit/mobile/paykit_mobile.kt` to your project
   - Ensure it's in the correct package structure

3. **Load Native Library:**
   ```kotlin
   init {
       System.loadLibrary("paykit_mobile")
   }
   ```

4. **Import in Kotlin:**
   ```kotlin
   import com.paykit.mobile.*
   ```

## Android NDK Setup

If Android builds fail, you may need to configure the NDK toolchain:

1. **Install Android NDK:**
   - Download from [Android Developer](https://developer.android.com/ndk/downloads)
   - Extract and set `ANDROID_NDK_HOME` environment variable

2. **Configure Cargo for Android:**
   Create `~/.cargo/config.toml`:
   ```toml
   [target.aarch64-linux-android]
   ar = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
   linker = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android21-clang"

   [target.armv7-linux-androideabi]
   ar = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
   linker = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/armv7a-linux-androideabi21-clang"

   [target.x86_64-linux-android]
   ar = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
   linker = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/x86_64-linux-android21-clang"

   [target.i686-linux-android]
   ar = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
   linker = "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/i686-linux-android21-clang"
   ```

   Adjust paths based on your NDK version and platform (darwin-x86_64 for macOS, linux-x86_64 for Linux).

3. **Verify Setup:**
   ```bash
   cargo build --release -p paykit-mobile --target aarch64-linux-android
   ```

## Troubleshooting

### iOS Build Issues

**Issue:** "No such module 'PaykitMobile'**
- Ensure XCFramework is added to project and embedded
- Check that `PaykitMobileFFI.modulemap` is included
- Verify framework search paths in Xcode project settings

**Issue:** "Undefined symbols"
- Ensure all required architectures are included in XCFramework
- Check that static library is linked correctly

### Android Build Issues

**Issue:** "linker not found"
- Configure NDK toolchain in `~/.cargo/config.toml` (see above)
- Verify `ANDROID_NDK_HOME` is set correctly

**Issue:** "library not found" at runtime
- Ensure `.so` files are in correct `jniLibs/{abi}/` directory structure
- Verify `System.loadLibrary("paykit_mobile")` is called before using FFI

**Issue:** "UnsatisfiedLinkError"
- Check that library name matches (should be `paykit_mobile`, not `libpaykit_mobile`)
- Verify correct ABI is being used for the device/emulator

### Binding Generation Issues

**Issue:** "uniffi-bindgen not found"
- Install: `cargo install uniffi-bindgen-cli@0.25`
- Or use the built-in generator: `cargo run --bin generate-bindings --features bindgen-cli`

**Issue:** "Library not found" during binding generation
- Build the library first: `cargo build --release -p paykit-mobile`
- Ensure you're running from the workspace root directory

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build Paykit Mobile

on: [push, pull_request]

jobs:
  build-ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          targets: aarch64-apple-ios,aarch64-apple-ios-sim,x86_64-apple-ios
      - run: cd paykit-mobile && ./build-ios.sh --framework

  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          targets: aarch64-linux-android,armv7-linux-androideabi,x86_64-linux-android,i686-linux-android
      - uses: android-actions/setup-android@v2
      - run: cd paykit-mobile && ./build-android.sh --jniLibs
```

## Verification

After building, verify the outputs:

### iOS
```bash
# Check XCFramework structure
ls -la ios-demo/PaykitDemo/PaykitDemo/Frameworks/PaykitMobile.xcframework/

# Verify architectures
lipo -info ios-demo/PaykitDemo/PaykitDemo/Frameworks/PaykitMobile.xcframework/ios-arm64/libpaykit_mobile.a
```

### Android
```bash
# Check jniLibs structure
find android-demo/app/src/main/jniLibs -name "*.so"

# Verify library architecture
file android-demo/app/src/main/jniLibs/arm64-v8a/libpaykit_mobile.so
```

## Next Steps

After building:
1. Test the demo apps (see `ios-demo/README.md` and `android-demo/README.md`)
2. Integrate into your application (see integration sections above)
3. Run tests: `cargo test --all`

## Related Documentation

- [Paykit Mobile README](README.md)
- [FFI API Reference](FFI_API_REFERENCE.md)
- [Bitkit Integration Guide](BITKIT_INTEGRATION_GUIDE.md)

# Paykit Mobile Testing Guide

This guide covers testing strategies for the Paykit mobile implementation, including
Noise protocol payment flows, cross-platform testing, and integration with the CLI demo.

## Table of Contents

1. [Overview](#overview)
2. [Test Categories](#test-categories)
3. [Running Tests](#running-tests)
4. [Cross-Platform Testing](#cross-platform-testing)
5. [Manual Testing Checklist](#manual-testing-checklist)
6. [CI/CD Integration](#cicd-integration)

---

## Overview

The Paykit mobile testing suite covers:

- **Unit Tests**: Individual function and module tests in Rust
- **Integration Tests**: End-to-end flows using mock transports
- **FFI Tests**: Verification of Swift/Kotlin bindings
- **Cross-Platform Tests**: Interoperability between iOS, Android, and CLI

### Test Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Test Categories                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  Unit Tests  │  │ Integration  │  │  FFI Tests   │       │
│  │  (Rust)      │  │   Tests      │  │ (Swift/Kt)   │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│         │                 │                  │               │
│         └────────────────┼──────────────────┘               │
│                          │                                   │
│                          ▼                                   │
│              ┌─────────────────────┐                        │
│              │  Cross-Platform     │                        │
│              │  Integration Tests  │                        │
│              └─────────────────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Test Categories

### 1. Noise FFI Integration Tests

**Location**: `paykit-mobile/tests/noise_ffi_integration.rs`

Tests the FFI layer for Noise protocol operations:

- Endpoint discovery (publish, discover, remove)
- Message creation (receipt request, confirmation, error, private endpoint offer)
- Message parsing
- Server configuration
- End-to-end message exchange flows

```bash
cargo test -p paykit-mobile --test noise_ffi_integration
```

### 2. Server Mode Tests

**Location**: `paykit-mobile/tests/noise_server_mode.rs`

Tests server mode (receiving payments):

- Server configuration defaults
- Endpoint publishing for discovery
- Handling client requests
- Private endpoint offers
- Multiple client handling
- Server shutdown and endpoint cleanup

```bash
cargo test -p paykit-mobile --test noise_server_mode
```

### 3. Interactive Payment Tests

**Location**: `paykit-interactive/tests/`

Tests the core interactive payment logic:

- `integration_noise.rs`: Real Noise handshakes over TCP
- `e2e_mobile_flow.rs`: Mobile wallet simulation
- `e2e_payment_flows.rs`: Complete payment scenarios
- `manager_tests.rs`: PaykitInteractiveManager tests
- `serialization.rs`: Message serialization tests

```bash
cargo test -p paykit-interactive
```

### 4. CLI Integration Tests

**Location**: `paykit-demo-cli/tests/noise_integration.rs`

Tests CLI commands for Noise operations:

- Endpoint discovery commands
- Connection commands
- Payment flow commands
- Key management commands
- Cross-platform message compatibility

```bash
cargo test -p paykit-demo-cli --test noise_integration
```

---

## Running Tests

### All Rust Tests

```bash
# Run all tests in the workspace
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_publish_and_discover_endpoint_roundtrip
```

### Mobile-Specific Tests

```bash
# All paykit-mobile tests
cargo test -p paykit-mobile

# Specific test file
cargo test -p paykit-mobile --test noise_ffi_integration
cargo test -p paykit-mobile --test noise_server_mode
```

### iOS Tests (Xcode)

**Prerequisites**:
- Xcode 15.0+
- iOS Simulator (iPhone 17 Pro or later recommended)
- Test target `PaykitDemoTests` is configured in the project

**From Xcode**:
1. Open `paykit-mobile/ios-demo/PaykitDemo/PaykitDemo/PaykitDemo.xcodeproj`
2. Select the `PaykitDemo` scheme
3. Select a simulator target (e.g., iPhone 17 Pro)
4. Press `Cmd+U` to run all tests
5. Or select specific test classes/functions in the test navigator

**From Command Line**:
```bash
cd paykit-mobile/ios-demo/PaykitDemo/PaykitDemo
xcodebuild test \
  -project PaykitDemo.xcodeproj \
  -scheme PaykitDemo \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  CODE_SIGNING_ALLOWED=NO
```

**Test Infrastructure**:
- Test target: `PaykitDemoTests` (unit test bundle)
- Test files: Located in `PaykitDemoTests/` directory
- Test helpers: `TestHelpers.swift` provides mock services and utilities
- All tests use async/await for asynchronous operations

**Known Issues**:
- Test target must be built before running tests
- Some tests may require network connectivity for directory operations

### Android Tests (Gradle)

**Prerequisites**:
- Android SDK 26+
- Gradle 9.0+
- Java 17+
- Android emulator or connected device

**Emulator Setup** (Automated):
```bash
# Option 1: Use setup script
cd paykit-mobile/android-demo
./scripts/setup_emulator.sh [avd_name]

# Option 2: Manual setup
# Set ANDROID_HOME if not already set
export ANDROID_HOME=~/Library/Android/sdk
export PATH=$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$PATH

# List available AVDs
$ANDROID_HOME/emulator/emulator -list-avds

# Start emulator (headless mode)
$ANDROID_HOME/emulator/emulator -avd <avd_name> -no-window -no-audio &

# Wait for emulator to boot
adb wait-for-device
timeout=120
elapsed=0
while [ $elapsed -lt $timeout ]; do
    boot=$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
    if [ "$boot" = "1" ]; then
        echo "Emulator ready"
        break
    fi
    sleep 2
    elapsed=$((elapsed + 2))
done

# Verify device is connected
adb devices
```

**Unit Tests**:
```bash
cd paykit-mobile/android-demo

# Run all unit tests
./gradlew test

# Run specific test class
./gradlew test --tests "com.paykit.demo.KeyManagementE2ETest"
```

**Instrumented E2E Tests**:
```bash
cd paykit-mobile/android-demo

# Ensure emulator is running and connected
adb devices

# Run all instrumented tests
./gradlew connectedAndroidTest

# Run with verbose output
./gradlew connectedAndroidTest --info
```

**Test Results**:
- ✅ **50 E2E tests passing** across 4 test suites
- Test suites:
  - `NoisePaymentE2ETest`: 10 tests ✅
  - `KeyManagementE2ETest`: 13 tests ✅
  - `DirectoryE2ETest`: 15 tests ✅
  - `ServerModeE2ETest`: 12 tests ✅

**Test Infrastructure**:
- Unit tests: `app/src/test/` - JUnit4 tests
- Instrumented tests: `app/src/androidTest/` - AndroidJUnit4 tests
- Mock services available (MockKeyManager, MockReceiptStore, MockDirectoryService)
- Test Application class (`TestApplication.kt`) avoids native library initialization
- Native libraries included in test APK via `sourceSets` configuration
- All tests use Kotlin Coroutines for async operations

**Test Results Location**:
- Unit tests: `build/test-results/test/`
- Instrumented tests: `build/reports/androidTests/connected/debug/`
- XML results: `build/outputs/androidTest-results/`

---

## Cross-Platform Testing

### Test Matrix

| Scenario | iOS → Android | Android → iOS | CLI → Mobile | Mobile → CLI |
|----------|---------------|---------------|--------------|--------------|
| Payment Request | ✅ | ✅ | ✅ | ✅ |
| Receipt Confirmation | ✅ | ✅ | ✅ | ✅ |
| Private Endpoint | ✅ | ✅ | ✅ | ✅ |
| Error Handling | ✅ | ✅ | ✅ | ✅ |

### Manual Cross-Platform Test

#### Prerequisites

1. iOS Simulator running the demo app
2. Android Emulator running the demo app
3. CLI built and ready

#### Test 1: iOS Sends to Android

1. **Android (Receiver)**:
   - Open Android demo app
   - Go to "Receive" tab
   - Tap "Start Listening"
   - Note the connection info

2. **iOS (Sender)**:
   - Open iOS demo app
   - Go to "Send" tab
   - Enter Android's connection info
   - Enter amount and tap "Send"

3. **Expected**:
   - Android shows pending request
   - Accept on Android
   - iOS shows success with receipt ID
   - Both apps show receipt in history

#### Test 2: CLI Sends to Mobile

1. **Mobile (Receiver)**:
   - Start server mode (listen)
   - Note connection info (host:port:pubkey)

2. **CLI (Sender)**:
   ```bash
   paykit-demo-cli noise send \
     --recipient <MOBILE_PUBKEY> \
     --host <HOST> \
     --port <PORT> \
     --amount 1000 \
     --method lightning
   ```

3. **Expected**:
   - Mobile shows pending request
   - Accept on mobile
   - CLI shows success

#### Test 3: Mobile Sends to CLI

1. **CLI (Receiver)**:
   ```bash
   paykit-demo-cli noise listen --port 8888
   ```

2. **Mobile (Sender)**:
   - Enter CLI's connection info
   - Send payment

3. **Expected**:
   - CLI shows incoming request
   - Accept in CLI
   - Mobile shows success

---

## Manual Testing Checklist

### iOS Demo App

- [ ] App launches successfully
- [ ] Can navigate to Send tab
- [ ] Can navigate to Receive tab
- [ ] Send: Form validates input
- [ ] Send: Shows progress states
- [ ] Send: Can cancel payment
- [ ] Send: Shows success dialog
- [ ] Send: Shows error dialog on failure
- [ ] Receive: Can start listening
- [ ] Receive: Shows connection info
- [ ] Receive: Can copy connection string
- [ ] Receive: Can show QR code
- [ ] Receive: Shows pending requests
- [ ] Receive: Can accept/decline requests
- [ ] Receive: Shows recent receipts
- [ ] Receipts saved to storage

### Android Demo App

- [ ] App launches successfully
- [ ] Can navigate to Send tab
- [ ] Can navigate to Receive tab
- [ ] Send: Form validates input
- [ ] Send: Shows progress states (StateFlow)
- [ ] Send: Can cancel payment
- [ ] Send: Shows success dialog
- [ ] Send: Shows error dialog on failure
- [ ] Receive: Can start listening
- [ ] Receive: Shows connection info
- [ ] Receive: Can copy connection string
- [ ] Receive: Can show QR code
- [ ] Receive: Shows pending requests
- [ ] Receive: Can accept/decline requests
- [ ] Receive: Shows recent receipts
- [ ] Receipts saved to storage

### CLI Demo

- [ ] `noise discover` works
- [ ] `noise publish` works
- [ ] `noise remove` works
- [ ] `noise connect` shows errors for invalid hosts
- [ ] `noise send` requires recipient
- [ ] `noise listen` starts server
- [ ] `noise pubkey` shows key
- [ ] `noise --help` shows usage

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Mobile Tests

on: [push, pull_request]

jobs:
  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run Rust tests
        run: |
          cargo test -p paykit-mobile
          cargo test -p paykit-interactive
          cargo test -p paykit-demo-cli --test noise_integration

  ios-tests:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build iOS
        run: |
          cd paykit-mobile/ios-demo/PaykitDemo
          xcodebuild build \
            -project PaykitDemo.xcodeproj \
            -scheme PaykitDemo \
            -destination 'platform=iOS Simulator,name=iPhone 15'

  android-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-java@v3
        with:
          java-version: '17'
          distribution: 'temurin'
      - name: Run Android tests
        run: |
          cd paykit-mobile/android-demo
          ./gradlew test
```

---

## Troubleshooting

### Common Issues

#### "No Noise endpoint found"

- Ensure the recipient has published their endpoint
- Check network connectivity
- Verify the pubkey is correct

#### "Connection refused"

- Ensure the server is running
- Check firewall settings
- Verify port is correct

#### "Handshake failed"

- Verify server public key is correct
- Check that both parties are using compatible Noise versions
- Ensure keys are properly derived

#### iOS Build Fails

- Run `pod install` in ios-demo directory
- Ensure Xcode command line tools are installed
- Check that PubkyNoise.xcframework is present

#### Android Build Fails

- Ensure JNI libraries are in jniLibs directory
- Check Gradle version compatibility
- Verify Kotlin version matches

---

## Related Documentation

- [NOISE_INTEGRATION_GUIDE.md](./NOISE_INTEGRATION_GUIDE.md) - Integration overview
- [NOISE_PAYMENTS_IMPLEMENTATION.md](./NOISE_PAYMENTS_IMPLEMENTATION.md) - Implementation details
- [ios-demo/README.md](./ios-demo/README.md) - iOS setup
- [android-demo/README.md](./android-demo/README.md) - Android setup


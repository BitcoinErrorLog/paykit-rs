# E2E Testing Guide

This guide documents the End-to-End (E2E) test suite for the Paykit Mobile demo applications.

## Overview

The E2E test suite provides comprehensive testing for:
- **Payment Flows**: Send/receive payments, receipt exchange
- **Key Management**: Identity creation, key derivation, caching
- **Directory Operations**: Endpoint publishing, discovery, removal
- **Server Mode**: Server lifecycle, client handling, message processing
- **Cross-Platform Compatibility**: iOS ↔ Android, Mobile ↔ CLI

## Test Architecture

### Test Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Cross-Platform Tests                      │
│                  (paykit-mobile/tests/)                      │
│  - cross_platform_e2e.rs: iOS↔Android, Mobile↔CLI tests     │
│  - e2e_helpers.rs: Shared test infrastructure                │
└─────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   iOS Tests     │ │  Android Tests  │ │   Rust Tests    │
│   (XCTest)      │ │   (JUnit4)      │ │  (cargo test)   │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

### Test Files

#### iOS (Swift/XCTest)

Location: `ios-demo/PaykitDemo/PaykitDemo/PaykitDemoTests/`

| File | Description |
|------|-------------|
| `TestHelpers.swift` | Mock services, fixtures, assertion helpers |
| `NoisePaymentE2ETests.swift` | Payment flow tests |
| `KeyManagementE2ETests.swift` | Key derivation and caching tests |
| `DirectoryE2ETests.swift` | Endpoint discovery tests |
| `ServerModeE2ETests.swift` | Server mode tests |

#### Android (Kotlin/JUnit4)

Location: `android-demo/app/src/androidTest/java/com/paykit/demo/`

| File | Description |
|------|-------------|
| `TestHelpers.kt` | Mock services, fixtures, assertion helpers |
| `NoisePaymentE2ETest.kt` | Payment flow tests |
| `KeyManagementE2ETest.kt` | Key derivation and caching tests |
| `DirectoryE2ETest.kt` | Endpoint discovery tests |
| `ServerModeE2ETest.kt` | Server mode tests |

#### Cross-Platform (Rust)

Location: `paykit-mobile/tests/`

| File | Description |
|------|-------------|
| `e2e_helpers.rs` | Shared test infrastructure |
| `cross_platform_e2e.rs` | Cross-platform payment tests |

## Running Tests

### iOS Tests

```bash
cd paykit-mobile/ios-demo/PaykitDemo/PaykitDemo

# Run all tests
xcodebuild test \
  -project PaykitDemo.xcodeproj \
  -scheme PaykitDemo \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro'

# Run specific test file
xcodebuild test \
  -project PaykitDemo.xcodeproj \
  -scheme PaykitDemo \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  -only-testing:PaykitDemoTests/NoisePaymentE2ETests
```

### Android Tests

```bash
cd paykit-mobile/android-demo

# Run instrumented tests (on device/emulator)
./gradlew connectedAndroidTest

# Run specific test class
./gradlew connectedAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.class=com.paykit.demo.NoisePaymentE2ETest
```

### Rust/Cross-Platform Tests

```bash
cd paykit-mobile

# Run all tests
cargo test

# Run specific test file
cargo test --test cross_platform_e2e

# Run with output
cargo test -- --nocapture
```

## Test Categories

### 1. Payment Flow Tests

Tests the complete payment lifecycle:

```swift
// iOS Example
func testCompletePaymentFlowMocked() throws {
    // 1. Create sender/receiver identities
    // 2. Receiver publishes endpoint
    // 3. Sender discovers endpoint
    // 4. Sender creates payment request
    // 5. Process payment
    // 6. Verify receipt stored
}
```

**Test Cases:**
- Create payment request
- Payment with optional fields
- Receipt confirmation
- Receipt storage/retrieval
- Complete payment flow
- Multiple concurrent payments
- Payment to unknown recipient
- Payment with no identity

### 2. Key Management Tests

Tests key derivation and caching:

**Test Cases:**
- Create identity
- Create multiple identities
- Switch between identities
- Get public key
- Secret key generation
- Noise key derivation
- Key caching behavior
- Rapid identity switching

### 3. Directory Tests

Tests endpoint discovery:

**Test Cases:**
- Publish endpoint
- Publish without metadata
- Update existing endpoint
- Discover existing endpoint
- Discover non-existent endpoint
- Remove endpoint
- Endpoint with localhost/IPv6/domain
- Long metadata handling
- Special characters in metadata

### 4. Server Mode Tests

Tests the server (receiver) functionality:

**Test Cases:**
- Server configuration
- Server lifecycle (start/stop)
- Server restart
- Single client connection
- Multiple client connections
- Process payment request
- Process invalid message
- Receipt generation
- Client disconnect handling
- Max connections limit

### 5. Cross-Platform Tests

Tests platform interoperability:

**Test Cases:**
- iOS → Android payment
- Android → iOS payment
- Mobile → CLI payment
- CLI → Mobile payment
- Bidirectional payment flow
- Multiple concurrent cross-platform payments
- Message format compatibility
- Receipt format compatibility

## Mock Services

### MockKeyManager

Simulates key management:

```swift
class MockKeyManager {
    func createIdentity(nickname: String) -> MockIdentity
    func setCurrentIdentity(_ nickname: String) -> Bool
    func getCurrentIdentity() -> MockIdentity?
    func getPublicKey() -> String?
}
```

### MockReceiptStore

Simulates receipt storage:

```kotlin
class MockReceiptStore {
    fun storeReceipt(receipt: MockReceipt)
    fun getReceipt(id: String): MockReceipt?
    fun getAllReceipts(): List<MockReceipt>
    fun clear()
}
```

### MockDirectoryService

Simulates endpoint directory:

```rust
impl MockDirectoryService {
    pub fn publish(&self, endpoint: MockEndpoint);
    pub fn discover(&self, pubkey: &str) -> Option<MockEndpoint>;
    pub fn remove(&self, pubkey: &str);
    pub fn clear(&self);
}
```

### MockNoiseServer

Simulates server mode:

```swift
class MockNoiseServer {
    func start(port: UInt16) -> Bool
    func stop()
    func acceptConnection() -> String?
    func disconnectClient(_ clientId: String)
    func processMessage(...) -> [String: Any]?
}
```

## Test Helpers

### Test Configuration

```swift
struct TestConfig {
    static let defaultTimeout: TimeInterval = 30.0
    static let shortTimeout: TimeInterval = 5.0
    static func randomPort() -> UInt16
}
```

### Test Data Generators

```kotlin
object TestDataGenerator {
    fun createPaymentRequest(from, to, amount): Pair<String, Map>
    fun createReceiptConfirmation(receiptId, payee): Map
    fun randomBytes(count: Int): ByteArray
    fun mockNoisePubkey(): String
}
```

### Assertion Helpers

```rust
pub fn assert_receipt_valid(receipt, expected_payer, expected_payee);
pub fn assert_endpoint_valid(endpoint);
```

## Best Practices

### Test Isolation

- Each test creates its own mock services
- Clear mock stores in tearDown/cleanup
- Use random ports to avoid conflicts

### Async Testing

- Use appropriate timeout values
- Handle async operations properly
- Don't rely on timing for assertions

### Mock vs Real

- Use mocks for unit-style E2E tests
- Integrate with real FFI for integration tests
- Test against real network only in dedicated environments

## Extending Tests

### Adding New Test Cases

1. Add test method to appropriate test class
2. Use existing mock services
3. Follow naming convention: `test<Feature><Scenario>`

### Adding New Mock Services

1. Create mock class with required interface
2. Add to TestHelpers file
3. Document in this guide

### Cross-Platform Test Additions

1. Add test to `cross_platform_e2e.rs`
2. Use `PlatformIdentity` for platform-specific behavior
3. Verify protocol compatibility

## CI Integration

### GitHub Actions (Example)

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test -p paykit-mobile

  ios-tests:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - run: |
          cd paykit-mobile/ios-demo/PaykitDemo/PaykitDemo
          xcodebuild test \
            -project PaykitDemo.xcodeproj \
            -scheme PaykitDemo \
            -destination 'platform=iOS Simulator,name=iPhone 15'

  android-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: |
          cd paykit-mobile/android-demo
          ./gradlew test
```

## Troubleshooting

### Common Issues

**Test timeout**: Increase timeout or check for blocking operations

**Port conflicts**: Use `find_available_port()` helper

**Flaky tests**: Add retries for network-dependent tests

**Mock state leakage**: Ensure proper cleanup in tearDown

### Debug Tips

1. Enable verbose logging in mock services
2. Print intermediate states
3. Use debugger breakpoints
4. Check test isolation

## Summary

The E2E test suite provides comprehensive coverage for:

- ✅ Payment flows (send, receive, receipts)
- ✅ Key management (identity, derivation, caching)
- ✅ Directory operations (publish, discover, remove)
- ✅ Server mode (lifecycle, connections, messages)
- ✅ Cross-platform compatibility (iOS, Android, CLI)

Total test count: 50+ E2E tests across all platforms.

# Building and Testing the Paykit iOS Demo

Complete step-by-step guide to build and test the Paykit iOS demo application.

## Prerequisites

### Required Tools

1. **Rust 1.70+** (via Rustup, NOT Homebrew)
   ```bash
   rustc --version  # Should be 1.70.0 or higher
   which rustc     # Should show ~/.cargo/bin/rustc
   ```

2. **UniFFI 0.25+**
   ```bash
   cargo install uniffi-bindgen-cli@0.25
   ```

3. **Xcode 15.0+** with:
   - iOS 16.0+ SDK
   - Swift 5.9+
   - Command Line Tools installed

### Verify Prerequisites

```bash
# Check Rust
rustc --version
cargo --version

# Check UniFFI
uniffi-bindgen --version

# Check Xcode
xcodebuild -version
swift --version
```

## Step 1: Build the Paykit Mobile Library

From the workspace root:

```bash
cd /Users/john/vibes-dev/paykit-rs-master

# Build the library in release mode
cargo build --release -p paykit-mobile
```

This creates:
- `target/release/libpaykit_mobile.dylib` (macOS dynamic library)
- `target/release/libpaykit_mobile.a` (static library)

## Step 2: Generate Swift Bindings

### Option A: Use the Build Script (Recommended)

```bash
cd paykit-mobile
./generate-bindings.sh
```

This will:
- Build the library (if not already built)
- Generate Swift bindings in `swift/generated/`
- Generate Kotlin bindings in `kotlin/generated/`

### Option B: Manual Generation

```bash
cd paykit-mobile

# Generate Swift bindings
uniffi-bindgen generate \
    --library ../target/release/libpaykit_mobile.dylib \
    -l swift \
    -o swift/generated
```

**Expected Output:**
```
swift/generated/
├── paykit_mobile.swift
├── paykit_mobileFFI.h
└── paykit_mobileFFI.modulemap
```

## Step 3: Create Xcode Project

1. **Open Xcode** and create a new project:
   - File → New → Project
   - Choose "iOS" → "App"
   - Product Name: `PaykitDemo`
   - Interface: SwiftUI
   - Language: Swift
   - Minimum Deployment: iOS 16.0

2. **Save the project** in a temporary location (we'll move files next)

## Step 4: Add Demo Files to Xcode Project

1. **Copy Swift source files** from `paykit-mobile/ios-demo/PaykitDemo/`:
   ```bash
   # From paykit-rs-master root
   cp -r paykit-mobile/ios-demo/PaykitDemo/* <YourXcodeProject>/PaykitDemo/
   ```

   Files to copy:
   - `PaykitDemoApp.swift`
   - `Models/AutoPayModels.swift`
   - `ViewModels/AutoPayViewModel.swift`
   - `Views/ContentView.swift`
   - `Views/PaymentMethodsView.swift`
   - `Views/SubscriptionsView.swift`
   - `Views/AutoPayView.swift`
   - `Views/PaymentRequestsView.swift`
   - `Views/SettingsView.swift`

2. **In Xcode**, drag these files into your project:
   - Right-click on `PaykitDemo` folder in Project Navigator
   - Choose "Add Files to PaykitDemo..."
   - Select all the copied Swift files
   - Ensure "Copy items if needed" is checked
   - Ensure your app target is selected

## Step 5: Add Generated Bindings

1. **Copy generated bindings**:
   ```bash
   cp paykit-mobile/swift/generated/* <YourXcodeProject>/PaykitDemo/
   ```

2. **In Xcode**, add the generated files:
   - `paykit_mobile.swift`
   - `paykit_mobileFFI.h`
   - `paykit_mobileFFI.modulemap`

3. **Add KeychainStorage** (if available):
   ```bash
   # Check if KeychainStorage.swift exists
   find paykit-mobile -name "KeychainStorage.swift"
   
   # If found, copy it
   cp paykit-mobile/swift/KeychainStorage.swift <YourXcodeProject>/PaykitDemo/
   ```

## Step 6: Configure Build Settings

### 6.1 Link the Library

1. In Xcode, select your project in Project Navigator
2. Select your target (`PaykitDemo`)
3. Go to **Build Phases** tab
4. Expand **Link Binary with Libraries**
5. Click **+** and add:
   - `libpaykit_mobile.a` (or use "Add Other..." to browse to `target/release/`)

### 6.2 Configure Library Search Paths

1. Go to **Build Settings** tab
2. Search for "Library Search Paths"
3. Add:
   ```
   $(PROJECT_DIR)/../target/release
   ```
   (Adjust path relative to your Xcode project location)

### 6.3 Configure Header Search Paths

1. In **Build Settings**, search for "Header Search Paths"
2. Add:
   ```
   $(PROJECT_DIR)
   ```

### 6.4 Configure Module Map

1. In **Build Settings**, search for "Import Paths" or "Swift Compiler - Search Paths"
2. Add:
   ```
   $(PROJECT_DIR)
   ```

### 6.5 Enable Modules

1. In **Build Settings**, search for "Enable Modules (C and Objective-C)"
2. Set to **Yes**

## Step 7: Build and Run

### 7.1 Build

1. In Xcode, select a simulator (e.g., "iPhone 15 Pro")
2. Press **⌘B** (or Product → Build)
3. Fix any import or linking errors

### 7.2 Run

1. Press **⌘R** (or Product → Run)
2. The app should launch in the simulator

## Step 8: Testing

### 8.1 Basic Functionality Tests

1. **Payment Methods Tab**:
   - Verify payment methods are listed
   - Check health status indicators
   - Test endpoint validation

2. **Subscriptions Tab**:
   - Create a new subscription
   - Test proration calculator
   - Verify subscription list

3. **Auto-Pay Tab**:
   - Enable/disable auto-pay
   - Set global daily limit
   - Add per-peer limits
   - Create auto-pay rules

4. **Payment Requests Tab**:
   - Create a payment request
   - Accept/decline requests
   - Check request history

5. **Settings Tab**:
   - Change network (Testnet/Regtest for testing)
   - Configure security settings
   - Test key management

### 8.2 Unit Tests (Optional)

Create a test target:

1. File → New → Target
2. Choose "iOS Unit Testing Bundle"
3. Add test files:

```swift
import XCTest
@testable import PaykitDemo

class PaykitDemoTests: XCTestCase {
    func testPaykitClientInitialization() {
        do {
            let client = try PaykitClient()
            XCTAssertNotNil(client)
        } catch {
            XCTFail("Failed to initialize PaykitClient: \(error)")
        }
    }
    
    func testListPaymentMethods() {
        do {
            let client = try PaykitClient()
            let methods = client.listMethods()
            XCTAssertFalse(methods.isEmpty, "Should have at least one payment method")
        } catch {
            XCTFail("Failed to list methods: \(error)")
        }
    }
}
```

## Troubleshooting

### Error: "No such module 'paykit_mobile'"

**Solution:**
1. Ensure `paykit_mobileFFI.modulemap` is in your project
2. Check "Import Paths" in Build Settings includes `$(PROJECT_DIR)`
3. Clean build folder: Product → Clean Build Folder (⇧⌘K)

### Error: "Undefined symbol: _paykit_mobile_..."

**Solution:**
1. Verify library is linked in Build Phases
2. Check Library Search Paths includes the release directory
3. Ensure you're using the correct architecture (arm64 for iOS Simulator on Apple Silicon)

### Error: "Library not found for -lpaykit_mobile"

**Solution:**
1. Verify `libpaykit_mobile.a` exists in `target/release/`
2. Check Library Search Paths is correctly configured
3. Try using full path: `$(PROJECT_DIR)/../target/release/libpaykit_mobile.a`

### Error: "Cannot find 'PaykitClient' in scope"

**Solution:**
1. Ensure `paykit_mobile.swift` is added to your target
2. Check file is included in "Compile Sources" in Build Phases
3. Verify import statement: `import Foundation` (PaykitClient should be available globally)

### Build Fails: "Command uniffi-bindgen failed"

**Solution:**
```bash
# Reinstall uniffi-bindgen
cargo install --force uniffi-bindgen-cli@0.25

# Verify installation
uniffi-bindgen --version
```

### Simulator Crashes on Launch

**Solution:**
1. Check console logs in Xcode for specific errors
2. Verify all required files are added to target
3. Ensure minimum iOS deployment target is 16.0
4. Try cleaning derived data: Xcode → Settings → Locations → Derived Data → Delete

## Quick Test Script

Save this as `test-ios-demo.sh`:

```bash
#!/bin/bash
set -e

echo "=== Building Paykit Mobile Library ==="
cargo build --release -p paykit-mobile

echo ""
echo "=== Generating Swift Bindings ==="
cd paykit-mobile
./generate-bindings.sh

echo ""
echo "=== Bindings Generated ==="
echo "Next steps:"
echo "1. Create Xcode project"
echo "2. Copy files from ios-demo/PaykitDemo/"
echo "3. Add generated bindings from swift/generated/"
echo "4. Configure build settings"
echo "5. Build and run!"
```

Make it executable:
```bash
chmod +x test-ios-demo.sh
./test-ios-demo.sh
```

## Additional Resources

- **Full Documentation**: `paykit-mobile/README.md`
- **Mobile Integration Guide**: `docs/mobile-integration.md`
- **iOS Demo README**: `paykit-mobile/ios-demo/README.md`

## Next Steps

After successfully building and testing:

1. **Customize the UI** for your app's design
2. **Add real payment method endpoints** (Lightning node, on-chain addresses)
3. **Configure testnet mode** for safe testing
4. **Integrate with your backend** using the transport APIs
5. **Add error handling** for production use


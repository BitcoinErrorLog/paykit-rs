# Quick Start: iOS Demo Setup

## ✅ What's Already Done

1. ✅ **Library Built**: `target/release/libpaykit_mobile.a` (49MB) ready
2. ✅ **All Demo Files**: 9 Swift files ready in `ios-demo/PaykitDemo/`
3. ✅ **Directories Created**: Ready for generated bindings

## 🚀 Next Steps (5 minutes)

### Step 1: Install uniffi-bindgen (One-time)

Run the automated installer:

```bash
cd "/Users/john/vibes-dev/paykit-rs-master/paykit-mobile/ios-demo"
./install-uniffi-bindgen.sh
```

**OR** install manually:
```bash
cd /tmp
git clone https://github.com/mozilla/uniffi-rs.git
cd uniffi-rs && git checkout v0.25.0
cd uniffi_bindgen
cargo install --path . --force
```

### Step 2: Generate Swift Bindings

```bash
cd "/Users/john/vibes-dev/paykit-rs-master/paykit-mobile"
./generate-bindings.sh
```

This creates:
- `swift/generated/paykit_mobile.swift`
- `swift/generated/paykit_mobileFFI.h`
- `swift/generated/paykit_mobileFFI.modulemap`

### Step 3: Create Xcode Project

1. Open **Xcode**
2. **File → New → Project**
3. Choose **iOS → App**
4. Configure:
   - Product Name: `PaykitDemo`
   - Interface: **SwiftUI**
   - Language: **Swift**
   - Minimum Deployment: **iOS 16.0**
5. Save anywhere (we'll copy files next)

### Step 4: Add Files to Xcode

**Copy demo files:**
```bash
# From paykit-rs-master root
cp -r paykit-mobile/ios-demo/PaykitDemo/* <YourXcodeProjectPath>/PaykitDemo/
```

**In Xcode:**
1. Right-click `PaykitDemo` folder → **Add Files to PaykitDemo...**
2. Select all copied Swift files
3. Check "Copy items if needed" ✅
4. Check your target ✅

**Add generated bindings:**
```bash
cp paykit-mobile/swift/generated/* <YourXcodeProjectPath>/PaykitDemo/
```

**In Xcode:**
1. Add `paykit_mobile.swift`, `paykit_mobileFFI.h`, `paykit_mobileFFI.modulemap`
2. Ensure they're added to your target

**Add library:**
1. In Xcode: Target → **Build Phases** → **Link Binary with Libraries**
2. Click **+** → **Add Other...**
3. Navigate to: `target/release/libpaykit_mobile.a`
4. Add it

### Step 5: Configure Build Settings

1. Select your **target** (`PaykitDemo`)
2. Go to **Build Settings** tab
3. Search for "Library Search Paths"
4. Add: `$(PROJECT_DIR)/../target/release` (adjust path as needed)
5. Search for "Header Search Paths"
6. Add: `$(PROJECT_DIR)`
7. Search for "Enable Modules"
8. Set to **Yes**

### Step 6: Build & Run

1. Select a simulator (e.g., iPhone 15 Pro)
2. Press **⌘R** (or Product → Run)
3. App should launch! 🎉

## 📁 File Locations Reference

**Library:**
- `target/release/libpaykit_mobile.a` (49MB) ✅

**Demo Source:**
- `paykit-mobile/ios-demo/PaykitDemo/` (9 files) ✅

**Generated Bindings (after Step 2):**
- `paykit-mobile/swift/generated/` (3 files)

## 🐛 Troubleshooting

**"No such module 'paykit_mobile'"**
- Ensure `paykit_mobileFFI.modulemap` is in project
- Check "Import Paths" includes `$(PROJECT_DIR)`

**"Library not found"**
- Verify library search path points to `target/release/`
- Check file exists: `ls target/release/libpaykit_mobile.a`

**"Undefined symbol"**
- Ensure library is linked in Build Phases
- Clean build: **⇧⌘K**

## 📚 Full Documentation

- **Detailed Guide**: `BUILD_AND_TEST.md`
- **Setup Status**: `SETUP_STATUS.md`
- **Main README**: `README.md`


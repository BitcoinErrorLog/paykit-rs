# Bitkit + Paykit Integration Master Guide

> **For Synonym Development Team**  
> **Version**: 1.0  
> **Last Updated**: December 22, 2025  
> **Status**: Reference Implementation Complete

This guide documents the complete integration of Paykit into Bitkit iOS, Bitkit Android, and Pubky Ring. It serves as a detailed map for production developers to follow, including all steps, quirks, stubs, and future work.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture Overview](#2-architecture-overview)
3. [Prerequisites](#3-prerequisites)
4. [Building paykit-rs](#4-building-paykit-rs)
5. [iOS Integration](#5-ios-integration)
6. [Android Integration](#6-android-integration)
7. [Pubky Ring Integration](#7-pubky-ring-integration)
8. [Feature Implementation Guide](#8-feature-implementation-guide)
9. [Known Quirks & Footguns](#9-known-quirks--footguns)
10. [Stubs & Mocks Inventory](#10-stubs--mocks-inventory)
11. [Testing Requirements](#11-testing-requirements)
12. [Production Configuration](#12-production-configuration)
13. [Security Checklist](#13-security-checklist)
14. [Troubleshooting](#14-troubleshooting)
15. [Future Work](#15-future-work)

---

## 1. Executive Summary

### What is Paykit?

Paykit is a decentralized payment protocol built on Pubky that enables:
- **Payment Method Discovery**: Query public directories to find how someone accepts payments
- **Encrypted Payment Channels**: Noise Protocol (Noise_IK) for secure payment negotiation
- **Multi-Method Support**: Bitcoin onchain, Lightning, and extensible to other methods
- **Subscriptions & Auto-Pay**: Recurring payments with cryptographic agreements

### What This Integration Accomplishes

| Feature | iOS | Android | Ring |
|---------|-----|---------|------|
| Payment Method Discovery | ✅ | ✅ | ✅ |
| Directory Publishing | ✅ | ✅ | ✅ |
| Noise Protocol Payments | ✅ | ✅ | N/A |
| Subscriptions | ✅ | ✅ | N/A |
| Auto-Pay Rules | ✅ | ✅ | N/A |
| Spending Limits | ✅ | ✅ | N/A |
| Smart Checkout | ✅ | ✅ | N/A |
| Cross-App Key Sharing | ✅ | ✅ | ✅ |

### Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| `paykit-lib` | ✅ Production-Ready | Core protocol library |
| `paykit-interactive` | ✅ Production-Ready | Noise payments |
| `paykit-subscriptions` | ✅ Production-Ready | Recurring payments |
| `paykit-mobile` | ✅ Production-Ready | FFI bindings |
| Bitkit iOS Integration | ✅ Demo Complete | Needs production testing |
| Bitkit Android Integration | ✅ Demo Complete | Needs production testing |
| Ring Integration | ✅ Key Sharing Working | Session management complete |

---

## 2. Architecture Overview

### Component Diagram

```mermaid
flowchart TB
    subgraph bitkit [Bitkit App]
        UI[SwiftUI / Compose UI]
        VM[ViewModels]
        SVC[Services Layer]
        STORE[Secure Storage]
    end

    subgraph ring [Pubky Ring]
        RING_UI[Ring UI]
        RING_KEYS[Key Manager]
        RING_SESSION[Session Manager]
    end

    subgraph paykit [paykit-mobile FFI]
        FFI[UniFFI Bindings]
        CLIENT[PaykitClient]
        TRANSPORT[Transport Adapters]
    end

    subgraph rust [Rust Core]
        LIB[paykit-lib]
        INTER[paykit-interactive]
        SUBS[paykit-subscriptions]
    end

    subgraph external [External Services]
        PUBKY[Pubky Homeserver]
        LN[Lightning Network]
        BTC[Bitcoin Network]
    end

    UI --> VM --> SVC --> STORE
    SVC --> FFI
    FFI --> CLIENT --> LIB
    CLIENT --> INTER
    CLIENT --> SUBS
    
    RING_SESSION <-.-> SVC
    RING_KEYS --> TRANSPORT
    
    LIB --> PUBKY
    INTER --> PUBKY
    SVC --> LN
    SVC --> BTC
```

### Key Architecture: "Cold Pkarr, Hot Noise"

This architecture separates key responsibilities:

| Key Type | Purpose | Storage | Rotation |
|----------|---------|---------|----------|
| **Ed25519 (pkarr)** | Identity, signatures | Ring (cold) | Rarely |
| **X25519 (noise)** | Session encryption | Bitkit (hot) | Per-session |

**Flow:**
1. Ring holds the master Ed25519 identity key ("cold")
2. Bitkit derives X25519 session keys via HKDF ("hot")
3. Noise channels use X25519 for encryption
4. Signatures for subscriptions use Ed25519 from Ring

### Data Flow: Payment Discovery

```mermaid
sequenceDiagram
    participant User as User (Bitkit)
    participant Paykit as Paykit FFI
    participant Ring as Pubky Ring
    participant HS as Pubky Homeserver

    User->>Ring: Request session
    Ring-->>User: Signed session token
    User->>Paykit: Initialize with session
    Paykit->>HS: Publish payment methods
    Note over HS: /pub/paykit.app/v0/{methodId}
    
    User->>Paykit: Discover peer methods
    Paykit->>HS: GET /pub/paykit.app/v0/
    HS-->>Paykit: Available methods
    Paykit-->>User: SupportedPayments
```

---

## 3. Prerequisites

### Development Environment

| Tool | Required Version | Purpose |
|------|------------------|---------|
| Rust | 1.70+ (via Rustup, NOT Homebrew) | Build paykit-rs |
| UniFFI | 0.25+ | Generate FFI bindings |
| Xcode | 14+ | iOS build |
| Swift | 5.5+ | iOS bindings |
| Android Studio | Latest | Android build |
| Kotlin | 1.8+ | Android bindings |
| Android NDK | r25+ | Native library compilation |

### ⚠️ CRITICAL: Rust Installation

**DO NOT use Homebrew Rust.** WASM targets and cross-compilation require Rustup.

```bash
# If you have Homebrew Rust, remove it first
brew uninstall rust

# Install Rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add targets
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android
rustup target add wasm32-unknown-unknown
```

### Repository Setup

Clone all required repositories:

```bash
mkdir -p ~/vibes-dev && cd ~/vibes-dev

# Core Paykit
git clone https://github.com/synonymdev/paykit-rs.git

# Mobile apps
git clone https://github.com/synonymdev/bitkit-ios.git
git clone https://github.com/synonymdev/bitkit-android.git

# Pubky ecosystem
git clone https://github.com/pubky/pubky-ring.git
git clone https://github.com/pubky/pubky-noise.git
git clone https://github.com/pubky/pubky-core.git
```

---

## 4. Building paykit-rs

### Step 1: Build the Core Library

```bash
cd ~/vibes-dev/paykit-rs

# Build release for current platform
cargo build --release -p paykit-mobile

# Verify build artifacts
ls -la target/release/libpaykit_mobile.*
# Should see: libpaykit_mobile.dylib (macOS) or .so (Linux)
```

### Step 2: Generate FFI Bindings

```bash
# Install uniffi-bindgen if not installed
cargo install uniffi-bindgen-cli@0.25

# Generate Swift bindings
uniffi-bindgen generate \
    --library target/release/libpaykit_mobile.dylib \
    -l swift \
    -o generated-bindings/swift/

# Generate Kotlin bindings
uniffi-bindgen generate \
    --library target/release/libpaykit_mobile.dylib \
    -l kotlin \
    -o generated-bindings/uniffi/
```

### Step 3: Build for iOS (All Architectures)

```bash
cd paykit-mobile

# Run the iOS build script
./build-ios.sh

# This creates:
# - target/aarch64-apple-ios/release/libpaykit_mobile.a
# - target/aarch64-apple-ios-sim/release/libpaykit_mobile.a
# - target/x86_64-apple-ios/release/libpaykit_mobile.a
```

### Step 4: Build for Android (All ABIs)

```bash
# Set NDK path
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/25.2.9519653

# Run the Android build script
./build-android.sh

# This creates libraries for each ABI:
# - jniLibs/arm64-v8a/libpaykit_mobile.so
# - jniLibs/armeabi-v7a/libpaykit_mobile.so
# - jniLibs/x86/libpaykit_mobile.so
# - jniLibs/x86_64/libpaykit_mobile.so
```

---

## 5. iOS Integration

### Step 1: Add Framework to Xcode

1. **Copy files to project:**
   ```
   Bitkit/
   └── PaykitIntegration/
       ├── FFI/
       │   ├── paykit_mobile.swift      # Generated bindings
       │   └── PaykitMobileFFI.h        # C header
       ├── Frameworks/
       │   └── libpaykit_mobile.a       # Static library
       └── Services/
           ├── PaykitManager.swift
           ├── DirectoryService.swift
           └── NoisePaymentService.swift
   ```

2. **Configure Xcode project:**
   - Add `PaykitIntegration` folder to project
   - Go to **Build Settings** → **Library Search Paths**:
     ```
     $(PROJECT_DIR)/Bitkit/PaykitIntegration/Frameworks
     ```
   - Go to **Build Settings** → **Other Linker Flags**:
     ```
     -lpaykit_mobile
     ```

3. **Add bridging header** (if not already present):
   ```c
   // Bitkit-Bridging-Header.h
   #import "PaykitMobileFFI.h"
   ```

### Step 2: Initialize PaykitManager

```swift
// In AppScene.swift or app initialization
import PaykitMobile

@main
struct BitkitApp: App {
    init() {
        // Initialize Paykit early
        Task {
            await PaykitManager.shared.initialize()
        }
    }
}

// PaykitManager.swift
@Observable
public class PaykitManager {
    public static let shared = PaykitManager()
    
    private var client: PaykitClient?
    public var isReady: Bool = false
    
    public func initialize() async {
        do {
            client = try PaykitClient()
            isReady = true
        } catch {
            Logger.error("Paykit init failed: \(error)")
        }
    }
}
```

### Step 3: Implement Transport Callbacks

The transport layer bridges Paykit to actual Pubky homeserver operations:

```swift
// DirectoryService.swift
class DirectoryTransportCallback: AuthenticatedTransportCallback {
    func upsertPaymentEndpoint(methodId: String, endpointData: String) async throws {
        // Call Pubky SDK to write to homeserver
        let path = "/pub/paykit.app/v0/\(methodId)"
        try await PubkySession.shared.put(path: path, data: endpointData.data(using: .utf8)!)
    }
    
    func removePaymentEndpoint(methodId: String) async throws {
        let path = "/pub/paykit.app/v0/\(methodId)"
        try await PubkySession.shared.delete(path: path)
    }
    
    func fetchPaymentEndpoint(methodId: String) async throws -> String? {
        let path = "/pub/paykit.app/v0/\(methodId)"
        guard let data = try await PubkySession.shared.get(path: path) else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }
}
```

### Step 4: Register Deep Links

In `Info.plist`, add URL schemes:

```xml
<key>CFBundleURLTypes</key>
<array>
    <dict>
        <key>CFBundleURLSchemes</key>
        <array>
            <string>bitkit</string>
            <string>paykit</string>
            <string>pubky</string>
        </array>
    </dict>
</array>
```

Handle in `AppDelegate` or `SceneDelegate`:

```swift
// In your SceneDelegate.swift or AppDelegate.swift
func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    guard let url = URLContexts.first?.url else { return }
    handleIncomingURL(url)
}

func application(_ app: UIApplication, 
                 open url: URL, 
                 options: [UIApplication.OpenURLOptionsKey : Any] = [:]) -> Bool {
    handleIncomingURL(url)
    return true
}

private func handleIncomingURL(_ url: URL) {
    // Handle different URL schemes
    switch url.scheme {
    case "pubky", "paykit":
        Task {
            await handlePaykitDeepLink(url)
        }
    case "bitkit":
        Task {
            await handleBitkitDeepLink(url)
        }
    default:
        break
    }
}

func handlePaykitDeepLink(_ url: URL) async {
    // Parse Pubky URI: pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo
    // or payment URI: paykit://8pinxxgqs41n4aidi.../pay?amount=50000&memo=coffee
    
    guard let host = url.host else {
        Logger.error("Invalid Paykit URL: no host")
        return
    }
    
    // Validate pubkey format (z-base-32 encoded Ed25519 key)
    guard isValidPubkey(host) else {
        Logger.error("Invalid pubkey format: \(host)")
        await MainActor.run {
            showError("Invalid Pubky ID")
        }
        return
    }
    
    do {
        // Discover available payment methods for this pubkey
        let methods = try await PaykitManager.shared.discoverMethods(pubkey: host)
        
        guard !methods.entries.isEmpty else {
            await MainActor.run {
                showError("No payment methods found for this user")
            }
            return
        }
        
        // Parse optional parameters
        let components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        let amount = components?.queryItems?
            .first(where: { $0.name == "amount" })?.value
            .flatMap { UInt64($0) }
        let memo = components?.queryItems?
            .first(where: { $0.name == "memo" })?.value
        
        // Navigate to smart checkout with pre-filled data
        await MainActor.run {
            NavigationViewModel.shared.navigate(
                to: .smartCheckout(
                    pubkey: host,
                    methods: methods,
                    amount: amount,
                    memo: memo
                )
            )
        }
    } catch {
        Logger.error("Failed to discover methods: \(error)")
        await MainActor.run {
            showError("Could not connect to payment provider")
        }
    }
}

private func isValidPubkey(_ pubkey: String) -> Bool {
    // z-base-32 encoded Ed25519 keys are 52 characters
    // Valid charset: ybndrfg8ejkmcpqxot1uwisza345h769
    let validCharset = CharacterSet(charactersIn: "ybndrfg8ejkmcpqxot1uwisza345h769")
    return pubkey.count == 52 && 
           pubkey.rangeOfCharacter(from: validCharset.inverted) == nil
}
```

### Step 5: Implement Keychain Storage

```swift
// PaykitKeychainStorage.swift
import Security

class PaykitKeychainStorage {
    private let service = "to.bitkit.paykit"
    
    func save(key: String, data: Data) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock
        ]
        
        SecItemDelete(query as CFDictionary)
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status)
        }
    }
    
    func load(key: String) throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else {
            throw KeychainError.loadFailed(status)
        }
        return result as? Data
    }
}
```

---

## 6. Android Integration

### Step 1: Add JNI Libraries

1. **Copy SO files:**
   ```
   app/src/main/jniLibs/
   ├── arm64-v8a/
   │   └── libpaykit_mobile.so
   ├── armeabi-v7a/
   │   └── libpaykit_mobile.so
   ├── x86/
   │   └── libpaykit_mobile.so
   └── x86_64/
       └── libpaykit_mobile.so
   ```

2. **Copy Kotlin bindings:**
   ```
   app/src/main/java/uniffi/paykit_mobile/
   └── paykit_mobile.kt
   ```

### Step 2: Configure Gradle

```kotlin
// app/build.gradle.kts
android {
    defaultConfig {
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
        }
    }
    
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

dependencies {
    implementation("net.java.dev.jna:jna:5.13.0@aar")
}
```

### Step 3: Initialize PaykitManager

```kotlin
// PaykitManager.kt
@Singleton
class PaykitManager @Inject constructor(
    @ApplicationContext private val context: Context
) {
    private var client: PaykitClient? = null
    
    val isReady: Boolean
        get() = client != null
    
    suspend fun initialize() = withContext(Dispatchers.IO) {
        try {
            // Load native library
            System.loadLibrary("paykit_mobile")
            client = PaykitClient()
        } catch (e: Exception) {
            Logger.error("Paykit init failed", e = e, context = TAG)
        }
    }
    
    companion object {
        private const val TAG = "PaykitManager"
    }
}
```

### Step 4: Implement Encrypted Storage

```kotlin
// PaykitSecureStorage.kt
class PaykitSecureStorage(context: Context) {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()
    
    private val prefs = EncryptedSharedPreferences.create(
        context,
        "paykit_secure_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )
    
    fun save(key: String, value: String) {
        prefs.edit().putString(key, value).apply()
    }
    
    fun load(key: String): String? {
        return prefs.getString(key, null)
    }
}
```

### Step 5: Register Deep Links

```xml
<!-- AndroidManifest.xml -->
<activity android:name=".ui.MainActivity">
    <intent-filter>
        <action android:name="android.intent.action.VIEW" />
        <category android:name="android.intent.category.DEFAULT" />
        <category android:name="android.intent.category.BROWSABLE" />
        <data android:scheme="bitkit" />
        <data android:scheme="paykit" />
        <data android:scheme="pubky" />
    </intent-filter>
</activity>
```

Handle in ViewModel:

```kotlin
// AppViewModel.kt
fun handleDeepLink(uri: Uri) {
    when (uri.scheme) {
        "pubky", "paykit" -> {
            val pubkey = uri.host ?: return
            viewModelScope.launch {
                val methods = paykitManager.discoverMethods(pubkey)
                _uiState.update { it.copy(smartCheckoutData = SmartCheckoutData(pubkey, methods)) }
            }
        }
    }
}
```

---

## 7. Pubky Ring Integration

### Overview

Pubky Ring is a separate app that manages identity keys. Bitkit communicates with Ring to:
1. Get the user's Pubky identity (Ed25519 public key)
2. Request signatures for subscriptions
3. Establish authenticated sessions with homeservers

### Cross-App Communication (iOS)

```swift
// PubkyRingBridge.swift
class PubkyRingBridge {
    static let shared = PubkyRingBridge()
    
    /// Request current identity from Ring
    func requestIdentity() async throws -> String {
        let url = URL(string: "pubkyring://identity/current")!
        
        // Use URL scheme to open Ring and get callback
        await UIApplication.shared.open(url)
        
        // Ring will callback with: bitkit://pubky-identity?pubkey=<52-char-z-base32>&timestamp=<unix-ts>
        // Handle in deep link handler (see Section 5, Step 4)
        // Store the callback expectation and return via continuation
        
        return try await withCheckedThrowingContinuation { continuation in
            // Set up callback expectation
            self.pendingIdentityCallback = continuation
            // Timeout after 30 seconds
            DispatchQueue.main.asyncAfter(deadline: .now() + 30) {
                if self.pendingIdentityCallback != nil {
                    self.pendingIdentityCallback = nil
                    continuation.resume(throwing: RingError.timeout)
                }
            }
        }
    }
    
    /// Request signature from Ring
    func requestSignature(message: Data) async throws -> Data {
        let messageB64 = message.base64EncodedString()
        let url = URL(string: "pubkyring://sign?message=\(messageB64)&callback=bitkit://signature")!
        
        await UIApplication.shared.open(url)
        // Ring will callback with: bitkit://signature?sig=<base64-signature>&pubkey=<signer-pubkey>
        // Signature is 64-byte Ed25519 signature, base64-encoded
        
        return try await withCheckedThrowingContinuation { continuation in
            self.pendingSignatureCallback = continuation
            DispatchQueue.main.asyncAfter(deadline: .now() + 30) {
                if self.pendingSignatureCallback != nil {
                    self.pendingSignatureCallback = nil
                    continuation.resume(throwing: RingError.timeout)
                }
            }
        }
    }
}
```

### Cross-App Communication (Android)

```kotlin
// PubkyRingBridge.kt
class PubkyRingBridge(private val context: Context) {
    
    fun requestIdentity(): String? {
        val intent = Intent().apply {
            action = "to.pubky.ring.ACTION_GET_IDENTITY"
            addCategory(Intent.CATEGORY_DEFAULT)
        }
        
        // Use startActivityForResult pattern
        // Ring returns: intent.getStringExtra("pubkey")
    }
    
    fun requestSignature(message: ByteArray): ByteArray? {
        val intent = Intent().apply {
            action = "to.pubky.ring.ACTION_SIGN"
            putExtra("message", message)
        }
        // Ring returns: intent.getByteArrayExtra("signature")
    }
}
```

### Session Management

```swift
// Bitkit requests a session from Ring
let session = try await PubkyRingBridge.shared.requestSession()

// Session structure contains three critical components:
// 1. pubkey: User's Ed25519 public key (z-base-32 encoded, 52 characters)
//    Example: "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
//    This is the user's Pubky identity - their globally unique identifier
//
// 2. sessionToken: Signed session token for homeserver authentication
//    Format: base64-encoded JSON containing:
//    {
//      "pubkey": "<user-pubkey>",
//      "homeserver": "https://homeserver.pubky.org",
//      "issued_at": 1703001234,
//      "expires_at": 1703087634,
//      "signature": "<ed25519-signature-of-above>"
//    }
//    The signature is created by Ring using the user's private key
//    Homeserver verifies this signature to authenticate requests
//
// 3. expiresAt: Session expiration timestamp (Unix epoch seconds)
//    Typically 24 hours from issuance
//    After expiration, must request new session from Ring

// Initialize transport with session
let transport = AuthenticatedTransportFfi.fromCallback(
    callback: DirectoryCallback(session: session)
)

// The transport uses the session token in the Authorization header:
// Authorization: Bearer <sessionToken>
// Homeserver validates the signature and checks expiration
```

---

## 8. Feature Implementation Guide

### 8.1 Payment Method Discovery

**Publishing your payment methods:**

```swift
// Publish onchain address
try await paykitClient.publishPaymentMethod(
    methodId: "onchain",
    endpoint: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
)

// Publish Lightning node with detailed endpoint
try await paykitClient.publishPaymentMethod(
    methodId: "lightning",
    endpoint: "03abc123def456789012345678901234567890123456789012345678901234@node.example.com:9735"
    // Format: <node_pubkey>@<host>:<port>
    // - node_pubkey: 66 hex character Lightning node public key
    // - host: Domain name or IP address
    // - port: Lightning P2P port (typically 9735)
)
```

**Discovering peer methods:**

```swift
let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
let methods = try await paykitClient.discoverMethods(pubkey: pubkey)

for method in methods.entries {
    print("Method: \(method.methodId) -> \(method.endpoint)")
}
```

### 8.2 Smart Checkout

```swift
// Get optimal payment method for amount
let result = try await paykitClient.smartCheckout(
    peerPubkey: pubkey,
    amountSats: 50000,
    strategy: .balanced  // or .cost, .speed, .privacy
)

switch result.method {
case "lightning":
    // Request invoice and pay via LDK
case "onchain":
    // Send to Bitcoin address
case "noise":
    // Establish Noise channel for private payment
}
```

### 8.3 Noise Protocol Payments

**Overview:** Noise Protocol provides end-to-end encrypted payment channels. The implementation uses Noise_IK (Interactive Knowledge) handshake pattern, which provides:
- Forward secrecy
- Identity hiding for the responder
- Mutual authentication
- Protection against replay attacks

**Complete Payment Flow:**

```swift
// Step 1: Establish encrypted channel
// The Noise_IK handshake requires:
// - Initiator (payer) knows responder's (payee) static public key
// - Responder doesn't know initiator's identity until after handshake
let channel = try await NoiseChannelManager.shared.connect(
    peerPubkey: pubkey,                    // Payee's X25519 static public key
    peerEndpoint: "wss://peer.example.com/noise"  // WebSocket endpoint for Noise transport
)

// Handshake sequence (automatic, shown for reference):
// 1. Initiator → Responder: e, es, s, ss
//    - e: Initiator's ephemeral public key
//    - es: DH(initiator_ephemeral, responder_static)
//    - s: Initiator's encrypted static public key
//    - ss: DH(initiator_static, responder_static)
// 2. Responder → Initiator: e, ee, se
//    - e: Responder's ephemeral public key
//    - ee: DH(responder_ephemeral, initiator_ephemeral)
//    - se: DH(responder_static, initiator_ephemeral)
// After these two messages, both parties have a shared secret

// Step 2: Request receipt (invoice)
// This is the first application-level message over the encrypted channel
let receipt = try await channel.requestReceipt(
    amount: 50000,              // Amount in satoshis
    currency: "SAT",            // Currency code (SAT, BTC, USD)
    memo: "Payment for services"  // Optional memo/description
)

// Receipt structure:
// {
//   "id": "<unique-receipt-id>",
//   "amount": 50000,
//   "currency": "SAT",
//   "invoice": "lnbc500n1...",  // Actual Lightning invoice or Bitcoin address
//   "created_at": 1703001234,
//   "expires_at": 1703001534,   // Typically 5 minutes
//   "memo": "Payment for services",
//   "payee_pubkey": "<payee-pubkey>",
//   "status": "pending"
// }

// Step 3: Validate receipt
guard receipt.expiresAt > Date().timeIntervalSince1970 else {
    throw PaymentError.receiptExpired
}
guard receipt.amount == 50000 else {
    throw PaymentError.amountMismatch
}

// Step 4: Execute payment via Lightning or onchain
// The actual payment happens OUTSIDE the Noise channel
// This maintains separation of concerns and allows different payment methods
let paymentResult: PaymentResult

switch detectPaymentType(receipt.invoice) {
case .lightning:
    // Pay Lightning invoice via LDK
    paymentResult = try await LightningService.shared.pay(
        invoice: receipt.invoice,
        amountMsat: receipt.amount * 1000  // Convert sats to millisats
    )
    // paymentResult contains: preimage, payment_hash, fee_msat, route
    
case .onchain:
    // Send to Bitcoin address
    paymentResult = try await BitcoinService.shared.send(
        address: receipt.invoice,
        amountSats: receipt.amount,
        feeRate: .medium
    )
    // paymentResult contains: txid, vsize, fee_sats, confirmations
}

// Step 5: Send payment confirmation over Noise channel
// This proves to the payee that we completed the payment
try await channel.confirmPayment(
    receiptId: receipt.id,
    txid: paymentResult.txid,           // Transaction ID or payment hash
    proof: paymentResult.proof,          // Payment proof (preimage for LN, tx for onchain)
    timestamp: Date().timeIntervalSince1970
)

// The payee verifies the proof and marks the receipt as "completed"
// Both parties now have cryptographic proof of the payment

// Step 6: Store receipt for records
try await ReceiptStore.shared.save(receipt, status: .completed, proof: paymentResult.proof)

// Step 7: Close channel (optional - can reuse for multiple payments)
try await channel.close()
```

**Error Handling:**

```swift
do {
    let result = try await executeNoisePayment(pubkey: pubkey, amount: amount)
} catch NoiseError.handshakeFailed(let reason) {
    // Handshake failure reasons:
    // - "invalid_static_key": Peer's pubkey doesn't match expected
    // - "timeout": Handshake didn't complete within 10 seconds
    // - "version_mismatch": Incompatible Noise protocol version
    Logger.error("Noise handshake failed: \(reason)")
    showError("Could not establish secure channel")
    
} catch NoiseError.channelClosed {
    // Channel closed by peer or network issue
    // Safe to retry - idempotent operations
    Logger.warn("Channel closed unexpectedly, retrying...")
    retry()
    
} catch PaymentError.receiptExpired {
    // Receipt expired before payment was executed
    // Request a new receipt
    Logger.warn("Receipt expired, requesting new receipt")
    requestNewReceipt()
    
} catch PaymentError.insufficientFunds(let needed, let available) {
    // Not enough balance
    Logger.error("Insufficient funds: need \(needed) sats, have \(available) sats")
    showError("Insufficient balance")
    
} catch {
    Logger.error("Payment failed: \(error)")
    showError("Payment failed")
}
```

### 8.4 Subscriptions

```swift
// Create subscription
let subscription = try await paykitClient.createSubscription(
    providerPubkey: providerPubkey,
    amount: 10000,
    currency: "SAT",
    frequency: .monthly,
    description: "Premium membership"
)

// Enable auto-pay for this subscription
try await paykitClient.enableAutoPay(
    subscriptionId: subscription.id,
    maxAmountSats: 15000,
    requireConfirmation: false
)
```

### 8.5 Spending Limits

```swift
// Set global daily limit
try await paykitClient.setGlobalDailyLimit(amountSats: 100000)

// Set per-peer limit
try await paykitClient.setPeerLimit(
    peerPubkey: merchantPubkey,
    amountSats: 50000,
    period: .weekly
)

// Check remaining limit before payment
let remaining = try await paykitClient.getRemainingLimit(peerPubkey: merchantPubkey)
if remaining >= paymentAmount {
    // Proceed with payment
}
```

---

## 9. Known Quirks & Footguns

### 9.1 Build Issues

#### ⚠️ Homebrew Rust vs Rustup

**Problem:** Homebrew Rust doesn't support cross-compilation targets.

**Symptom:**
```
Error: wasm32-unknown-unknown target not found in sysroot
```

**Solution:**
```bash
brew uninstall rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

#### ⚠️ WASM async_trait Send Bounds

**Problem:** `async_trait` requires `Send` by default, but WASM futures aren't `Send`.

**Symptom:**
```
future cannot be sent between threads safely
```

**Solution:** Use conditional compilation:
```rust
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait PrivateEndpointStore: Send + Sync {
    // ...
}
```

#### ⚠️ UniFFI Version Mismatch

**Problem:** Generated bindings must match the UniFFI version used to build.

**Symptom:**
```
uniffi checksum mismatch
```

**Solution:** Always regenerate bindings after updating UniFFI:
```bash
cargo install uniffi-bindgen-cli@0.25  # Match Cargo.toml version
./paykit-mobile/generate-bindings.sh
```

#### ⚠️ Android NDK Path

**Problem:** Build scripts can't find NDK.

**Solution:** Set environment variable:
```bash
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/25.2.9519653
```

Also create `local.properties` in Android project:
```properties
sdk.dir=/Users/YOUR_USER/Library/Android/sdk
```

### 9.2 Runtime Issues

#### ⚠️ Thread Safety with Noise Channels

**Problem:** Noise channels are not `Send` - cannot be used across threads.

**Solution:** Keep channel operations on the same thread/task:
```swift
// WRONG
Task.detached {
    await channel.send(message)  // May be different thread
}

// RIGHT
await withCheckedContinuation { continuation in
    channelQueue.async {
        channel.send(message)
        continuation.resume()
    }
}
```

#### ⚠️ Lock Poisoning Policy

**Problem:** Mutex poisoning after panic can cause cascading failures.

**Policy:** We use `lock().expect()` and accept panics on poison.

**Rationale:** If a thread panics while holding a lock, the data may be corrupt. Better to crash than corrupt payments.

See: `paykit-rs/docs/CONCURRENCY.md`

#### ⚠️ Never Call block_on() in Async Context

**Problem:** Calling `block_on()` from an async task deadlocks.

**Symptom:** App hangs indefinitely.

**Solution:** Use the FFI's async bridge:
```kotlin
// WRONG
runBlocking {
    paykitClient.discover(pubkey)
}

// RIGHT
viewModelScope.launch {
    paykitClient.discoverAsync(pubkey)
}
```

### 9.3 Platform-Specific Issues

#### iOS Keychain Entitlements

**Problem:** Keychain access fails without proper entitlements.

**Solution:** Add to `Bitkit.entitlements`:
```xml
<key>keychain-access-groups</key>
<array>
    <string>$(AppIdentifierPrefix)to.bitkit.paykit</string>
</array>
```

#### Android ProGuard Rules

**Problem:** ProGuard strips JNA classes.

**Solution:** Add to `proguard-rules.pro`:
```proguard
-keep class com.sun.jna.** { *; }
-keep class uniffi.paykit_mobile.** { *; }
```

#### Background Processing Limits

**Problem:** iOS kills background tasks after ~30 seconds.

**Solution:** Use `BGProcessingTask` for subscription checks:
```swift
BGTaskScheduler.shared.register(
    forTaskWithIdentifier: "to.bitkit.paykit.subscriptionCheck",
    using: nil
) { task in
    self.handleSubscriptionCheck(task as! BGProcessingTask)
}
```

---

## 10. Stubs & Mocks Inventory

### Components Still Using Mocks

| Component | Location | What's Mocked | Production Requirement |
|-----------|----------|---------------|------------------------|
| Directory Transport | `paykit-demo-web/src/directory.rs` | localStorage publishing | Real Pubky homeserver |
| Payment Execution | `paykit-lib/src/methods/onchain.rs` | Mock transaction result | Real Esplora/LND executor |
| Noise Transport | Demo apps | TCP/WebSocket | Real Noise over WS |
| Key Storage | `paykit-demo-cli` | Plaintext JSON | OS Keychain/Keystore |

### Mock APIs Available

```rust
// These are for testing ONLY - do not use in production

// Mock transport (no network calls)
let transport = AuthenticatedTransportFfi::new_mock();
assert!(transport.is_mock());  // Returns true

// Production transport
let transport = AuthenticatedTransportFfi::from_callback(callback);
assert!(!transport.is_mock());  // Returns false
```

### Production Transport Implementation

For production, you must implement the callback interface with real homeserver operations:

```swift
// iOS Production Implementation
class ProductionTransportCallback: AuthenticatedTransportCallback {
    private let session: PubkySession
    private let homeserverURL: String
    
    init(session: PubkySession, homeserverURL: String = "https://homeserver.pubky.org") {
        self.session = session
        self.homeserverURL = homeserverURL
    }
    
    // Write payment endpoint to homeserver
    func upsertPaymentEndpoint(methodId: String, endpointData: String) async throws {
        // Construct path: /pub/paykit.app/v0/{methodId}
        let path = "/pub/paykit.app/v0/\(methodId)"
        
        // Convert endpoint data to Data
        guard let data = endpointData.data(using: .utf8) else {
            throw TransportError.invalidData
        }
        
        // PUT request to homeserver with authentication
        let url = URL(string: "\(homeserverURL)\(path)")!
        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.httpBody = data
        request.addValue("Bearer \(session.token)", forHTTPHeaderField: "Authorization")
        request.addValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let (responseData, response) = try await URLSession.shared.data(for: request)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TransportError.invalidResponse
        }
        
        // Handle response codes
        switch httpResponse.statusCode {
        case 200, 201:
            // Success
            Logger.info("Published \(methodId) to \(path)")
        case 401:
            // Unauthorized - session expired or invalid
            throw TransportError.unauthorized
        case 403:
            // Forbidden - not allowed to write to this path
            throw TransportError.forbidden
        case 413:
            // Payload too large
            throw TransportError.payloadTooLarge
        case 429:
            // Rate limited
            throw TransportError.rateLimited
        case 500...599:
            // Server error
            throw TransportError.serverError(httpResponse.statusCode)
        default:
            throw TransportError.unexpectedStatus(httpResponse.statusCode)
        }
    }
    
    // Delete payment endpoint from homeserver
    func removePaymentEndpoint(methodId: String) async throws {
        let path = "/pub/paykit.app/v0/\(methodId)"
        let url = URL(string: "\(homeserverURL)\(path)")!
        
        var request = URLRequest(url: url)
        request.httpMethod = "DELETE"
        request.addValue("Bearer \(session.token)", forHTTPHeaderField: "Authorization")
        
        let (_, response) = try await URLSession.shared.data(for: request)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TransportError.invalidResponse
        }
        
        switch httpResponse.statusCode {
        case 200, 204:
            // Successfully deleted
            Logger.info("Removed \(methodId) from \(path)")
        case 404:
            // Already doesn't exist - this is OK
            Logger.debug("\(methodId) not found, already removed")
        case 401:
            throw TransportError.unauthorized
        default:
            throw TransportError.unexpectedStatus(httpResponse.statusCode)
        }
    }
    
    // Fetch single payment endpoint
    func fetchPaymentEndpoint(methodId: String) async throws -> String? {
        let path = "/pub/paykit.app/v0/\(methodId)"
        let url = URL(string: "\(homeserverURL)\(path)")!
        
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        // Note: GET requests don't require authentication for public data
        
        let (data, response) = try await URLSession.shared.data(for: request)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TransportError.invalidResponse
        }
        
        switch httpResponse.statusCode {
        case 200:
            // Found - return the data
            return String(data: data, encoding: .utf8)
        case 404:
            // Not found - this is OK, return nil
            return nil
        default:
            throw TransportError.unexpectedStatus(httpResponse.statusCode)
        }
    }
    
    // List all payment endpoints for a user
    func fetchSupportedPayments(pubkey: String) async throws -> SupportedPayments {
        let path = "/pub/paykit.app/v0/"
        let url = URL(string: "\(homeserverURL)/pub/\(pubkey)/paykit.app/v0/")!
        
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        
        let (data, response) = try await URLSession.shared.data(for: request)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TransportError.invalidResponse
        }
        
        switch httpResponse.statusCode {
        case 200:
            // Parse directory listing
            // Response format: array of filenames
            // ["onchain", "lightning", "lnurl"]
            let decoder = JSONDecoder()
            let methodIds = try decoder.decode([String].self, from: data)
            
            // Fetch each method's endpoint data
            var entries: [PaymentMethodEntry] = []
            for methodId in methodIds {
                if let endpointData = try await fetchPaymentEndpoint(methodId) {
                    entries.append(PaymentMethodEntry(
                        methodId: methodId,
                        endpoint: endpointData
                    ))
                }
            }
            
            return SupportedPayments(entries: entries)
            
        case 404:
            // User has no published methods
            return SupportedPayments(entries: [])
            
        default:
            throw TransportError.unexpectedStatus(httpResponse.statusCode)
        }
    }
}

// Error types
enum TransportError: Error {
    case invalidData
    case invalidResponse
    case unauthorized              // 401 - session expired
    case forbidden                 // 403 - not allowed
    case payloadTooLarge          // 413 - data too big
    case rateLimited              // 429 - too many requests
    case serverError(Int)         // 5xx - homeserver issue
    case unexpectedStatus(Int)    // Other status codes
}
```

### What Needs Real Implementation

| Feature | Demo Behavior | Production Need |
|---------|---------------|-----------------|
| `OnchainPlugin.execute()` | Returns mock txid | Connect to Esplora/electrum |
| `LightningPlugin.execute()` | Returns mock preimage | Connect to LND/CLN/LDK |
| `NoiseServerHelper` | In-memory | Persistent connection state |
| `FileStorage` | Plaintext JSON | Encrypted database |

---

## 11. Testing Requirements

### 11.1 Unit Tests

**Location:** Each crate's `tests/` directory

**Run all tests:**
```bash
cd paykit-rs
cargo test --all --all-features
```

**Key test files:**
- `paykit-lib/tests/methods_test.rs` - Payment method validation
- `paykit-subscriptions/tests/subscription_test.rs` - Subscription lifecycle
- `paykit-interactive/tests/protocol_test.rs` - Noise protocol messages

### 11.2 Integration Tests

**Run with network access:**
```bash
cargo test --features integration-tests -- --test-threads=1
```

**Disabled tests (need SDK update):**
- `pubky_sdk_compliance.rs` - Pubky SDK API changed

### 11.3 Mobile Tests

**iOS:**
```bash
cd bitkit-ios
xcodebuild test -scheme Bitkit -destination 'platform=iOS Simulator,name=iPhone 15'
```

**Android:**
```bash
cd bitkit-android
./gradlew testDevDebugUnitTest
./gradlew connectedDevDebugAndroidTest
```

### 11.4 Manual Test Checklist

Before release, manually verify:

- [ ] Create identity in Ring
- [ ] Import identity in Bitkit
- [ ] Publish payment methods
- [ ] Scan QR code for Pubky URI
- [ ] Smart checkout flow completes
- [ ] Lightning payment executes
- [ ] Onchain payment executes
- [ ] Create subscription
- [ ] Auto-pay triggers correctly
- [ ] Spending limit enforced
- [ ] Deep links work (all schemes)
- [ ] Background subscription check runs
- [ ] App recovers from network failure
- [ ] Keys persist across app restart

### 11.5 E2E Test Scenarios

```bash
# Start test environment
cd paykit-rs
./scripts/start-testnet.sh

# Run E2E tests
cargo test --features e2e-tests
```

---

## 12. Production Configuration

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `PAYKIT_HOMESERVER_URL` | Pubky homeserver URL | `https://homeserver.pubky.org` |
| `PAYKIT_LOG_LEVEL` | Logging verbosity | `info`, `debug`, `trace` |
| `PAYKIT_RATE_LIMIT_BURST` | Rate limit burst size | `10` |
| `PAYKIT_RATE_LIMIT_PERIOD_SECS` | Rate limit window | `60` |

### iOS Configuration

```swift
// Config.swift
struct PaykitConfig {
    static let homeserverURL = ProcessInfo.processInfo.environment["PAYKIT_HOMESERVER_URL"] 
        ?? "https://homeserver.pubky.org"
    
    static let rateLimitConfig = RateLimitConfig(
        maxHandshakesPerMinute: 10,
        maxHandshakesGlobal: 100
    )
}
```

### Android Configuration

```kotlin
// PaykitConfig.kt
object PaykitConfig {
    val homeserverUrl: String = BuildConfig.PAYKIT_HOMESERVER_URL
    
    val rateLimitConfig = RateLimitConfig(
        maxHandshakesPerMinute = 10,
        maxHandshakesGlobal = 100
    )
}
```

### Server Requirements

| Service | Purpose | Minimum Spec |
|---------|---------|--------------|
| Pubky Homeserver | Directory storage | 2 CPU, 4GB RAM |
| Lightning Node | Payment execution | 4 CPU, 8GB RAM |
| Bitcoin Node | Onchain payments | 8 CPU, 16GB RAM |

---

## 13. Security Checklist

### Cryptographic Requirements

- [x] Ed25519 for identity and signatures
- [x] X25519 for Noise key exchange
- [x] HKDF for key derivation
- [x] AES-256-GCM for storage encryption
- [x] Argon2 for password-based key derivation

### Key Storage

- [ ] iOS: Keys in Keychain with `kSecAttrAccessibleAfterFirstUnlock`
- [ ] Android: Keys in EncryptedSharedPreferences with hardware-backed keystore
- [ ] Never log keys or secrets
- [ ] Zeroize sensitive data after use

### Transport Security

- [ ] TLS 1.3 for all HTTP connections
- [ ] Certificate pinning for homeserver
- [ ] Noise_IK for payment channels
- [ ] No sensitive data in URLs

### Input Validation

- [ ] Validate all pubkeys are valid z-base-32
- [ ] Validate all amounts are positive
- [ ] Sanitize paths (no `..` traversal)
- [ ] Validate invoice expiration before payment

### Replay Protection

- [ ] Nonces stored in persistent database
- [ ] Nonce checked BEFORE signature verification
- [ ] Expired nonces cleaned up automatically
- [ ] Timestamps validated (not future-dated)

---

## 14. Troubleshooting

### Build Errors

**"Library not found for -lpaykit_mobile"**
- Check Library Search Paths in Xcode
- Verify `.a` file is in the correct location
- Run `cargo build --release -p paykit-mobile`

**"uniffi checksum mismatch"**
- Regenerate bindings with matching UniFFI version
- Delete derived data and rebuild

**"wasm32-unknown-unknown target not found"**
- Switch from Homebrew Rust to Rustup
- Run `rustup target add wasm32-unknown-unknown`

### Runtime Errors

**"Failed to load native library"**
- Check SO files are in correct jniLibs folders
- Verify ABI filters in build.gradle match
- Check ProGuard isn't stripping JNA

**"Keychain access denied"**
- Add keychain-access-groups entitlement
- Check app identifier prefix

**"Session expired"**
- Request new session from Ring
- Check system clock is accurate

### Network Errors

**"Homeserver unreachable"**
- Check network connectivity
- Verify homeserver URL is correct
- Check for certificate issues

**"Noise handshake failed"**
- Verify peer pubkey is correct
- Check rate limiting isn't triggered
- Ensure both sides support Noise_IK

---

## 15. Future Work

### Planned Features

| Feature | Priority | Status |
|---------|----------|--------|
| Hardware wallet signing | High | Not started |
| Multi-signature support | Medium | Design phase |
| LNURL integration | Medium | Planned |
| Bolt12 support | Medium | Planned |
| Desktop Electron app | Low | Not started |

### Known Limitations

1. **Single homeserver**: Currently only supports one homeserver per user
2. **No offline payments**: Requires network for all operations
3. **Manual key backup**: No automatic cloud backup
4. **Limited payment proofs**: Basic receipt, not cryptographic proof

### Upgrade Paths

**Pubky SDK Migration:**
When Pubky SDK updates, check:
- `PubkyClient` API changes
- Session management changes
- Homeserver protocol version

**UniFFI Updates:**
When updating UniFFI:
1. Update version in all `Cargo.toml`
2. Regenerate all bindings
3. Test on all platforms

---

## Appendices

### A. File Manifest

**paykit-rs files created/modified:**
```
paykit-mobile/
├── src/lib.rs                    # FFI exports
├── src/interactive_ffi.rs        # Noise protocol FFI
├── src/executor_ffi.rs           # Payment executor FFI
├── swift/                        # iOS storage adapters
└── kotlin/                       # Android storage adapters
```

**bitkit-ios files created:**
```
Bitkit/PaykitIntegration/
├── FFI/paykit_mobile.swift       # Generated bindings
├── Services/PaykitManager.swift
├── Services/DirectoryService.swift
├── Services/NoisePaymentService.swift
├── Storage/PaykitKeychainStorage.swift
└── Views/*.swift                 # UI components
```

**bitkit-android files created:**
```
app/src/main/java/
├── uniffi/paykit_mobile/         # Generated bindings
└── to/bitkit/paykit/
    ├── services/PaykitManager.kt
    ├── services/DirectoryService.kt
    ├── storage/PaykitSecureStorage.kt
    └── ui/screens/*.kt           # UI components
```

### B. Dependency Versions

| Dependency | Version | Notes |
|------------|---------|-------|
| Rust | 1.75+ | Via Rustup |
| UniFFI | 0.25.3 | Must match across all crates |
| Pubky SDK | 0.6.0-rc.6 | API breaking changes pending |
| pubky-noise | latest | Used for Noise protocol |
| LDK Node | 0.3.0 | Lightning payments |

### C. Glossary

| Term | Definition |
|------|------------|
| **Pkarr** | Public Key Addressable Resource Records - DNS-like system for pubkeys |
| **Pubky** | Public Key + Y (identity) - decentralized identity system |
| **Noise Protocol** | Cryptographic handshake framework for secure channels |
| **z-base-32** | Human-friendly encoding for Ed25519 public keys |
| **Homeserver** | Pubky server that stores user data |
| **FFI** | Foreign Function Interface - bridge between Rust and mobile |
| **UniFFI** | Mozilla's tool for generating FFI bindings |

---

*This guide was generated from the reference implementation in the BitcoinErrorLog repositories. For questions, open an issue in the relevant repository.*


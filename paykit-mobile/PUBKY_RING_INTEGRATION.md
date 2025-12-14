# Pubky Ring Integration Guide

This guide explains how Paykit mobile apps integrate with the Pubky Ring app for secure key derivation. Pubky Ring manages "cold" Ed25519 identity keys and derives "hot" X25519 Noise keys on-demand for Paykit apps.

## Overview

Paykit apps use a split-key architecture where:
- **Ed25519 (pkarr) keys**: Managed by Pubky Ring app (never stored in Paykit)
- **X25519 (Noise) keys**: Derived on-demand by Pubky Ring and cached locally in Paykit

This architecture provides:
- **Security**: Identity keys never leave the Pubky Ring app
- **Privacy**: Noise keys can be rotated without changing identity
- **Flexibility**: Multiple devices can derive unique keys from the same identity

## Integration Flow

```
┌─────────────────┐                    ┌──────────────────┐
│   Paykit App    │                    │   Pubky Ring      │
│                 │                    │                   │
│ 1. Need X25519  │                    │                   │
│    keypair      │                    │                   │
│                 │                    │                   │
│ 2. Request via  │ ──────────────────>│ 3. Derive from    │
│    URL/Intent   │                    │    Ed25519 seed   │
│                 │                    │                   │
│                 │ <──────────────────│ 4. Return X25519  │
│ 5. Cache key    │                    │    keypair        │
│    locally      │                    │                   │
└─────────────────┘                    └──────────────────┘
```

## iOS Integration

### URL Scheme Protocol

Paykit apps communicate with Pubky Ring using URL schemes:

**Request Format:**
```
pubkyring://derive-keypair?deviceId={deviceId}&epoch={epoch}&callback={callbackScheme}
```

**Parameters:**
- `deviceId`: Unique device identifier (e.g., UUID)
- `epoch`: Key rotation epoch (0 for initial keys)
- `callback`: Your app's URL scheme for receiving the response

**Response Format:**
```
{callbackScheme}://keypair-derived?secret_key_hex={hex}&public_key_hex={hex}
```

Or on error:
```
{callbackScheme}://keypair-error?error={errorCode}&message={message}
```

### Implementation Example

```swift
import UIKit

// Request key derivation
let deviceId = UIDevice.current.identifierForVendor?.uuidString ?? "unknown"
let epoch: UInt32 = 0
let callbackScheme = "paykitdemo"

let urlString = "pubkyring://derive-keypair?deviceId=\(deviceId)&epoch=\(epoch)&callback=\(callbackScheme)"
guard let url = URL(string: urlString) else { return }

if UIApplication.shared.canOpenURL(url) {
    UIApplication.shared.open(url)
} else {
    // Fallback to mock service
    useMockPubkyRingService()
}

// Handle callback in AppDelegate
func application(_ app: UIApplication, open url: URL, options: [UIApplication.OpenURLOptionsKey : Any] = [:]) -> Bool {
    if url.scheme == "paykitdemo" && url.host == "keypair-derived" {
        // Parse response
        let components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        let secretKeyHex = components?.queryItems?.first(where: { $0.name == "secret_key_hex" })?.value
        let publicKeyHex = components?.queryItems?.first(where: { $0.name == "public_key_hex" })?.value
        
        // Use the derived keypair
        if let secret = secretKeyHex, let publicKey = publicKeyHex {
            handleDerivedKeypair(secretKeyHex: secret, publicKeyHex: publicKey)
        }
        return true
    }
    return false
}
```

### URL Scheme Registration

Add to your `Info.plist`:

```xml
<key>CFBundleURLTypes</key>
<array>
    <dict>
        <key>CFBundleURLSchemes</key>
        <array>
            <string>paykitdemo</string>
        </array>
    </dict>
</array>
```

## Android Integration

### Intent Protocol

Paykit apps communicate with Pubky Ring using Android Intents:

**Intent Action:**
```
com.pubky.ring.DERIVE_KEYPAIR
```

**Intent Extras:**
- `deviceId`: String - Unique device identifier
- `epoch`: Int - Key rotation epoch (0 for initial keys)
- `callbackPackage`: String - Your app's package name
- `callbackActivity`: String - Activity to receive result

**Response:**
The result is returned via `Activity.onActivityResult()` with:
- `RESULT_OK` and Intent containing:
  - `secret_key_hex`: String
  - `public_key_hex`: String
- `RESULT_CANCELED` with error information

### Implementation Example

```kotlin
import android.content.Intent
import android.app.Activity

// Request key derivation
fun requestKeyDerivation(activity: Activity, deviceId: String, epoch: Int) {
    val intent = Intent("com.pubky.ring.DERIVE_KEYPAIR").apply {
        putExtra("deviceId", deviceId)
        putExtra("epoch", epoch)
        putExtra("callbackPackage", activity.packageName)
        putExtra("callbackActivity", "com.paykit.demo.MainActivity")
    }
    
    // Check if Pubky Ring is installed
    if (intent.resolveActivity(activity.packageManager) != null) {
        activity.startActivityForResult(intent, REQUEST_CODE_DERIVE_KEYPAIR)
    } else {
        // Fallback to mock service
        useMockPubkyRingService()
    }
}

// Handle result
override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
    if (requestCode == REQUEST_CODE_DERIVE_KEYPAIR) {
        if (resultCode == Activity.RESULT_OK && data != null) {
            val secretKeyHex = data.getStringExtra("secret_key_hex")
            val publicKeyHex = data.getStringExtra("public_key_hex")
            
            if (secretKeyHex != null && publicKeyHex != null) {
                handleDerivedKeypair(secretKeyHex, publicKeyHex)
            }
        } else {
            // Handle error
            val errorCode = data?.getStringExtra("error_code")
            val errorMessage = data?.getStringExtra("error_message")
            handleError(errorCode, errorMessage)
        }
    }
}
```

### Manifest Registration

Add intent filter to your `AndroidManifest.xml`:

```xml
<activity android:name=".MainActivity">
    <intent-filter>
        <action android:name="android.intent.action.MAIN" />
        <category android:name="android.intent.category.LAUNCHER" />
    </intent-filter>
    
    <!-- Handle Pubky Ring callbacks -->
    <intent-filter>
        <action android:name="com.pubky.ring.KEYPAIR_DERIVED" />
        <category android:name="android.intent.category.DEFAULT" />
    </intent-filter>
</activity>
```

## Error Handling

### Error Codes

| Code | Description |
|------|-------------|
| `app_not_installed` | Pubky Ring app is not installed |
| `request_failed` | Request to Pubky Ring failed |
| `invalid_response` | Invalid response format |
| `derivation_failed` | Key derivation failed |
| `service_unavailable` | Pubky Ring service unavailable |
| `timeout` | Request timed out |
| `user_cancelled` | User cancelled the request |

### Fallback Strategy

When Pubky Ring is unavailable, Paykit apps should fall back to `MockPubkyRingService`:

```swift
// iOS
if !canOpenPubkyRing() {
    let mockService = MockPubkyRingService.shared
    let keypair = try await mockService.getOrDeriveKeypair(deviceId: deviceId, epoch: epoch)
    // Use keypair
}
```

```kotlin
// Android
if (!isPubkyRingInstalled()) {
    val mockService = MockPubkyRingService.getInstance(context)
    val keypair = mockService.getOrDeriveKeypair(deviceId, epoch)
    // Use keypair
}
```

## Key Derivation Details

Pubky Ring uses HKDF-SHA512 to derive X25519 keys:

```
X25519_keypair = HKDF-SHA512(
    salt: "paykit-noise-derivation",
    ikm: Ed25519_seed,
    info: "device_id={deviceId}&epoch={epoch}",
    length: 32 bytes
)
```

The derived secret key is then used to generate the X25519 keypair using `x25519_pk_from_sk()`.

## Security Considerations

1. **Never store Ed25519 seeds**: Paykit apps should never request or store Ed25519 seeds
2. **Cache X25519 keys securely**: Use platform keychain/keystore for caching
3. **Rotate keys periodically**: Increment epoch to rotate keys
4. **Validate responses**: Always validate keypair format and length
5. **Handle timeouts**: Implement proper timeout handling for requests

## Testing

### With Mock Service

For development and testing, use `MockPubkyRingService`:

```swift
// iOS
let mockService = MockPubkyRingService.shared
let keypair = try await mockService.getOrDeriveKeypair(
    deviceId: "test-device",
    epoch: 0
)
```

```kotlin
// Android
val mockService = MockPubkyRingService.getInstance(context)
val keypair = mockService.getOrDeriveKeypair("test-device", 0)
```

### With Real Pubky Ring

1. Install Pubky Ring app on device
2. Set up identity in Pubky Ring
3. Test key derivation flow
4. Verify keys are cached correctly

## Troubleshooting

### Pubky Ring Not Responding

- Check if app is installed: `UIApplication.shared.canOpenURL()` (iOS) or `resolveActivity()` (Android)
- Verify URL scheme/Intent action is correct
- Check app permissions
- Review logs for error messages

### Invalid Keypair Response

- Verify response format matches specification
- Check hex encoding is correct (64 characters for 32-byte keys)
- Validate keypair using test vectors

### Timeout Issues

- Increase timeout value (default: 30 seconds)
- Check network connectivity
- Verify Pubky Ring app is running

## References

- [Pubky Ring Repository](https://github.com/pubky/pubky-ring)
- [Key Architecture Guide](./KEY_ARCHITECTURE.md)
- [Noise Protocol Specification](https://noiseprotocol.org/)


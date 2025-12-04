# Bitkit Integration Guide

This guide covers integrating Paykit with pubky-noise v0.8.0 into Bitkit for production payments.

> **Implementation Status**: The FFI layer is complete. `FfiRawNoiseManager` and all pkarr helper
> functions (`ffiSignPkarrKeyBinding`, `ffiFormatX25519ForPkarr`, `ffiDeriveX25519Static`, etc.)
> are available via UniFFI bindings for Swift (iOS) and Kotlin (Android).

## Overview

Bitkit is a self-custodial Bitcoin and Lightning wallet that uses:
- **BIP-39 seed phrase** as the root of all key derivation
- **Ed25519 identity** derived from the seed for Pubky
- **X25519 session keys** derived from Ed25519 for Noise encryption

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        BITKIT APP                               │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    UI Layer (React Native)                │  │
│  └───────────────────────────┬───────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │                   Business Logic                          │  │
│  │  - Payment flows                                          │  │
│  │  - Contact management                                     │  │
│  │  - Directory sync                                         │  │
│  └───────────────────────────┬───────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │              Native Bridge (iOS/Android)                  │  │
│  └───────────────────────────┬───────────────────────────────┘  │
│                              │                                  │
└──────────────────────────────┼──────────────────────────────────┘
                               │
┌──────────────────────────────┼──────────────────────────────────┐
│                              │   NATIVE LIBRARIES               │
│                              │                                  │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │                  pubky-noise (UniFFI)                     │  │
│  │  FfiRawNoiseManager                                       │  │
│  │  - initiateIkRaw()                                        │  │
│  │  - acceptIkRaw()                                          │  │
│  │  - encrypt() / decrypt()                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  pkarr-ffi (separate)                     │  │
│  │  - publishNoiseKey()                                      │  │
│  │  - lookupNoiseKey()                                       │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Key Derivation Path

```
BIP-39 Seed Phrase
        │
        ▼
┌───────────────────────────────────────┐
│  BIP-32 Master Key                    │
│  m/                                   │
└───────────────────────┬───────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│  m/84'/0'/0'  │ │  m/44'/0'/0'  │ │  m/9178'/0'   │
│  Bitcoin      │ │  Legacy BTC   │ │  Pubky ID     │
│  (Native SegWit) │ (P2PKH)      │ │  (Ed25519)    │
└───────────────┘ └───────────────┘ └───────┬───────┘
                                            │
                                            ▼
                                  ┌───────────────────┐
                                  │  HKDF-SHA512      │
                                  │  context: device  │
                                  └─────────┬─────────┘
                                            │
                                            ▼
                                  ┌───────────────────┐
                                  │  X25519 Keypair   │
                                  │  (Noise sessions) │
                                  └───────────────────┘
```

## One-Time Setup (Cold Signing)

This happens once when the user sets up Bitkit with their seed phrase:

### 1. Derive Ed25519 Identity

```typescript
// React Native (TypeScript)
import { derivePubkyIdentity } from 'bitkit-core';

const ed25519Keypair = derivePubkyIdentity(seedPhrase);
// ed25519Keypair.publicKey - 32 bytes
// ed25519Keypair.secretKey - 32 bytes
```

### 2. Derive X25519 Session Key

```typescript
import { PubkyNoise } from 'pubky-noise-native';

const deviceId = getUniqueDeviceId(); // e.g., "bitkit-ios-abc123"
const x25519SecretKey = PubkyNoise.ffiDeriveX25519Static(
  ed25519Keypair.secretKey,
  deviceId
);
const x25519PublicKey = PubkyNoise.ffiX25519PublicKey(x25519SecretKey);

// Store x25519SecretKey in Keychain/Keystore
await SecureStorage.set('x25519_sk', x25519SecretKey);
```

### 3. Sign and Publish to pkarr

```typescript
// Sign the X25519 key binding with Ed25519 (FFI function)
const signature = PubkyNoise.ffiSignPkarrKeyBinding(
  ed25519Keypair.secretKey,
  x25519PublicKey,
  deviceId
);

// Format for pkarr publication (FFI function)
const txtRecord = PubkyNoise.ffiFormatX25519ForPkarr(x25519PublicKey, signature);

// Get the subdomain for the Noise key
const subdomain = PubkyNoise.ffiPkarrNoiseSubdomain(deviceId);
// Returns "_noise.{deviceId}"

// Publish to pubky storage at /pub/noise.app/v0/{deviceId}
await pubkySession.put(`/pub/noise.app/v0/${deviceId}`, txtRecord);

// Ed25519 secret key can now be stored cold or derived on-demand from seed
```

**Available FFI Functions for pkarr:**
- `ffiSignPkarrKeyBinding(ed25519Sk, x25519Pk, deviceId)` → signature
- `ffiFormatX25519ForPkarr(x25519Pk, signature?)` → TXT record string
- `ffiParseX25519FromPkarr(txtRecord)` → x25519Pk
- `ffiParseAndVerifyPkarrKey(txtRecord, ed25519Pk, deviceId)` → verified x25519Pk
- `ffiVerifyPkarrKeyBinding(ed25519Pk, x25519Pk, signature, deviceId)` → boolean
- `ffiPkarrNoiseSubdomain(deviceId)` → subdomain string
- `ffiCreatePkarrBindingMessage(ed25519Pk, x25519Pk, deviceId)` → binding message

## Runtime Payment Flow

### Receiving a Payment

```typescript
// 1. Start listening for connections
const manager = new PubkyNoise.FfiRawNoiseManager(config);
const x25519SecretKey = await SecureStorage.get('x25519_sk');

server.onConnection(async (firstMessage) => {
  // 2. Accept IK-raw connection
  const result = manager.acceptIkRaw(x25519SecretKey, firstMessage);
  
  // 3. Send response
  await socket.send(result.response);
  
  // 4. Session is now established
  const sessionId = result.sessionId;
  const clientPublicKey = result.clientStaticPk; // For identity verification
  
  // 5. Receive encrypted payment request
  const encryptedRequest = await socket.receive();
  const paymentRequest = manager.decrypt(sessionId, encryptedRequest);
  
  // 6. Process payment and respond
  const invoice = await generateInvoice(paymentRequest);
  const encryptedInvoice = manager.encrypt(sessionId, invoice);
  await socket.send(encryptedInvoice);
});
```

### Sending a Payment

```typescript
// 1. Lookup recipient's X25519 key via pkarr
const recipientPubky = "pk:abc123...";
const recipientNoiseKey = await pkarr.lookupNoiseKey(recipientPubky);

// 2. Initialize connection
const manager = new PubkyNoise.FfiRawNoiseManager(config);
const x25519SecretKey = await SecureStorage.get('x25519_sk');

// 3. Initiate IK-raw handshake
const result = manager.initiateIkRaw(x25519SecretKey, recipientNoiseKey);

// 4. Send first message
await socket.send(result.message);

// 5. Receive and complete handshake
const response = await socket.receive();
manager.completeHandshake(result.sessionId, response);

// 6. Send encrypted payment request
const paymentRequest = { amount: 1000, memo: "Coffee" };
const encrypted = manager.encrypt(result.sessionId, JSON.stringify(paymentRequest));
await socket.send(encrypted);

// 7. Receive invoice
const encryptedInvoice = await socket.receive();
const invoice = manager.decrypt(result.sessionId, encryptedInvoice);

// 8. Pay the invoice
await lightning.payInvoice(invoice);
```

## React Native Bridge

### iOS (Swift)

```swift
@objc(PubkyNoiseModule)
class PubkyNoiseModule: NSObject {
    private var manager: FfiRawNoiseManager?
    
    @objc
    func initialize(_ config: NSDictionary, resolver: @escaping RCTPromiseResolveBlock, rejecter: @escaping RCTPromiseRejectBlock) {
        let ffiConfig = FfiMobileConfig(
            autoReconnect: config["autoReconnect"] as? Bool ?? true,
            maxReconnectAttempts: UInt32(config["maxReconnectAttempts"] as? Int ?? 3),
            reconnectDelayMs: UInt64(config["reconnectDelayMs"] as? Int ?? 1000),
            batterySaver: config["batterySaver"] as? Bool ?? true,
            chunkSize: UInt64(config["chunkSize"] as? Int ?? 65535)
        )
        
        manager = FfiRawNoiseManager(config: ffiConfig)
        resolver(nil)
    }
    
    @objc
    func deriveX25519Static(_ seed: String, context: String, resolver: @escaping RCTPromiseResolveBlock, rejecter: @escaping RCTPromiseRejectBlock) {
        do {
            let seedBytes = Data(base64Encoded: seed)!
            let contextBytes = context.data(using: .utf8)!
            
            let secretKey = try ffiDeriveX25519Static(
                seed: Array(seedBytes),
                context: Array(contextBytes)
            )
            
            resolver(Data(secretKey).base64EncodedString())
        } catch {
            rejecter("DERIVE_ERROR", error.localizedDescription, error)
        }
    }
    
    // ... more methods
}
```

### Android (Kotlin)

```kotlin
class PubkyNoiseModule(reactContext: ReactApplicationContext) : ReactContextBaseJavaModule(reactContext) {
    private var manager: FfiRawNoiseManager? = null
    
    override fun getName() = "PubkyNoise"
    
    @ReactMethod
    fun initialize(config: ReadableMap, promise: Promise) {
        val ffiConfig = FfiMobileConfig(
            autoReconnect = config.getBoolean("autoReconnect"),
            maxReconnectAttempts = config.getInt("maxReconnectAttempts").toUInt(),
            reconnectDelayMs = config.getInt("reconnectDelayMs").toULong(),
            batterySaver = config.getBoolean("batterySaver"),
            chunkSize = config.getInt("chunkSize").toULong()
        )
        
        manager = FfiRawNoiseManager(ffiConfig)
        promise.resolve(null)
    }
    
    @ReactMethod
    fun deriveX25519Static(seedBase64: String, context: String, promise: Promise) {
        try {
            val seedBytes = Base64.decode(seedBase64, Base64.DEFAULT)
            val contextBytes = context.toByteArray(Charsets.UTF_8)
            
            val secretKey = ffiDeriveX25519Static(
                seedBytes.toList().map { it.toUByte() },
                contextBytes.toList().map { it.toUByte() }
            )
            
            val result = secretKey.map { it.toByte() }.toByteArray()
            promise.resolve(Base64.encodeToString(result, Base64.DEFAULT))
        } catch (e: Exception) {
            promise.reject("DERIVE_ERROR", e.message, e)
        }
    }
    
    // ... more methods
}
```

## Security Considerations

### Key Storage

| Platform | Storage | Protection |
|----------|---------|------------|
| iOS | Keychain | SecureEnclave when available |
| Android | Keystore | Hardware-backed when available |

### Memory Safety

- All Noise keys are `Zeroizing<[u8; 32]>` in Rust
- React Native: Clear byte arrays after use
- Avoid caching keys in JS state

### Network Security

- Always use TLS for socket connections
- Verify pkarr signatures before using keys
- Implement connection timeouts

## Testing

### Mock Mode

```typescript
// For development without network
const mockTransport = new MockNoiseTransport();
mockTransport.setRecipientKey(testRecipientKey);
```

### Test Vectors

```typescript
const TEST_VECTORS = {
  ed25519Seed: 'base64...',
  deviceId: 'test-device',
  expectedX25519Pk: 'base64...',
  expectedSignature: 'base64...',
};

test('key derivation matches test vector', () => {
  const sk = PubkyNoise.ffiDeriveX25519Static(
    TEST_VECTORS.ed25519Seed,
    TEST_VECTORS.deviceId
  );
  const pk = PubkyNoise.ffiX25519PublicKey(sk);
  expect(pk).toEqual(TEST_VECTORS.expectedX25519Pk);
});
```

## Pattern Selection for Bitkit

| Scenario | Pattern | When |
|----------|---------|------|
| Known contact | IK-raw | Recipient's X25519 key in pkarr |
| First contact | XX | No key available, learn during handshake |
| Anonymous donation | N | Client anonymity required |

### Using XX for First Contact

When contacting a new recipient for the first time and their key isn't in pkarr:

```typescript
// 1. Derive X25519 for this session
const x25519SecretKey = await SecureStorage.get('x25519_sk');

// 2. Connect with XX pattern (learns server's key during handshake)
const result = manager.initiateXx(x25519SecretKey);
await socket.send(result.message);

const response1 = await socket.receive();
const response2 = manager.processXxResponse(result.sessionId, response1);
await socket.send(response2);

// 3. Session established - extract server's static key
const serverStaticPk = manager.getRemoteStatic(result.sessionId);

// 4. Cache server's key for future IK-raw connections
await ContactStorage.saveNoiseKey(recipientPubky, serverStaticPk);

// 5. Next time, use IK-raw with cached key
```

## Migration from v0.7.0

If upgrading from pubky-noise v0.7.0:

1. **Remove epoch parameter** - No longer needed
2. **Use new pattern types** - `IKRaw` instead of `IK` for cold keys
3. **Update handshake calls** - Return types are now 2-tuples
4. **Update key derivation** - Use `derive_x25519_static` instead of `derive_x25519_for_device_epoch`

## Troubleshooting

### Connection Timeouts

- Check network connectivity
- Verify pkarr is accessible
- Ensure recipient is online

### Handshake Failures

- Verify key derivation path
- Check device ID consistency
- Ensure pkarr record is published

### Decryption Errors

- Session may have expired
- Counter mismatch (session corruption)
- Wrong session ID

## Support

- GitHub: https://github.com/synonymdev/paykit-rs
- Docs: See `pubky-noise/docs/COLD_KEY_ARCHITECTURE.md`


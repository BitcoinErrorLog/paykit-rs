# Noise Payment Flow Guide

This guide explains the complete payment flow when sending and receiving payments
over encrypted Noise protocol channels in Paykit mobile apps.

## Overview

Paykit uses the Noise protocol (specifically Noise_IK) to establish encrypted
peer-to-peer channels for payment negotiation. This provides:

- **End-to-end encryption**: All payment data is encrypted
- **Authentication**: Both parties verify each other's identity
- **Forward secrecy**: Session keys are ephemeral
- **Privacy**: Payment details are never exposed to third parties

## Sending a Payment

### Flow Diagram

```
┌──────────────┐                              ┌──────────────┐
│    Payer     │                              │    Payee     │
│   (Client)   │                              │   (Server)   │
└──────────────┘                              └──────────────┘
       │                                             │
       │  1. Discover Noise Endpoint                 │
       │────────────────────────────────────────────>│
       │                                             │
       │  2. TCP Connect                             │
       │────────────────────────────────────────────>│
       │                                             │
       │  3. Noise IK Handshake (Message 1)          │
       │────────────────────────────────────────────>│
       │                                             │
       │  4. Noise IK Handshake (Message 2)          │
       │<────────────────────────────────────────────│
       │                                             │
       │  5. [Encrypted] Receipt Request             │
       │────────────────────────────────────────────>│
       │                                             │
       │  6. [Encrypted] Receipt Confirmation        │
       │<────────────────────────────────────────────│
       │                                             │
       │  7. Close Connection                        │
       │────────────────────────────────────────────>│
       │                                             │
```

### Step-by-Step

#### 1. Key Derivation

Before connecting, the payer derives their X25519 keys:

```swift
// iOS
let keypair = try await pubkyRing.getOrDeriveKeypair(
    deviceId: deviceId,
    epoch: currentEpoch
)
// Cache keys locally
keyCache.storeKey(deviceId: deviceId, epoch: epoch, keypair: keypair)
```

```kotlin
// Android
val keypair = pubkyRing.getOrDeriveKeypair(
    deviceId = deviceId,
    epoch = currentEpoch
)
// Cache keys locally
keyCache.storeKey(deviceId, epoch, keypair)
```

#### 2. Endpoint Discovery

Discover the payee's Noise endpoint from the Pubky directory:

```swift
// iOS
let endpoint = try await directoryService.discoverNoiseEndpoint(
    recipientPubkey: payeePubkey
)
// endpoint.host, endpoint.port, endpoint.serverPubkeyHex
```

```kotlin
// Android
val endpoint = directoryService.discoverNoiseEndpoint(
    recipientPubkey = payeePubkey
)
// endpoint.host, endpoint.port, endpoint.serverPubkeyHex
```

#### 3. Establish Connection

Connect via TCP and perform Noise handshake:

```swift
// iOS (using NWConnection)
let connection = NWConnection(
    host: NWEndpoint.Host(endpoint.host),
    port: NWEndpoint.Port(integerLiteral: endpoint.port),
    using: .tcp
)
connection.start(queue: .global())

// Perform Noise IK handshake
let noiseManager = FfiNoiseManager.newClient(
    config: config,
    clientSeed: seedBytes,
    clientKid: "paykit-ios",
    deviceId: deviceIdBytes
)

let initResult = noiseManager.initiateConnection(
    serverPk: endpoint.serverPubkeyBytes,
    hint: nil
)

// Send handshake message 1
send(data: initResult.firstMessage)

// Receive handshake message 2
let response = receive()

// Complete handshake
let sessionId = noiseManager.completeConnection(
    sessionId: initResult.sessionId,
    serverResponse: response
)
```

#### 4. Send Payment Request

Create and send encrypted payment request:

```swift
// iOS
let requestMessage = try createReceiptRequestMessage(
    receiptId: "rcpt_\(UUID().uuidString)",
    payerPubkey: myPubkey,
    payeePubkey: payeePubkey,
    methodId: "lightning",
    amount: "1000",
    currency: "SAT"
)

let plaintext = requestMessage.payloadJson.data(using: .utf8)!
let ciphertext = noiseManager.encrypt(sessionId: sessionId, plaintext: plaintext)
send(data: ciphertext)
```

#### 5. Receive Confirmation

Receive and decrypt the confirmation:

```swift
// iOS
let responseCiphertext = receive()
let responsePlaintext = noiseManager.decrypt(
    sessionId: sessionId,
    ciphertext: responseCiphertext
)

let response = try parsePaymentMessage(
    jsonString: String(data: responsePlaintext, encoding: .utf8)!
)

switch response.messageType {
case .receiptConfirmation:
    // Payment successful! Store receipt
    receiptStorage.saveReceipt(receipt)
case .error:
    // Handle error
    throw PaymentError.rejected(response.errorMessage)
default:
    throw PaymentError.unexpectedResponse
}
```

#### 6. Cleanup

Close connection and cleanup session:

```swift
// iOS
noiseManager.removeSession(sessionId)
connection.cancel()
```

## Receiving a Payment

### Flow Diagram

```
┌──────────────┐                              ┌──────────────┐
│    Payee     │                              │    Payer     │
│   (Server)   │                              │   (Client)   │
└──────────────┘                              └──────────────┘
       │                                             │
       │  1. Start Listening                         │
       │                                             │
       │  2. Publish Noise Endpoint                  │
       │                                             │
       │  3. Accept TCP Connection                   │
       │<────────────────────────────────────────────│
       │                                             │
       │  4. Noise IK Handshake (Server Side)        │
       │<───────────────────────────────────────────>│
       │                                             │
       │  5. [Encrypted] Receive Request             │
       │<────────────────────────────────────────────│
       │                                             │
       │  6. Validate & Process Request              │
       │                                             │
       │  7. [Encrypted] Send Confirmation           │
       │────────────────────────────────────────────>│
       │                                             │
```

### Step-by-Step

#### 1. Start Server

```swift
// iOS
let listener = try NWListener(using: .tcp, on: .any)
listener.stateUpdateHandler = { state in
    if case .ready = state {
        print("Listening on port: \(listener.port?.rawValue ?? 0)")
    }
}
listener.start(queue: .global())
```

#### 2. Publish Endpoint

Publish your Noise endpoint to the directory:

```swift
// iOS
try await directoryService.publishNoiseEndpoint(
    host: getLocalIPAddress(),
    port: listener.port!.rawValue,
    noisePubkey: myNoisePubkeyHex,
    metadata: "Mobile wallet"
)
```

#### 3. Accept Connection

```swift
// iOS
listener.newConnectionHandler = { connection in
    connection.start(queue: .global())
    handleIncomingConnection(connection)
}
```

#### 4. Handle Request

```swift
// iOS
func handleIncomingConnection(_ connection: NWConnection) {
    // Receive handshake message 1
    let handshakeData = receive(connection)
    
    // Process handshake (server side)
    let (hsState, identity) = noiseServer.buildResponderReadIk(handshakeData)
    
    // Send handshake message 2
    let response = hsState.writeMessage(payload: [])
    send(connection, data: response)
    
    // Complete handshake
    let link = serverCompleteIk(hsState)
    
    // Receive encrypted request
    let requestCiphertext = receive(connection)
    let requestPlaintext = link.decrypt(requestCiphertext)
    
    let request = try parsePaymentMessage(
        jsonString: String(data: requestPlaintext, encoding: .utf8)!
    )
    
    // Process request and send confirmation
    let confirmation = try createReceiptConfirmationMessage(
        receiptId: request.receiptId,
        payerPubkey: request.payerPubkey,
        payeePubkey: myPubkey,
        methodId: request.methodId,
        amount: request.amount,
        currency: request.currency,
        notes: nil
    )
    
    let confirmCiphertext = link.encrypt(
        confirmation.payloadJson.data(using: .utf8)!
    )
    send(connection, data: confirmCiphertext)
    
    // Store receipt
    receiptStorage.saveReceipt(receipt)
}
```

## Message Types

### Receipt Request

Sent by payer to initiate payment:

```json
{
  "type": "request_receipt",
  "receipt_id": "rcpt_abc123",
  "payer": "pk_payer_z32",
  "payee": "pk_payee_z32",
  "method_id": "lightning",
  "amount": "1000",
  "currency": "SAT",
  "created_at": 1702500000
}
```

### Receipt Confirmation

Sent by payee to confirm payment:

```json
{
  "type": "confirm_receipt",
  "receipt_id": "rcpt_abc123",
  "payer": "pk_payer_z32",
  "payee": "pk_payee_z32",
  "method_id": "lightning",
  "amount": "1000",
  "currency": "SAT",
  "confirmed_at": 1702500005,
  "notes": null
}
```

### Private Endpoint Offer

Sent by payee to provide a fresh payment address:

```json
{
  "type": "private_endpoint_offer",
  "method_id": "lightning",
  "endpoint": "lnbc1000n1pj...",
  "expires_at": 1702500600
}
```

### Error

Sent when a request cannot be processed:

```json
{
  "type": "error",
  "code": "insufficient_funds",
  "message": "Payment amount exceeds available balance"
}
```

## Error Handling

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `EndpointNotFound` | Recipient has no published endpoint | Ensure recipient is online |
| `ConnectionFailed` | Cannot reach host | Check network, firewall |
| `HandshakeFailed` | Key mismatch or protocol error | Verify pubkeys |
| `Timeout` | No response within timeout | Increase timeout, retry |
| `InvalidResponse` | Unexpected message format | Check protocol version |

### Retry Strategy

```swift
// iOS
func sendPaymentWithRetry(maxRetries: Int = 3) async throws -> Receipt {
    var lastError: Error?
    
    for attempt in 1...maxRetries {
        do {
            return try await sendPayment()
        } catch NoisePaymentError.connectionFailed(_),
                NoisePaymentError.timeout {
            lastError = error
            // Exponential backoff
            try await Task.sleep(nanoseconds: UInt64(pow(2.0, Double(attempt))) * 1_000_000_000)
            continue
        } catch {
            throw error // Don't retry other errors
        }
    }
    
    throw lastError!
}
```

## Security Considerations

### Key Storage

- **X25519 private keys**: Stored in platform secure storage (Keychain/EncryptedSharedPreferences)
- **Session keys**: Ephemeral, never persisted
- **Ed25519 seeds**: Managed by Pubky Ring (not in Paykit app)

### Channel Security

- **Forward secrecy**: Each session uses fresh ephemeral keys
- **Replay protection**: Nonce counters prevent replay attacks
- **Tampering detection**: AEAD encryption detects modifications

### Best Practices

1. **Always validate pubkeys** before connecting
2. **Use short timeouts** to detect dead connections
3. **Clear session keys** immediately after use
4. **Don't log plaintext** payment data
5. **Rotate X25519 keys** periodically (epoch increment)

## Related Documentation

- [KEY_ARCHITECTURE.md](./KEY_ARCHITECTURE.md) - Key management architecture
- [TESTING_GUIDE.md](./TESTING_GUIDE.md) - Testing documentation
- [NOISE_INTEGRATION_GUIDE.md](./NOISE_INTEGRATION_GUIDE.md) - Integration overview


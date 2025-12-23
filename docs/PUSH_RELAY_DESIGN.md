# Private Push Relay Service Design

## Overview

This document describes the design for a private push relay service that handles wake notifications for Paykit/Noise connections without exposing push tokens publicly.

## Problem Statement

The current implementation publishes APNs/FCM device tokens to the public homeserver directory. This creates two security/privacy issues:

1. **DoS Risk**: Anyone can read the push token and spam the device with notifications
2. **Metadata Leak**: Links devices to pubkeys, enabling tracking/surveillance

## Solution: Private Push Relay

A relay service that:
1. Stores push tokens server-side (never published publicly)
2. Accepts authorized wake requests from peers
3. Validates requests before forwarding to APNs/FCM
4. Rate-limits to prevent abuse

## Architecture

```
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│   Sender App    │         │   Push Relay    │         │  Recipient App  │
│   (Bitkit)      │         │   Service       │         │   (Bitkit)      │
└────────┬────────┘         └────────┬────────┘         └────────┬────────┘
         │                           │                           │
         │                           │  1. Register              │
         │                           │◀─────────────────────────│
         │                           │  (pubkey, token, proof)   │
         │                           │                           │
         │  2. Wake Request          │                           │
         │  (recipient_pk, nonce)    │                           │
         │─────────────────────────▶│                           │
         │                           │                           │
         │                           │  3. Push Notification     │
         │                           │─────────────────────────▶│
         │                           │  (via APNs/FCM)           │
         │                           │                           │
```

## API Specification

### Base URL

Production: `https://push.paykit.app/v1`
Staging: `https://push-staging.paykit.app/v1`

### Authentication

All endpoints require Ed25519 signature authentication:

```
X-Pubky-Signature: <signature_hex>
X-Pubky-Timestamp: <unix_timestamp>
X-Pubky-Pubkey: <pubkey_z32>
```

The signature is over: `<method>:<path>:<timestamp>:<body_hash>`

### Endpoints

#### POST /register

Register or update a push token for a pubkey.

**Request:**
```json
{
  "platform": "ios" | "android",
  "token": "<apns_or_fcm_token>",
  "capabilities": ["wake", "payment_received"],
  "device_id": "<optional_device_identifier>"
}
```

**Response:**
```json
{
  "status": "registered",
  "relay_id": "<unique_relay_identifier>",
  "expires_at": 1735689600
}
```

**Notes:**
- Token is stored encrypted at rest
- Registration must be renewed periodically (30 days)
- One pubkey can register multiple devices

#### DELETE /register

Unregister a push token.

**Request:**
```json
{
  "device_id": "<device_identifier>" 
}
```

**Response:**
```json
{
  "status": "unregistered"
}
```

#### POST /wake

Send a wake notification to a recipient.

**Request:**
```json
{
  "recipient_pubkey": "<recipient_z32_pubkey>",
  "wake_type": "noise_connect",
  "sender_pubkey": "<sender_z32_pubkey>",
  "nonce": "<random_nonce_hex>",
  "payload": "<optional_encrypted_payload>"
}
```

**Response:**
```json
{
  "status": "queued" | "delivered" | "not_registered",
  "wake_id": "<tracking_id>"
}
```

**Rate Limits:**
- 10 wake requests per minute per sender
- 100 wake requests per hour per recipient
- Exponential backoff on repeated failures

### Wake Types

| Type | Description | APNs Priority |
|------|-------------|---------------|
| `noise_connect` | Peer wants to establish Noise connection | background |
| `payment_received` | Incoming payment notification | high |
| `channel_update` | Lightning channel state change | background |

### Error Responses

```json
{
  "error": "error_code",
  "message": "Human readable message",
  "retry_after": 60
}
```

| Error Code | HTTP Status | Description |
|------------|-------------|-------------|
| `invalid_signature` | 401 | Signature verification failed |
| `expired_timestamp` | 401 | Timestamp too old (>5 min) |
| `rate_limited` | 429 | Too many requests |
| `recipient_not_found` | 404 | No push token registered |
| `invalid_token` | 400 | Push token format invalid |
| `platform_error` | 502 | APNs/FCM delivery failed |

## Security Considerations

### Token Storage
- Push tokens encrypted at rest using service master key
- Tokens never logged or exposed in responses
- Automatic deletion after 30 days of inactivity

### Request Validation
1. Verify Ed25519 signature
2. Check timestamp freshness (<5 minutes)
3. Validate sender pubkey format
4. Apply rate limits per sender and recipient

### Privacy
- Relay learns sender→recipient relationships (necessary for routing)
- Consider Tor/onion routing for sender anonymity in future
- Relay does not learn message content (encrypted payload)

### Rate Limiting
- Per-sender limits prevent DoS from single attacker
- Per-recipient limits prevent targeting specific users
- Global limits protect infrastructure

## Client Implementation

### Registration Flow (Bitkit)

```swift
// iOS
func registerForPushRelay() async throws {
    let token = await getAPNsToken()
    let signature = signRequest(method: "POST", path: "/register", body: ...)
    
    let response = try await pushRelayClient.register(
        platform: "ios",
        token: token,
        capabilities: ["wake", "payment_received"]
    )
    
    // Store relay_id for later unregistration
    KeychainStorage.set("push_relay_id", response.relayId)
}
```

### Wake Flow (Sender)

```swift
func wakeRecipient(pubkey: String) async throws {
    // Try Noise connection first
    if let connection = try? await noiseService.connect(to: pubkey) {
        return connection
    }
    
    // If not reachable, send wake notification
    try await pushRelayClient.wake(
        recipientPubkey: pubkey,
        wakeType: .noiseConnect,
        nonce: generateNonce()
    )
    
    // Retry Noise connection after short delay
    try await Task.sleep(for: .seconds(2))
    return try await noiseService.connect(to: pubkey)
}
```

### Receiving Wake (Recipient)

```swift
// In PushNotificationService
func handleWakeNotification(payload: [String: Any]) {
    guard let wakeType = payload["wake_type"] as? String,
          let senderPubkey = payload["sender_pubkey"] as? String else {
        return
    }
    
    switch wakeType {
    case "noise_connect":
        // Start Noise server if not running
        NoisePaymentService.shared.startServerIfNeeded()
        
    case "payment_received":
        // Show user notification
        showPaymentNotification(from: senderPubkey)
        
    default:
        break
    }
}
```

## Migration Plan

### Phase 1: Add Relay Support
1. Implement PushRelayService client in Bitkit
2. Register with relay on app launch
3. Continue publishing to homeserver (dual-write)

### Phase 2: Sender Migration  
1. Senders check relay first, fall back to homeserver
2. Monitor adoption metrics

### Phase 3: Remove Public Publishing
1. Stop publishing to homeserver
2. Remove discovery from DirectoryService
3. Relay becomes sole source of truth

## Future Enhancements

1. **Anonymous Wake via Onion Routing**: Sender anonymity using Tor
2. **Wake Receipts**: Delivery confirmation to sender
3. **Batch Wake**: Multiple recipients in single request
4. **Encrypted Relay Registration**: Hide pubkey→token mapping from relay

## Environment Variables

```bash
# Bitkit configuration
PUSH_RELAY_URL=https://push.paykit.app/v1
PUSH_RELAY_ENABLED=true
PUSH_RELAY_FALLBACK_TO_HOMESERVER=true  # During migration

# Relay service configuration (for operators)
PUSH_RELAY_APNS_KEY_ID=...
PUSH_RELAY_APNS_TEAM_ID=...
PUSH_RELAY_APNS_BUNDLE_ID=...
PUSH_RELAY_FCM_PROJECT_ID=...
PUSH_RELAY_ENCRYPTION_KEY=...
```


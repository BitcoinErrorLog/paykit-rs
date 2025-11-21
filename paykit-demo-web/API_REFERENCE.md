# Paykit Demo Web - API Reference

## Overview

This document provides complete API reference for all WebAssembly bindings exposed by the Paykit Demo Web module. All APIs are designed for JavaScript/TypeScript consumption in browser environments.

## Module Initialization

### `init()`

Initialize the WASM module. Called automatically when module loads.

```javascript
import init from './pkg/paykit_demo_web.js';
await init();
```

**Note**: This is called automatically via `#[wasm_bindgen(start)]`, but can be called manually if needed.

### `version()`

Get the version string of the WASM module.

```javascript
import { version } from './pkg/paykit_demo_web.js';
console.log(version()); // "0.1.0"
```

**Returns**: `string` - Version number

---

## Identity Management

### `Identity`

Ed25519 keypair-based identity for Paykit.

#### Constructor

```javascript
// Generate random identity
const identity = new Identity();

// Generate with nickname
const identity = Identity.withNickname("alice");
```

#### Methods

**`publicKey()` → `string`**

Get the z32-encoded public key.

```javascript
const pubkey = identity.publicKey();
// "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
```

**`pubkyUri()` → `string`**

Get the Pubky URI.

```javascript
const uri = identity.pubkyUri();
// "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
```

**`nickname()` → `Option<string>`**

Get the nickname if set.

```javascript
const nickname = identity.nickname();
// "alice" or null
```

**`toJSON()` → `string`**

Export identity as JSON string.

```javascript
const json = identity.toJSON();
// '{"public_key":"...","private_key":"...","nickname":"alice"}'
```

**`fromJSON(json: string)` → `Identity`**

Create identity from JSON string.

```javascript
const restored = Identity.fromJSON(json);
```

---

## Contact Management

### `WasmContact`

Represents a contact in the address book.

#### Constructor

```javascript
const contact = new WasmContact(
    "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
    "Alice Smith"
);
```

**Parameters**:
- `public_key: string` - Contact's z32-encoded public key
- `name: string` - Human-readable name

**Returns**: `WasmContact` or throws error if pubkey invalid

#### Methods

**`public_key()` → `string`**

Get contact's public key.

**`name()` → `string`**

Get contact's name.

**`notes()` → `Option<string>`**

Get contact's notes.

**`added_at()` → `number`**

Get timestamp when contact was added (Unix seconds).

**`payment_history()` → `Array<string>`**

Get array of receipt IDs for payments with this contact.

**`pubky_uri()` → `string`**

Get Pubky URI for this contact.

**`with_notes(notes: string)` → `WasmContact`**

Create a copy with notes added.

```javascript
const contactWithNotes = contact.with_notes("Met at conference");
```

**`to_json()` → `string`**

Export contact as JSON.

**`from_json(json: string)` → `WasmContact`**

Create contact from JSON.

### `WasmContactStorage`

Storage manager for contacts in localStorage.

#### Constructor

```javascript
const storage = new WasmContactStorage();
```

#### Methods

**`save_contact(contact: WasmContact)` → `Promise<void>`**

Save a contact to localStorage.

```javascript
await storage.save_contact(contact);
```

**`get_contact(public_key: string)` → `Promise<Option<WasmContact>>`**

Get contact by public key.

```javascript
const contact = await storage.get_contact(pubkey);
if (contact) {
    console.log(contact.name());
}
```

**`list_contacts()` → `Promise<Array<object>>`**

List all contacts, sorted alphabetically.

```javascript
const contacts = await storage.list_contacts();
// Returns array of objects with: public_key, name, notes, added_at, pubky_uri, payment_history
```

**`delete_contact(public_key: string)` → `Promise<void>`**

Delete a contact.

**`search_contacts(query: string)` → `Promise<Array<object>>`**

Search contacts by name (case-insensitive partial match).

**`update_payment_history(public_key: string, receipt_id: string)` → `Promise<void>`**

Add receipt ID to contact's payment history.

---

## Payment Methods

### `WasmPaymentMethodConfig`

Configuration for a payment method.

#### Constructor

```javascript
const method = new WasmPaymentMethodConfig(
    "lightning",           // method_id
    "alice@getalby.com",   // endpoint
    true,                  // is_public
    true,                  // is_preferred
    1                      // priority (1 = highest)
);
```

**Parameters**:
- `method_id: string` - Method identifier (e.g., "lightning", "onchain")
- `endpoint: string` - Payment endpoint (LNURL, address, etc.)
- `is_public: boolean` - Whether to publish publicly
- `is_preferred: boolean` - Whether this is preferred method
- `priority: number` - Priority order (1 = highest)

#### Methods

**`method_id()` → `string`**

**`endpoint()` → `string`**

**`is_public()` → `boolean`**

**`is_preferred()` → `boolean`**

**`priority()` → `number`**

**`to_json()` → `string`**

**`from_json(json: string)` → `WasmPaymentMethodConfig`**

### `WasmPaymentMethodStorage`

Storage manager for payment methods.

#### Constructor

```javascript
const storage = new WasmPaymentMethodStorage();
```

#### Methods

**`save_method(method: WasmPaymentMethodConfig)` → `Promise<void>`**

Save or update a payment method.

**`get_method(method_id: string)` → `Promise<Option<WasmPaymentMethodConfig>>`**

Get method by ID.

**`list_methods()` → `Promise<Array<object>>`**

List all methods, sorted by priority (lowest first).

**`delete_method(method_id: string)` → `Promise<void>`**

**`set_preferred(method_id: string, preferred: boolean)` → `Promise<void>`**

Update preferred status.

**`update_priority(method_id: string, priority: number)` → `Promise<void>`**

Change priority (lower = higher priority).

**`get_preferred_methods()` → `Promise<Array<object>>`**

Get only preferred methods.

**`mock_publish()` → `Promise<string>`**

Mock publish methods (demo only, returns warning message).

---

## Receipt Management

### `WasmReceiptStorage`

Storage manager for payment receipts.

#### Constructor

```javascript
const storage = new WasmReceiptStorage();
```

#### Methods

**`save_receipt(receipt_id: string, receipt_json: string)` → `Promise<void>`**

Save a receipt.

```javascript
const receipt = JSON.stringify({
    receipt_id: "receipt_123",
    payer: "alice_pubkey",
    payee: "bob_pubkey",
    amount: "1000",
    currency: "SAT",
    method: "lightning",
    timestamp: Math.floor(Date.now() / 1000)
});

await storage.save_receipt("receipt_123", receipt);
```

**`get_receipt(receipt_id: string)` → `Promise<Option<string>>`**

Get receipt JSON by ID.

**`list_receipts()` → `Promise<Array<string>>`**

List all receipts (returns JSON strings).

**`delete_receipt(receipt_id: string)` → `Promise<void>`**

**`filter_by_direction(direction: string, current_pubkey: string)` → `Promise<Array<string>>`**

Filter by "sent" or "received".

**`filter_by_method(method: string)` → `Promise<Array<string>>`**

Filter by payment method.

**`filter_by_contact(contact_pubkey: string, current_pubkey: string)` → `Promise<Array<string>>`**

Filter receipts involving a specific contact.

**`get_statistics(current_pubkey: string)` → `Promise<object>`**

Get receipt statistics.

```javascript
const stats = await storage.get_statistics(myPubkey);
// { total: 10, sent: 6, received: 4 }
```

**`export_as_json()` → `Promise<string>`**

Export all receipts as JSON array string.

**`clear_all()` → `Promise<void>`**

Delete all receipts.

---

## Dashboard

### `WasmDashboard`

Statistics aggregator for dashboard overview.

#### Constructor

```javascript
const dashboard = new WasmDashboard();
```

#### Methods

**`get_overview_stats(current_pubkey: string)` → `Promise<object>`**

Get comprehensive statistics from all features.

```javascript
const stats = await dashboard.get_overview_stats(myPubkey);
// {
//   contacts: 5,
//   payment_methods: 3,
//   preferred_methods: 1,
//   total_receipts: 20,
//   sent_receipts: 12,
//   received_receipts: 8,
//   total_subscriptions: 2,
//   active_subscriptions: 1
// }
```

**`get_recent_activity(current_pubkey: string, limit: number)` → `Promise<Array<object>>`**

Get recent transaction activity.

```javascript
const activity = await dashboard.get_recent_activity(myPubkey, 10);
// [
//   {
//     type: "receipt",
//     timestamp: 1700000000,
//     direction: "sent",
//     amount: "1000",
//     currency: "SAT"
//   }
// ]
```

**`get_setup_checklist()` → `Promise<object>`**

Get setup progress checklist.

```javascript
const checklist = await dashboard.get_setup_checklist();
// {
//   has_contacts: true,
//   has_payment_methods: true,
//   has_preferred_method: true
// }
```

**`is_setup_complete()` → `Promise<boolean>`**

Check if user has completed basic setup (has contacts and methods).

---

## Directory Operations

### `DirectoryClient`

Client for querying Pubky homeservers.

#### Constructor

```javascript
const client = new DirectoryClient("https://demo.httprelay.io");
```

**Parameters**:
- `homeserver_url: string` - Base URL of Pubky homeserver

#### Methods

**`queryMethods(public_key: string)` → `Promise<object>`**

Query payment methods for a public key.

```javascript
const methods = await client.queryMethods(recipientPubkey);
// {
//   "lightning": "lnurl1...",
//   "onchain": "bc1q..."
// }
```

**Returns**: Object mapping method IDs to endpoints, or empty object if none found.

---

## Storage

### `BrowserStorage`

Browser localStorage manager for identities.

#### Constructor

```javascript
const storage = new BrowserStorage();
```

#### Methods

**`saveIdentity(name: string, identity: Identity)` → `void`**

Save identity to localStorage.

**`loadIdentity(name: string)` → `Identity`**

Load identity by name.

**`listIdentities()` → `Array<string>`**

List all saved identity names.

**`deleteIdentity(name: string)` → `void`**

**`getCurrentIdentity()` → `Option<string>`**

Get current active identity name.

**`setCurrentIdentity(name: string)` → `void`**

**`clearAll()` → `void`**

Clear all Paykit data from localStorage.

---

## Subscriptions

### `WasmSubscription`

Subscription agreement representation.

#### Constructor

```javascript
const subscription = new WasmSubscription(
    subscriber_pubkey,
    provider_pubkey,
    "10000",      // amount
    "SAT",        // currency
    "monthly:1",  // frequency
    "lightning",  // method
    "Netflix subscription" // description
);
```

#### Methods

**`subscription_id()` → `string`**

**`subscriber()` → `string`**

**`provider()` → `string`**

**`amount()` → `string`**

**`currency()` → `string`**

**`frequency()` → `string`**

**`method()` → `string`**

**`description()` → `string`**

**`to_json()` → `string`**

**`from_json(json: string)` → `WasmSubscription`**

### `WasmSubscriptionAgreementStorage`

Storage for subscription agreements.

#### Methods

**`save_subscription(subscription: WasmSubscription)` → `Promise<void>`**

**`get_subscription(subscription_id: string)` → `Promise<Option<WasmSubscription>>`**

**`save_signed_subscription(subscription: WasmSubscription)` → `Promise<void>`**

**`get_signed_subscription(subscription_id: string)` → `Promise<Option<WasmSubscription>>`**

**`delete_subscription(subscription_id: string)` → `Promise<void>`**

**`delete_signed_subscription(subscription_id: string)` → `Promise<void>`**

**`list_active_subscriptions()` → `Promise<Array<object>>`**

**`list_all_subscriptions()` → `Promise<Array<object>>`**

**`clear_all()` → `Promise<void>`**

---

## Payment Requests

### `WasmPaymentRequest`

One-time payment request.

#### Constructor

```javascript
const request = new WasmPaymentRequest(
    from_pubkey,
    to_pubkey,
    "1000",      // amount
    "SAT",       // currency
    "lightning"  // method
);
```

#### Methods

**`with_description(desc: string)` → `WasmPaymentRequest`**

**`with_expiration(timestamp: number)` → `WasmPaymentRequest`**

**`request_id()` → `string`**

**`from()` → `string`**

**`to()` → `string`**

**`amount()` → `string`**

**`currency()` → `string`**

**`method()` → `string`**

**`description()` → `Option<string>`**

**`expiration()` → `Option<number>`**

**`to_json()` → `string`**

**`from_json(json: string)` → `WasmPaymentRequest`**

### `WasmRequestStorage`

Storage for payment requests.

#### Methods

**`save_request(request: WasmPaymentRequest)` → `Promise<void>`**

**`get_request(request_id: string)` → `Promise<Option<WasmPaymentRequest>>`**

**`list_requests()` → `Promise<Array<object>>`**

**`delete_request(request_id: string)` → `Promise<void>`**

**`clear_all()` → `Promise<void>`**

---

## Payment Operations

### `WasmPaymentCoordinator`

Payment coordinator for initiating interactive payments via Noise protocol over WebSocket.

#### Constructor

```javascript
const coordinator = new WasmPaymentCoordinator();
```

#### Methods

**`initiate_payment(payer_identity_json: string, ws_url: string, payee_pubkey: string, server_static_key_hex: string, amount: string, currency: string, method: string)` → `Promise<string>`**

Initiate a complete payment flow:
1. Connect to payee's WebSocket endpoint
2. Perform Noise IK handshake
3. Send payment request
4. Receive receipt confirmation
5. Store receipt

**Parameters**:
- `payer_identity_json: string` - JSON-serialized Identity (from `identity.toJSON()`)
- `ws_url: string` - WebSocket URL (e.g., `ws://localhost:9735` or `wss://example.com:9735`)
- `payee_pubkey: string` - Recipient's public key (z32-encoded)
- `server_static_key_hex: string` - Server's static public key (64 hex characters)
- `amount: string` - Payment amount (e.g., `"1000"`)
- `currency: string` - Currency code (e.g., `"SAT"`, `"USD"`)
- `method: string` - Payment method ID (e.g., `"lightning"`, `"onchain"`)

**Returns**: Receipt JSON string on success

**Throws**: Error with descriptive message on failure

**Example**:
```javascript
const coordinator = new WasmPaymentCoordinator();
const identity = Identity.fromJSON(identityJson);

try {
    const receiptJson = await coordinator.initiate_payment(
        identity.toJSON(),
        'ws://localhost:9735',
        '8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo',
        '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
        '1000',
        'SAT',
        'lightning'
    );
    
    const receipt = JSON.parse(receiptJson);
    console.log('Payment successful:', receipt);
} catch (error) {
    console.error('Payment failed:', error.message);
}
```

**`get_receipts()` → `Promise<Array<object>>`**

Get all stored receipts.

**Returns**: Array of receipt JSON objects

### `WasmPaymentReceiver`

Receiver for accepting payments.

#### Constructor

```javascript
const receiver = new WasmPaymentReceiver();
```

#### Methods

**`acceptPayment(request_json: string)` → `Promise<PaykitReceipt>`**

Accept and process payment request.

---

## Utility Functions

### `format_timestamp(timestamp: number)` → `string`

Format Unix timestamp to human-readable string.

```javascript
import { format_timestamp } from './pkg/paykit_demo_web.js';
const formatted = format_timestamp(1700000000);
// "2023-11-15 12:00:00"
```

### `is_valid_pubkey(pubkey: string)` → `boolean`

Validate if string is a valid z32-encoded public key.

```javascript
import { is_valid_pubkey } from './pkg/paykit_demo_web.js';
if (is_valid_pubkey(input)) {
    // Valid pubkey
}
```

### `parse_noise_endpoint_wasm(endpoint: string)` → `Promise<object>`

Parse a Noise endpoint string and return WebSocket URL and server key.

**Format**: `noise://host:port@pubkey_hex`

**Returns**: JSON object with:
- `ws_url: string` - WebSocket URL (ws:// for localhost, wss:// for remote)
- `server_key_hex: string` - Server static key (64 hex characters)
- `host: string` - Host address
- `port: number` - Port number

**Example**:
```javascript
import { parse_noise_endpoint_wasm } from './pkg/paykit_demo_web.js';

const result = await parse_noise_endpoint_wasm(
    'noise://127.0.0.1:9735@0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'
);
// {
//   ws_url: "ws://127.0.0.1:9735",
//   server_key_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
//   host: "127.0.0.1",
//   port: 9735
// }
```

**Throws**: Error if endpoint format is invalid

### `extract_pubkey_from_uri_wasm(uri: string)` → `Promise<string>`

Extract public key from pubky:// URI or raw public key.

**Parameters**:
- `uri: string` - Pubky URI (`pubky://...`) or raw public key

**Returns**: Public key string

**Example**:
```javascript
import { extract_pubkey_from_uri_wasm } from './pkg/paykit_demo_web.js';

const pubkey1 = await extract_pubkey_from_uri_wasm('pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo');
// Returns: "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"

const pubkey2 = await extract_pubkey_from_uri_wasm('8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo');
// Returns: "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
```

---

## Type Definitions

### Receipt Format

```typescript
interface Receipt {
    receipt_id: string;
    payer: string;
    payee: string;
    amount: string;
    currency: string;
    method: string;
    timestamp: number; // Unix seconds
}
```

### Contact Format

```typescript
interface Contact {
    public_key: string;
    name: string;
    notes?: string;
    added_at: number;
    payment_history: string[];
}
```

### Payment Method Format

```typescript
interface PaymentMethod {
    method_id: string;
    endpoint: string;
    is_public: boolean;
    is_preferred: boolean;
    priority: number;
}
```

### Subscription Format

```typescript
interface Subscription {
    subscription_id: string;
    subscriber: string;
    provider: string;
    amount: string;
    currency: string;
    frequency: string;
    method: string;
    description: string;
}
```

---

## Error Handling

All async methods return `Promise<T>` or `Promise<Result<T, JsValue>>`.

### Error Patterns

```javascript
try {
    const result = await storage.save_contact(contact);
} catch (error) {
    console.error("Failed:", error);
    // error is a JsValue, can be converted to string
}
```

### Common Errors

- **Invalid public key**: Thrown when pubkey format is invalid
- **localStorage unavailable**: Thrown if browser doesn't support localStorage
- **Storage quota exceeded**: Thrown when localStorage is full
- **JSON parse error**: Thrown when deserializing invalid JSON

---

## Best Practices

### 1. Always Await Async Operations

```javascript
// ✅ Correct
await storage.save_contact(contact);

// ❌ Wrong
storage.save_contact(contact); // Won't complete
```

### 2. Check for None/Optional Values

```javascript
const contact = await storage.get_contact(pubkey);
if (contact) {
    // Use contact
} else {
    // Handle not found
}
```

### 3. Handle Errors

```javascript
try {
    await storage.save_contact(contact);
} catch (error) {
    showErrorToUser(error);
}
```

### 4. Clean Up Test Data

```javascript
// In tests, always clean up
await storage.delete_contact(testPubkey);
```

### 5. Use TypeScript

The generated `.d.ts` files provide full type information:

```typescript
import { WasmContactStorage, WasmContact } from './pkg/paykit_demo_web';

const storage: WasmContactStorage = new WasmContactStorage();
const contact: WasmContact = new WasmContact(pubkey, name);
```

---

## Module Import

### ES6 Modules

```javascript
import init, {
    Identity,
    WasmContactStorage,
    WasmPaymentMethodStorage,
    WasmReceiptStorage,
    WasmDashboard,
    // ... other exports
} from './pkg/paykit_demo_web.js';

await init();
```

### CommonJS (if needed)

```javascript
const paykit = require('./pkg/paykit_demo_web.js');
await paykit.default();
const Identity = paykit.Identity;
```

---

## Complete Example

```javascript
import init, {
    Identity,
    BrowserStorage,
    WasmContactStorage,
    WasmPaymentMethodStorage,
    WasmReceiptStorage,
    WasmDashboard
} from './pkg/paykit_demo_web.js';

// Initialize
await init();

// Create identity
const identity = Identity.withNickname("alice");
const storage = new BrowserStorage();
storage.saveIdentity("alice", identity);
storage.setCurrentIdentity("alice");

// Add contact
const contactStorage = new WasmContactStorage();
const contact = new WasmContact(
    "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
    "Bob"
);
await contactStorage.save_contact(contact);

// Configure payment method
const methodStorage = new WasmPaymentMethodStorage();
const method = new WasmPaymentMethodConfig(
    "lightning",
    "alice@getalby.com",
    true,
    true,
    1
);
await methodStorage.save_method(method);

// View dashboard
const dashboard = new WasmDashboard();
const stats = await dashboard.get_overview_stats(identity.publicKey());
console.log("Contacts:", stats.contacts);
console.log("Methods:", stats.payment_methods);
```

---

## See Also

- [README.md](./README.md) - Project overview
- [TESTING.md](./TESTING.md) - Testing guide
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Architecture details
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Deployment guide

---

**Last Updated**: November 2024  
**WASM Module Version**: 0.1.0


# Paykit Mobile API Reference

Complete API reference for the Paykit Mobile FFI bindings.

## Table of Contents

- [PaykitClient](#paykit-client)
- [Executor Interfaces](#executor-interfaces)
  - [BitcoinExecutorFFI](#bitcoinexecutorffi)
  - [LightningExecutorFFI](#lightningexecutorffi)
- [Result Types](#result-types)
  - [BitcoinTxResultFFI](#bitcointxresultffi)
  - [LightningPaymentResultFFI](#lightningpaymentresultffi)
  - [DecodedInvoiceFFI](#decodedinvoiceffi)
  - [PaymentExecutionResult](#paymentexecutionresult)
  - [PaymentProofResult](#paymentproofresult)
- [Enums](#enums)
  - [BitcoinNetworkFFI](#bitcoinnetworkffi)
  - [LightningNetworkFFI](#lightningnetworkffi)
  - [LightningPaymentStatusFFI](#lightningpaymentstatusffi)
- [Error Types](#error-types)

---

## PaykitClient

The main entry point for Paykit mobile integration.

### Constructors

#### `new()`

Creates a new PaykitClient with default mainnet configuration.

```swift
// Swift
let client = try PaykitClient.new()
```

```kotlin
// Kotlin
val client = PaykitClient.new()
```

**Returns:** `PaykitClient`

**Throws:** `PaykitMobileError` if initialization fails

---

#### `new_with_network(bitcoinNetwork, lightningNetwork)`

Creates a new PaykitClient with specified network configuration.

```swift
// Swift
let client = try PaykitClient.newWithNetwork(
    bitcoinNetwork: .testnet,
    lightningNetwork: .testnet
)
```

```kotlin
// Kotlin
val client = PaykitClient.newWithNetwork(
    bitcoinNetwork = BitcoinNetworkFfi.TESTNET,
    lightningNetwork = LightningNetworkFfi.TESTNET
)
```

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `bitcoinNetwork` | `BitcoinNetworkFFI` | Bitcoin network to use |
| `lightningNetwork` | `LightningNetworkFFI` | Lightning network to use |

**Returns:** `PaykitClient`

**Throws:** `PaykitMobileError` if initialization fails

---

### Network Configuration

#### `bitcoin_network()`

Returns the configured Bitcoin network.

```swift
let network = client.bitcoinNetwork() // .mainnet, .testnet, or .regtest
```

**Returns:** `BitcoinNetworkFFI`

---

#### `lightning_network()`

Returns the configured Lightning network.

```swift
let network = client.lightningNetwork() // .mainnet, .testnet, or .regtest
```

**Returns:** `LightningNetworkFFI`

---

### Executor Registration

#### `register_bitcoin_executor(executor)`

Registers a Bitcoin wallet executor for on-chain payments.

```swift
// Swift
try client.registerBitcoinExecutor(executor: myBitcoinExecutor)
```

```kotlin
// Kotlin
client.registerBitcoinExecutor(myBitcoinExecutor)
```

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `executor` | `BitcoinExecutorFFI` | Implementation of Bitcoin executor interface |

**Throws:** `PaykitMobileError` if registration fails

**Notes:**
- Calling this method multiple times replaces the previous executor
- The executor must be thread-safe

---

#### `register_lightning_executor(executor)`

Registers a Lightning node executor for Lightning payments.

```swift
// Swift
try client.registerLightningExecutor(executor: myLightningExecutor)
```

```kotlin
// Kotlin
client.registerLightningExecutor(myLightningExecutor)
```

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `executor` | `LightningExecutorFFI` | Implementation of Lightning executor interface |

**Throws:** `PaykitMobileError` if registration fails

---

#### `has_bitcoin_executor()`

Checks if a Bitcoin executor is registered.

```swift
if client.hasBitcoinExecutor() {
    // Can execute on-chain payments
}
```

**Returns:** `Bool`

---

#### `has_lightning_executor()`

Checks if a Lightning executor is registered.

```swift
if client.hasLightningExecutor() {
    // Can execute Lightning payments
}
```

**Returns:** `Bool`

---

### Payment Execution

#### `execute_payment(methodId, endpoint, amountSats, metadataJson)`

Executes a payment using the specified method.

```swift
// Swift
let result = try client.executePayment(
    methodId: "lightning",
    endpoint: invoice,
    amountSats: 1000,
    metadataJson: nil
)
```

```kotlin
// Kotlin
val result = client.executePayment(
    methodId = "lightning",
    endpoint = invoice,
    amountSats = 1000UL,
    metadataJson = null
)
```

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `methodId` | `String` | Payment method: `"onchain"` or `"lightning"` |
| `endpoint` | `String` | Payment destination (address or invoice) |
| `amountSats` | `UInt64` | Amount in satoshis |
| `metadataJson` | `String?` | Optional JSON metadata (e.g., fee rate) |

**Returns:** `PaymentExecutionResult`

**Throws:** `PaykitMobileError`
- `.NotFound` if method not registered
- `.Validation` if endpoint invalid
- `.Transport` if payment fails

**Metadata Format:**

For on-chain payments:
```json
{
    "fee_rate": 5.0
}
```

---

#### `generate_payment_proof(methodId, executionDataJson)`

Generates a proof of payment from execution data.

```swift
// Swift
let proof = try client.generatePaymentProof(
    methodId: result.methodId,
    executionDataJson: result.executionDataJson
)
```

```kotlin
// Kotlin
val proof = client.generatePaymentProof(
    methodId = result.methodId,
    executionDataJson = result.executionDataJson
)
```

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `methodId` | `String` | Payment method used |
| `executionDataJson` | `String` | JSON from `PaymentExecutionResult.executionDataJson` |

**Returns:** `PaymentProofResult`

**Throws:** `PaykitMobileError`
- `.NotFound` if method unknown
- `.Serialization` if JSON invalid

---

#### `validate_endpoint(methodId, endpoint)`

Validates a payment endpoint (address or invoice).

```swift
// Swift
let result = try client.validateEndpoint(methodId: "onchain", endpoint: address)
```

```kotlin
// Kotlin
val result = client.validateEndpoint(methodId = "onchain", endpoint = address)
```

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `methodId` | `String` | Payment method |
| `endpoint` | `String` | Address or invoice to validate |

**Returns:** `ValidationResult` with `isValid` and optional `message`

---

## Executor Interfaces

### BitcoinExecutorFFI

Interface for Bitcoin wallet operations. Implement this to connect your wallet to Paykit.

```swift
// Swift
protocol BitcoinExecutorFFI {
    func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFfi
    func estimateFee(address: String, amountSats: UInt64, targetBlocks: UInt32) throws -> UInt64
    func getTransaction(txid: String) throws -> BitcoinTxResultFfi?
    func verifyTransaction(txid: String, address: String, amountSats: UInt64) throws -> Bool
}
```

```kotlin
// Kotlin
interface BitcoinExecutorFfi {
    fun sendToAddress(address: String, amountSats: ULong, feeRate: Double?): BitcoinTxResultFfi
    fun estimateFee(address: String, amountSats: ULong, targetBlocks: UInt): ULong
    fun getTransaction(txid: String): BitcoinTxResultFfi?
    fun verifyTransaction(txid: String, address: String, amountSats: ULong): Boolean
}
```

#### Methods

##### `sendToAddress(address, amountSats, feeRate)`

Sends Bitcoin to the specified address.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `address` | `String` | Bitcoin address (any format) |
| `amountSats` | `UInt64` | Amount to send in satoshis |
| `feeRate` | `Double?` | Optional fee rate in sat/vB |

**Returns:** `BitcoinTxResultFFI` with transaction details

**Throws:** `PaykitMobileError` on failure

**Implementation Notes:**
- If `feeRate` is `nil`, use wallet's default fee estimation
- Amount must be above dust limit (546 sats for P2PKH, 294 for P2WSH)
- Transaction should be broadcast before returning

---

##### `estimateFee(address, amountSats, targetBlocks)`

Estimates the fee for a transaction.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `address` | `String` | Destination address |
| `amountSats` | `UInt64` | Amount to send |
| `targetBlocks` | `UInt32` | Target confirmation blocks (1-1008) |

**Returns:** `UInt64` - Estimated fee in satoshis

---

##### `getTransaction(txid)`

Retrieves transaction details by txid.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `txid` | `String` | Transaction ID (64 hex characters) |

**Returns:** `BitcoinTxResultFFI?` - Transaction details or `nil` if not found

---

##### `verifyTransaction(txid, address, amountSats)`

Verifies a transaction matches expected parameters.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `txid` | `String` | Transaction ID to verify |
| `address` | `String` | Expected recipient address |
| `amountSats` | `UInt64` | Expected amount |

**Returns:** `Bool` - `true` if transaction matches

---

### LightningExecutorFFI

Interface for Lightning node operations.

```swift
// Swift
protocol LightningExecutorFFI {
    func payInvoice(invoice: String, amountMsat: UInt64?, maxFeeMsat: UInt64?) throws -> LightningPaymentResultFfi
    func decodeInvoice(invoice: String) throws -> DecodedInvoiceFfi
    func estimateFee(invoice: String) throws -> UInt64
    func getPayment(paymentHash: String) throws -> LightningPaymentResultFfi?
    func verifyPreimage(preimage: String, paymentHash: String) -> Bool
}
```

```kotlin
// Kotlin
interface LightningExecutorFfi {
    fun payInvoice(invoice: String, amountMsat: ULong?, maxFeeMsat: ULong?): LightningPaymentResultFfi
    fun decodeInvoice(invoice: String): DecodedInvoiceFfi
    fun estimateFee(invoice: String): ULong
    fun getPayment(paymentHash: String): LightningPaymentResultFfi?
    fun verifyPreimage(preimage: String, paymentHash: String): Boolean
}
```

#### Methods

##### `payInvoice(invoice, amountMsat, maxFeeMsat)`

Pays a BOLT11 Lightning invoice.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `invoice` | `String` | BOLT11 invoice string |
| `amountMsat` | `UInt64?` | Amount for zero-amount invoices (millisatoshis) |
| `maxFeeMsat` | `UInt64?` | Maximum routing fee allowed |

**Returns:** `LightningPaymentResultFFI` with preimage and payment details

**Throws:** `PaykitMobileError` on failure

**Implementation Notes:**
- `amountMsat` is required for invoices without amount
- If `maxFeeMsat` is `nil`, use node's default fee limit
- Wait for payment to settle before returning (or return with `Pending` status)

---

##### `decodeInvoice(invoice)`

Decodes a BOLT11 invoice without paying.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `invoice` | `String` | BOLT11 invoice string |

**Returns:** `DecodedInvoiceFFI` with parsed invoice details

---

##### `estimateFee(invoice)`

Estimates the routing fee for an invoice.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `invoice` | `String` | BOLT11 invoice to estimate |

**Returns:** `UInt64` - Estimated fee in millisatoshis

---

##### `getPayment(paymentHash)`

Gets payment status by payment hash.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `paymentHash` | `String` | Payment hash (64 hex characters) |

**Returns:** `LightningPaymentResultFFI?` - Payment details or `nil` if not found

---

##### `verifyPreimage(preimage, paymentHash)`

Verifies a preimage matches its payment hash.

**Parameters:**
| Name | Type | Description |
|------|------|-------------|
| `preimage` | `String` | Payment preimage (64 hex characters) |
| `paymentHash` | `String` | Payment hash (64 hex characters) |

**Returns:** `Bool` - `true` if SHA256(preimage) == paymentHash

**Note:** This is a pure function with no side effects.

---

## Result Types

### BitcoinTxResultFFI

Result of a Bitcoin transaction.

```swift
struct BitcoinTxResultFfi {
    let txid: String           // Transaction ID (64 hex chars)
    let rawTx: String?         // Raw transaction hex (optional)
    let vout: UInt32           // Output index
    let feeSats: UInt64        // Fee paid in satoshis
    let feeRate: Double        // Fee rate in sat/vB
    let blockHeight: UInt64?   // Block height if confirmed
    let confirmations: UInt64  // Number of confirmations (0 if unconfirmed)
}
```

| Field | Type | Description |
|-------|------|-------------|
| `txid` | `String` | Transaction ID (64 lowercase hex characters) |
| `rawTx` | `String?` | Full raw transaction in hex (for broadcasting) |
| `vout` | `UInt32` | Output index for the payment |
| `feeSats` | `UInt64` | Total fee paid in satoshis |
| `feeRate` | `Double` | Effective fee rate in sat/vB |
| `blockHeight` | `UInt64?` | Block height if confirmed, `nil` if pending |
| `confirmations` | `UInt64` | Number of confirmations (0 for mempool) |

---

### LightningPaymentResultFFI

Result of a Lightning payment.

```swift
struct LightningPaymentResultFfi {
    let preimage: String                    // Payment preimage (64 hex chars)
    let paymentHash: String                 // Payment hash (64 hex chars)
    let amountMsat: UInt64                  // Amount paid in millisatoshis
    let feeMsat: UInt64                     // Routing fee in millisatoshis
    let hops: UInt32                        // Number of hops in route
    let status: LightningPaymentStatusFfi  // Payment status
}
```

| Field | Type | Description |
|-------|------|-------------|
| `preimage` | `String` | Payment preimage (64 lowercase hex) |
| `paymentHash` | `String` | Payment hash (64 lowercase hex) |
| `amountMsat` | `UInt64` | Amount paid in millisatoshis |
| `feeMsat` | `UInt64` | Routing fee paid in millisatoshis |
| `hops` | `UInt32` | Number of hops in the payment route |
| `status` | `LightningPaymentStatusFFI` | Current payment status |

---

### DecodedInvoiceFFI

Decoded BOLT11 invoice information.

```swift
struct DecodedInvoiceFfi {
    let paymentHash: String       // Payment hash (64 hex chars)
    let amountMsat: UInt64?       // Amount if specified
    let description: String?      // Invoice description
    let descriptionHash: String?  // Description hash (for long descriptions)
    let payee: String             // Payee public key
    let expiry: UInt64            // Expiry time in seconds
    let timestamp: UInt64         // Creation timestamp (Unix)
    let expired: Bool             // Whether invoice has expired
}
```

| Field | Type | Description |
|-------|------|-------------|
| `paymentHash` | `String` | Payment hash from invoice |
| `amountMsat` | `UInt64?` | Invoice amount if specified, `nil` for any-amount |
| `description` | `String?` | Human-readable description |
| `descriptionHash` | `String?` | SHA256 of description (for long descriptions) |
| `payee` | `String` | Payee node public key (66 hex chars) |
| `expiry` | `UInt64` | Seconds until expiry from timestamp |
| `timestamp` | `UInt64` | Invoice creation time (Unix timestamp) |
| `expired` | `Bool` | `true` if invoice has expired |

---

### PaymentExecutionResult

Result from `executePayment()`.

```swift
struct PaymentExecutionResult {
    let executionId: String        // Unique execution ID
    let methodId: String           // Payment method used
    let endpoint: String           // Payment destination
    let amountSats: UInt64         // Amount in satoshis
    let success: Bool              // Whether payment succeeded
    let executedAt: Int64          // Unix timestamp
    let executionDataJson: String  // JSON with payment details
    let error: String?             // Error message if failed
}
```

| Field | Type | Description |
|-------|------|-------------|
| `executionId` | `String` | Unique identifier (format: `exec_XXXXXXXX`) |
| `methodId` | `String` | Method used: `"onchain"` or `"lightning"` |
| `endpoint` | `String` | Address or invoice |
| `amountSats` | `UInt64` | Amount paid in satoshis |
| `success` | `Bool` | `true` if payment completed |
| `executedAt` | `Int64` | Execution timestamp (Unix) |
| `executionDataJson` | `String` | JSON containing method-specific data |
| `error` | `String?` | Error message if `success` is `false` |

**executionDataJson Format:**

For on-chain:
```json
{
    "txid": "abc123...",
    "address": "bc1q...",
    "amount_sats": 10000,
    "vout": 0,
    "fee_sats": 210,
    "fee_rate": 1.5,
    "confirmations": 0
}
```

For Lightning:
```json
{
    "preimage": "0123...",
    "payment_hash": "abcd...",
    "invoice": "lnbc...",
    "amount_msat": 10000000,
    "fee_msat": 100,
    "hops": 3,
    "status": "Succeeded"
}
```

---

### PaymentProofResult

Result from `generatePaymentProof()`.

```swift
struct PaymentProofResult {
    let proofType: String       // Type of proof
    let proofDataJson: String   // JSON proof data
}
```

| Field | Type | Description |
|-------|------|-------------|
| `proofType` | `String` | `"bitcoin_txid"` or `"lightning_preimage"` |
| `proofDataJson` | `String` | JSON containing proof data |

**proofDataJson Format:**

For Bitcoin:
```json
{
    "txid": "abc123...",
    "address": "bc1q...",
    "amount_sats": 10000,
    "confirmations": 6
}
```

For Lightning:
```json
{
    "preimage": "0123...",
    "payment_hash": "abcd...",
    "amount_msat": 10000000
}
```

---

## Enums

### BitcoinNetworkFFI

Bitcoin network configuration.

```swift
enum BitcoinNetworkFfi {
    case mainnet  // Bitcoin mainnet
    case testnet  // Bitcoin testnet3
    case regtest  // Local regtest network
}
```

```kotlin
enum class BitcoinNetworkFfi {
    MAINNET,  // Bitcoin mainnet
    TESTNET,  // Bitcoin testnet3
    REGTEST   // Local regtest network
}
```

---

### LightningNetworkFFI

Lightning network configuration.

```swift
enum LightningNetworkFfi {
    case mainnet  // Lightning mainnet
    case testnet  // Lightning testnet
    case regtest  // Local regtest/signet
}
```

```kotlin
enum class LightningNetworkFfi {
    MAINNET,  // Lightning mainnet
    TESTNET,  // Lightning testnet
    REGTEST   // Local regtest/signet
}
```

---

### LightningPaymentStatusFFI

Lightning payment status.

```swift
enum LightningPaymentStatusFfi {
    case pending    // Payment in flight
    case succeeded  // Payment completed
    case failed     // Payment failed
}
```

```kotlin
enum class LightningPaymentStatusFfi {
    PENDING,   // Payment in flight
    SUCCEEDED, // Payment completed
    FAILED     // Payment failed
}
```

---

## Error Types

### PaykitMobileError

All errors thrown by Paykit Mobile.

```swift
enum PaykitMobileError: Error {
    case Transport(message: String)      // Network or I/O error
    case Validation(message: String)     // Invalid input
    case NotFound(message: String)       // Resource not found
    case Serialization(message: String)  // JSON parse error
    case Internal(message: String)       // Unexpected error
}
```

```kotlin
sealed class PaykitMobileException : Exception() {
    class Transport(override val message: String) : PaykitMobileException()
    class Validation(override val message: String) : PaykitMobileException()
    class NotFound(override val message: String) : PaykitMobileException()
    class Serialization(override val message: String) : PaykitMobileException()
    class Internal(override val message: String) : PaykitMobileException()
}
```

| Error | When Thrown |
|-------|-------------|
| `Transport` | Network failure, wallet unavailable, payment failed |
| `Validation` | Invalid address, invalid invoice, amount too low |
| `NotFound` | Unknown payment method, transaction not found |
| `Serialization` | Invalid JSON input |
| `Internal` | Unexpected internal error |

---

## Thread Safety

All PaykitClient methods are thread-safe and can be called from any thread.

**Important:** Your executor implementations (`BitcoinExecutorFFI`, `LightningExecutorFFI`) MUST be thread-safe. Paykit may call executor methods from background threads.

Example thread-safe implementation:

```swift
class ThreadSafeBitcoinExecutor: BitcoinExecutorFFI {
    private let wallet: MyWallet
    private let lock = NSLock()
    
    func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFfi {
        lock.lock()
        defer { lock.unlock() }
        return try wallet.send(to: address, amount: amountSats, feeRate: feeRate)
    }
}
```

---

## Version Compatibility

| Paykit Mobile | Rust | UniFFI | iOS | Android |
|---------------|------|--------|-----|---------|
| 0.1.x | 1.70+ | 0.25+ | 15.0+ | API 24+ |

---

## See Also

- [BITKIT_INTEGRATION_GUIDE.md](BITKIT_INTEGRATION_GUIDE.md) - Step-by-step integration guide
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [swift/BitkitExecutorExample.swift](swift/BitkitExecutorExample.swift) - Swift example
- [kotlin/BitkitExecutorExample.kt](kotlin/BitkitExecutorExample.kt) - Kotlin example

# Paykit Mobile FFI API Reference

This document provides a complete reference of all types, functions, and callbacks
exported via UniFFI for iOS (Swift) and Android (Kotlin) integration.

## Table of Contents

1. [Core Types](#core-types)
2. [PaykitClient](#paykitclient)
3. [Executor FFI (Bitkit Integration)](#executor-ffi-bitkit-integration)
4. [Transport Layer](#transport-layer)
5. [Interactive Protocol](#interactive-protocol)
6. [Noise Protocol](#noise-protocol)
7. [Key Management](#key-management)
8. [Scanner](#scanner)
9. [Storage](#storage)
10. [Async Bridge](#async-bridge)

---

## Core Types

### Error Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `PaykitMobileError` | `PaykitMobileError` | `PaykitMobileError` | Main error enum |

**Error Variants:**
- `Transport { message: String }` - Network/I/O errors
- `Validation { message: String }` - Invalid input
- `NotFound { message: String }` - Resource not found
- `Serialization { message: String }` - JSON errors
- `Internal { message: String }` - Unexpected state
- `NetworkTimeout { message: String }` - Timeout
- `ConnectionError { message: String }` - Connection failed
- `AuthenticationError { message: String }` - Auth failed
- `SessionError { message: String }` - Session expired
- `RateLimitError { message: String }` - Rate limited
- `PermissionDenied { message: String }` - No permission

### Basic Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `MethodId` | `MethodId` | `MethodId` | Payment method identifier |
| `EndpointData` | `EndpointData` | `EndpointData` | Endpoint data wrapper |
| `PaymentMethod` | `PaymentMethod` | `PaymentMethod` | Method with endpoint |
| `Amount` | `Amount` | `Amount` | Payment amount |

**PaymentMethod Fields:**
```rust
pub struct PaymentMethod {
    pub method_id: String,
    pub endpoint: String,
}
```

**Amount Fields:**
```rust
pub struct Amount {
    pub value: String,
    pub currency: String,
}
```

### Selection Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `SelectionStrategy` | `SelectionStrategy` | `SelectionStrategy` | Selection strategy enum |
| `SelectionPreferences` | `SelectionPreferences` | `SelectionPreferences` | Selection preferences |
| `SelectionResult` | `SelectionResult` | `SelectionResult` | Selection result |

**SelectionStrategy Variants:**
- `Balanced`
- `CostOptimized`
- `SpeedOptimized`
- `PrivacyOptimized`

### Status Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `PaymentStatus` | `PaymentStatus` | `PaymentStatus` | Payment status enum |
| `PaymentStatusInfo` | `PaymentStatusInfo` | `PaymentStatusInfo` | Status with details |
| `HealthStatus` | `HealthStatus` | `HealthStatus` | Health status enum |
| `HealthCheckResult` | `HealthCheckResult` | `HealthCheckResult` | Health check result |

**PaymentStatus Variants:**
- `Pending`
- `Processing`
- `Confirmed`
- `Finalized`
- `Failed`
- `Cancelled`
- `Expired`

**HealthStatus Variants:**
- `Healthy`
- `Degraded`
- `Unavailable`
- `Unknown`

### Subscription Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `PaymentFrequency` | `PaymentFrequency` | `PaymentFrequency` | Billing frequency |
| `SubscriptionTerms` | `SubscriptionTerms` | `SubscriptionTerms` | Subscription terms |
| `Subscription` | `Subscription` | `Subscription` | Subscription record |
| `ModificationType` | `ModificationType` | `ModificationType` | Modification type |
| `ProrationResult` | `ProrationResult` | `ProrationResult` | Proration calculation |

### Request Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `PaymentRequest` | `PaymentRequest` | `PaymentRequest` | Payment request |
| `RequestStatus` | `RequestStatus` | `RequestStatus` | Request status enum |
| `Receipt` | `Receipt` | `Receipt` | Payment receipt |
| `PrivateEndpoint` | `PrivateEndpoint` | `PrivateEndpoint` | Private endpoint |

---

## PaykitClient

**Class:** `PaykitClient` (UniFFI Object)

### Constructor

```swift
// Swift
let client = try PaykitClient()

// Kotlin
val client = PaykitClient()
```

### Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `listMethods()` | - | `[String]` | List registered payment methods |
| `validateEndpoint(methodId:endpoint:)` | `String, String` | `Bool` | Validate an endpoint |
| `selectMethod(supportedMethods:amountSats:preferences:)` | `[PaymentMethod], UInt64, SelectionPreferences?` | `SelectionResult` | Select best payment method |
| `checkHealth()` | - | `[HealthCheckResult]` | Check health of all methods |
| `getHealthStatus(methodId:)` | `String` | `HealthStatus?` | Get health of one method |
| `isMethodUsable(methodId:)` | `String` | `Bool` | Check if method is usable |
| `getPaymentStatus(receiptId:)` | `String` | `PaymentStatusInfo?` | Get payment status |
| `getInProgressPayments()` | - | `[PaymentStatusInfo]` | Get all in-progress payments |

### Subscription Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `createSubscription(subscriber:provider:terms:)` | `String, String, SubscriptionTerms` | `Subscription` | Create subscription |
| `calculateProration(...)` | `i64, i64, i64, i64, i64` | `ProrationResult` | Calculate proration |
| `daysRemainingInPeriod(periodEnd:)` | `i64` | `UInt32` | Days left in period |

### Payment Request Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `createPaymentRequest(...)` | `String, String, i64, String, String, String, UInt64?` | `PaymentRequest` | Create payment request |
| `createReceipt(payer:payee:methodId:amount:currency:)` | `String, String, String, String?, String?` | `Receipt` | Create receipt |
| `parseReceiptMetadata(metadataJson:)` | `String` | `String` | Parse receipt metadata |

### Scanner Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `parseScannedQr(scannedData:)` | `String` | `ScannedUri` | Parse QR code |
| `isPaykitQr(scannedData:)` | `String` | `Bool` | Check if Paykit URI |
| `extractKeyFromQr(scannedData:)` | `String` | `String?` | Extract public key |
| `extractMethodFromQr(scannedData:)` | `String` | `String?` | Extract payment method |

### Directory Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `publishPaymentEndpoint(transport:methodId:endpointData:)` | `AuthenticatedTransportFfi, String, String` | - | Publish endpoint |
| `removePaymentEndpointFromDirectory(transport:methodId:)` | `AuthenticatedTransportFfi, String` | - | Remove endpoint |
| `fetchSupportedPayments(transport:ownerPubkey:)` | `UnauthenticatedTransportFfi, String` | `[PaymentMethod]` | Fetch all methods |
| `fetchPaymentEndpoint(transport:ownerPubkey:methodId:)` | `UnauthenticatedTransportFfi, String, String` | `String?` | Fetch one endpoint |
| `fetchKnownContacts(transport:ownerPubkey:)` | `UnauthenticatedTransportFfi, String` | `[String]` | Fetch contacts |
| `addContact(transport:contactPubkey:)` | `AuthenticatedTransportFfi, String` | - | Add contact |
| `removeContact(transport:contactPubkey:)` | `AuthenticatedTransportFfi, String` | - | Remove contact |
| `listContacts(transport:)` | `AuthenticatedTransportFfi` | `[String]` | List contacts |

### Noise Protocol Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `discoverNoiseEndpoint(transport:recipientPubkey:)` | `UnauthenticatedTransportFfi, String` | `NoiseEndpointInfo?` | Discover noise endpoint |
| `publishNoiseEndpoint(transport:host:port:noisePubkey:metadata:)` | `AuthenticatedTransportFfi, String, UInt16, String, String?` | - | Publish noise endpoint |
| `removeNoiseEndpoint(transport:)` | `AuthenticatedTransportFfi` | - | Remove noise endpoint |
| `createReceiptRequestMessage(...)` | `String, String, String, String, String?, String?` | `NoisePaymentMessage` | Create receipt request |
| `createReceiptConfirmationMessage(...)` | `String, String, String, String, String?, String?, String?` | `NoisePaymentMessage` | Create confirmation |
| `createNoiseErrorMessage(code:message:)` | `String, String` | `NoisePaymentMessage` | Create error message |
| `parseNoisePaymentMessage(json:)` | `String` | `NoisePaymentMessage` | Parse payment message |

---

## Executor FFI (Bitkit Integration)

This section documents the executor callback interfaces for integrating external wallet
implementations (like Bitkit) with Paykit.

### Network Enums

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `BitcoinNetworkFFI` | `BitcoinNetworkFfi` | `BitcoinNetworkFfi` | Bitcoin network enum |
| `LightningNetworkFFI` | `LightningNetworkFfi` | `LightningNetworkFfi` | Lightning network enum |

**BitcoinNetworkFFI Variants:**
- `Mainnet` (default)
- `Testnet`
- `Regtest`

**LightningNetworkFFI Variants:**
- `Mainnet` (default)
- `Testnet`
- `Regtest`

### Executor Callback Interfaces

#### BitcoinExecutorFFI

Mobile apps implement this protocol to provide Bitcoin wallet functionality:

```swift
// Swift
protocol BitcoinExecutorFfi {
    func sendToAddress(
        address: String,
        amountSats: UInt64,
        feeRate: Double?
    ) throws -> BitcoinTxResultFfi
    
    func estimateFee(
        address: String,
        amountSats: UInt64,
        targetBlocks: UInt32
    ) throws -> UInt64
    
    func getTransaction(txid: String) throws -> BitcoinTxResultFfi?
    
    func verifyTransaction(
        txid: String,
        address: String,
        amountSats: UInt64
    ) throws -> Bool
}
```

```kotlin
// Kotlin
interface BitcoinExecutorFfi {
    fun sendToAddress(
        address: String,
        amountSats: ULong,
        feeRate: Double?
    ): BitcoinTxResultFfi
    
    fun estimateFee(
        address: String,
        amountSats: ULong,
        targetBlocks: UInt
    ): ULong
    
    fun getTransaction(txid: String): BitcoinTxResultFfi?
    
    fun verifyTransaction(
        txid: String,
        address: String,
        amountSats: ULong
    ): Boolean
}
```

#### LightningExecutorFFI

Mobile apps implement this protocol to provide Lightning wallet functionality:

```swift
// Swift
protocol LightningExecutorFfi {
    func payInvoice(
        invoice: String,
        amountMsat: UInt64?,
        maxFeeMsat: UInt64?
    ) throws -> LightningPaymentResultFfi
    
    func decodeInvoice(invoice: String) throws -> DecodedInvoiceFfi
    
    func estimateFee(invoice: String) throws -> UInt64
    
    func getPayment(paymentHash: String) throws -> LightningPaymentResultFfi?
    
    func verifyPreimage(preimage: String, paymentHash: String) -> Bool
}
```

```kotlin
// Kotlin
interface LightningExecutorFfi {
    fun payInvoice(
        invoice: String,
        amountMsat: ULong?,
        maxFeeMsat: ULong?
    ): LightningPaymentResultFfi
    
    fun decodeInvoice(invoice: String): DecodedInvoiceFfi
    
    fun estimateFee(invoice: String): ULong
    
    fun getPayment(paymentHash: String): LightningPaymentResultFfi?
    
    fun verifyPreimage(preimage: String, paymentHash: String): Boolean
}
```

### Executor Result Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `BitcoinTxResultFFI` | `BitcoinTxResultFfi` | `BitcoinTxResultFfi` | Bitcoin transaction result |
| `LightningPaymentResultFFI` | `LightningPaymentResultFfi` | `LightningPaymentResultFfi` | Lightning payment result |
| `DecodedInvoiceFFI` | `DecodedInvoiceFfi` | `DecodedInvoiceFfi` | Decoded invoice info |
| `LightningPaymentStatusFFI` | `LightningPaymentStatusFfi` | `LightningPaymentStatusFfi` | Payment status enum |

**BitcoinTxResultFFI Fields:**
```rust
pub struct BitcoinTxResultFFI {
    pub txid: String,           // Transaction ID
    pub raw_tx: Option<String>, // Raw transaction hex
    pub vout: u32,              // Output index
    pub fee_sats: u64,          // Fee in satoshis
    pub fee_rate: f64,          // Fee rate (sat/vB)
    pub block_height: Option<u64>, // Confirmation height
    pub confirmations: u64,     // Number of confirmations
}
```

**LightningPaymentResultFFI Fields:**
```rust
pub struct LightningPaymentResultFFI {
    pub preimage: String,       // Payment preimage (hex)
    pub payment_hash: String,   // Payment hash (hex)
    pub amount_msat: u64,       // Amount in millisatoshis
    pub fee_msat: u64,          // Fee in millisatoshis
    pub hops: u32,              // Number of hops
    pub status: LightningPaymentStatusFFI,
}
```

**DecodedInvoiceFFI Fields:**
```rust
pub struct DecodedInvoiceFFI {
    pub payment_hash: String,   // Payment hash (hex)
    pub amount_msat: Option<u64>, // Amount if specified
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub payee: String,          // Payee pubkey
    pub expiry: u64,            // Expiry in seconds
    pub timestamp: u64,         // Creation timestamp
    pub expired: bool,          // Whether expired
}
```

**LightningPaymentStatusFFI Variants:**
- `Pending`
- `Succeeded`
- `Failed`

### PaykitClient Executor Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `newWithNetwork(bitcoinNetwork:lightningNetwork:)` | `BitcoinNetworkFfi, LightningNetworkFfi` | `PaykitClient` | Create client with network |
| `registerBitcoinExecutor(executor:)` | `BitcoinExecutorFfi` | - | Register Bitcoin executor |
| `registerLightningExecutor(executor:)` | `LightningExecutorFfi` | - | Register Lightning executor |
| `hasBitcoinExecutor()` | - | `Bool` | Check if Bitcoin executor registered |
| `hasLightningExecutor()` | - | `Bool` | Check if Lightning executor registered |
| `bitcoinNetwork()` | - | `BitcoinNetworkFfi` | Get configured Bitcoin network |
| `lightningNetwork()` | - | `LightningNetworkFfi` | Get configured Lightning network |

### Payment Execution Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `executePayment(methodId:endpoint:amountSats:metadata:)` | `String, String, UInt64, String?` | `PaymentExecutionResult` | Execute payment |
| `generatePaymentProof(methodId:executionDataJson:)` | `String, String` | `PaymentProofResult` | Generate proof |

**PaymentExecutionResult Fields:**
```rust
pub struct PaymentExecutionResult {
    pub execution_id: String,
    pub method_id: String,
    pub endpoint: String,
    pub amount_sats: u64,
    pub success: bool,
    pub executed_at: i64,
    pub execution_data_json: String,
    pub error: Option<String>,
}
```

**PaymentProofResult Fields:**
```rust
pub struct PaymentProofResult {
    pub proof_type: String,      // "bitcoin_txid" or "lightning_preimage"
    pub proof_data_json: String, // JSON with proof details
}
```

### Executor Async Bridge

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `ExecutorAsyncBridge` | `ExecutorAsyncBridge` | `ExecutorAsyncBridge` | Async executor wrapper |

**ExecutorAsyncBridge Methods:**

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `new()` | - | `ExecutorAsyncBridge` | Create with 30s timeout |
| `withTimeout(timeoutMs:)` | `UInt64` | `ExecutorAsyncBridge` | Create with custom timeout |
| `defaultTimeoutMs()` | - | `UInt64` | Get default timeout |

### Bitkit Integration Example

```swift
// Swift - Implementing BitcoinExecutorFfi with Bitkit wallet
class BitkitBitcoinExecutor: BitcoinExecutorFfi {
    private let wallet: BitkitWallet
    
    init(wallet: BitkitWallet) {
        self.wallet = wallet
    }
    
    func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFfi {
        let tx = try wallet.send(to: address, amount: amountSats, feeRate: feeRate ?? 1.0)
        return BitcoinTxResultFfi(
            txid: tx.txid,
            rawTx: tx.rawHex,
            vout: 0,
            feeSats: tx.fee,
            feeRate: tx.feeRate,
            blockHeight: nil,
            confirmations: 0
        )
    }
    
    func estimateFee(address: String, amountSats: UInt64, targetBlocks: UInt32) throws -> UInt64 {
        return try wallet.estimateFee(amount: amountSats, target: targetBlocks)
    }
    
    func getTransaction(txid: String) throws -> BitcoinTxResultFfi? {
        guard let tx = try wallet.getTransaction(txid: txid) else { return nil }
        return BitcoinTxResultFfi(
            txid: tx.txid,
            rawTx: nil,
            vout: 0,
            feeSats: tx.fee,
            feeRate: tx.feeRate,
            blockHeight: tx.blockHeight,
            confirmations: tx.confirmations
        )
    }
    
    func verifyTransaction(txid: String, address: String, amountSats: UInt64) throws -> Bool {
        return try wallet.verifyPayment(txid: txid, to: address, amount: amountSats)
    }
}

// Register with PaykitClient
let client = try PaykitClient.newWithNetwork(
    bitcoinNetwork: .mainnet,
    lightningNetwork: .mainnet
)
let executor = BitkitBitcoinExecutor(wallet: bitkitWallet)
try client.registerBitcoinExecutor(executor: executor)

// Execute payment
let result = try client.executePayment(
    methodId: "onchain",
    endpoint: "bc1q...",
    amountSats: 50000,
    metadata: nil
)

if result.success {
    let proof = try client.generatePaymentProof(
        methodId: "onchain",
        executionDataJson: result.executionDataJson
    )
    print("Payment proof: \(proof.proofDataJson)")
}
```

```kotlin
// Kotlin - Implementing BitcoinExecutorFfi with Bitkit wallet
class BitkitBitcoinExecutor(
    private val wallet: BitkitWallet
) : BitcoinExecutorFfi {
    
    override fun sendToAddress(
        address: String,
        amountSats: ULong,
        feeRate: Double?
    ): BitcoinTxResultFfi {
        val tx = wallet.send(address, amountSats, feeRate ?: 1.0)
        return BitcoinTxResultFfi(
            txid = tx.txid,
            rawTx = tx.rawHex,
            vout = 0u,
            feeSats = tx.fee,
            feeRate = tx.feeRate,
            blockHeight = null,
            confirmations = 0u
        )
    }
    
    override fun estimateFee(
        address: String,
        amountSats: ULong,
        targetBlocks: UInt
    ): ULong = wallet.estimateFee(amountSats, targetBlocks)
    
    override fun getTransaction(txid: String): BitcoinTxResultFfi? {
        val tx = wallet.getTransaction(txid) ?: return null
        return BitcoinTxResultFfi(
            txid = tx.txid,
            rawTx = null,
            vout = 0u,
            feeSats = tx.fee,
            feeRate = tx.feeRate,
            blockHeight = tx.blockHeight,
            confirmations = tx.confirmations
        )
    }
    
    override fun verifyTransaction(
        txid: String,
        address: String,
        amountSats: ULong
    ): Boolean = wallet.verifyPayment(txid, address, amountSats)
}

// Register with PaykitClient
val client = PaykitClient.newWithNetwork(
    bitcoinNetwork = BitcoinNetworkFfi.MAINNET,
    lightningNetwork = LightningNetworkFfi.MAINNET
)
val executor = BitkitBitcoinExecutor(bitkitWallet)
client.registerBitcoinExecutor(executor)

// Execute payment
val result = client.executePayment(
    methodId = "onchain",
    endpoint = "bc1q...",
    amountSats = 50000u,
    metadata = null
)

if (result.success) {
    val proof = client.generatePaymentProof(
        methodId = "onchain",
        executionDataJson = result.executionDataJson
    )
    println("Payment proof: ${proof.proofDataJson}")
}
```

---

## Transport Layer

### Transport Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `AuthenticatedTransportFFI` | `AuthenticatedTransportFfi` | `AuthenticatedTransportFfi` | Write transport |
| `UnauthenticatedTransportFFI` | `UnauthenticatedTransportFfi` | `UnauthenticatedTransportFfi` | Read transport |
| `StorageGetResult` | `StorageGetResult` | `StorageGetResult` | Get result |
| `StorageListResult` | `StorageListResult` | `StorageListResult` | List result |
| `StorageOperationResult` | `StorageOperationResult` | `StorageOperationResult` | Operation result |

### Callback Protocols

**PubkyAuthenticatedStorageCallback:**
```swift
protocol PubkyAuthenticatedStorageCallback {
    func get(path: String) -> StorageGetResult
    func put(path: String, content: String) -> StorageOperationResult
    func delete(path: String) -> StorageOperationResult
    func list(prefix: String) -> StorageListResult
}
```

**PubkyUnauthenticatedStorageCallback:**
```swift
protocol PubkyUnauthenticatedStorageCallback {
    func get(ownerPubkey: String, path: String) -> StorageGetResult
    func list(ownerPubkey: String, prefix: String) -> StorageListResult
}
```

### AuthenticatedTransportFfi Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `newMock(ownerPubkey:)` | `String` | `AuthenticatedTransportFfi` | Create mock transport |
| `fromCallback(ownerPubkey:callback:)` | `String, PubkyAuthenticatedStorageCallback` | `AuthenticatedTransportFfi` | Create from callback |
| `fromSessionJson(sessionJson:ownerPubkey:)` | `String, String` | `AuthenticatedTransportFfi` | Create from session |
| `ownerPubkey()` | - | `String` | Get owner pubkey |
| `isMock()` | - | `Bool` | Check if mock |
| `put(path:content:)` | `String, String` | - | Put content |
| `get(path:)` | `String` | `String?` | Get content |
| `delete(path:)` | `String` | - | Delete content |
| `list(prefix:)` | `String` | `[String]` | List entries |

### UnauthenticatedTransportFfi Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `newMock()` | - | `UnauthenticatedTransportFfi` | Create mock transport |
| `fromCallback(callback:)` | `PubkyUnauthenticatedStorageCallback` | `UnauthenticatedTransportFfi` | Create from callback |
| `fromConfigJson(configJson:)` | `String` | `UnauthenticatedTransportFfi` | Create from config |
| `fromAuthenticated(auth:)` | `AuthenticatedTransportFfi` | `UnauthenticatedTransportFfi` | Create from auth |
| `isMock()` | - | `Bool` | Check if mock |
| `get(ownerPubkey:path:)` | `String, String` | `String?` | Get content |
| `list(ownerPubkey:prefix:)` | `String, String` | `[String]` | List entries |

### Result Struct Initializers

**StorageGetResult:**
```swift
StorageGetResult(success: Bool, content: String?, error: String?)
```

**StorageListResult:**
```swift
StorageListResult(success: Bool, entries: [String], error: String?)
```

**StorageOperationResult:**
```swift
StorageOperationResult(success: Bool, error: String?)
```

---

## Interactive Protocol

### Interactive Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `PaykitMessageBuilder` | `PaykitMessageBuilder` | `PaykitMessageBuilder` | Message builder |
| `PaykitInteractiveManagerFFI` | `PaykitInteractiveManagerFfi` | `PaykitInteractiveManagerFfi` | Interactive manager |
| `ReceiptStore` | `ReceiptStore` | `ReceiptStore` | Receipt storage |
| `PaykitMessageType` | `PaykitMessageType` | `PaykitMessageType` | Message type enum |
| `ParsedMessage` | `ParsedMessage` | `ParsedMessage` | Parsed message enum |
| `PrivateEndpointOffer` | `PrivateEndpointOffer` | `PrivateEndpointOffer` | Endpoint offer |
| `ReceiptRequest` | `ReceiptRequest` | `ReceiptRequest` | Receipt request |
| `ErrorMessage` | `ErrorMessage` | `ErrorMessage` | Error message |
| `ReceiptGenerationResult` | `ReceiptGenerationResult` | `ReceiptGenerationResult` | Generation result |

### PaykitMessageType Variants
- `OfferPrivateEndpoint`
- `RequestReceipt`
- `ConfirmReceipt`
- `Ack`
- `Error`

### Global Functions

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `createMessageBuilder()` | - | `PaykitMessageBuilder` | Create message builder |
| `createReceiptStore()` | - | `ReceiptStore` | Create receipt store |
| `createInteractiveManager(store:)` | `ReceiptStore` | `PaykitInteractiveManagerFfi` | Create manager |

### PaykitMessageBuilder Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `createEndpointOffer(methodId:endpoint:)` | `String, String` | `String` | Create endpoint offer JSON |
| `createReceiptRequest(request:)` | `ReceiptRequest` | `String` | Create receipt request JSON |
| `createReceiptConfirm(receipt:)` | `ReceiptRequest` | `String` | Create confirmation JSON |
| `createAck()` | - | `String` | Create ack JSON |
| `createError(code:message:)` | `String, String` | `String` | Create error JSON |
| `parseMessage(messageJson:)` | `String` | `ParsedMessage` | Parse message |
| `getMessageType(messageJson:)` | `String` | `PaykitMessageType` | Get message type |

### ReceiptStore Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `saveReceipt(receipt:)` | `ReceiptRequest` | - | Save receipt |
| `getReceipt(receiptId:)` | `String` | `ReceiptRequest?` | Get receipt |
| `listReceipts()` | - | `[ReceiptRequest]` | List all receipts |
| `deleteReceipt(receiptId:)` | `String` | - | Delete receipt |
| `savePrivateEndpoint(peer:offer:)` | `String, PrivateEndpointOffer` | - | Save private endpoint |
| `getPrivateEndpoint(peer:methodId:)` | `String, String` | `PrivateEndpointOffer?` | Get private endpoint |
| `listPrivateEndpoints(peer:)` | `String` | `[PrivateEndpointOffer]` | List private endpoints |
| `clear()` | - | - | Clear all data |
| `exportReceiptsJson()` | - | `String` | Export as JSON |
| `importReceiptsJson(json:)` | `String` | `UInt32` | Import from JSON |

### ReceiptGeneratorCallback Protocol

```swift
protocol ReceiptGeneratorCallback {
    func generate(request: ReceiptRequest) -> ReceiptGenerationResult
}
```

### PaykitInteractiveManagerFfi Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `setGenerator(generator:)` | `ReceiptGeneratorCallback` | - | Set receipt generator |
| `handleMessage(messageJson:peerPubkey:localPubkey:)` | `String, String, String` | `String?` | Handle message |
| `createPaymentRequest(...)` | `String, String, String, String?, String?` | `String` | Create payment request |
| `handlePaymentResponse(responseJson:)` | `String` | `ReceiptRequest?` | Handle response |
| `createEndpointOffer(methodId:endpoint:)` | `String, String` | `String` | Create endpoint offer |
| `getStore()` | - | `ReceiptStore` | Get receipt store |
| `getReceipt(receiptId:)` | `String` | `ReceiptRequest?` | Get receipt |
| `listReceipts()` | - | `[ReceiptRequest]` | List receipts |
| `getPrivateEndpoint(peer:methodId:)` | `String, String` | `PrivateEndpointOffer?` | Get private endpoint |
| `listPrivateEndpoints(peer:)` | `String` | `[PrivateEndpointOffer]` | List private endpoints |

---

## Noise Protocol

### Noise Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `NoiseEndpointInfo` | `NoiseEndpointInfo` | `NoiseEndpointInfo` | Endpoint info |
| `NoiseConnectionStatus` | `NoiseConnectionStatus` | `NoiseConnectionStatus` | Connection status |
| `NoiseHandshakeResult` | `NoiseHandshakeResult` | `NoiseHandshakeResult` | Handshake result |
| `NoiseServerConfig` | `NoiseServerConfig` | `NoiseServerConfig` | Server config |
| `NoiseSessionInfo` | `NoiseSessionInfo` | `NoiseSessionInfo` | Session info |
| `NoiseServerStatus` | `NoiseServerStatus` | `NoiseServerStatus` | Server status |
| `NoisePaymentMessageType` | `NoisePaymentMessageType` | `NoisePaymentMessageType` | Message type |
| `NoisePaymentMessage` | `NoisePaymentMessage` | `NoisePaymentMessage` | Payment message |

### NoiseEndpointInfo Fields

```swift
struct NoiseEndpointInfo {
    var recipientPubkey: String  // z-base32 encoded
    var host: String             // IP or hostname
    var port: UInt16             // Port number
    var serverNoisePubkey: String // X25519, hex encoded
    var metadata: String?        // Optional metadata
}
```

### NoiseConnectionStatus Variants
- `Disconnected`
- `Connecting`
- `Handshaking`
- `Connected`
- `Failed`

### NoisePaymentMessageType Variants
- `ReceiptRequest`
- `ReceiptConfirmation`
- `PrivateEndpointOffer`
- `Error`
- `Ping`
- `Pong`

### Global Functions

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `discoverNoiseEndpoint(transport:recipientPubkey:)` | `UnauthenticatedTransportFfi, String` | `NoiseEndpointInfo?` | Discover endpoint |
| `publishNoiseEndpoint(transport:host:port:noisePubkey:metadata:)` | `AuthenticatedTransportFfi, String, UInt16, String, String?` | - | Publish endpoint |
| `removeNoiseEndpoint(transport:)` | `AuthenticatedTransportFfi` | - | Remove endpoint |
| `createNoiseServerConfig()` | - | `NoiseServerConfig` | Create default config |
| `createNoiseServerConfigWithPort(port:)` | `UInt16` | `NoiseServerConfig` | Create config with port |
| `createReceiptRequestMessage(...)` | See below | `NoisePaymentMessage` | Create receipt request |
| `createReceiptConfirmationMessage(...)` | See below | `NoisePaymentMessage` | Create confirmation |
| `createPrivateEndpointOfferMessage(...)` | See below | `NoisePaymentMessage` | Create endpoint offer |
| `createErrorMessage(code:message:)` | `String, String` | `NoisePaymentMessage` | Create error |
| `parsePaymentMessage(json:)` | `String` | `NoisePaymentMessage` | Parse message |

### Message Creation Functions

**createReceiptRequestMessage:**
```swift
func createReceiptRequestMessage(
    receiptId: String,
    payerPubkey: String,
    payeePubkey: String,
    methodId: String,
    amount: String?,
    currency: String?
) throws -> NoisePaymentMessage
```

**createReceiptConfirmationMessage:**
```swift
func createReceiptConfirmationMessage(
    receiptId: String,
    payerPubkey: String,
    payeePubkey: String,
    methodId: String,
    amount: String?,
    currency: String?,
    signature: String?
) throws -> NoisePaymentMessage
```

**createPrivateEndpointOfferMessage:**
```swift
func createPrivateEndpointOfferMessage(
    methodId: String,
    endpoint: String,
    expiresInSecs: UInt32
) throws -> NoisePaymentMessage
```

---

## Key Management

### Key Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `Ed25519Keypair` | `Ed25519Keypair` | `Ed25519Keypair` | Identity keypair |
| `X25519Keypair` | `X25519Keypair` | `X25519Keypair` | Noise keypair |
| `KeyBackup` | `KeyBackup` | `KeyBackup` | Encrypted backup |

### Ed25519Keypair Fields

```swift
struct Ed25519Keypair {
    var secretKeyHex: String  // 64 hex chars
    var publicKeyHex: String  // 64 hex chars
    var publicKeyZ32: String  // z-base32 encoded
}
```

### X25519Keypair Fields

```swift
struct X25519Keypair {
    var secretKeyHex: String  // 64 hex chars
    var publicKeyHex: String  // 64 hex chars
    var deviceId: String      // Device identifier
    var epoch: UInt32         // Key epoch
}
```

### Global Functions

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `generateEd25519Keypair()` | - | `Ed25519Keypair` | Generate new identity |
| `ed25519KeypairFromSecret(secretKeyHex:)` | `String` | `Ed25519Keypair` | Restore from secret |
| `deriveX25519Keypair(ed25519SecretHex:deviceId:epoch:)` | `String, String, UInt32` | `X25519Keypair` | Derive noise key |
| `signMessage(secretKeyHex:message:)` | `String, [UInt8]` | `String` | Sign message |
| `verifySignature(publicKeyHex:message:signatureHex:)` | `String, [UInt8], String` | `Bool` | Verify signature |
| `exportKeypairToBackup(secretKeyHex:password:)` | `String, String` | `KeyBackup` | Export encrypted |
| `importKeypairFromBackup(backup:password:)` | `KeyBackup, String` | `Ed25519Keypair` | Import backup |
| `formatPublicKeyZ32(publicKeyHex:)` | `String` | `String` | Format as z-base32 |
| `parsePublicKeyZ32(publicKeyZ32:)` | `String` | `String` | Parse z-base32 to hex |
| `generateDeviceId()` | - | `String` | Generate device ID |

---

## Scanner

### Scanner Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `ScannedUri` | `ScannedUri` | `ScannedUri` | Parsed URI |
| `UriType` | `UriType` | `UriType` | URI type enum |

### UriType Variants
- `Pubky`
- `Lightning`
- `Bitcoin`
- `PaykitRequest`
- `Unknown`

### ScannedUri Fields

```swift
struct ScannedUri {
    var uriType: UriType
    var rawUri: String
    var publicKey: String?
    var methodId: String?
    var amount: String?
    var memo: String?
}
```

### Global Functions (via PaykitClient)

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `parseScannedUri(scannedData:)` | `String` | `ScannedUri` | Parse scanned data |
| `isPaykitUri(scannedData:)` | `String` | `Bool` | Check if Paykit URI |
| `extractPublicKey(scannedData:)` | `String` | `String?` | Extract public key |
| `extractPaymentMethod(scannedData:)` | `String` | `String?` | Extract method |

---

## Storage

### Storage Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `StorageError` | `StorageError` | `StorageError` | Storage error |
| `StorageErrorCode` | `StorageErrorCode` | `StorageErrorCode` | Error code enum |
| `InMemoryStorage` | `InMemoryStorage` | `InMemoryStorage` | Memory storage |
| `CachedContact` | `CachedContact` | `CachedContact` | Cached contact |

### SecureStorage Trait

```swift
protocol SecureStorage {
    func get(key: String) throws -> String?
    func set(key: String, value: String) throws
    func delete(key: String) throws
    func clear() throws
    func listKeys() throws -> [String]
}
```

**Note:** Mobile apps implement this trait using platform-specific secure storage:
- iOS: Keychain
- Android: EncryptedSharedPreferences

---

## Async Bridge

### Async Types

| Rust Type | Swift Type | Kotlin Type | Description |
|-----------|------------|-------------|-------------|
| `AsyncHandle` | `AsyncHandle` | `AsyncHandle` | Cancellable handle |
| `AsyncRuntime` | `AsyncRuntime` | `AsyncRuntime` | Tokio runtime |
| `DirectoryOperationsAsync` | `DirectoryOperationsAsync` | `DirectoryOperationsAsync` | Async directory ops |
| `Debouncer` | `Debouncer` | `Debouncer` | Debounce utility |
| `RetryConfig` | `RetryConfig` | `RetryConfig` | Retry configuration |

### DirectoryOperationsAsync Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `publishPaymentEndpoint(transport:methodId:endpointData:)` | `AuthenticatedTransportFfi, String, String` | - | Publish endpoint |
| `removePaymentEndpoint(transport:methodId:)` | `AuthenticatedTransportFfi, String` | - | Remove endpoint |
| `fetchSupportedPayments(transport:ownerPubkey:)` | `UnauthenticatedTransportFfi, String` | `[PaymentMethod]` | Fetch methods |
| `addContact(transport:contactPubkey:)` | `AuthenticatedTransportFfi, String` | - | Add contact |
| `removeContact(transport:contactPubkey:)` | `AuthenticatedTransportFfi, String` | - | Remove contact |
| `listContacts(transport:)` | `AuthenticatedTransportFfi` | `[String]` | List contacts |

---

## Usage Examples

### Swift (iOS)

```swift
import PaykitMobile

// Create client
let client = try PaykitClient()

// Create mock transport
let auth = AuthenticatedTransportFfi.newMock(ownerPubkey: "my_pubkey")
let unauth = try UnauthenticatedTransportFfi.fromAuthenticated(auth: auth)

// Publish payment endpoint
try client.publishPaymentEndpoint(
    transport: auth,
    methodId: "lightning",
    endpointData: "lnbc1..."
)

// Discover noise endpoint
if let endpoint = try client.discoverNoiseEndpoint(
    transport: unauth,
    recipientPubkey: "recipient_pubkey"
) {
    print("Connect to \(endpoint.host):\(endpoint.port)")
}

// Generate keys
let identity = try generateEd25519Keypair()
let noiseKey = try deriveX25519Keypair(
    ed25519SecretHex: identity.secretKeyHex,
    deviceId: generateDeviceId(),
    epoch: 0
)
```

### Kotlin (Android)

```kotlin
import com.paykit.mobile.*

// Create client
val client = PaykitClient()

// Create mock transport
val auth = AuthenticatedTransportFfi.newMock("my_pubkey")
val unauth = UnauthenticatedTransportFfi.fromAuthenticated(auth)

// Publish payment endpoint
client.publishPaymentEndpoint(
    transport = auth,
    methodId = "lightning",
    endpointData = "lnbc1..."
)

// Discover noise endpoint
val endpoint = client.discoverNoiseEndpoint(
    transport = unauth,
    recipientPubkey = "recipient_pubkey"
)
endpoint?.let {
    println("Connect to ${it.host}:${it.port}")
}

// Generate keys
val identity = generateEd25519Keypair()
val noiseKey = deriveX25519Keypair(
    ed25519SecretHex = identity.secretKeyHex,
    deviceId = generateDeviceId(),
    epoch = 0u
)
```

---

## Notes

1. **Thread Safety**: All exported types are thread-safe and can be used from any thread.

2. **Async Operations**: Use `DirectoryOperationsAsync` for non-blocking operations on mobile.

3. **Error Handling**: All throwing functions throw `PaykitMobileError`.

4. **Naming Conventions**:
   - Rust `snake_case` → Swift/Kotlin `camelCase`
   - Rust `Result<T>` → Swift `throws` / Kotlin exceptions
   - Rust `Option<T>` → Swift `T?` / Kotlin `T?`

5. **Generated Bindings Location**:
   - Swift: `ios-demo/PaykitDemo/PaykitDemo/PaykitDemo/PaykitMobile.swift`
   - Kotlin: `android-demo/app/src/main/java/com/paykit/mobile/paykit_mobile.kt`

6. **Regenerating Bindings**:
   ```bash
   # Build library
   cargo build --release -p paykit-mobile
   
   # Generate Swift
   uniffi-bindgen generate \
     --library target/release/libpaykit_mobile.dylib \
     -l swift -o ios-demo/PaykitDemo/PaykitDemo/PaykitDemo
   
   # Generate Kotlin
   uniffi-bindgen generate \
     --library target/release/libpaykit_mobile.dylib \
     -l kotlin -o kotlin/generated
   ```


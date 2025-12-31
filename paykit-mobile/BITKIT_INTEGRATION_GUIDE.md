# Bitkit Integration Guide

This guide explains how to integrate Paykit with Bitkit iOS and Android applications to enable real Bitcoin and Lightning payments.

> **IMPORTANT: SecureStorage FFI Not Connected**
>
> The Rust `SecureStorage` module is NOT connected to platform-native secure storage (iOS Keychain / Android Keystore). All methods currently return `SecureStorageError::unsupported`. 
>
> For production, you must either:
> 1. Implement the FFI bridge to connect Rust to platform-native secure storage
> 2. Handle key storage in platform-native code before passing keys to PaykitClient
>
> See `PRODUCTION_CHECKLIST.md` for detailed integration requirements.

## Overview

Paykit provides a flexible payment method framework that can use any wallet as the underlying payment executor. For Bitkit integration, you implement two callback interfaces:

- **BitcoinExecutorFFI** - For on-chain Bitcoin payments
- **LightningExecutorFFI** - For Lightning Network payments

These interfaces bridge Bitkit's wallet functionality to Paykit's payment infrastructure.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Bitkit App (Swift/Kotlin)                    │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │     BitkitBitcoinExecutor / BitkitLightningExecutor       │  │
│  │     (Implements FFI callback interfaces)                   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                               │                                  │
│                               ▼                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    PaykitClient                            │  │
│  │  • registerBitcoinExecutor()                              │  │
│  │  • registerLightningExecutor()                            │  │
│  │  • executePayment()                                        │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼ (Rust FFI)
┌─────────────────────────────────────────────────────────────────┐
│                      Paykit Rust Core                            │
│  • Payment method selection                                      │
│  • Address/invoice validation                                    │
│  • Receipt generation                                            │
│  • Proof verification                                            │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

### iOS (Swift)

```swift
import PaykitMobile

// 1. Create PaykitClient with network configuration
let client = try PaykitClient.newWithNetwork(
    bitcoinNetwork: .mainnet,
    lightningNetwork: .mainnet
)

// 2. Implement executor interfaces
class MyBitcoinExecutor: BitcoinExecutorFFI {
    func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFfi {
        // Call your wallet's send function
        let tx = try wallet.send(to: address, amount: amountSats, feeRate: feeRate)
        return BitcoinTxResultFfi(
            txid: tx.txid,
            rawTx: tx.rawHex,
            vout: tx.outputIndex,
            feeSats: tx.fee,
            feeRate: tx.feeRate,
            blockHeight: tx.blockHeight,
            confirmations: tx.confirmations
        )
    }
    
    func estimateFee(address: String, amountSats: UInt64, targetBlocks: UInt32) throws -> UInt64 {
        return try wallet.estimateFee(for: amountSats, target: targetBlocks)
    }
    
    func getTransaction(txid: String) throws -> BitcoinTxResultFfi? {
        guard let tx = try wallet.getTransaction(txid: txid) else { return nil }
        return BitcoinTxResultFfi(/* ... */)
    }
    
    func verifyTransaction(txid: String, address: String, amountSats: UInt64) throws -> Bool {
        return try wallet.verifyTransaction(txid: txid, address: address, amount: amountSats)
    }
}

// 3. Register executors
try client.registerBitcoinExecutor(executor: MyBitcoinExecutor())
try client.registerLightningExecutor(executor: MyLightningExecutor())

// 4. Execute payments
let result = try client.executePayment(
    methodId: "lightning",
    endpoint: invoice,
    amountSats: 1000,
    metadataJson: nil
)

if result.success {
    print("Payment succeeded! Data: \(result.executionDataJson)")
}
```

### Android (Kotlin)

```kotlin
import com.paykit.mobile.*

// 1. Create PaykitClient with network configuration
val client = PaykitClient.newWithNetwork(
    bitcoinNetwork = BitcoinNetworkFfi.MAINNET,
    lightningNetwork = LightningNetworkFfi.MAINNET
)

// 2. Implement executor interfaces
class MyBitcoinExecutor(private val wallet: MyWallet) : BitcoinExecutorFfi {
    override fun sendToAddress(
        address: String,
        amountSats: ULong,
        feeRate: Double?
    ): BitcoinTxResultFfi {
        val tx = wallet.send(address, amountSats, feeRate)
        return BitcoinTxResultFfi(
            txid = tx.txid,
            rawTx = tx.rawHex,
            vout = tx.outputIndex,
            feeSats = tx.fee,
            feeRate = tx.feeRate,
            blockHeight = tx.blockHeight,
            confirmations = tx.confirmations
        )
    }
    
    override fun estimateFee(address: String, amountSats: ULong, targetBlocks: UInt): ULong {
        return wallet.estimateFee(amountSats, targetBlocks)
    }
    
    override fun getTransaction(txid: String): BitcoinTxResultFfi? {
        return wallet.getTransaction(txid)?.let { tx ->
            BitcoinTxResultFfi(/* ... */)
        }
    }
    
    override fun verifyTransaction(txid: String, address: String, amountSats: ULong): Boolean {
        return wallet.verifyTransaction(txid, address, amountSats)
    }
}

// 3. Register executors
client.registerBitcoinExecutor(MyBitcoinExecutor(wallet))
client.registerLightningExecutor(MyLightningExecutor(node))

// 4. Execute payments
val result = client.executePayment(
    methodId = "lightning",
    endpoint = invoice,
    amountSats = 1000UL,
    metadataJson = null
)

if (result.success) {
    println("Payment succeeded! Data: ${result.executionDataJson}")
}
```

## Interface Reference

### BitcoinExecutorFFI

| Method | Description |
|--------|-------------|
| `sendToAddress(address, amountSats, feeRate)` | Send Bitcoin to an address. Returns `BitcoinTxResultFfi` with txid, fees, etc. |
| `estimateFee(address, amountSats, targetBlocks)` | Estimate fee for a transaction. Returns fee in satoshis. |
| `getTransaction(txid)` | Get transaction details by txid. Returns `BitcoinTxResultFfi?`. |
| `verifyTransaction(txid, address, amountSats)` | Verify transaction matches expected address/amount. Returns `Boolean`. |

### LightningExecutorFFI

| Method | Description |
|--------|-------------|
| `payInvoice(invoice, amountMsat, maxFeeMsat)` | Pay a BOLT11 invoice. Returns `LightningPaymentResultFfi` with preimage. |
| `decodeInvoice(invoice)` | Decode invoice without paying. Returns `DecodedInvoiceFfi`. |
| `estimateFee(invoice)` | Estimate routing fee. Returns fee in millisatoshis. |
| `getPayment(paymentHash)` | Get payment status by hash. Returns `LightningPaymentResultFfi?`. |
| `verifyPreimage(preimage, paymentHash)` | Verify preimage matches hash. Returns `Boolean`. |

### Result Types

#### BitcoinTxResultFfi

```swift
struct BitcoinTxResultFfi {
    txid: String           // Transaction ID (64 hex chars)
    rawTx: String?         // Raw transaction hex (optional)
    vout: UInt32           // Output index
    feeSats: UInt64        // Fee paid in satoshis
    feeRate: Double        // Fee rate in sat/vB
    blockHeight: UInt64?   // Block height if confirmed
    confirmations: UInt64  // Number of confirmations
}
```

#### LightningPaymentResultFfi

```swift
struct LightningPaymentResultFfi {
    preimage: String       // Payment preimage (64 hex chars)
    paymentHash: String    // Payment hash (64 hex chars)
    amountMsat: UInt64     // Amount paid in millisatoshis
    feeMsat: UInt64        // Fee paid in millisatoshis
    hops: UInt32           // Number of hops in route
    status: LightningPaymentStatusFfi  // .succeeded, .pending, .failed
}
```

## Network Configuration

Paykit supports mainnet, testnet, and regtest networks:

```swift
// Mainnet (default)
let client = try PaykitClient.new()

// Testnet
let client = try PaykitClient.newWithNetwork(
    bitcoinNetwork: .testnet,
    lightningNetwork: .testnet
)

// Regtest (local development)
let client = try PaykitClient.newWithNetwork(
    bitcoinNetwork: .regtest,
    lightningNetwork: .regtest
)
```

The network configuration affects:
- Address validation (mainnet vs testnet prefixes)
- Invoice validation (lnbc vs lntb prefixes)
- Plugin behavior

## Error Handling

All executor methods should throw `PaykitMobileError` on failure:

```swift
// Swift
func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFfi {
    do {
        return try wallet.send(...)
    } catch {
        throw PaykitMobileError.Transport(message: "Send failed: \(error.localizedDescription)")
    }
}
```

```kotlin
// Kotlin
override fun sendToAddress(address: String, amountSats: ULong, feeRate: Double?): BitcoinTxResultFfi {
    try {
        return wallet.send(...)
    } catch (e: Exception) {
        throw PaykitMobileException.Transport("Send failed: ${e.message}")
    }
}
```

Available error types:
- `Transport` - Network or I/O errors
- `Validation` - Invalid input
- `NotFound` - Resource not found
- `Internal` - Unexpected errors

## Thread Safety

**Important**: All executor methods may be called from any thread. Ensure your implementations are thread-safe:

```swift
class ThreadSafeExecutor: BitcoinExecutorFFI {
    private let wallet: BitkitWallet
    private let queue = DispatchQueue(label: "com.bitkit.executor")
    
    func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFfi {
        return try queue.sync {
            try wallet.send(...)
        }
    }
}
```

## Payment Flow

1. **User initiates payment** - App receives payment destination (address or invoice)

2. **Validate endpoint** - Paykit validates the address/invoice format:
   ```swift
   let isValid = client.validateEndpoint(methodId: "lightning", endpoint: invoice)
   ```

3. **Execute payment** - Paykit calls your executor:
   ```swift
   let result = try client.executePayment(
       methodId: "lightning",
       endpoint: invoice,
       amountSats: 1000,
       metadataJson: nil
   )
   ```

4. **Generate proof** - Extract payment proof for receipts:
   ```swift
   let proof = try client.generatePaymentProof(
       methodId: result.methodId,
       executionDataJson: result.executionDataJson
   )
   // proof.proofType: "lightning_preimage" or "bitcoin_txid"
   // proof.proofDataJson: Contains preimage or txid
   ```

5. **Track status** - Monitor payment status:
   ```swift
   if let status = client.getPaymentStatus(receiptId: receiptId) {
       switch status.status {
       case .confirmed: print("Payment confirmed!")
       case .pending: print("Waiting for confirmation...")
       case .failed: print("Payment failed: \(status.error ?? "")")
       }
   }
   ```

## Example Files

Complete example implementations are provided:

- **Swift**: `paykit-mobile/swift/BitkitExecutorExample.swift`
- **Kotlin**: `paykit-mobile/kotlin/BitkitExecutorExample.kt`

These examples include:
- Full executor implementations
- Placeholder wallet/node protocols
- Integration helper class
- Usage examples

## Testing

For testing without a real wallet, use mock executors:

```swift
class MockBitcoinExecutor: BitcoinExecutorFFI {
    func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFfi {
        // Return mock result
        return BitcoinTxResultFfi(
            txid: "mock_\(UUID().uuidString)",
            rawTx: nil,
            vout: 0,
            feeSats: 210,
            feeRate: 1.5,
            blockHeight: nil,
            confirmations: 0
        )
    }
    // ... other mock methods
}
```

## Checklist

Before going to production:

- [ ] Implement all `BitcoinExecutorFFI` methods
- [ ] Implement all `LightningExecutorFFI` methods
- [ ] Handle all error cases with appropriate `PaykitMobileError` types
- [ ] Ensure thread safety in executor implementations
- [ ] Test on testnet before mainnet
- [ ] Add logging for debugging
- [ ] Handle wallet lock/unlock states
- [ ] Implement proper fee estimation
- [ ] Test payment proof generation

## Support

For questions or issues:
- GitHub Issues: https://github.com/synonymdev/paykit-rs/issues
- Bitkit Discord: https://discord.gg/synonymdev

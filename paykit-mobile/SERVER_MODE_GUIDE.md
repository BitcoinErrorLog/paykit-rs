# Server Mode Guide

This guide explains how Paykit mobile apps operate in server mode to receive payments via the Noise protocol. Server mode allows apps to listen for incoming connections, perform Noise handshakes, and process payment requests.

## Overview

Server mode enables Paykit apps to:
- Listen for incoming TCP connections
- Perform Noise_IK protocol handshakes
- Receive and process encrypted payment messages
- Generate receipts and send confirmations
- Handle multiple concurrent connections

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Server Mode Flow                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Start Server (NWListener / ServerSocket)                │
│     │                                                        │
│     ├─► Bind to port                                        │
│     ├─► Create Noise server config                         │
│     └─► Begin listening                                     │
│                                                              │
│  2. Accept Connection                                       │
│     │                                                        │
│     ├─► New TCP connection                                 │
│     ├─► Create ServerConnection                            │
│     └─► Begin handshake                                     │
│                                                              │
│  3. Noise Handshake                                         │
│     │                                                        │
│     ├─► Receive first message from client                  │
│     ├─► Call FfiNoiseManager.acceptConnection()            │
│     ├─► Send handshake response                             │
│     └─► Mark handshake complete                             │
│                                                              │
│  4. Receive Payment Message                                  │
│     │                                                        │
│     ├─► Receive encrypted message                          │
│     ├─► Decrypt using Noise session                        │
│     ├─► Parse JSON message                                 │
│     ├─► Process via PaykitInteractiveManagerFFI            │
│     ├─► Generate receipt (if RequestReceipt)               │
│     └─► Encrypt and send response                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## iOS Implementation

### Starting the Server

```swift
import Network

let paymentService = NoisePaymentService.shared

// Start server on any available port
let status = try await paymentService.startServer(port: 0)

print("Server running on port: \(status.port)")
print("Noise public key: \(status.noisePubkeyHex)")
```

### Server Configuration

The server uses `NWListener` from Network.framework:

```swift
// Create TCP parameters
let parameters = NWParameters.tcp
parameters.allowLocalEndpointReuse = true

// Create listener
let listener = try NWListener(using: parameters, on: .any)

// Handle new connections
listener.newConnectionHandler = { connection in
    handleNewConnection(connection)
}

// Start listening
listener.start(queue: serverQueue)
```

### Noise Handshake

```swift
private func handleServerHandshake(_ serverConnection: ServerConnection, data: Data) {
    guard let noiseManager = serverConnection.noiseManager else { return }
    
    do {
        // Accept connection (server-side handshake)
        let acceptResult = try noiseManager.acceptConnection(firstMsg: data)
        
        // Store session ID
        serverConnection.sessionId = acceptResult.sessionId
        serverConnection.isHandshakeComplete = true
        
        // Send handshake response
        serverConnection.connection.send(
            content: acceptResult.responseMessage,
            completion: .contentProcessed { error in
                if error == nil {
                    // Handshake complete, start receiving messages
                    receiveNextMessage(serverConnection)
                }
            }
        )
    } catch {
        print("Handshake failed: \(error)")
        serverConnection.connection.cancel()
    }
}
```

### Processing Messages

```swift
private func handleServerMessage(_ serverConnection: ServerConnection, data: Data) {
    guard let manager = interactiveManager,
          let sessionId = serverConnection.sessionId,
          let noiseManager = serverConnection.noiseManager else { return }
    
    do {
        // Decrypt message
        let plaintext = try noiseManager.decrypt(sessionId: sessionId, ciphertext: data)
        let messageJson = String(data: plaintext, encoding: .utf8)!
        
        // Process message
        let responseJson = try manager.handleMessage(
            messageJson: messageJson,
            peerPubkey: serverConnection.clientPubkeyHex ?? "unknown",
            myPubkey: keyManager.publicKeyZ32
        )
        
        // Send response if available
        if let response = responseJson {
            sendEncryptedResponse(serverConnection, responseJson: response)
        }
    } catch {
        print("Error processing message: \(error)")
        sendErrorResponse(serverConnection, code: "PROCESSING_ERROR", message: error.localizedDescription)
    }
}
```

### Receipt Generation

The server uses `ReceiptGeneratorCallback` to generate receipts:

```swift
class ServerReceiptGenerator: ReceiptGeneratorCallback {
    func generateReceipt(request: ReceiptRequest) -> ReceiptGenerationResult {
        // Generate invoice (e.g., Lightning invoice)
        let invoice = generateLightningInvoice(for: request)
        
        // Update receipt metadata
        var metadata = parseMetadata(request.metadataJson)
        metadata["invoice"] = invoice
        metadata["confirmed_at"] = ISO8601DateFormatter().string(from: Date())
        
        let confirmedReceipt = ReceiptRequest(
            receiptId: request.receiptId,
            payer: request.payer,
            payee: request.payee,
            methodId: request.methodId,
            amount: request.amount,
            currency: request.currency,
            metadataJson: jsonString(from: metadata)
        )
        
        return ReceiptGenerationResult.ok(receipt: confirmedReceipt)
    }
}
```

### Background Operation

iOS servers can run in the background using background tasks:

```swift
private func registerBackgroundTask() {
    backgroundTask = UIApplication.shared.beginBackgroundTask { [weak self] in
        self?.endBackgroundTask()
    }
}

private func endBackgroundTask() {
    if backgroundTask != .invalid {
        UIApplication.shared.endBackgroundTask(backgroundTask)
        backgroundTask = .invalid
    }
}
```

## Android Implementation

### Starting the Server

```kotlin
val paymentService = NoisePaymentService.getInstance(context)

// Start server on any available port
val status = paymentService.startServer(port = 0)

println("Server running on port: ${status.port}")
println("Noise public key: ${status.noisePubkeyHex}")
```

### Server Configuration

The server uses `ServerSocket`:

```kotlin
// Create server socket
val serverSocket = ServerSocket(0) // 0 = any available port

// Accept connections in coroutine
serverScope.launch {
    while (isActive) {
        try {
            val clientSocket = serverSocket.accept()
            handleNewConnection(clientSocket)
        } catch (e: Exception) {
            Log.e("NoisePaymentService", "Error accepting connection: ${e.message}")
        }
    }
}
```

### Noise Handshake

```kotlin
private suspend fun handleServerHandshake(
    serverConnection: ServerConnection,
    data: ByteArray
) = withContext(Dispatchers.IO) {
    val noiseManager = serverConnection.noiseManager ?: return@withContext
    
    try {
        // Accept connection (server-side handshake)
        val acceptResult = noiseManager.acceptConnection(data)
        
        // Store session ID
        serverConnection.sessionId = acceptResult.sessionId
        serverConnection.isHandshakeComplete = true
        
        // Send handshake response
        val outputStream = DataOutputStream(serverConnection.socket.getOutputStream())
        outputStream.write(acceptResult.responseMessage)
        outputStream.flush()
        
        // Start receiving messages
        receiveNextMessage(serverConnection)
    } catch (e: Exception) {
        Log.e("NoisePaymentService", "Handshake failed: ${e.message}")
        serverConnection.socket.close()
    }
}
```

### Processing Messages

```kotlin
private suspend fun handleServerMessage(
    serverConnection: ServerConnection,
    data: ByteArray
) {
    val manager = interactiveManager ?: return
    val sessionId = serverConnection.sessionId ?: return
    val noiseManager = serverConnection.noiseManager ?: return
    
    try {
        // Decrypt message
        val plaintext = noiseManager.decrypt(sessionId, data)
        val messageJson = String(plaintext, Charsets.UTF_8)
        
        // Process message
        val responseJson = manager.handleMessage(
            messageJson = messageJson,
            peerPubkey = serverConnection.clientPubkeyHex ?: "unknown",
            myPubkey = keyManager.getPublicKeyZ32() ?: "unknown"
        )
        
        // Send response if available
        responseJson?.let { response ->
            sendEncryptedResponse(serverConnection, response)
        }
    } catch (e: Exception) {
        Log.e("NoisePaymentService", "Error processing message: ${e.message}")
        sendErrorResponse(serverConnection, "PROCESSING_ERROR", e.message ?: "Unknown error")
    }
}
```

### Receipt Generation

```kotlin
class ServerReceiptGenerator(
    private val onPaymentRequest: ((ReceiptRequest) -> Unit)? = null
) : ReceiptGeneratorCallback {
    
    override fun generateReceipt(request: ReceiptRequest): ReceiptGenerationResult {
        // Notify about pending request
        onPaymentRequest?.invoke(request)
        
        // Generate invoice
        val invoice = generateLightningInvoice(request)
        
        // Update metadata
        val metadata = JSONObject(request.metadataJson).apply {
            put("invoice", invoice)
            put("confirmed_at", Instant.now().toString())
        }
        
        val confirmedReceipt = ReceiptRequest(
            receiptId = request.receiptId,
            payer = request.payer,
            payee = request.payee,
            methodId = request.methodId,
            amount = request.amount,
            currency = request.currency,
            metadataJson = metadata.toString()
        )
        
        return ReceiptGenerationResult(
            success = true,
            receipt = confirmedReceipt,
            error = null
        )
    }
}
```

### Foreground Service

Android servers should run as foreground services:

```kotlin
// In AndroidManifest.xml
<service
    android:name=".NoisePaymentService"
    android:enabled="true"
    android:exported="false"
    android:foregroundServiceType="dataSync" />

// Start foreground service
val serviceIntent = Intent(context, NoisePaymentService::class.java)
ContextCompat.startForegroundService(context, serviceIntent)
```

## Message Types

### RequestReceipt

Client requests a receipt for a payment:

```json
{
  "type": "RequestReceipt",
  "receipt_id": "rcpt_123",
  "payer": "pubkey_z32...",
  "payee": "pubkey_z32...",
  "method_id": "lightning",
  "amount": "1000",
  "currency": "SAT",
  "metadata": "{}"
}
```

Server responds with `ConfirmReceipt` containing the invoice.

### ConfirmReceipt

Server confirms receipt with payment endpoint:

```json
{
  "type": "ConfirmReceipt",
  "receipt_id": "rcpt_123",
  "payer": "pubkey_z32...",
  "payee": "pubkey_z32...",
  "method_id": "lightning",
  "amount": "1000",
  "currency": "SAT",
  "metadata": "{\"invoice\": \"lnbc10u1p3...\"}"
}
```

### OfferPrivateEndpoint

Client offers a private payment endpoint:

```json
{
  "type": "OfferPrivateEndpoint",
  "method_id": "lightning",
  "endpoint": "lnbc10u1p3...",
  "expires_at": 1234567890
}
```

Server responds with `Ack`.

### Error

Error response:

```json
{
  "type": "Error",
  "code": "WRONG_PAYEE",
  "message": "I am not the intended payee"
}
```

## Connection Lifecycle

1. **Connection Established**: TCP connection accepted
2. **Handshake**: Noise_IK protocol handshake
3. **Message Exchange**: Encrypted payment messages
4. **Connection Closed**: Either party closes connection

### Handling Disconnections

```swift
// iOS
connection.stateUpdateHandler = { state in
    switch state {
    case .failed(let error), .cancelled:
        serverConnections.removeValue(forKey: connectionId)
    default:
        break
    }
}
```

```kotlin
// Android
try {
    // Read from socket
    val data = inputStream.readBytes()
    // Process message
} catch (e: IOException) {
    // Connection closed
    serverConnections.remove(connectionId)
    socket.close()
}
```

## Multiple Concurrent Connections

Both implementations support multiple concurrent connections:

```swift
// iOS
private var serverConnections: [UUID: ServerConnection] = [:]

func handleNewConnection(_ connection: NWConnection) {
    let connectionId = UUID()
    let serverConnection = ServerConnection(
        id: connectionId,
        connection: connection,
        noiseManager: serverNoiseManager
    )
    serverConnections[connectionId] = serverConnection
    // Handle connection...
}
```

```kotlin
// Android
private val serverConnections = mutableMapOf<String, ServerConnection>()

fun handleNewConnection(socket: Socket) {
    val connectionId = UUID.randomUUID().toString()
    val serverConnection = ServerConnection(
        id = connectionId,
        socket = socket,
        noiseManager = serverNoiseManager
    )
    serverConnections[connectionId] = serverConnection
    // Handle connection...
}
```

## Testing

### Manual Testing

1. Start server in one app instance
2. Connect from another app instance (or test client)
3. Send payment request
4. Verify receipt generation and confirmation

### Unit Testing

```swift
// iOS
func testServerHandshake() async throws {
    let service = NoisePaymentService.shared
    let status = try await service.startServer(port: 0)
    XCTAssertTrue(status.isRunning)
    XCTAssertNotNil(status.port)
}
```

```kotlin
// Android
@Test
fun testServerHandshake() = runTest {
    val service = NoisePaymentService.getInstance(context)
    val status = service.startServer(port = 0)
    assertTrue(status.isRunning)
    assertNotNull(status.port)
}
```

## Troubleshooting

### Server Won't Start

- Check port availability
- Verify network permissions
- Check firewall settings
- Review logs for errors

### Handshake Fails

- Verify server keypair is valid
- Check client is using correct server public key
- Ensure Noise protocol version matches
- Review handshake logs

### Messages Not Received

- Verify connection is established
- Check handshake completed successfully
- Ensure message format is correct
- Review decryption logs

### Receipt Generation Fails

- Verify `ReceiptGeneratorCallback` is set
- Check invoice generation logic
- Ensure metadata format is valid
- Review error logs

## References

- [Noise Protocol Specification](https://noiseprotocol.org/)
- [Payment Flow Guide](./PAYMENT_FLOW_GUIDE.md)
- [Key Architecture Guide](./KEY_ARCHITECTURE.md)


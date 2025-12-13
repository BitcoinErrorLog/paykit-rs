package com.paykit.demo.ui

import androidx.compose.animation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.paykit.mobile.KeyManager
import com.paykit.demo.storage.ContactStorage
import com.paykit.demo.storage.ReceiptStorage
import com.paykit.demo.storage.StoredReceipt
import com.pubky.noise.*
import kotlinx.coroutines.*
import java.io.DataInputStream
import java.io.DataOutputStream
import java.net.Socket
import java.util.*

/**
 * Interactive payment screen using Noise protocol for secure payment negotiation.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentScreen(
    keyManager: KeyManager,
    onPaymentComplete: () -> Unit = {}
) {
    val context = LocalContext.current
    var recipientUri by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("1000") }
    var currency by remember { mutableStateOf("SAT") }
    var paymentMethod by remember { mutableStateOf("lightning") }
    var description by remember { mutableStateOf("") }
    
    var paymentStatus by remember { mutableStateOf(PaymentStatus.IDLE) }
    var isProcessing by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var confirmedReceiptId by remember { mutableStateOf<String?>(null) }
    
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()
    
    var showSuccessDialog by remember { mutableStateOf(false) }
    var showErrorDialog by remember { mutableStateOf(false) }
    
    val canSend = recipientUri.isNotBlank() && amount.isNotBlank() && !isProcessing
    
    // Success Dialog
    if (showSuccessDialog) {
        AlertDialog(
            onDismissRequest = { 
                showSuccessDialog = false
                recipientUri = ""
                amount = "1000"
                description = ""
                paymentStatus = PaymentStatus.IDLE
                onPaymentComplete()
            },
            title = { Text("Payment Successful!") },
            text = { Text("Receipt ID: ${confirmedReceiptId ?: "Unknown"}") },
            confirmButton = {
                TextButton(onClick = {
                    showSuccessDialog = false
                    recipientUri = ""
                    amount = "1000"
                    description = ""
                    paymentStatus = PaymentStatus.IDLE
                    onPaymentComplete()
                }) {
                    Text("OK")
                }
            }
        )
    }
    
    // Error Dialog
    if (showErrorDialog) {
        AlertDialog(
            onDismissRequest = { showErrorDialog = false },
            title = { Text("Payment Error") },
            text = { Text(errorMessage ?: "Unknown error") },
            confirmButton = {
                TextButton(onClick = { showErrorDialog = false }) {
                    Text("OK")
                }
            }
        )
    }
    
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        // Header
        Text(
            text = "Send Payment",
            style = MaterialTheme.typography.headlineMedium
        )
        
        // Payment Form Card
        Card(
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                // Recipient
                OutlinedTextField(
                    value = recipientUri,
                    onValueChange = { recipientUri = it },
                    label = { Text("Recipient") },
                    placeholder = { Text("pubky://... or contact name") },
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isProcessing,
                    singleLine = true,
                    leadingIcon = {
                        Icon(Icons.Default.Person, contentDescription = null)
                    }
                )
                
                // Amount Row
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    OutlinedTextField(
                        value = amount,
                        onValueChange = { amount = it },
                        label = { Text("Amount") },
                        modifier = Modifier.weight(1f),
                        enabled = !isProcessing,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true,
                        leadingIcon = {
                            Icon(Icons.Default.AttachMoney, contentDescription = null)
                        }
                    )
                    
                    // Currency Dropdown
                    var currencyExpanded by remember { mutableStateOf(false) }
                    ExposedDropdownMenuBox(
                        expanded = currencyExpanded,
                        onExpandedChange = { currencyExpanded = it },
                        modifier = Modifier.width(100.dp)
                    ) {
                        OutlinedTextField(
                            value = currency,
                            onValueChange = {},
                            readOnly = true,
                            label = { Text("Currency") },
                            modifier = Modifier.menuAnchor(),
                            enabled = !isProcessing,
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(currencyExpanded) }
                        )
                        ExposedDropdownMenu(
                            expanded = currencyExpanded,
                            onDismissRequest = { currencyExpanded = false }
                        ) {
                            listOf("SAT", "BTC", "USD").forEach { option ->
                                DropdownMenuItem(
                                    text = { Text(option) },
                                    onClick = {
                                        currency = option
                                        currencyExpanded = false
                                    }
                                )
                            }
                        }
                    }
                }
                
                // Payment Method
                Text("Payment Method", style = MaterialTheme.typography.labelLarge)
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = paymentMethod == "lightning",
                        onClick = { paymentMethod = "lightning" },
                        label = { Text("Lightning") },
                        enabled = !isProcessing,
                        leadingIcon = if (paymentMethod == "lightning") {
                            { Icon(Icons.Default.Check, contentDescription = null) }
                        } else null
                    )
                    FilterChip(
                        selected = paymentMethod == "onchain",
                        onClick = { paymentMethod = "onchain" },
                        label = { Text("On-Chain") },
                        enabled = !isProcessing,
                        leadingIcon = if (paymentMethod == "onchain") {
                            { Icon(Icons.Default.Check, contentDescription = null) }
                        } else null
                    )
                }
                
                // Description
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description (optional)") },
                    placeholder = { Text("Payment for...") },
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isProcessing,
                    singleLine = true,
                    leadingIcon = {
                        Icon(Icons.Default.Description, contentDescription = null)
                    }
                )
            }
        }
        
        // Status Card
        AnimatedVisibility(
            visible = paymentStatus != PaymentStatus.IDLE,
            enter = fadeIn() + expandVertically(),
            exit = fadeOut() + shrinkVertically()
        ) {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(
                    containerColor = paymentStatus.color.copy(alpha = 0.1f)
                )
            ) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Icon(
                            imageVector = paymentStatus.icon,
                            contentDescription = null,
                            tint = paymentStatus.color
                        )
                        Text(
                            text = paymentStatus.title,
                            style = MaterialTheme.typography.titleMedium
                        )
                    }
                    
                    if (isProcessing) {
                        LinearProgressIndicator(
                            progress = paymentStatus.progress,
                            modifier = Modifier.fillMaxWidth()
                        )
                    }
                    
                    Text(
                        text = paymentStatus.message,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
        
        // Action Buttons
        Column(
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Button(
                onClick = {
                    scope.launch {
                        isProcessing = true
                        try {
                            performPayment(
                                context = context,
                                keyManager = keyManager,
                                recipientUri = recipientUri,
                                amount = amount,
                                currency = currency,
                                method = paymentMethod,
                                description = description,
                                onStatusChange = { status -> paymentStatus = status },
                                onComplete = { receiptId ->
                                    confirmedReceiptId = receiptId
                                    paymentStatus = PaymentStatus.COMPLETED
                                    showSuccessDialog = true
                                },
                                onError = { error ->
                                    errorMessage = error
                                    paymentStatus = PaymentStatus.FAILED
                                    showErrorDialog = true
                                }
                            )
                        } catch (e: Exception) {
                            errorMessage = e.message ?: "Unknown error"
                            paymentStatus = PaymentStatus.FAILED
                            showErrorDialog = true
                        } finally {
                            isProcessing = false
                        }
                    }
                },
                modifier = Modifier.fillMaxWidth(),
                enabled = canSend
            ) {
                if (isProcessing) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(24.dp),
                        color = MaterialTheme.colorScheme.onPrimary,
                        strokeWidth = 2.dp
                    )
                    Spacer(Modifier.width(8.dp))
                    Text("Processing...")
                } else {
                    Icon(Icons.Default.Send, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("Send Payment")
                }
            }
            
            if (isProcessing) {
                OutlinedButton(
                    onClick = {
                        isProcessing = false
                        paymentStatus = PaymentStatus.CANCELLED
                    },
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Text("Cancel")
                }
            }
        }
        
        Spacer(Modifier.weight(1f))
    }
}

enum class PaymentStatus(
    val title: String,
    val message: String,
    val progress: Float,
    val icon: androidx.compose.ui.graphics.vector.ImageVector,
    val color: Color
) {
    IDLE(
        title = "Ready",
        message = "Ready to send payment",
        progress = 0f,
        icon = Icons.Default.HourglassEmpty,
        color = Color.Gray
    ),
    RESOLVING_RECIPIENT(
        title = "Resolving recipient...",
        message = "Looking up recipient information",
        progress = 0.1f,
        icon = Icons.Default.Search,
        color = Color.Blue
    ),
    DISCOVERING_ENDPOINT(
        title = "Discovering endpoint...",
        message = "Finding recipient's payment endpoint",
        progress = 0.2f,
        icon = Icons.Default.Explore,
        color = Color.Blue
    ),
    CONNECTING(
        title = "Connecting...",
        message = "Establishing encrypted connection",
        progress = 0.4f,
        icon = Icons.Default.Wifi,
        color = Color.Blue
    ),
    HANDSHAKING(
        title = "Handshaking...",
        message = "Performing secure Noise protocol handshake",
        progress = 0.5f,
        icon = Icons.Default.Handshake,
        color = Color.Blue
    ),
    SENDING_REQUEST(
        title = "Sending request...",
        message = "Sending payment request over secure channel",
        progress = 0.6f,
        icon = Icons.Default.Send,
        color = Color.Blue
    ),
    AWAITING_CONFIRMATION(
        title = "Awaiting confirmation...",
        message = "Waiting for recipient to confirm payment",
        progress = 0.8f,
        icon = Icons.Default.HourglassTop,
        color = Color.Blue
    ),
    COMPLETED(
        title = "Completed!",
        message = "Payment completed and receipt stored",
        progress = 1f,
        icon = Icons.Default.CheckCircle,
        color = Color(0xFF4CAF50)
    ),
    FAILED(
        title = "Failed",
        message = "Payment failed",
        progress = 0f,
        icon = Icons.Default.Error,
        color = Color(0xFFF44336)
    ),
    CANCELLED(
        title = "Cancelled",
        message = "Payment was cancelled",
        progress = 0f,
        icon = Icons.Default.Cancel,
        color = Color(0xFFFF9800)
    )
}

/**
 * Perform the interactive payment using Noise protocol.
 */
private suspend fun performPayment(
    context: android.content.Context,
    keyManager: KeyManager,
    recipientUri: String,
    amount: String,
    currency: String,
    method: String,
    description: String,
    onStatusChange: (PaymentStatus) -> Unit,
    onComplete: (String) -> Unit,
    onError: (String) -> Unit
) = withContext(Dispatchers.IO) {
    var socket: Socket? = null
    var noiseManager: FfiNoiseManager? = null
    var sessionId: String? = null
    
    try {
        // Step 1: Resolve recipient
        onStatusChange(PaymentStatus.RESOLVING_RECIPIENT)
        val payeePubkey = resolveRecipient(context, keyManager, recipientUri)
        
        // Step 2: Discover endpoint
        onStatusChange(PaymentStatus.DISCOVERING_ENDPOINT)
        val noiseEndpoint = discoverNoiseEndpoint(payeePubkey)
            ?: throw PaymentException("Recipient has no noise:// endpoint published")
        
        // Step 3: Parse endpoint and connect
        onStatusChange(PaymentStatus.CONNECTING)
        val (host, port, serverPk) = parseNoiseEndpoint(noiseEndpoint)
        socket = Socket(host, port)
        
        // Step 4: Perform Noise handshake
        onStatusChange(PaymentStatus.HANDSHAKING)
        val (manager, sid) = performHandshake(keyManager, socket, serverPk)
        noiseManager = manager
        sessionId = sid
        
        // Step 5: Send payment request
        onStatusChange(PaymentStatus.SENDING_REQUEST)
        sendPaymentRequest(
            socket = socket,
            noiseManager = noiseManager,
            sessionId = sessionId,
            payeePubkey = payeePubkey,
            payerPubkey = keyManager.publicKeyZ32.value,
            amount = amount,
            currency = currency,
            method = method
        )
        
        // Step 6: Await confirmation
        onStatusChange(PaymentStatus.AWAITING_CONFIRMATION)
        val receipt = receiveConfirmation(
            context = context,
            socket = socket,
            noiseManager = noiseManager,
            sessionId = sessionId,
            keyManager = keyManager,
            amount = amount.toLongOrNull() ?: 0,
            currency = currency,
            method = method,
            description = description
        )
        
        onComplete(receipt.id)
        
    } catch (e: Exception) {
        onError(e.message ?: "Unknown error")
    } finally {
        sessionId?.let { sid ->
            noiseManager?.removeSession(sid)
        }
        socket?.close()
    }
}

private fun resolveRecipient(context: android.content.Context, keyManager: KeyManager, uri: String): String {
    // Extract public key from pubky:// URI or contact name
    if (uri.startsWith("pubky://")) {
        return uri.removePrefix("pubky://")
    }
    
    // Try to find in contacts
    val currentIdentity = keyManager.currentIdentityName.value ?: "default"
    val storage = ContactStorage(context, currentIdentity)
    val contacts = storage.listContacts()
    
    contacts.find { it.name.equals(uri, ignoreCase = true) }?.let {
        return it.publicKey
    }
    
    // Assume it's a raw public key
    return uri
}

private fun discoverNoiseEndpoint(payeePubkey: String): String? {
    // TODO: In a real implementation, query the Pubky directory for noise endpoint
    // For demo purposes, return null to indicate no endpoint
    return null
}

private fun parseNoiseEndpoint(endpoint: String): Triple<String, Int, ByteArray> {
    // Format: noise://host:port@pubkey_hex
    require(endpoint.startsWith("noise://")) { "Invalid noise endpoint" }
    
    val withoutPrefix = endpoint.removePrefix("noise://")
    val parts = withoutPrefix.split("@")
    require(parts.size == 2) { "Invalid noise endpoint format" }
    
    val hostPort = parts[0]
    val pkHex = parts[1]
    
    val colonIndex = hostPort.lastIndexOf(':')
    require(colonIndex > 0) { "Invalid host:port format" }
    
    val host = hostPort.substring(0, colonIndex)
    val port = hostPort.substring(colonIndex + 1).toInt()
    
    require(pkHex.length == 64) { "Invalid public key length" }
    val serverPk = pkHex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    
    return Triple(host, port, serverPk)
}

private fun performHandshake(
    keyManager: KeyManager,
    socket: Socket,
    serverPk: ByteArray
): Pair<FfiNoiseManager, String> {
    // Get identity seed
    val secretKey = keyManager.getSecretKeyBytes()
        ?: throw PaymentException("No identity configured")
    
    val deviceId = android.os.Build.MODEL.toByteArray()
    
    val config = FfiMobileConfig(
        autoReconnect = false,
        maxReconnectAttempts = 0u,
        reconnectDelayMs = 0u,
        batterySaver = false,
        chunkSize = 32768u
    )
    
    val noiseManager = FfiNoiseManager.newClient(
        config = config,
        clientSeed = secretKey,
        clientKid = "paykit-android",
        deviceId = deviceId
    )
    
    // Step 1: Initiate connection
    val result = noiseManager.initiateConnection(serverPk, null)
    
    // Step 2: Send first message
    val output = DataOutputStream(socket.getOutputStream())
    output.writeInt(result.firstMessage.size)
    output.write(result.firstMessage)
    output.flush()
    
    // Step 3: Receive server response
    val input = DataInputStream(socket.getInputStream())
    val responseLength = input.readInt()
    val response = ByteArray(responseLength)
    input.readFully(response)
    
    // Step 4: Complete handshake
    val sessionId = noiseManager.completeConnection(result.sessionId, response)
    
    return Pair(noiseManager, sessionId)
}

private fun sendPaymentRequest(
    socket: Socket,
    noiseManager: FfiNoiseManager,
    sessionId: String,
    payeePubkey: String,
    payerPubkey: String,
    amount: String,
    currency: String,
    method: String
) {
    // Create payment request JSON
    val requestJson = """
        {
            "type": "payment_request",
            "receipt_id": "${UUID.randomUUID()}",
            "payer_pubkey": "$payerPubkey",
            "payee_pubkey": "$payeePubkey",
            "method_id": "$method",
            "amount": "$amount",
            "currency": "$currency"
        }
    """.trimIndent()
    
    // Encrypt
    val plaintext = requestJson.toByteArray()
    val ciphertext = noiseManager.encrypt(sessionId, plaintext)
    
    // Send with length prefix
    val output = DataOutputStream(socket.getOutputStream())
    output.writeInt(ciphertext.size)
    output.write(ciphertext)
    output.flush()
}

private fun receiveConfirmation(
    context: android.content.Context,
    socket: Socket,
    noiseManager: FfiNoiseManager,
    sessionId: String,
    keyManager: KeyManager,
    amount: Long,
    currency: String,
    method: String,
    description: String
): StoredReceipt {
    // Receive length prefix
    val input = DataInputStream(socket.getInputStream())
    val length = input.readInt()
    
    // Receive ciphertext
    val ciphertext = ByteArray(length)
    input.readFully(ciphertext)
    
    // Decrypt
    val plaintext = noiseManager.decrypt(sessionId, ciphertext)
    val json = String(plaintext)
    
    // Parse response - in a real implementation, use proper JSON parsing
    // For demo, create a receipt
    val receiptId = UUID.randomUUID().toString()
    
    // Store receipt
    val currentIdentity = keyManager.currentIdentityName.value ?: "default"
    val storage = ReceiptStorage(context, currentIdentity)
    val receipt = StoredReceipt(
        id = receiptId,
        payer = keyManager.publicKeyZ32.value,
        payee = "",
        amount = amount,
        currency = currency,
        method = method,
        timestamp = System.currentTimeMillis(),
        status = "completed",
        notes = description.ifEmpty { null }
    )
    storage.saveReceipt(receipt)
    
    return receipt
}

class PaymentException(message: String) : Exception(message)


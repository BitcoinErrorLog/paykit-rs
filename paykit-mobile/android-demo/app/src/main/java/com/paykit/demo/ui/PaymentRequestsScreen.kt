package com.paykit.demo.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.paykit.demo.PaykitClientWrapper
import com.paykit.demo.storage.PaymentRequestStorage
import com.paykit.demo.storage.PaymentRequestStatus
import com.paykit.demo.storage.RequestDirection
import com.paykit.demo.storage.StoredPaymentRequest
import java.text.SimpleDateFormat
import java.util.*

/**
 * Payment Requests Screen
 *
 * Displays and manages payment requests including:
 * - Pending requests (accept/decline)
 * - Create new requests (persisted to EncryptedSharedPreferences)
 * - Request history
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentRequestsScreen(
    paykitClient: PaykitClientWrapper? = null
) {
    val context = LocalContext.current
    val storage = remember { PaymentRequestStorage(context) }
    
    var recipientPubkey by remember { mutableStateOf("") }
    var requestAmount by remember { mutableStateOf(1000L) }
    var requestDescription by remember { mutableStateOf("") }
    var selectedMethod by remember { mutableStateOf("lightning") }
    var hasExpiry by remember { mutableStateOf(true) }
    var expiryHours by remember { mutableStateOf(24) }
    var showError by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf("") }

    // Load requests from storage
    var allRequests by remember { mutableStateOf(storage.listRequests()) }
    val pendingRequests = allRequests.filter { it.status == PaymentRequestStatus.PENDING }
    val requestHistory = allRequests.filter { it.status != PaymentRequestStatus.PENDING }
    
    // Check for expired requests on load
    LaunchedEffect(Unit) {
        storage.checkExpirations()
        allRequests = storage.listRequests()
    }
    
    fun refreshRequests() {
        storage.checkExpirations()
        allRequests = storage.listRequests()
    }
    
    fun createRequest() {
        val client = paykitClient ?: return
        val myPubkey = "pk1demo..." // TODO: Get from KeyManager
        
        val expirySeconds = if (hasExpiry) expiryHours.toLong() * 3600 else null
        
        val ffiRequest = client.createPaymentRequest(
            fromPubkey = myPubkey,
            toPubkey = recipientPubkey,
            amountSats = requestAmount,
            currency = "SAT",
            methodId = selectedMethod,
            description = requestDescription,
            expiresInSecs = expirySeconds?.toULong()
        )
        
        if (ffiRequest != null) {
            val storedRequest = StoredPaymentRequest.fromFFI(ffiRequest, RequestDirection.OUTGOING)
            storage.addRequest(storedRequest)
            refreshRequests()
            
            // Reset form
            recipientPubkey = ""
            requestDescription = ""
        } else {
            errorMessage = "Failed to create payment request"
            showError = true
        }
    }
    
    fun handleAccept(request: StoredPaymentRequest) {
        storage.updateStatus(request.id, PaymentRequestStatus.ACCEPTED)
        refreshRequests()
    }
    
    fun handleDecline(request: StoredPaymentRequest) {
        storage.updateStatus(request.id, PaymentRequestStatus.DECLINED)
        refreshRequests()
    }
    
    fun deleteRequest(id: String) {
        storage.deleteRequest(id)
        refreshRequests()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Payment Requests") },
                actions = {
                    IconButton(onClick = { refreshRequests() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                }
            )
        },
        snackbarHost = {
            if (showError) {
                Snackbar(
                    action = {
                        TextButton(onClick = { showError = false }) {
                            Text("Dismiss")
                        }
                    }
                ) {
                    Text(errorMessage)
                }
            }
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Pending Requests Section
            item {
                Text(
                    text = "Pending Requests (${pendingRequests.size})",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(top = 16.dp)
                )
            }

            items(pendingRequests, key = { it.id }) { request ->
                PendingRequestCard(
                    request = request,
                    onAccept = { handleAccept(request) },
                    onDecline = { handleDecline(request) }
                )
            }

            if (pendingRequests.isEmpty()) {
                item {
                    EmptyState("No pending requests")
                }
            }

            // Create Request Section
            item {
                Card(
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp)
                    ) {
                        Text(
                            text = "Create Payment Request",
                            style = MaterialTheme.typography.titleMedium
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        OutlinedTextField(
                            value = recipientPubkey,
                            onValueChange = { recipientPubkey = it },
                            label = { Text("Recipient Public Key") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        OutlinedTextField(
                            value = requestAmount.toString(),
                            onValueChange = { it.toLongOrNull()?.let { a -> requestAmount = a } },
                            label = { Text("Amount (sats)") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            FilterChip(
                                selected = selectedMethod == "lightning",
                                onClick = { selectedMethod = "lightning" },
                                label = { Text("Lightning") }
                            )
                            FilterChip(
                                selected = selectedMethod == "onchain",
                                onClick = { selectedMethod = "onchain" },
                                label = { Text("On-Chain") }
                            )
                        }

                        Spacer(modifier = Modifier.height(8.dp))

                        OutlinedTextField(
                            value = requestDescription,
                            onValueChange = { requestDescription = it },
                            label = { Text("Description") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        Row(
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Checkbox(
                                checked = hasExpiry,
                                onCheckedChange = { hasExpiry = it }
                            )
                            Text("Set Expiry")
                        }

                        if (hasExpiry) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text("Expires in $expiryHours hours")
                            }
                            Slider(
                                value = expiryHours.toFloat(),
                                onValueChange = { expiryHours = it.toInt() },
                                valueRange = 1f..168f,
                                steps = 166,
                                modifier = Modifier.fillMaxWidth()
                            )
                        }

                        Spacer(modifier = Modifier.height(16.dp))

                        Button(
                            onClick = { createRequest() },
                            modifier = Modifier.fillMaxWidth(),
                            enabled = recipientPubkey.isNotEmpty() && paykitClient != null
                        ) {
                            Text("Create Request")
                        }
                    }
                }
            }

            // History Section
            item {
                Text(
                    text = "History",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(top = 16.dp)
                )
            }

            items(requestHistory, key = { it.id }) { request ->
                RequestHistoryCard(request)
            }

            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
    }
}

@Composable
fun PendingRequestCard(
    request: StoredPaymentRequest,
    onAccept: () -> Unit,
    onDecline: () -> Unit
) {
    val dateFormat = remember { SimpleDateFormat("MMM d, HH:mm", Locale.getDefault()) }

    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Icon(
                            if (request.direction == RequestDirection.INCOMING) 
                                Icons.Default.ArrowDownward 
                            else 
                                Icons.Default.ArrowUpward,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                            tint = if (request.direction == RequestDirection.INCOMING) 
                                Color(0xFF4CAF50) 
                            else 
                                Color(0xFF2196F3)
                        )
                        Text(
                            text = request.counterpartyName,
                            style = MaterialTheme.typography.titleSmall
                        )
                    }
                    Text(
                        text = request.description,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        text = "${request.amountSats} sats",
                        style = MaterialTheme.typography.titleSmall
                    )
                    Text(
                        text = request.methodId,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            request.expiresAtDate?.let { expires ->
                Spacer(modifier = Modifier.height(8.dp))
                Row(
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        Icons.Default.Schedule,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp),
                        tint = Color(0xFFFFA500)
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = "Expires ${dateFormat.format(expires)}",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFFFFA500)
                    )
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Button(
                    onClick = onAccept,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Color(0xFF4CAF50)
                    ),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Accept")
                }
                OutlinedButton(
                    onClick = onDecline,
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = Color.Red
                    ),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Decline")
                }
            }
        }
    }
}

@Composable
fun RequestHistoryCard(request: StoredPaymentRequest) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp)
                ) {
                    Icon(
                        if (request.direction == RequestDirection.INCOMING) 
                            Icons.Default.ArrowDownward 
                        else 
                            Icons.Default.ArrowUpward,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp),
                        tint = if (request.direction == RequestDirection.INCOMING) 
                            Color(0xFF4CAF50) 
                        else 
                            Color(0xFF2196F3)
                    )
                    Text(
                        text = request.counterpartyName,
                        style = MaterialTheme.typography.titleSmall
                    )
                }
                Text(
                    text = request.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = "${request.amountSats} sats",
                    style = MaterialTheme.typography.bodySmall
                )
                StatusChip(request.status)
            }
        }
    }
}

@Composable
fun StatusChip(status: PaymentRequestStatus) {
    val (color, text) = when (status) {
        PaymentRequestStatus.PENDING -> Color(0xFFFFA500) to "Pending"
        PaymentRequestStatus.ACCEPTED -> Color(0xFF4CAF50) to "Accepted"
        PaymentRequestStatus.DECLINED -> Color.Red to "Declined"
        PaymentRequestStatus.EXPIRED -> Color.Gray to "Expired"
        PaymentRequestStatus.PAID -> Color(0xFF2196F3) to "Paid"
    }

    Surface(
        color = color.copy(alpha = 0.2f),
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = color,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
        )
    }
}

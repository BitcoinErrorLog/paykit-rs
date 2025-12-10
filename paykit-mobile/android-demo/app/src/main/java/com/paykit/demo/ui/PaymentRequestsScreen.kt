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
import androidx.compose.ui.unit.dp
import java.text.SimpleDateFormat
import java.util.*

/**
 * Payment Requests Screen
 *
 * Displays and manages payment requests including:
 * - Pending requests (accept/decline)
 * - Create new requests
 * - Request history
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentRequestsScreen() {
    var recipientPubkey by remember { mutableStateOf("") }
    var requestAmount by remember { mutableStateOf(1000L) }
    var requestDescription by remember { mutableStateOf("") }
    var selectedMethod by remember { mutableStateOf("lightning") }
    var hasExpiry by remember { mutableStateOf(true) }
    var expiryHours by remember { mutableStateOf(24) }

    val pendingRequests = remember {
        mutableStateListOf(
            PaymentRequestData(
                id = "1",
                fromName = "Alice",
                fromPubkey = "pk1alice...",
                amountSats = 5000,
                methodId = "lightning",
                description = "Split dinner bill",
                createdAt = Date(System.currentTimeMillis() - 3600000),
                expiresAt = Date(System.currentTimeMillis() + 86400000),
                status = RequestStatus.PENDING
            ),
            PaymentRequestData(
                id = "2",
                fromName = "Bob",
                fromPubkey = "pk1bob...",
                amountSats = 2500,
                methodId = "lightning",
                description = "Concert tickets",
                createdAt = Date(System.currentTimeMillis() - 7200000),
                expiresAt = null,
                status = RequestStatus.PENDING
            )
        )
    }

    val requestHistory = remember {
        listOf(
            PaymentRequestData(
                id = "3",
                fromName = "Charlie",
                fromPubkey = "pk1charlie...",
                amountSats = 1000,
                methodId = "lightning",
                description = "Coffee",
                createdAt = Date(System.currentTimeMillis() - 86400000),
                expiresAt = null,
                status = RequestStatus.PAID
            )
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Payment Requests") },
                actions = {
                    IconButton(onClick = { /* Refresh */ }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                }
            )
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
                    text = "Pending Requests",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(top = 16.dp)
                )
            }

            items(pendingRequests) { request ->
                PendingRequestCard(
                    request = request,
                    onAccept = {
                        pendingRequests.remove(request)
                    },
                    onDecline = {
                        pendingRequests.remove(request)
                    }
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
                            onClick = { /* Create request */ },
                            modifier = Modifier.fillMaxWidth(),
                            enabled = recipientPubkey.isNotEmpty()
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

            items(requestHistory) { request ->
                RequestHistoryCard(request)
            }

            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
    }
}

@Composable
fun PendingRequestCard(
    request: PaymentRequestData,
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
                    Text(
                        text = request.fromName,
                        style = MaterialTheme.typography.titleSmall
                    )
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

            request.expiresAt?.let { expires ->
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
fun RequestHistoryCard(request: PaymentRequestData) {
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
                Text(
                    text = request.fromName,
                    style = MaterialTheme.typography.titleSmall
                )
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
fun StatusChip(status: RequestStatus) {
    val (color, text) = when (status) {
        RequestStatus.PENDING -> Color(0xFFFFA500) to "Pending"
        RequestStatus.ACCEPTED -> Color(0xFF4CAF50) to "Accepted"
        RequestStatus.DECLINED -> Color.Red to "Declined"
        RequestStatus.EXPIRED -> Color.Gray to "Expired"
        RequestStatus.PAID -> Color(0xFF2196F3) to "Paid"
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

data class PaymentRequestData(
    val id: String,
    val fromName: String,
    val fromPubkey: String,
    val amountSats: Long,
    val methodId: String,
    val description: String,
    val createdAt: Date,
    val expiresAt: Date?,
    val status: RequestStatus
)

enum class RequestStatus {
    PENDING,
    ACCEPTED,
    DECLINED,
    EXPIRED,
    PAID
}

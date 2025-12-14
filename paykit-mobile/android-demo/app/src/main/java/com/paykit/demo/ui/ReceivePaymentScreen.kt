// ReceivePaymentScreen.kt
// Receive Payment Screen (Server Mode)
//
// This screen allows users to receive payments via Noise protocol.
// It displays connection info for payers and shows incoming payment requests.

package com.paykit.demo.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.paykit.demo.storage.StoredReceipt
import com.paykit.demo.viewmodel.NoiseReceiveViewModel

/**
 * Screen for receiving payments via Noise protocol
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReceivePaymentScreen(
    viewModel: NoiseReceiveViewModel = viewModel()
) {
    val context = LocalContext.current
    
    val isListening by viewModel.isListening.collectAsState()
    val listeningPort by viewModel.listeningPort.collectAsState()
    val noisePubkeyHex by viewModel.noisePubkeyHex.collectAsState()
    val pendingRequests by viewModel.pendingRequests.collectAsState()
    val recentReceipts by viewModel.recentReceipts.collectAsState()
    
    var showQRDialog by remember { mutableStateOf(false) }
    var isPublishedToDirectory by remember { mutableStateOf(false) }
    
    LaunchedEffect(Unit) {
        viewModel.loadRecentReceipts()
    }
    
    // QR Code Dialog
    if (showQRDialog) {
        AlertDialog(
            onDismissRequest = { showQRDialog = false },
            title = { Text("Share Connection") },
            text = {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    // QR Code placeholder
                    Surface(
                        modifier = Modifier.size(200.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant,
                        shape = MaterialTheme.shapes.medium
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Icon(
                                    Icons.Default.QrCode,
                                    contentDescription = null,
                                    modifier = Modifier.size(80.dp)
                                )
                                Text("QR Code", style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                    
                    // Connection string
                    viewModel.getConnectionInfo()?.let { info ->
                        Text(
                            text = info,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace
                        )
                    }
                }
            },
            confirmButton = {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = {
                        viewModel.getConnectionInfo()?.let { info ->
                            copyToClipboard(context, info)
                        }
                    }) {
                        Icon(Icons.Default.ContentCopy, contentDescription = null)
                        Spacer(Modifier.width(4.dp))
                        Text("Copy")
                    }
                    TextButton(onClick = { showQRDialog = false }) {
                        Text("Done")
                    }
                }
            }
        )
    }
    
    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        // Header
        item {
            Text(
                text = "Receive Payments",
                style = MaterialTheme.typography.headlineMedium
            )
        }
        
        // Server Status Card
        item {
            ServerStatusCard(
                isListening = isListening,
                port = listeningPort,
                onToggle = {
                    if (isListening) {
                        viewModel.stopListening()
                    } else {
                        viewModel.startListening()
                    }
                }
            )
        }
        
        // Connection Info Card (when listening)
        if (isListening) {
            item {
                ConnectionInfoCard(
                    noisePubkey = noisePubkeyHex,
                    connectionInfo = viewModel.getConnectionInfo(),
                    onShowQR = { showQRDialog = true },
                    onCopy = { info ->
                        copyToClipboard(context, info)
                    }
                )
            }
            
            // Directory Publishing Card
            item {
                DirectoryPublishingCard(
                    isPublished = isPublishedToDirectory,
                    onToggle = { isPublishedToDirectory = it },
                    onUnpublish = { isPublishedToDirectory = false }
                )
            }
        }
        
        // Pending Requests
        if (pendingRequests.isNotEmpty()) {
            item {
                Text(
                    text = "Pending Requests",
                    style = MaterialTheme.typography.titleMedium
                )
            }
            
            items(pendingRequests) { request ->
                PendingRequestCard(
                    request = request,
                    onAccept = { viewModel.acceptRequest(request) },
                    onDecline = { viewModel.declineRequest(request) }
                )
            }
        }
        
        // Recent Receipts
        item {
            Text(
                text = "Recent Receipts",
                style = MaterialTheme.typography.titleMedium
            )
        }
        
        if (recentReceipts.isEmpty()) {
            item {
                Card(
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = "No receipts yet",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp)
                    )
                }
            }
        } else {
            items(recentReceipts) { receipt ->
                ReceiptCard(receipt = receipt)
            }
        }
    }
}

@Composable
private fun DirectoryPublishingCard(
    isPublished: Boolean,
    onToggle: (Boolean) -> Unit,
    onUnpublish: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    "Publish to Directory",
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium
                )
                Switch(
                    checked = isPublished,
                    onCheckedChange = onToggle
                )
            }
            
            if (isPublished) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Icon(
                        Icons.Default.CheckCircle,
                        contentDescription = null,
                        tint = Color(0xFF4CAF50)
                    )
                    Text(
                        "Noise endpoint is publicly discoverable",
                        style = MaterialTheme.typography.bodySmall
                    )
                }
                
                OutlinedButton(
                    onClick = onUnpublish,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text("Unpublish")
                }
            } else {
                Text(
                    "Endpoint is not publicly discoverable",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
private fun ServerStatusCard(
    isListening: Boolean,
    port: Int?,
    onToggle: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Surface(
                    modifier = Modifier.size(12.dp),
                    shape = MaterialTheme.shapes.small,
                    color = if (isListening) Color(0xFF4CAF50) else Color.Gray
                ) {}
                
                Text(
                    text = if (isListening) "Listening for Payments" else "Not Listening",
                    style = MaterialTheme.typography.titleMedium
                )
            }
            
            if (isListening && port != null) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Icon(
                        Icons.Default.Wifi,
                        contentDescription = null,
                        tint = Color(0xFF4CAF50)
                    )
                    Text(
                        text = "Port: $port",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            
            Button(
                onClick = onToggle,
                modifier = Modifier.fillMaxWidth(),
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (isListening) 
                        MaterialTheme.colorScheme.error 
                    else 
                        MaterialTheme.colorScheme.primary
                )
            ) {
                Icon(
                    if (isListening) Icons.Default.Stop else Icons.Default.PlayArrow,
                    contentDescription = null
                )
                Spacer(Modifier.width(8.dp))
                Text(if (isListening) "Stop Listening" else "Start Listening")
            }
        }
    }
}

@Composable
private fun ConnectionInfoCard(
    noisePubkey: String?,
    connectionInfo: String?,
    onShowQR: () -> Unit,
    onCopy: (String) -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Connection Info",
                    style = MaterialTheme.typography.titleMedium
                )
                
                IconButton(onClick = onShowQR) {
                    Icon(Icons.Default.QrCode, contentDescription = "Show QR")
                }
            }
            
            Text(
                text = "Share this with payers to receive payments:",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            // Noise Public Key
            noisePubkey?.let { pubkey ->
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        text = "Noise Public Key",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "${pubkey.take(16)}...${pubkey.takeLast(8)}",
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace
                        )
                        
                        IconButton(
                            onClick = { onCopy(pubkey) },
                            modifier = Modifier.size(24.dp)
                        ) {
                            Icon(
                                Icons.Default.ContentCopy,
                                contentDescription = "Copy",
                                modifier = Modifier.size(16.dp)
                            )
                        }
                    }
                }
            }
            
            // Connection String
            connectionInfo?.let { info ->
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        text = "Connection String",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "${info.take(30)}...",
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace
                        )
                        
                        IconButton(
                            onClick = { onCopy(info) },
                            modifier = Modifier.size(24.dp)
                        ) {
                            Icon(
                                Icons.Default.ContentCopy,
                                contentDescription = "Copy",
                                modifier = Modifier.size(16.dp)
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun PendingRequestCard(
    request: NoiseReceiveViewModel.PendingPaymentRequest,
    onAccept: () -> Unit,
    onDecline: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.secondaryContainer
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Text(
                        text = "From: ${request.payerPubkey.take(12)}...",
                        style = MaterialTheme.typography.bodyMedium
                    )
                    
                    request.amount?.let { amount ->
                        Text(
                            text = "$amount ${request.currency ?: "SAT"}",
                            style = MaterialTheme.typography.titleMedium
                        )
                    }
                    
                    Text(
                        text = request.methodId.replaceFirstChar { it.uppercase() },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OutlinedButton(
                    onClick = onDecline,
                    modifier = Modifier.weight(1f),
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Text("Decline")
                }
                
                Button(
                    onClick = onAccept,
                    modifier = Modifier.weight(1f),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Color(0xFF4CAF50)
                    )
                ) {
                    Text("Accept")
                }
            }
        }
    }
}

@Composable
private fun ReceiptCard(receipt: StoredReceipt) {
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
                    text = "${receipt.id.take(16)}...",
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace
                )
                Text(
                    text = "${receipt.amount} ${receipt.currency}",
                    style = MaterialTheme.typography.titleSmall
                )
            }
            
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = java.text.SimpleDateFormat("MMM dd", java.util.Locale.getDefault())
                        .format(java.util.Date(receipt.timestamp)),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                
                StatusBadge(status = receipt.status)
            }
        }
    }
}

@Composable
private fun StatusBadge(status: String) {
    val (color, text) = when (status.lowercase()) {
        "completed" -> Color(0xFF4CAF50) to "Completed"
        "pending" -> Color(0xFFFF9800) to "Pending"
        "failed" -> Color(0xFFF44336) to "Failed"
        else -> Color.Gray to status
    }
    
    Surface(
        color = color,
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = Color.White,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
        )
    }
}

private fun copyToClipboard(context: Context, text: String) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = ClipData.newPlainText("Connection Info", text)
    clipboard.setPrimaryClip(clip)
    Toast.makeText(context, "Copied to clipboard", Toast.LENGTH_SHORT).show()
}


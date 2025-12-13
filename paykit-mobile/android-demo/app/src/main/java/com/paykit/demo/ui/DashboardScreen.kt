package com.paykit.demo.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import com.paykit.demo.model.PaymentDirection
import com.paykit.demo.model.PaymentStatus
import com.paykit.demo.model.Receipt
import com.paykit.demo.storage.ContactStorage
import com.paykit.demo.storage.ReceiptStorage
import com.paykit.mobile.KeyManager
import java.text.NumberFormat

class DashboardViewModel : ViewModel() {
    var recentReceipts by mutableStateOf<List<Receipt>>(emptyList())
        private set
    var contactCount by mutableIntStateOf(0)
        private set
    var totalSent by mutableLongStateOf(0L)
        private set
    var totalReceived by mutableLongStateOf(0L)
        private set
    var pendingCount by mutableIntStateOf(0)
        private set
    var isLoading by mutableStateOf(true)
        private set
    
    fun loadDashboard(receiptStorage: ReceiptStorage, contactStorage: ContactStorage) {
        isLoading = true
        
        // Load recent receipts
        recentReceipts = receiptStorage.recentReceipts(5)
        
        // Load stats
        contactCount = contactStorage.listContacts().size
        totalSent = receiptStorage.totalSent()
        totalReceived = receiptStorage.totalReceived()
        pendingCount = receiptStorage.pendingCount()
        
        isLoading = false
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    viewModel: DashboardViewModel = viewModel()
) {
    val context = LocalContext.current
    val keyManager = remember { KeyManager(context) }
    val currentIdentityName by keyManager.currentIdentityName.collectAsState()
    val receiptStorage = remember(currentIdentityName) {
        ReceiptStorage(context, currentIdentityName ?: "default")
    }
    val contactStorage = remember(currentIdentityName) {
        ContactStorage(context, currentIdentityName ?: "default")
    }
    var showQRScanner by remember { mutableStateOf(false) }
    var showPaymentScreen by remember { mutableStateOf(false) }
    
    LaunchedEffect(Unit) {
        viewModel.loadDashboard(receiptStorage, contactStorage)
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Dashboard") }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp)
        ) {
            // Quick Stats Section
            item {
                Text(
                    text = "Overview",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    StatCard(
                        modifier = Modifier.weight(1f),
                        title = "Total Sent",
                        value = formatSats(viewModel.totalSent),
                        icon = Icons.Default.ArrowUpward,
                        color = MaterialTheme.colorScheme.error
                    )
                    StatCard(
                        modifier = Modifier.weight(1f),
                        title = "Total Received",
                        value = formatSats(viewModel.totalReceived),
                        icon = Icons.Default.ArrowDownward,
                        color = Color(0xFF4CAF50)
                    )
                }
            }
            
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    StatCard(
                        modifier = Modifier.weight(1f),
                        title = "Contacts",
                        value = "${viewModel.contactCount}",
                        icon = Icons.Default.People,
                        color = MaterialTheme.colorScheme.primary
                    )
                    StatCard(
                        modifier = Modifier.weight(1f),
                        title = "Pending",
                        value = "${viewModel.pendingCount}",
                        icon = Icons.Default.Schedule,
                        color = Color(0xFFFF9800)
                    )
                }
            }
            
            // Recent Activity Section
            item {
                Text(
                    text = "Recent Activity",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            if (viewModel.recentReceipts.isEmpty()) {
                item {
                    EmptyActivityCard()
                }
            } else {
                items(viewModel.recentReceipts) { receipt ->
                    ReceiptRowCard(receipt = receipt)
                }
            }
            
            // Quick Actions Section
            item {
                Text(
                    text = "Quick Actions",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    QuickActionCard(
                        modifier = Modifier.weight(1f),
                        title = "Send",
                        icon = Icons.Default.Send,
                        color = MaterialTheme.colorScheme.primary,
                        onClick = { showPaymentScreen = true }
                    )
                    QuickActionCard(
                        modifier = Modifier.weight(1f),
                        title = "Receive",
                        icon = Icons.Default.QrCode,
                        color = Color(0xFF4CAF50),
                        onClick = { /* TODO */ }
                    )
                    QuickActionCard(
                        modifier = Modifier.weight(1f),
                        title = "Scan",
                        icon = Icons.Default.QrCodeScanner,
                        color = Color(0xFF9C27B0),
                        onClick = { showQRScanner = true }
                    )
                }
            }
        }
    }
    
    // QR Scanner
    if (showQRScanner) {
        QRScannerScreen(
            onDismiss = { showQRScanner = false },
            onScanned = { result ->
                // Handle scanned result
                // TODO: Navigate to appropriate flow based on result type
                showQRScanner = false
            }
        )
    }
    
    // Payment Screen
    if (showPaymentScreen) {
        PaymentScreen(
            keyManager = keyManager,
            onPaymentComplete = { 
                showPaymentScreen = false
                viewModel.loadDashboard(receiptStorage, contactStorage)
            }
        )
    }
}

@Composable
private fun StatCard(
    modifier: Modifier = Modifier,
    title: String,
    value: String,
    icon: ImageVector,
    color: Color
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = color
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = value,
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold
            )
            Text(
                text = title,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun ReceiptRowCard(receipt: Receipt) {
    Card(
        shape = RoundedCornerShape(12.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Direction indicator
            Icon(
                imageVector = if (receipt.direction == PaymentDirection.SENT) 
                    Icons.Default.ArrowUpward else Icons.Default.ArrowDownward,
                contentDescription = null,
                tint = if (receipt.direction == PaymentDirection.SENT) 
                    MaterialTheme.colorScheme.error else Color(0xFF4CAF50),
                modifier = Modifier.size(32.dp)
            )
            
            Spacer(modifier = Modifier.width(12.dp))
            
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = receipt.displayName,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = receipt.paymentMethod,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = receipt.formattedAmount,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                    color = if (receipt.direction == PaymentDirection.SENT) 
                        MaterialTheme.colorScheme.error else Color(0xFF4CAF50)
                )
                StatusChip(status = receipt.status)
            }
        }
    }
}

@Composable
private fun StatusChip(status: PaymentStatus) {
    val (color, text) = when (status) {
        PaymentStatus.PENDING -> Color(0xFFFF9800) to "Pending"
        PaymentStatus.COMPLETED -> Color(0xFF4CAF50) to "Completed"
        PaymentStatus.FAILED -> MaterialTheme.colorScheme.error to "Failed"
        PaymentStatus.REFUNDED -> Color(0xFF9C27B0) to "Refunded"
    }
    
    Surface(
        color = color.copy(alpha = 0.15f),
        shape = RoundedCornerShape(4.dp)
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = color,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp)
        )
    }
}

@Composable
private fun EmptyActivityCard() {
    Card(
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(40.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Icon(
                imageVector = Icons.Default.Inbox,
                contentDescription = null,
                modifier = Modifier.size(48.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "No recent activity",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun QuickActionCard(
    modifier: Modifier = Modifier,
    title: String,
    icon: ImageVector,
    color: Color,
    onClick: () -> Unit
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp),
        onClick = onClick
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = color
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = title,
                style = MaterialTheme.typography.bodySmall,
                fontWeight = FontWeight.Medium,
                textAlign = TextAlign.Center
            )
        }
    }
}

private fun formatSats(amount: Long): String {
    return "${NumberFormat.getNumberInstance().format(amount)} sats"
}


package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.paykit.mobile.paykit_mobile.PaykitClient
import com.paykit.mobile.paykit_mobile.Receipt

/**
 * Dashboard view model for Bitkit integration
 */
class BitkitDashboardViewModel(private val paykitClient: PaykitClient) {
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
    
    // Quick Access properties
    var autoPayEnabled by mutableStateOf(false)
        private set
    var activeSubscriptions by mutableIntStateOf(0)
        private set
    var pendingRequests by mutableIntStateOf(0)
        private set
    var publishedMethodsCount by mutableIntStateOf(0)
        private set
    var overallHealthStatus by mutableStateOf("Unknown")
        private set
    
    fun loadDashboard(
        receiptStorage: ReceiptStorageProtocol? = null,
        contactStorage: ContactStorageProtocol? = null,
        autoPayStorage: AutoPayStorageProtocol? = null,
        subscriptionStorage: SubscriptionStorageProtocol? = null,
        paymentRequestStorage: PaymentRequestStorageProtocol? = null
    ) {
        isLoading = true
        
        // Load recent receipts
        receiptStorage?.let {
            recentReceipts = it.recentReceipts(5)
            totalSent = it.totalSent()
            totalReceived = it.totalReceived()
            pendingCount = it.pendingCount()
        }
        
        // Load contact count
        contactStorage?.let {
            contactCount = it.listContacts().size
        }
        
        // Load auto-pay status
        autoPayStorage?.let {
            autoPayEnabled = it.getSettings().isEnabled
        }
        
        // Load subscriptions
        subscriptionStorage?.let {
            activeSubscriptions = it.activeSubscriptions().size
        }
        
        // Load payment requests
        paymentRequestStorage?.let {
            pendingRequests = it.pendingCount()
        }
        
        // Check payment methods health
        val healthResults = paykitClient.checkHealth()
        val healthyCount = healthResults.count { it.isHealthy }
        val totalCount = healthResults.size
        publishedMethodsCount = totalCount
        overallHealthStatus = if (totalCount > 0) "$healthyCount/$totalCount healthy" else "No methods"
        
        isLoading = false
    }
}

// Storage protocols are defined in StorageProtocols.kt

/**
 * Dashboard screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitDashboardScreen(
    viewModel: BitkitDashboardViewModel,
    onSendPayment: () -> Unit = {},
    onReceivePayment: () -> Unit = {},
    onScanQR: () -> Unit = {},
    onViewReceipts: () -> Unit = {},
    onViewContacts: () -> Unit = {},
    onViewPaymentMethods: () -> Unit = {},
    onViewAutoPay: () -> Unit = {},
    onViewSubscriptions: () -> Unit = {},
    onViewPaymentRequests: () -> Unit = {}
) {
    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Dashboard") })
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacing(20.dp)
        ) {
            // Stats Cards
            item {
                LazyVerticalGrid(
                    columns = GridCells.Fixed(2),
                    horizontalArrangement = Arrangement.spacing(16.dp),
                    verticalArrangement = Arrangement.spacing(16.dp)
                ) {
                    item {
                        StatCard(
                            title = "Total Sent",
                            value = formatSats(viewModel.totalSent),
                            icon = Icons.Default.ArrowUpward,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                    item {
                        StatCard(
                            title = "Total Received",
                            value = formatSats(viewModel.totalReceived),
                            icon = Icons.Default.ArrowDownward,
                            color = MaterialTheme.colorScheme.tertiary
                        )
                    }
                    item {
                        StatCard(
                            title = "Contacts",
                            value = "${viewModel.contactCount}",
                            icon = Icons.Default.People,
                            color = MaterialTheme.colorScheme.secondary
                        )
                    }
                    item {
                        StatCard(
                            title = "Pending",
                            value = "${viewModel.pendingCount}",
                            icon = Icons.Default.Schedule,
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                }
            }
            
            // Quick Actions
            item {
                QuickActionsSection(
                    onSend = onSendPayment,
                    onReceive = onReceivePayment,
                    onScan = onScanQR
                )
            }
            
            // Recent Activity
            item {
                RecentActivitySection(
                    receipts = viewModel.recentReceipts,
                    isLoading = viewModel.isLoading,
                    onViewAll = onViewReceipts
                )
            }
            
            // Quick Access
            item {
                QuickAccessSection(
                    autoPayEnabled = viewModel.autoPayEnabled,
                    activeSubscriptions = viewModel.activeSubscriptions,
                    pendingRequests = viewModel.pendingRequests,
                    healthStatus = viewModel.overallHealthStatus,
                    onAutoPay = onViewAutoPay,
                    onSubscriptions = onViewSubscriptions,
                    onPaymentRequests = onViewPaymentRequests,
                    onPaymentMethods = onViewPaymentMethods
                )
            }
        }
    }
}

@Composable
fun StatCard(
    title: String,
    value: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    color: androidx.compose.ui.graphics.Color
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacing(8.dp)
        ) {
            Icon(icon, contentDescription = null, tint = color)
            Text(
                text = value,
                style = MaterialTheme.typography.headlineMedium,
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
fun QuickActionsSection(
    onSend: () -> Unit,
    onReceive: () -> Unit,
    onScan: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacing(12.dp)) {
        Text(
            text = "Quick Actions",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold
        )
        
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacing(12.dp)
        ) {
            QuickActionButton(
                title = "Send",
                icon = Icons.Default.ArrowUpward,
                onClick = onSend,
                modifier = Modifier.weight(1f)
            )
            QuickActionButton(
                title = "Receive",
                icon = Icons.Default.ArrowDownward,
                onClick = onReceive,
                modifier = Modifier.weight(1f)
            )
            QuickActionButton(
                title = "Scan",
                icon = Icons.Default.QrCodeScanner,
                onClick = onScan,
                modifier = Modifier.weight(1f)
            )
        }
    }
}

@Composable
fun QuickActionButton(
    title: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier,
        onClick = onClick,
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacing(8.dp)
        ) {
            Icon(icon, contentDescription = null)
            Text(text = title, style = MaterialTheme.typography.bodySmall)
        }
    }
}

@Composable
fun RecentActivitySection(
    receipts: List<Receipt>,
    isLoading: Boolean,
    onViewAll: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacing(12.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Recent Activity",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )
            TextButton(onClick = onViewAll) {
                Text("View All")
            }
        }
        
        if (isLoading) {
            CircularProgressIndicator(modifier = Modifier.fillMaxWidth())
        } else if (receipts.isEmpty()) {
            Text(
                text = "No recent activity",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.fillMaxWidth()
            )
        } else {
            receipts.take(5).forEach { receipt ->
                ReceiptRow(receipt = receipt)
            }
        }
    }
}

@Composable
fun ReceiptRow(receipt: Receipt) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(8.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Column {
                Text(
                    text = receipt.payer,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium
                )
                receipt.amount?.let {
                    Text(
                        text = it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            receipt.currency?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
fun QuickAccessSection(
    autoPayEnabled: Boolean,
    activeSubscriptions: Int,
    pendingRequests: Int,
    healthStatus: String,
    onAutoPay: () -> Unit,
    onSubscriptions: () -> Unit,
    onPaymentRequests: () -> Unit,
    onPaymentMethods: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacing(12.dp)) {
        Text(
            text = "Quick Access",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold
        )
        
        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            horizontalArrangement = Arrangement.spacing(12.dp),
            verticalArrangement = Arrangement.spacing(12.dp)
        ) {
            if (autoPayEnabled) {
                item {
                    QuickAccessCard(
                        title = "Auto-Pay",
                        subtitle = "ON",
                        icon = Icons.Default.Repeat,
                        onClick = onAutoPay
                    )
                }
            }
            
            if (activeSubscriptions > 0) {
                item {
                    QuickAccessCard(
                        title = "Subscriptions",
                        subtitle = "$activeSubscriptions active",
                        icon = Icons.Default.CalendarToday,
                        onClick = onSubscriptions
                    )
                }
            }
            
            if (pendingRequests > 0) {
                item {
                    QuickAccessCard(
                        title = "Requests",
                        subtitle = "$pendingRequests pending",
                        icon = Icons.Default.Notifications,
                        onClick = onPaymentRequests
                    )
                }
            }
            
            item {
                QuickAccessCard(
                    title = "Payment Methods",
                    subtitle = healthStatus,
                    icon = Icons.Default.CreditCard,
                    onClick = onPaymentMethods
                )
            }
        }
    }
}

@Composable
fun QuickAccessCard(
    title: String,
    subtitle: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    onClick: () -> Unit
) {
    Card(
        onClick = onClick,
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacing(8.dp)
        ) {
            Icon(icon, contentDescription = null)
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.Bold
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

fun formatSats(sats: Long): String {
    return when {
        sats >= 1_000_000 -> String.format("%.2fM", sats / 1_000_000.0)
        sats >= 1_000 -> String.format("%.1fK", sats / 1_000.0)
        else -> sats.toString()
    }
}

package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.paykit.mobile.paykit_mobile.Receipt

/**
 * Receipts view model for Bitkit integration
 */
class BitkitReceiptsViewModel(private val receiptStorage: ReceiptStorageProtocol) {
    var receipts by mutableStateOf<List<Receipt>>(emptyList())
        private set
    var filteredReceipts by mutableStateOf<List<Receipt>>(emptyList())
        private set
    var isLoading by mutableStateOf(false)
        private set
    var searchText by mutableStateOf("")
    var filterDirection by mutableStateOf(PaymentDirectionFilter.ALL)
    var totalSent by mutableLongStateOf(0L)
        private set
    var totalReceived by mutableLongStateOf(0L)
        private set
    
    enum class PaymentDirectionFilter {
        ALL, SENT, RECEIVED
    }
    
    fun loadReceipts() {
        isLoading = true
        receipts = receiptStorage.recentReceipts(100)
        totalSent = receiptStorage.totalSent()
        totalReceived = receiptStorage.totalReceived()
        applyFilters()
        isLoading = false
    }
    
    // Identity checker callback - Bitkit must provide this to determine payment direction
    var isMyPubkey: ((String) -> Boolean)? = null
    
    fun applyFilters() {
        var filtered = receipts
        
        // Apply direction filter
        when (filterDirection) {
            PaymentDirectionFilter.ALL -> { /* No filter */ }
            PaymentDirectionFilter.SENT -> {
                // Filter for sent payments - receipts where we are the payer
                val checker = isMyPubkey
                if (checker != null) {
                    filtered = filtered.filter { receipt ->
                        checker(receipt.payer)
                    }
                }
            }
            PaymentDirectionFilter.RECEIVED -> {
                // Filter for received payments - receipts where we are the payee
                val checker = isMyPubkey
                if (checker != null) {
                    filtered = filtered.filter { receipt ->
                        checker(receipt.payee)
                    }
                }
            }
        }
        
        // Apply search filter
        if (searchText.isNotEmpty()) {
            filtered = filtered.filter { receipt ->
                receipt.payer.contains(searchText, ignoreCase = true) ||
                receipt.payee.contains(searchText, ignoreCase = true) ||
                receipt.amount?.contains(searchText, ignoreCase = true) == true
            }
        }
        
        filteredReceipts = filtered
    }
}

/**
 * Receipts screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitReceiptsScreen(viewModel: BitkitReceiptsViewModel) {
    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Receipts") })
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Stats
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                horizontalArrangement = Arrangement.spacing(16.dp)
            ) {
                StatBox(
                    title = "Total Sent",
                    value = formatSats(viewModel.totalSent),
                    modifier = Modifier.weight(1f)
                )
                StatBox(
                    title = "Total Received",
                    value = formatSats(viewModel.totalReceived),
                    modifier = Modifier.weight(1f)
                )
            }
            
            // Filter
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp)
            ) {
                BitkitReceiptsViewModel.PaymentDirectionFilter.values().forEach { filter ->
                    FilterChip(
                        selected = viewModel.filterDirection == filter,
                        onClick = {
                            viewModel.filterDirection = filter
                            viewModel.applyFilters()
                        },
                        label = { Text(filter.name) },
                        modifier = Modifier.padding(end = 8.dp)
                    )
                }
            }
            
            Spacer(modifier = Modifier.height(8.dp))
            
            // Receipts List
            when {
                viewModel.isLoading -> {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center
                    ) {
                        CircularProgressIndicator()
                    }
                }
                viewModel.filteredReceipts.isEmpty() -> {
                    EmptyReceiptsView(modifier = Modifier.fillMaxSize())
                }
                else -> {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(16.dp),
                        verticalArrangement = Arrangement.spacing(8.dp)
                    ) {
                        items(viewModel.filteredReceipts) { receipt ->
                            ReceiptRow(receipt = receipt)
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun StatBox(
    title: String,
    value: String,
    modifier: Modifier = Modifier
) {
    Card(modifier = modifier) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacing(4.dp)
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                text = value,
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold
            )
        }
    }
}

@Composable
fun ReceiptRow(receipt: Receipt) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacing(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    text = receipt.payer,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium
                )
                receipt.amount?.let {
                    Text(
                        text = it,
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.Semibold
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
            Text(
                text = "Method: ${receipt.methodId}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun EmptyReceiptsView(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "No Receipts",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Semibold
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Your payment history will appear here",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

fun formatSats(sats: Long): String {
    return when {
        sats >= 1_000_000 -> String.format("%.2fM", sats / 1_000_000.0)
        sats >= 1_000 -> String.format("%.1fK", sats / 1_000.0)
        else -> sats.toString()
    }
}

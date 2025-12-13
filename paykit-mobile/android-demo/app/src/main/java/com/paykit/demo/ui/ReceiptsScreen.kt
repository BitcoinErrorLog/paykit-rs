package com.paykit.demo.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import com.paykit.demo.model.PaymentDirection
import com.paykit.demo.model.PaymentStatus
import com.paykit.demo.model.Receipt
import com.paykit.demo.storage.ReceiptStorage
import java.text.DateFormat
import java.text.NumberFormat
import java.util.Date

class ReceiptsViewModel : ViewModel() {
    var receipts by mutableStateOf<List<Receipt>>(emptyList())
        private set
    var searchText by mutableStateOf("")
    var filterDirection by mutableStateOf<PaymentDirection?>(null)
    var filterStatus by mutableStateOf<PaymentStatus?>(null)
    var isLoading by mutableStateOf(true)
        private set
    
    val filteredReceipts: List<Receipt>
        get() {
            var result = receipts
            
            // Apply direction filter
            filterDirection?.let { direction ->
                result = result.filter { it.direction == direction }
            }
            
            // Apply status filter
            filterStatus?.let { status ->
                result = result.filter { it.status == status }
            }
            
            // Apply search
            if (searchText.isNotBlank()) {
                val query = searchText.lowercase()
                result = result.filter { receipt ->
                    receipt.displayName.lowercase().contains(query) ||
                    receipt.counterpartyKey.lowercase().contains(query) ||
                    (receipt.memo?.lowercase()?.contains(query) ?: false)
                }
            }
            
            return result
        }
    
    fun loadReceipts(storage: ReceiptStorage) {
        isLoading = true
        receipts = storage.listReceipts()
        isLoading = false
    }
    
    fun deleteReceipt(storage: ReceiptStorage, receipt: Receipt) {
        storage.deleteReceipt(receipt.id)
        loadReceipts(storage)
    }
    
    fun clearFilters() {
        filterDirection = null
        filterStatus = null
        searchText = ""
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReceiptsScreen(
    viewModel: ReceiptsViewModel = viewModel()
) {
    val context = LocalContext.current
    val storage = remember { ReceiptStorage(context) }
    var showFilterSheet by remember { mutableStateOf(false) }
    
    LaunchedEffect(Unit) {
        viewModel.loadReceipts(storage)
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Receipts") },
                actions = {
                    IconButton(onClick = { showFilterSheet = true }) {
                        Icon(Icons.Default.FilterList, contentDescription = "Filter")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Search Bar
            SearchBar(
                query = viewModel.searchText,
                onQueryChange = { viewModel.searchText = it },
                onSearch = {},
                active = false,
                onActiveChange = {},
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                placeholder = { Text("Search receipts...") },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                trailingIcon = {
                    if (viewModel.searchText.isNotEmpty()) {
                        IconButton(onClick = { viewModel.searchText = "" }) {
                            Icon(Icons.Default.Close, contentDescription = "Clear")
                        }
                    }
                }
            ) {}
            
            // Active Filters
            if (viewModel.filterDirection != null || viewModel.filterStatus != null) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    viewModel.filterDirection?.let { direction ->
                        FilterChipView(
                            text = direction.name.lowercase().replaceFirstChar { it.uppercase() },
                            onRemove = { viewModel.filterDirection = null }
                        )
                    }
                    viewModel.filterStatus?.let { status ->
                        FilterChipView(
                            text = status.name.lowercase().replaceFirstChar { it.uppercase() },
                            onRemove = { viewModel.filterStatus = null }
                        )
                    }
                    Spacer(modifier = Modifier.weight(1f))
                    TextButton(onClick = { viewModel.clearFilters() }) {
                        Text("Clear All")
                    }
                }
            }
            
            // Receipt List
            if (viewModel.isLoading) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            } else if (viewModel.filteredReceipts.isEmpty()) {
                EmptyReceiptsView(
                    hasFilters = viewModel.filterDirection != null || 
                        viewModel.filterStatus != null || 
                        viewModel.searchText.isNotBlank(),
                    onClearFilters = { viewModel.clearFilters() }
                )
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(
                        viewModel.filteredReceipts,
                        key = { it.id }
                    ) { receipt ->
                        ReceiptListItem(
                            receipt = receipt,
                            onDelete = { viewModel.deleteReceipt(storage, receipt) }
                        )
                    }
                }
            }
        }
    }
    
    // Filter Bottom Sheet
    if (showFilterSheet) {
        ModalBottomSheet(
            onDismissRequest = { showFilterSheet = false }
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp)
            ) {
                Text(
                    text = "Filters",
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold
                )
                
                Spacer(modifier = Modifier.height(16.dp))
                
                Text(
                    text = "Direction",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = viewModel.filterDirection == null,
                        onClick = { viewModel.filterDirection = null },
                        label = { Text("All") }
                    )
                    FilterChip(
                        selected = viewModel.filterDirection == PaymentDirection.SENT,
                        onClick = { viewModel.filterDirection = PaymentDirection.SENT },
                        label = { Text("Sent") }
                    )
                    FilterChip(
                        selected = viewModel.filterDirection == PaymentDirection.RECEIVED,
                        onClick = { viewModel.filterDirection = PaymentDirection.RECEIVED },
                        label = { Text("Received") }
                    )
                }
                
                Spacer(modifier = Modifier.height(16.dp))
                
                Text(
                    text = "Status",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = viewModel.filterStatus == null,
                        onClick = { viewModel.filterStatus = null },
                        label = { Text("All") }
                    )
                    FilterChip(
                        selected = viewModel.filterStatus == PaymentStatus.COMPLETED,
                        onClick = { viewModel.filterStatus = PaymentStatus.COMPLETED },
                        label = { Text("Completed") }
                    )
                }
                
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = viewModel.filterStatus == PaymentStatus.PENDING,
                        onClick = { viewModel.filterStatus = PaymentStatus.PENDING },
                        label = { Text("Pending") }
                    )
                    FilterChip(
                        selected = viewModel.filterStatus == PaymentStatus.FAILED,
                        onClick = { viewModel.filterStatus = PaymentStatus.FAILED },
                        label = { Text("Failed") }
                    )
                }
                
                Spacer(modifier = Modifier.height(24.dp))
                
                Button(
                    onClick = { showFilterSheet = false },
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text("Apply Filters")
                }
                
                Spacer(modifier = Modifier.height(32.dp))
            }
        }
    }
}

@Composable
private fun FilterChipView(
    text: String,
    onRemove: () -> Unit
) {
    Surface(
        color = MaterialTheme.colorScheme.primary.copy(alpha = 0.15f),
        shape = RoundedCornerShape(16.dp)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            Text(
                text = text,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.primary
            )
            IconButton(
                onClick = onRemove,
                modifier = Modifier.size(16.dp)
            ) {
                Icon(
                    Icons.Default.Close,
                    contentDescription = "Remove",
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(12.dp)
                )
            }
        }
    }
}

@Composable
private fun ReceiptListItem(
    receipt: Receipt,
    onDelete: () -> Unit
) {
    var showDeleteDialog by remember { mutableStateOf(false) }
    
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
            Surface(
                modifier = Modifier.size(44.dp),
                shape = CircleShape,
                color = if (receipt.direction == PaymentDirection.SENT) 
                    MaterialTheme.colorScheme.error.copy(alpha = 0.15f) 
                else Color(0xFF4CAF50).copy(alpha = 0.15f)
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Icon(
                        imageVector = if (receipt.direction == PaymentDirection.SENT) 
                            Icons.Default.ArrowUpward else Icons.Default.ArrowDownward,
                        contentDescription = null,
                        tint = if (receipt.direction == PaymentDirection.SENT) 
                            MaterialTheme.colorScheme.error else Color(0xFF4CAF50)
                    )
                }
            }
            
            Spacer(modifier = Modifier.width(12.dp))
            
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = receipt.displayName,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium
                )
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Text(
                        text = receipt.paymentMethod,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Text(
                        text = "•",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Text(
                        text = formatDate(receipt.createdAt),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = receipt.formattedAmount,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                    color = if (receipt.direction == PaymentDirection.SENT) 
                        MaterialTheme.colorScheme.error else Color(0xFF4CAF50)
                )
                StatusChipSmall(status = receipt.status)
            }
            
            IconButton(onClick = { showDeleteDialog = true }) {
                Icon(
                    Icons.Default.Delete,
                    contentDescription = "Delete",
                    tint = MaterialTheme.colorScheme.error.copy(alpha = 0.7f)
                )
            }
        }
    }
    
    if (showDeleteDialog) {
        AlertDialog(
            onDismissRequest = { showDeleteDialog = false },
            title = { Text("Delete Receipt") },
            text = { Text("Are you sure you want to delete this receipt?") },
            confirmButton = {
                TextButton(
                    onClick = {
                        onDelete()
                        showDeleteDialog = false
                    }
                ) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@Composable
private fun StatusChipSmall(status: PaymentStatus) {
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
private fun EmptyReceiptsView(
    hasFilters: Boolean,
    onClearFilters: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            imageVector = Icons.Default.ReceiptLong,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        
        Spacer(modifier = Modifier.height(16.dp))
        
        Text(
            text = "No receipts found",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Medium
        )
        
        Spacer(modifier = Modifier.height(8.dp))
        
        Text(
            text = if (hasFilters) 
                "Try adjusting your filters or search" 
            else "Payment receipts will appear here",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        
        if (hasFilters) {
            Spacer(modifier = Modifier.height(16.dp))
            OutlinedButton(onClick = onClearFilters) {
                Text("Clear Filters")
            }
        }
    }
}

private fun formatDate(timestamp: Long): String {
    return DateFormat.getDateInstance(DateFormat.SHORT).format(Date(timestamp))
}


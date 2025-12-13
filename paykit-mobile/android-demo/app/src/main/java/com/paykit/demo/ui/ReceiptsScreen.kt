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
import com.paykit.mobile.KeyManager
import android.content.Intent
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
    var errorMessage by mutableStateOf<String?>(null)
    var showError by mutableStateOf(false)
    var exportData by mutableStateOf<String?>(null)
    var showExportDialog by mutableStateOf(false)
    
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
    
    /**
     * Create a receipt using the PaykitClient FFI and store it
     * @param storage The ReceiptStorage instance
     * @param client The PaykitClientWrapper
     * @param direction Payment direction (sent or received)
     * @param counterpartyKey The counterparty's public key
     * @param counterpartyName Optional display name
     * @param amountSats Amount in satoshis
     * @param methodId Payment method (e.g., "lightning", "onchain")
     * @param memo Optional memo/note
     * @return The created receipt, or null if failed
     */
    fun createReceipt(
        storage: ReceiptStorage,
        client: com.paykit.demo.PaykitClientWrapper,
        direction: PaymentDirection,
        counterpartyKey: String,
        counterpartyName: String? = null,
        amountSats: Long,
        methodId: String,
        memo: String? = null
    ): Receipt? {
        val payer = if (direction == PaymentDirection.SENT) "self" else counterpartyKey
        val payee = if (direction == PaymentDirection.SENT) counterpartyKey else "self"
        
        val ffiReceipt = client.createReceipt(
            payer = payer,
            payee = payee,
            methodId = methodId,
            amount = amountSats.toString(),
            currency = "SAT"
        )
        
        if (ffiReceipt == null) {
            showErrorMessage("Failed to create receipt via FFI")
            return null
        }
        
        // Convert FFI receipt to local storage format
        var localReceipt = Receipt.fromFFI(ffiReceipt, direction, counterpartyName)
        if (memo != null) {
            localReceipt = localReceipt.copy(memo = memo)
        }
        
        storage.addReceipt(localReceipt)
        loadReceipts(storage)
        return localReceipt
    }
    
    /**
     * Mark a receipt as completed
     */
    fun completeReceipt(storage: ReceiptStorage, id: String, txId: String? = null) {
        val receipt = storage.getReceipt(id) ?: return
        storage.updateReceipt(receipt.complete(txId))
        loadReceipts(storage)
    }
    
    /**
     * Mark a receipt as failed
     */
    fun failReceipt(storage: ReceiptStorage, id: String) {
        val receipt = storage.getReceipt(id) ?: return
        storage.updateReceipt(receipt.fail())
        loadReceipts(storage)
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
    
    // Export Functions
    
    /**
     * Export receipts to JSON format
     */
    fun exportToJSON(): String {
        val receiptsList = filteredReceipts.map { receipt ->
            mapOf(
                "id" to receipt.id,
                "direction" to receipt.direction.name,
                "counterparty" to receipt.counterpartyKey,
                "displayName" to receipt.displayName,
                "amount" to receipt.amount,
                "currency" to receipt.currency,
                "paymentMethod" to receipt.paymentMethod,
                "status" to receipt.status.name,
                "createdAt" to receipt.createdAt,
                "completedAt" to receipt.completedAt,
                "memo" to receipt.memo,
                "txId" to receipt.txId
            )
        }
        return org.json.JSONArray(receiptsList.map { org.json.JSONObject(it) }).toString(2)
    }
    
    /**
     * Export receipts to CSV format
     */
    fun exportToCSV(): String {
        val header = "ID,Direction,Counterparty,Display Name,Amount,Currency,Payment Method,Status,Created At,Completed At,Memo,Transaction ID\n"
        val rows = filteredReceipts.joinToString("\n") { receipt ->
            listOf(
                receipt.id,
                receipt.direction.name,
                receipt.counterpartyKey,
                receipt.displayName.replace(",", ";"),
                receipt.amount.toString(),
                receipt.currency,
                receipt.paymentMethod,
                receipt.status.name,
                DateFormat.getDateTimeInstance().format(Date(receipt.createdAt)),
                receipt.completedAt?.let { DateFormat.getDateTimeInstance().format(Date(it)) } ?: "",
                (receipt.memo ?: "").replace(",", ";"),
                receipt.txId ?: ""
            ).joinToString(",")
        }
        return header + rows
    }
    
    /**
     * Prepare export data
     */
    fun prepareExport(format: ExportFormat) {
        exportData = when (format) {
            ExportFormat.JSON -> exportToJSON()
            ExportFormat.CSV -> exportToCSV()
        }
        showExportDialog = true
    }
    
    private fun showErrorMessage(message: String) {
        errorMessage = message
        showError = true
    }
}

enum class ExportFormat {
    JSON, CSV
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReceiptsScreen(
    viewModel: ReceiptsViewModel = viewModel()
) {
    val context = LocalContext.current
    val keyManager = remember { KeyManager(context) }
    val currentIdentityName by keyManager.currentIdentityName.collectAsState()
    val storage = remember(currentIdentityName) {
        ReceiptStorage(context, currentIdentityName ?: "default")
    }
    var showFilterSheet by remember { mutableStateOf(false) }
    
    LaunchedEffect(Unit) {
        viewModel.loadReceipts(storage)
    }
    
    var showExportMenu by remember { mutableStateOf(false) }
    
    // Handle share intent
    if (viewModel.showExportDialog && viewModel.exportData != null) {
        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, viewModel.exportData)
            putExtra(Intent.EXTRA_SUBJECT, "Paykit Receipts Export")
        }
        context.startActivity(Intent.createChooser(shareIntent, "Share Receipts"))
        viewModel.showExportDialog = false
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Receipts") },
                actions = {
                    Box {
                        IconButton(
                            onClick = { showExportMenu = true },
                            enabled = viewModel.receipts.isNotEmpty()
                        ) {
                            Icon(Icons.Default.Share, contentDescription = "Export")
                        }
                        DropdownMenu(
                            expanded = showExportMenu,
                            onDismissRequest = { showExportMenu = false }
                        ) {
                            DropdownMenuItem(
                                text = { Text("Export as JSON") },
                                onClick = {
                                    viewModel.prepareExport(ExportFormat.JSON)
                                    showExportMenu = false
                                },
                                leadingIcon = { Icon(Icons.Default.Description, null) }
                            )
                            DropdownMenuItem(
                                text = { Text("Export as CSV") },
                                onClick = {
                                    viewModel.prepareExport(ExportFormat.CSV)
                                    showExportMenu = false
                                },
                                leadingIcon = { Icon(Icons.Default.TableChart, null) }
                            )
                        }
                    }
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


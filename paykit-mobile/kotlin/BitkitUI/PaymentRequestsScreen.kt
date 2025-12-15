package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.paykit.mobile.paykit_mobile.PaykitClient
import com.paykit.mobile.paykit_mobile.PaymentRequest
import kotlinx.coroutines.launch

/**
 * Payment Requests view model for Bitkit integration
 */
class BitkitPaymentRequestsViewModel(
    private val paykitClient: PaykitClient,
    private val paymentRequestStorage: PaymentRequestStorageProtocol
) {
    var pendingRequests by mutableStateOf<List<PaymentRequest>>(emptyList())
        private set
    var requestHistory by mutableStateOf<List<PaymentRequest>>(emptyList())
        private set
    var isLoading by mutableStateOf(false)
        private set
    var errorMessage by mutableStateOf<String?>(null)
    var showError by mutableStateOf(false)
    
    // Create request form
    var recipientPubkey by mutableStateOf("")
    var requestAmount by mutableLongStateOf(1000L)
    var requestMethod by mutableStateOf("lightning")
    var requestDescription by mutableStateOf("")
    var hasExpiry by mutableStateOf(false)
    var expiryHours by mutableIntStateOf(24)
    
    fun refreshRequests() {
        isLoading = true
        pendingRequests = paymentRequestStorage.pendingRequests()
        requestHistory = paymentRequestStorage.requestHistory()
        isLoading = false
    }
    
    fun createRequest(
        scope: kotlinx.coroutines.CoroutineScope,
        myPublicKey: String,
        onSuccess: () -> Unit,
        onError: (String) -> Unit
    ) {
        if (recipientPubkey.isEmpty()) {
            onError("Please enter recipient public key")
            return
        }
        
        isLoading = true
        
        scope.launch {
            try {
                val expiresInSecs = if (hasExpiry) expiryHours.toULong() * 3600u else null
                
                val request = paykitClient.createPaymentRequest(
                    fromPubkey = myPublicKey,
                    toPubkey = recipientPubkey,
                    amountSats = requestAmount,
                    currency = "SAT",
                    methodId = requestMethod,
                    description = requestDescription,
                    expiresInSecs = expiresInSecs
                )
                
                pendingRequests = pendingRequests + request
                isLoading = false
                resetForm()
                onSuccess()
            } catch (e: Exception) {
                isLoading = false
                onError(e.message ?: "Unknown error")
            }
        }
    }
    
    fun handleRequestAction(request: PaymentRequest, action: RequestAction) {
        when (action) {
            is RequestAction.Accept -> {
                // Bitkit should implement payment request acceptance
            }
            is RequestAction.Decline -> {
                pendingRequests = pendingRequests.filter { it.requestId != request.requestId }
            }
        }
    }
    
    fun deleteRequest(id: String) {
        pendingRequests = pendingRequests.filter { it.requestId != id }
        requestHistory = requestHistory.filter { it.requestId != id }
    }
    
    private fun resetForm() {
        recipientPubkey = ""
        requestAmount = 1000L
        requestMethod = "lightning"
        requestDescription = ""
        hasExpiry = false
        expiryHours = 24
    }
}

sealed class RequestAction {
    object Accept : RequestAction()
    object Decline : RequestAction()
}

/**
 * Payment Requests screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitPaymentRequestsScreen(
    viewModel: BitkitPaymentRequestsViewModel,
    myPublicKey: String = "" // Bitkit should provide this
) {
    val scope = rememberCoroutineScope()
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Payment Requests") },
                actions = {
                    IconButton(onClick = { viewModel.refreshRequests() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Pending Requests
            item {
                Text(
                    "Pending Requests",
                    style = MaterialTheme.typography.titleMedium
                )
            }
            
            if (viewModel.pendingRequests.isEmpty()) {
                item {
                    Text(
                        "No pending requests",
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            } else {
                items(viewModel.pendingRequests) { request ->
                    PaymentRequestRow(
                        request = request,
                        onAction = { action -> viewModel.handleRequestAction(request, action) }
                    )
                }
            }
            
            // Create Request
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Text(
                            "Create Payment Request",
                            style = MaterialTheme.typography.titleMedium
                        )
                        
                        OutlinedTextField(
                            value = viewModel.recipientPubkey,
                            onValueChange = { viewModel.recipientPubkey = it },
                            label = { Text("Recipient Public Key") },
                            modifier = Modifier.fillMaxWidth()
                        )
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text("Amount:", modifier = Modifier.align(Alignment.CenterVertically))
                            OutlinedTextField(
                                value = viewModel.requestAmount.toString(),
                                onValueChange = { viewModel.requestAmount = it.toLongOrNull() ?: 0L },
                                label = { Text("sats") },
                                modifier = Modifier.weight(1f),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                            )
                        }
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            FilterChip(
                                selected = viewModel.requestMethod == "lightning",
                                onClick = { viewModel.requestMethod = "lightning" },
                                label = { Text("Lightning") }
                            )
                            FilterChip(
                                selected = viewModel.requestMethod == "onchain",
                                onClick = { viewModel.requestMethod = "onchain" },
                                label = { Text("On-Chain") }
                            )
                        }
                        
                        OutlinedTextField(
                            value = viewModel.requestDescription,
                            onValueChange = { viewModel.requestDescription = it },
                            label = { Text("Description") },
                            modifier = Modifier.fillMaxWidth()
                        )
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("Set Expiry")
                            Switch(
                                checked = viewModel.hasExpiry,
                                onCheckedChange = { viewModel.hasExpiry = it }
                            )
                        }
                        
                        if (viewModel.hasExpiry) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Text("Expires in:", modifier = Modifier.align(Alignment.CenterVertically))
                                OutlinedTextField(
                                    value = viewModel.expiryHours.toString(),
                                    onValueChange = { viewModel.expiryHours = it.toIntOrNull() ?: 24 },
                                    label = { Text("hours") },
                                    modifier = Modifier.weight(1f),
                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                                )
                            }
                        }
                        
                        Button(
                            onClick = {
                                viewModel.createRequest(
                                    scope = scope,
                                    myPublicKey = myPublicKey,
                                    onSuccess = { },
                                    onError = { error ->
                                        viewModel.errorMessage = error
                                        viewModel.showError = true
                                    }
                                )
                            },
                            modifier = Modifier.fillMaxWidth(),
                            enabled = !viewModel.isLoading && viewModel.recipientPubkey.isNotEmpty()
                        ) {
                            if (viewModel.isLoading) {
                                CircularProgressIndicator(modifier = Modifier.size(20.dp))
                            } else {
                                Text("Create Request")
                            }
                        }
                    }
                }
            }
            
            // Request History
            item {
                Text(
                    "History",
                    style = MaterialTheme.typography.titleMedium
                )
            }
            
            if (viewModel.requestHistory.isEmpty()) {
                item {
                    Text(
                        "No request history",
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            } else {
                items(viewModel.requestHistory) { request ->
                    RequestHistoryRow(request = request)
                }
            }
        }
    }
    
    // Error Dialog
    if (viewModel.showError) {
        AlertDialog(
            onDismissRequest = { viewModel.showError = false },
            title = { Text("Error") },
            text = { Text(viewModel.errorMessage ?: "Unknown error") },
            confirmButton = {
                TextButton(onClick = { viewModel.showError = false }) {
                    Text("OK")
                }
            }
        )
    }
}

@Composable
fun PaymentRequestRow(
    request: PaymentRequest,
    onAction: (RequestAction) -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Text(
                        request.fromPubkey.take(16) + "...",
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.Medium
                    )
                    if (request.description.isNotEmpty()) {
                        Text(
                            request.description,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        "${request.amountSats} sats",
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.Medium
                    )
                    Text(
                        request.methodId,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Button(
                    onClick = { onAction(RequestAction.Accept) },
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Accept")
                }
                OutlinedButton(
                    onClick = { onAction(RequestAction.Decline) },
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Decline")
                }
            }
        }
    }
}

@Composable
fun RequestHistoryRow(request: PaymentRequest) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Column {
                Text(
                    request.fromPubkey.take(16) + "...",
                    style = MaterialTheme.typography.bodyMedium
                )
                if (request.description.isNotEmpty()) {
                    Text(
                        request.description,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            Text(
                "${request.amountSats} sats",
                style = MaterialTheme.typography.bodyMedium
            )
        }
    }
}

// Extension for PaymentRequestStorageProtocol
fun PaymentRequestStorageProtocol.pendingRequests(): List<PaymentRequest> {
    // Bitkit should implement this
    return emptyList()
}

fun PaymentRequestStorageProtocol.requestHistory(): List<PaymentRequest> {
    // Bitkit should implement this
    return emptyList()
}

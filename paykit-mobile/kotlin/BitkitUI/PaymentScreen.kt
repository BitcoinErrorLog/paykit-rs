package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.paykit.mobile.paykit_mobile.PaykitClient
import com.paykit.mobile.paykit_mobile.PaymentMethodInfo
import kotlinx.coroutines.launch

/**
 * Payment view model for Bitkit integration
 */
class BitkitPaymentViewModel(private val paykitClient: PaykitClient) {
    var recipientUri by mutableStateOf("")
    var amount by mutableStateOf("")
    var currency by mutableStateOf("SAT")
    var methodId by mutableStateOf("")
    var availableMethods by mutableStateOf<List<PaymentMethodInfo>>(emptyList())
    var isProcessing by mutableStateOf(false)
    var errorMessage by mutableStateOf<String?>(null)
    var showError by mutableStateOf(false)
    var showSuccess by mutableStateOf(false)
    var confirmedReceiptId by mutableStateOf<String?>(null)
    
    init {
        loadPaymentMethods()
    }
    
    fun loadPaymentMethods() {
        // Load methods asynchronously
        availableMethods = try {
            paykitClient.listMethods()
        } catch (e: Exception) {
            emptyList()
        }
        
        if (availableMethods.isNotEmpty()) {
            methodId = availableMethods.first().methodId
        }
    }
    
    fun sendPayment(
        scope: kotlinx.coroutines.CoroutineScope,
        onSuccess: () -> Unit,
        onError: (String) -> Unit
    ) {
        if (recipientUri.isEmpty() || amount.isEmpty()) {
            onError("Please fill in all fields")
            return
        }
        
        val amountSats = amount.toLongOrNull() ?: run {
            onError("Invalid amount")
            return
        }
        
        isProcessing = true
        
        scope.launch {
            try {
                // Bitkit should implement endpoint resolution
                val endpoint = resolveEndpoint(recipientUri)
                
                val result = paykitClient.executePayment(
                    methodId = methodId,
                    endpoint = endpoint,
                    amountSats = amountSats.toULong(),
                    metadataJson = null
                )
                
                isProcessing = false
                if (result.success) {
                    confirmedReceiptId = result.executionId
                    showSuccess = true
                    resetForm()
                    onSuccess()
                } else {
                    onError(result.error ?: "Payment failed")
                }
            } catch (e: Exception) {
                isProcessing = false
                onError(e.message ?: "Unknown error")
            }
        }
    }
    
    fun resetForm() {
        recipientUri = ""
        amount = ""
        currency = "SAT"
    }
    
    // Placeholder - Bitkit should implement endpoint resolution
    private suspend fun resolveEndpoint(recipient: String): String {
        throw IllegalStateException(
            "Endpoint resolution not implemented. Bitkit should implement this."
        )
    }
}

/**
 * Send Payment screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitPaymentScreen(
    viewModel: BitkitPaymentViewModel,
    onPaymentComplete: () -> Unit = {}
) {
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()
    
    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Send Payment") })
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(scrollState)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacing(16.dp)
        ) {
            // Recipient
            OutlinedTextField(
                value = viewModel.recipientUri,
                onValueChange = { viewModel.recipientUri = it },
                label = { Text("Recipient") },
                placeholder = { Text("pubky://... or contact name") },
                modifier = Modifier.fillMaxWidth(),
                enabled = !viewModel.isProcessing
            )
            
            // Amount
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacing(8.dp)
            ) {
                OutlinedTextField(
                    value = viewModel.amount,
                    onValueChange = { viewModel.amount = it },
                    label = { Text("Amount") },
                    modifier = Modifier.weight(1f),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    enabled = !viewModel.isProcessing
                )
                
                var expanded by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(
                    expanded = expanded,
                    onExpandedChange = { expanded = !expanded }
                ) {
                    OutlinedTextField(
                        value = viewModel.currency,
                        onValueChange = {},
                        readOnly = true,
                        label = { Text("Currency") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
                        modifier = Modifier
                            .menuAnchor()
                            .width(100.dp),
                        enabled = !viewModel.isProcessing
                    )
                    ExposedDropdownMenu(
                        expanded = expanded,
                        onDismissRequest = { expanded = false }
                    ) {
                        listOf("SAT", "BTC", "USD").forEach { currency ->
                            DropdownMenuItem(
                                text = { Text(currency) },
                                onClick = {
                                    viewModel.currency = currency
                                    expanded = false
                                }
                            )
                        }
                    }
                }
            }
            
            // Payment Method
            var methodExpanded by remember { mutableStateOf(false) }
            ExposedDropdownMenuBox(
                expanded = methodExpanded,
                onExpandedChange = { methodExpanded = !methodExpanded }
            ) {
                OutlinedTextField(
                    value = viewModel.methodId,
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Payment Method") },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = methodExpanded) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .menuAnchor(),
                    enabled = !viewModel.isProcessing
                )
                ExposedDropdownMenu(
                    expanded = methodExpanded,
                    onDismissRequest = { methodExpanded = false }
                ) {
                    viewModel.availableMethods.forEach { method ->
                        DropdownMenuItem(
                            text = { Text(method.methodId) },
                            onClick = {
                                viewModel.methodId = method.methodId
                                methodExpanded = false
                            }
                        )
                    }
                }
            }
            
            // Send Button
            Button(
                onClick = {
                    viewModel.sendPayment(
                        scope = scope,
                        onSuccess = onPaymentComplete,
                        onError = { error ->
                            viewModel.errorMessage = error
                            viewModel.showError = true
                        }
                    )
                },
                modifier = Modifier.fillMaxWidth(),
                enabled = !viewModel.isProcessing && viewModel.recipientUri.isNotEmpty() && viewModel.amount.isNotEmpty()
            ) {
                if (viewModel.isProcessing) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp))
                } else {
                    Text("Send Payment")
                }
            }
        }
    }
    
    // Success Dialog
    if (viewModel.showSuccess) {
        AlertDialog(
            onDismissRequest = { viewModel.showSuccess = false },
            title = { Text("Success") },
            text = {
                if (viewModel.confirmedReceiptId != null) {
                    Text("Payment sent! Receipt ID: ${viewModel.confirmedReceiptId}")
                } else {
                    Text("Payment sent successfully!")
                }
            },
            confirmButton = {
                TextButton(onClick = { viewModel.showSuccess = false }) {
                    Text("OK")
                }
            }
        )
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

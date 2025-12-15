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
import com.paykit.mobile.paykit_mobile.Subscription
import com.paykit.mobile.paykit_mobile.SubscriptionTerms
import com.paykit.mobile.paykit_mobile.PaymentFrequency
import com.paykit.mobile.paykit_mobile.ProrationResult
import kotlinx.coroutines.launch

/**
 * Subscriptions view model for Bitkit integration
 */
class BitkitSubscriptionsViewModel(
    private val paykitClient: PaykitClient,
    private val subscriptionStorage: SubscriptionStorageProtocol
) {
    var subscriptions by mutableStateOf<List<Subscription>>(emptyList())
        private set
    var isLoading by mutableStateOf(false)
        private set
    var errorMessage by mutableStateOf<String?>(null)
    var showError by mutableStateOf(false)
    var showSuccess by mutableStateOf(false)
    
    // Create subscription form
    var providerName by mutableStateOf("")
    var providerPubkey by mutableStateOf("")
    var amount by mutableLongStateOf(1000L)
    var frequencyString by mutableStateOf("monthly")
    var methodId by mutableStateOf("lightning")
    var description by mutableStateOf("")
    
    // Proration calculator
    var prorationCurrentAmount by mutableLongStateOf(1000L)
    var prorationNewAmount by mutableLongStateOf(2000L)
    var daysIntoPeriod by mutableIntStateOf(15)
    var prorationResult by mutableStateOf<ProrationResult?>(null)
    
    fun loadSubscriptions() {
        isLoading = true
        subscriptions = subscriptionStorage.activeSubscriptions()
        isLoading = false
    }
    
    fun createSubscription(
        scope: kotlinx.coroutines.CoroutineScope,
        myPublicKey: String,
        onSuccess: () -> Unit,
        onError: (String) -> Unit
    ) {
        if (providerPubkey.isEmpty() || providerName.isEmpty()) {
            onError("Please fill in provider information")
            return
        }
        
        isLoading = true
        
        scope.launch {
            try {
                val frequency = when (frequencyString) {
                    "daily" -> PaymentFrequency.DAILY
                    "weekly" -> PaymentFrequency.WEEKLY
                    "monthly" -> PaymentFrequency.MONTHLY
                    "yearly" -> PaymentFrequency.YEARLY
                    else -> PaymentFrequency.MONTHLY
                }
                
                val terms = SubscriptionTerms(
                    amountSats = amount,
                    currency = "SAT",
                    frequency = frequency,
                    methodId = methodId,
                    description = description
                )
                
                val subscription = paykitClient.createSubscription(
                    subscriber = myPublicKey,
                    provider = providerPubkey,
                    terms = terms
                )
                
                subscriptions = subscriptions + subscription
                isLoading = false
                showSuccess = true
                resetForm()
                onSuccess()
            } catch (e: Exception) {
                isLoading = false
                onError(e.message ?: "Unknown error")
            }
        }
    }
    
    fun calculateProration(
        scope: kotlinx.coroutines.CoroutineScope,
        onError: (String) -> Unit
    ) {
        scope.launch {
            try {
                val periodStart = (System.currentTimeMillis() / 1000) - (daysIntoPeriod * 86400L)
                val periodEnd = periodStart + (30 * 86400L) // 30 days
                val changeDate = System.currentTimeMillis() / 1000
                
                val result = paykitClient.calculateProration(
                    currentAmountSats = prorationCurrentAmount,
                    newAmountSats = prorationNewAmount,
                    periodStart = periodStart,
                    periodEnd = periodEnd,
                    changeDate = changeDate
                )
                
                prorationResult = result
            } catch (e: Exception) {
                onError(e.message ?: "Unknown error")
            }
        }
    }
    
    fun deleteSubscriptions(indices: List<Int>) {
        subscriptions = subscriptions.filterIndexed { index, _ -> index !in indices }
        // Bitkit should delete from storage
    }
    
    private fun resetForm() {
        providerName = ""
        providerPubkey = ""
        amount = 1000L
        frequencyString = "monthly"
        methodId = "lightning"
        description = ""
    }
}

/**
 * Subscriptions screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitSubscriptionsScreen(
    viewModel: BitkitSubscriptionsViewModel,
    myPublicKey: String = "" // Bitkit should provide this
) {
    val scope = rememberCoroutineScope()
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Subscriptions") },
                actions = {
                    IconButton(onClick = { viewModel.loadSubscriptions() }) {
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
            // Active Subscriptions
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        "Active Subscriptions",
                        style = MaterialTheme.typography.titleMedium
                    )
                    if (viewModel.subscriptions.isNotEmpty()) {
                        Text("${viewModel.subscriptions.size}")
                    }
                }
            }
            
            if (viewModel.subscriptions.isEmpty()) {
                item {
                    Text(
                        "No active subscriptions",
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            } else {
                items(viewModel.subscriptions.size) { index ->
                    SubscriptionRow(subscription = viewModel.subscriptions[index])
                }
            }
            
            // Create Subscription
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Text(
                            "Create Subscription",
                            style = MaterialTheme.typography.titleMedium
                        )
                        
                        OutlinedTextField(
                            value = viewModel.providerName,
                            onValueChange = { viewModel.providerName = it },
                            label = { Text("Provider Name") },
                            modifier = Modifier.fillMaxWidth()
                        )
                        
                        OutlinedTextField(
                            value = viewModel.providerPubkey,
                            onValueChange = { viewModel.providerPubkey = it },
                            label = { Text("Provider Public Key") },
                            modifier = Modifier.fillMaxWidth()
                        )
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text("Amount:", modifier = Modifier.align(Alignment.CenterVertically))
                            OutlinedTextField(
                                value = viewModel.amount.toString(),
                                onValueChange = { viewModel.amount = it.toLongOrNull() ?: 0L },
                                label = { Text("sats") },
                                modifier = Modifier.weight(1f),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                            )
                        }
                        
                        var frequencyExpanded by remember { mutableStateOf(false) }
                        ExposedDropdownMenuBox(
                            expanded = frequencyExpanded,
                            onExpandedChange = { frequencyExpanded = !frequencyExpanded }
                        ) {
                            OutlinedTextField(
                                value = viewModel.frequencyString,
                                onValueChange = {},
                                readOnly = true,
                                label = { Text("Frequency") },
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = frequencyExpanded) },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .menuAnchor()
                            )
                            ExposedDropdownMenu(
                                expanded = frequencyExpanded,
                                onDismissRequest = { frequencyExpanded = false }
                            ) {
                                listOf("daily", "weekly", "monthly", "yearly").forEach { freq ->
                                    DropdownMenuItem(
                                        text = { Text(freq.capitalize()) },
                                        onClick = {
                                            viewModel.frequencyString = freq
                                            frequencyExpanded = false
                                        }
                                    )
                                }
                            }
                        }
                        
                        OutlinedTextField(
                            value = viewModel.description,
                            onValueChange = { viewModel.description = it },
                            label = { Text("Description") },
                            modifier = Modifier.fillMaxWidth()
                        )
                        
                        Button(
                            onClick = {
                                viewModel.createSubscription(
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
                            enabled = !viewModel.isLoading && viewModel.providerPubkey.isNotEmpty()
                        ) {
                            if (viewModel.isLoading) {
                                CircularProgressIndicator(modifier = Modifier.size(20.dp))
                            } else {
                                Text("Create Subscription")
                            }
                        }
                    }
                }
            }
            
            // Proration Calculator
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Text(
                            "Proration Calculator",
                            style = MaterialTheme.typography.titleMedium
                        )
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text("Current Amount:", modifier = Modifier.align(Alignment.CenterVertically))
                            OutlinedTextField(
                                value = viewModel.prorationCurrentAmount.toString(),
                                onValueChange = { viewModel.prorationCurrentAmount = it.toLongOrNull() ?: 0L },
                                label = { Text("sats") },
                                modifier = Modifier.weight(1f),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                            )
                        }
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text("New Amount:", modifier = Modifier.align(Alignment.CenterVertically))
                            OutlinedTextField(
                                value = viewModel.prorationNewAmount.toString(),
                                onValueChange = { viewModel.prorationNewAmount = it.toLongOrNull() ?: 0L },
                                label = { Text("sats") },
                                modifier = Modifier.weight(1f),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                            )
                        }
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text("Days Into Period:", modifier = Modifier.align(Alignment.CenterVertically))
                            OutlinedTextField(
                                value = viewModel.daysIntoPeriod.toString(),
                                onValueChange = { viewModel.daysIntoPeriod = it.toIntOrNull() ?: 0 },
                                label = { Text("days") },
                                modifier = Modifier.weight(1f),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                            )
                            Text("/ 30")
                        }
                        
                        Button(
                            onClick = {
                                viewModel.calculateProration(
                                    scope = scope,
                                    onError = { error ->
                                        viewModel.errorMessage = error
                                        viewModel.showError = true
                                    }
                                )
                            },
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text("Calculate Proration")
                        }
                        
                        viewModel.prorationResult?.let { result ->
                            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Divider()
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Text("Credit:")
                                    Text("${result.creditSats} sats")
                                }
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Text("Charge:")
                                    Text("${result.chargeSats} sats")
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Success Dialog
    if (viewModel.showSuccess) {
        AlertDialog(
            onDismissRequest = { viewModel.showSuccess = false },
            title = { Text("Success") },
            text = { Text("Subscription created successfully!") },
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

@Composable
fun SubscriptionRow(subscription: Subscription) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    subscription.provider,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium
                )
                if (subscription.isActive) {
                    Text(
                        "Active",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
            }
            Text(
                "${subscription.terms.amountSats} sats / ${subscription.terms.frequency}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            if (subscription.terms.description.isNotEmpty()) {
                Text(
                    subscription.terms.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

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
import androidx.lifecycle.viewmodel.compose.viewModel
import com.paykit.demo.model.StoredSubscription
import com.paykit.demo.viewmodel.ProrationResult
import com.paykit.demo.viewmodel.SubscriptionsViewModel
import java.text.SimpleDateFormat
import java.util.*

/**
 * Subscriptions Screen
 *
 * Displays and manages subscriptions including:
 * - Active subscriptions list with persistence
 * - Create new subscription
 * - Proration calculator
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SubscriptionsScreen(
    viewModel: SubscriptionsViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    
    var providerName by remember { mutableStateOf("") }
    var providerPubkey by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf(1000L) }
    var description by remember { mutableStateOf("") }
    var selectedFrequency by remember { mutableStateOf("Monthly") }

    // Proration calculator state
    var currentAmount by remember { mutableStateOf(1000L) }
    var newAmount by remember { mutableStateOf(2000L) }
    var daysIntoPeriod by remember { mutableStateOf(15) }
    var prorationResult by remember { mutableStateOf<ProrationResult?>(null) }

    // Clear form on success
    LaunchedEffect(uiState.showSuccess) {
        if (uiState.showSuccess) {
            providerName = ""
            providerPubkey = ""
            description = ""
            kotlinx.coroutines.delay(2000)
            viewModel.dismissSuccess()
        }
    }

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
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Active Subscriptions Section
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "Active Subscriptions",
                        style = MaterialTheme.typography.titleMedium
                    )
                    if (uiState.subscriptions.isNotEmpty()) {
                        Text(
                            text = "${uiState.subscriptions.size}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }

            items(uiState.subscriptions, key = { it.id }) { subscription ->
                SubscriptionCardFromStorage(
                    subscription = subscription,
                    onToggle = { viewModel.toggleSubscription(subscription.id) },
                    onDelete = { viewModel.deleteSubscription(subscription.id) }
                )
            }

            if (uiState.subscriptions.isEmpty()) {
                item {
                    EmptyState("No active subscriptions")
                }
            }

            // Create New Subscription Section
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
                            text = "Create Subscription",
                            style = MaterialTheme.typography.titleMedium
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        OutlinedTextField(
                            value = providerName,
                            onValueChange = { providerName = it },
                            label = { Text("Provider Name") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        OutlinedTextField(
                            value = providerPubkey,
                            onValueChange = { providerPubkey = it },
                            label = { Text("Provider Public Key") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        OutlinedTextField(
                            value = amount.toString(),
                            onValueChange = { it.toLongOrNull()?.let { a -> amount = a } },
                            label = { Text("Amount (sats)") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            listOf("Daily", "Weekly", "Monthly").forEach { freq ->
                                FilterChip(
                                    selected = selectedFrequency == freq,
                                    onClick = { selectedFrequency = freq },
                                    label = { Text(freq) }
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(8.dp))

                        OutlinedTextField(
                            value = description,
                            onValueChange = { description = it },
                            label = { Text("Description") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        Button(
                            onClick = { 
                                viewModel.createSubscription(
                                    providerName = providerName,
                                    providerPubkey = providerPubkey,
                                    amountSats = amount,
                                    frequency = selectedFrequency.lowercase(),
                                    description = description
                                )
                            },
                            modifier = Modifier.fillMaxWidth(),
                            enabled = providerPubkey.isNotEmpty() && providerName.isNotEmpty()
                        ) {
                            Text("Create Subscription")
                        }

                        // Success message
                        if (uiState.showSuccess) {
                            Spacer(modifier = Modifier.height(8.dp))
                            Text(
                                text = "Subscription created!",
                                style = MaterialTheme.typography.bodySmall,
                                color = Color(0xFF4CAF50)
                            )
                        }

                        // Error message
                        uiState.errorMessage?.let { error ->
                            Spacer(modifier = Modifier.height(8.dp))
                            Text(
                                text = error,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error
                            )
                        }
                    }
                }
            }

            // Proration Calculator Section
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
                            text = "Proration Calculator",
                            style = MaterialTheme.typography.titleMedium
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        OutlinedTextField(
                            value = currentAmount.toString(),
                            onValueChange = { it.toLongOrNull()?.let { a -> currentAmount = a } },
                            label = { Text("Current Amount (sats)") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        OutlinedTextField(
                            value = newAmount.toString(),
                            onValueChange = { it.toLongOrNull()?.let { a -> newAmount = a } },
                            label = { Text("New Amount (sats)") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("Days into period: $daysIntoPeriod / 30")
                        }

                        Slider(
                            value = daysIntoPeriod.toFloat(),
                            onValueChange = { daysIntoPeriod = it.toInt() },
                            valueRange = 1f..29f,
                            steps = 27,
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        Button(
                            onClick = {
                                prorationResult = viewModel.calculateProration(
                                    currentAmountSats = currentAmount,
                                    newAmountSats = newAmount,
                                    daysIntoPeriod = daysIntoPeriod
                                )
                            },
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text("Calculate Proration")
                        }

                        prorationResult?.let { result ->
                            Spacer(modifier = Modifier.height(16.dp))
                            HorizontalDivider()
                            Spacer(modifier = Modifier.height(8.dp))

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
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text("Net:", style = MaterialTheme.typography.titleSmall)
                                Text(
                                    text = "${result.netSats} sats",
                                    style = MaterialTheme.typography.titleSmall,
                                    color = if (result.isRefund) Color(0xFF4CAF50) else Color(0xFFFFA500)
                                )
                            }
                            Text(
                                text = if (result.isRefund) "Refund due" else "Additional charge",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }

            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
    }
}

@Composable
fun SubscriptionCardFromStorage(
    subscription: StoredSubscription,
    onToggle: () -> Unit,
    onDelete: () -> Unit
) {
    val dateFormat = remember { SimpleDateFormat("MMM d, yyyy", Locale.getDefault()) }

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
                        text = subscription.providerName,
                        style = MaterialTheme.typography.titleSmall
                    )
                    Text(
                        text = subscription.description,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        text = "${subscription.amountSats} sats",
                        style = MaterialTheme.typography.titleSmall
                    )
                    Text(
                        text = subscription.frequency.replaceFirstChar { it.uppercase() },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row {
                    subscription.nextPaymentAt?.let { nextMs ->
                        Text("Next payment: ", style = MaterialTheme.typography.bodySmall)
                        Text(
                            text = dateFormat.format(Date(nextMs)),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    } ?: Text(
                        text = "No scheduled payment",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                TextButton(onClick = onToggle) {
                    Surface(
                        color = if (subscription.isActive) Color(0xFF4CAF50).copy(alpha = 0.2f) 
                               else Color.Gray.copy(alpha = 0.2f),
                        shape = MaterialTheme.shapes.small
                    ) {
                        Text(
                            text = if (subscription.isActive) "Active" else "Paused",
                            style = MaterialTheme.typography.labelSmall,
                            color = if (subscription.isActive) Color(0xFF4CAF50) else Color.Gray,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                        )
                    }
                }
            }

            if (subscription.paymentCount > 0) {
                Text(
                    text = "${subscription.paymentCount} payment(s) made",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

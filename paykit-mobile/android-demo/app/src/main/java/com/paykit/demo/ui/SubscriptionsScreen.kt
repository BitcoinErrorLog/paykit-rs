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
import java.text.SimpleDateFormat
import java.util.*

/**
 * Subscriptions Screen
 *
 * Displays and manages subscriptions including:
 * - Active subscriptions list
 * - Create new subscription
 * - Proration calculator
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SubscriptionsScreen() {
    var providerPubkey by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf(1000L) }
    var description by remember { mutableStateOf("") }
    var selectedFrequency by remember { mutableStateOf("Monthly") }

    // Proration calculator state
    var currentAmount by remember { mutableStateOf(1000L) }
    var newAmount by remember { mutableStateOf(2000L) }
    var daysIntoPeriod by remember { mutableStateOf(15) }
    var prorationResult by remember { mutableStateOf<ProrationResultData?>(null) }

    val subscriptions = remember {
        listOf(
            SubscriptionData(
                id = "1",
                providerName = "Premium News",
                amountSats = 5000,
                frequency = "Monthly",
                description = "Monthly news subscription",
                nextPayment = Date(System.currentTimeMillis() + 15L * 24 * 3600 * 1000),
                isActive = true
            ),
            SubscriptionData(
                id = "2",
                providerName = "Coffee Club",
                amountSats = 10000,
                frequency = "Weekly",
                description = "Weekly coffee delivery",
                nextPayment = Date(System.currentTimeMillis() + 3L * 24 * 3600 * 1000),
                isActive = true
            )
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Subscriptions") },
                actions = {
                    IconButton(onClick = { /* Add */ }) {
                        Icon(Icons.Default.Add, contentDescription = "Add")
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
                Text(
                    text = "Active Subscriptions",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(top = 16.dp)
                )
            }

            items(subscriptions) { subscription ->
                SubscriptionCard(subscription)
            }

            if (subscriptions.isEmpty()) {
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
                            onClick = { /* Create subscription */ },
                            modifier = Modifier.fillMaxWidth(),
                            enabled = providerPubkey.isNotEmpty()
                        ) {
                            Text("Create Subscription")
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
                                // Calculate proration
                                val daysRemaining = 30 - daysIntoPeriod
                                val creditPerDay = currentAmount / 30.0
                                val chargePerDay = newAmount / 30.0

                                val credit = (creditPerDay * daysRemaining).toLong()
                                val charge = (chargePerDay * daysRemaining).toLong()
                                val net = charge - credit

                                prorationResult = ProrationResultData(
                                    credit = credit,
                                    charge = charge,
                                    net = net,
                                    isRefund = net < 0
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
                                Text("${result.credit} sats")
                            }
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text("Charge:")
                                Text("${result.charge} sats")
                            }
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text("Net:", style = MaterialTheme.typography.titleSmall)
                                Text(
                                    text = "${kotlin.math.abs(result.net)} sats",
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
fun SubscriptionCard(subscription: SubscriptionData) {
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
                        text = subscription.frequency,
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
                    Text("Next payment: ", style = MaterialTheme.typography.bodySmall)
                    Text(
                        text = dateFormat.format(subscription.nextPayment),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                if (subscription.isActive) {
                    Surface(
                        color = Color(0xFF4CAF50).copy(alpha = 0.2f),
                        shape = MaterialTheme.shapes.small
                    ) {
                        Text(
                            text = "Active",
                            style = MaterialTheme.typography.labelSmall,
                            color = Color(0xFF4CAF50),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                        )
                    }
                }
            }
        }
    }
}

data class SubscriptionData(
    val id: String,
    val providerName: String,
    val amountSats: Long,
    val frequency: String,
    val description: String,
    val nextPayment: Date,
    val isActive: Boolean
)

data class ProrationResultData(
    val credit: Long,
    val charge: Long,
    val net: Long,
    val isRefund: Boolean
)

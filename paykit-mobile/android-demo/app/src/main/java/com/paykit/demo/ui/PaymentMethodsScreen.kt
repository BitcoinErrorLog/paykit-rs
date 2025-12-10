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

/**
 * Payment Methods Screen
 *
 * Displays available payment methods, health status,
 * endpoint validation, and method selection testing.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentMethodsScreen() {
    var testAmount by remember { mutableStateOf(10000L) }
    var validationEndpoint by remember { mutableStateOf("") }
    var selectedMethod by remember { mutableStateOf("lightning") }
    var validationResult by remember { mutableStateOf<Boolean?>(null) }
    var selectionResult by remember { mutableStateOf<String?>(null) }

    val methods = remember {
        listOf(
            PaymentMethodInfo("lightning", "Lightning Network", "Fast, low-fee payments", true),
            PaymentMethodInfo("onchain", "Bitcoin On-Chain", "Standard Bitcoin transactions", true)
        )
    }

    val healthResults = remember {
        listOf(
            HealthResult("lightning", HealthStatus.HEALTHY, 45),
            HealthResult("onchain", HealthStatus.HEALTHY, 120)
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Payment Methods") }
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
            // Available Methods Section
            item {
                Text(
                    text = "Available Methods",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(top = 16.dp)
                )
            }

            items(methods) { method ->
                PaymentMethodCard(method)
            }

            // Health Status Section
            item {
                Text(
                    text = "Health Status",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(top = 16.dp)
                )
            }

            items(healthResults) { result ->
                HealthStatusCard(result)
            }

            // Method Selection Section
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
                            text = "Test Method Selection",
                            style = MaterialTheme.typography.titleMedium
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        OutlinedTextField(
                            value = testAmount.toString(),
                            onValueChange = { it.toLongOrNull()?.let { a -> testAmount = a } },
                            label = { Text("Amount (sats)") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        Button(
                            onClick = {
                                // Simulate selection - prefer Lightning for small amounts
                                selectionResult = if (testAmount < 100000) "lightning" else "onchain"
                            },
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text("Select Best Method")
                        }

                        selectionResult?.let { result ->
                            Spacer(modifier = Modifier.height(8.dp))
                            Row(
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text("Selected: ")
                                Text(
                                    text = result,
                                    color = Color(0xFF4CAF50),
                                    style = MaterialTheme.typography.bodyLarge
                                )
                            }
                        }
                    }
                }
            }

            // Endpoint Validation Section
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
                            text = "Endpoint Validation",
                            style = MaterialTheme.typography.titleMedium
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            FilterChip(
                                selected = selectedMethod == "lightning",
                                onClick = { selectedMethod = "lightning" },
                                label = { Text("Lightning") }
                            )
                            FilterChip(
                                selected = selectedMethod == "onchain",
                                onClick = { selectedMethod = "onchain" },
                                label = { Text("On-Chain") }
                            )
                        }

                        Spacer(modifier = Modifier.height(8.dp))

                        OutlinedTextField(
                            value = validationEndpoint,
                            onValueChange = { validationEndpoint = it },
                            label = { Text("Endpoint (address/invoice)") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Button(
                                onClick = {
                                    // Simple validation simulation
                                    validationResult = when (selectedMethod) {
                                        "lightning" -> validationEndpoint.startsWith("lnbc")
                                        "onchain" -> validationEndpoint.startsWith("bc1") ||
                                                validationEndpoint.startsWith("1") ||
                                                validationEndpoint.startsWith("3")
                                        else -> false
                                    }
                                }
                            ) {
                                Text("Validate")
                            }

                            validationResult?.let { isValid ->
                                Icon(
                                    imageVector = if (isValid) Icons.Default.CheckCircle else Icons.Default.Cancel,
                                    contentDescription = null,
                                    tint = if (isValid) Color(0xFF4CAF50) else Color.Red
                                )
                            }
                        }
                    }
                }
            }

            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
    }
}

@Composable
fun PaymentMethodCard(method: PaymentMethodInfo) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = if (method.id == "lightning") Icons.Default.FlashOn else Icons.Default.AccountBalance,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(40.dp)
            )

            Spacer(modifier = Modifier.width(16.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = method.name,
                    style = MaterialTheme.typography.titleSmall
                )
                Text(
                    text = method.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            if (method.isUsable) {
                Icon(
                    imageVector = Icons.Default.CheckCircle,
                    contentDescription = "Available",
                    tint = Color(0xFF4CAF50)
                )
            }
        }
    }
}

@Composable
fun HealthStatusCard(result: HealthResult) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = result.methodId,
                style = MaterialTheme.typography.titleSmall
            )

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(
                    text = "${result.latencyMs}ms",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Surface(
                    color = when (result.status) {
                        HealthStatus.HEALTHY -> Color(0xFF4CAF50)
                        HealthStatus.DEGRADED -> Color(0xFFFFA500)
                        HealthStatus.UNAVAILABLE -> Color.Red
                    },
                    shape = MaterialTheme.shapes.small
                ) {
                    Text(
                        text = result.status.name,
                        style = MaterialTheme.typography.labelSmall,
                        color = Color.White,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }
        }
    }
}

data class PaymentMethodInfo(
    val id: String,
    val name: String,
    val description: String,
    val isUsable: Boolean
)

data class HealthResult(
    val methodId: String,
    val status: HealthStatus,
    val latencyMs: Int
)

enum class HealthStatus {
    HEALTHY,
    DEGRADED,
    UNAVAILABLE
}

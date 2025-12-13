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
import com.paykit.demo.PaykitDemoApp
import com.paykit.mobile.HealthCheckResult
import com.paykit.mobile.HealthStatus
import com.paykit.mobile.PaymentMethod
import com.paykit.mobile.SelectionPreferences
import com.paykit.mobile.SelectionStrategy

/**
 * Payment Methods Screen
 *
 * Displays available payment methods, health status,
 * endpoint validation, and method selection testing.
 * 
 * This screen uses real PaykitClient FFI calls when available.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentMethodsScreen() {
    val paykitClient = remember { PaykitDemoApp.paykitClient }
    
    var testAmount by remember { mutableStateOf(10000L) }
    var validationEndpoint by remember { mutableStateOf("") }
    var selectedMethod by remember { mutableStateOf("lightning") }
    var validationResult by remember { mutableStateOf<Boolean?>(null) }
    var selectionResult by remember { mutableStateOf<String?>(null) }
    var selectionReason by remember { mutableStateOf<String?>(null) }

    // Load methods from FFI
    val methodIds = remember { paykitClient.listMethods() }
    
    val methods = remember(methodIds) {
        if (methodIds.isEmpty()) {
            // Fallback to static data when FFI unavailable
            listOf(
                PaymentMethodInfo("lightning", "Lightning Network", "Fast, low-fee payments", true),
                PaymentMethodInfo("onchain", "Bitcoin On-Chain", "Standard Bitcoin transactions", true)
            )
        } else {
            methodIds.map { id ->
                PaymentMethodInfo(
                    id = id,
                    name = when (id) {
                        "lightning" -> "Lightning Network"
                        "onchain" -> "Bitcoin On-Chain"
                        else -> id.replaceFirstChar { it.uppercase() }
                    },
                    description = when (id) {
                        "lightning" -> "Fast, low-fee payments"
                        "onchain" -> "Standard Bitcoin transactions"
                        else -> "Payment method"
                    },
                    isUsable = paykitClient.isMethodUsable(id)
                )
            }
        }
    }

    // Health results - loaded on demand
    var healthResults by remember { mutableStateOf<List<HealthCheckResult>>(emptyList()) }
    var isLoadingHealth by remember { mutableStateOf(false) }

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
            // SDK Status
            item {
                if (!paykitClient.isAvailable) {
                    Card(
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer
                        ),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier.padding(16.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                Icons.Default.Warning,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.error
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                "PaykitClient unavailable - using fallback mode",
                                color = MaterialTheme.colorScheme.onErrorContainer
                            )
                        }
                    }
                }
            }
            
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

            if (healthResults.isEmpty() && !isLoadingHealth) {
                item {
                    Button(
                        onClick = {
                            isLoadingHealth = true
                            healthResults = paykitClient.checkHealth()
                            isLoadingHealth = false
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Check Health Status")
                    }
                }
            }

            items(healthResults) { result ->
                HealthStatusCard(result)
            }

            if (healthResults.isNotEmpty()) {
                item {
                    TextButton(
                        onClick = {
                            isLoadingHealth = true
                            healthResults = paykitClient.checkHealth()
                            isLoadingHealth = false
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Refresh Health")
                    }
                }
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
                                // Use real FFI method selection
                                val paymentMethods = methods.map { m ->
                                    PaymentMethod(
                                        methodId = m.id,
                                        endpoint = when (m.id) {
                                            "lightning" -> "lnbc..."
                                            else -> "bc1q..."
                                        }
                                    )
                                }
                                
                                val prefs = SelectionPreferences(
                                    strategy = SelectionStrategy.BALANCED,
                                    excludedMethods = emptyList(),
                                    maxFeeSats = null,
                                    maxConfirmationTimeSecs = null
                                )
                                
                                val result = paykitClient.selectMethod(
                                    paymentMethods,
                                    testAmount.toULong(),
                                    prefs
                                )
                                
                                if (result != null) {
                                    selectionResult = result.primaryMethod
                                    selectionReason = result.reason
                                } else {
                                    // Fallback simulation
                                    selectionResult = if (testAmount < 100000) "lightning" else "onchain"
                                    selectionReason = "Fallback: FFI unavailable"
                                }
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
                            selectionReason?.let { reason ->
                                Text(
                                    text = reason,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
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
                                    // Use real FFI validation
                                    validationResult = paykitClient.validateEndpoint(
                                        selectedMethod,
                                        validationEndpoint
                                    )
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
fun HealthStatusCard(result: HealthCheckResult) {
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
                result.latencyMs?.let { latency ->
                    Text(
                        text = "${latency}ms",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                Surface(
                    color = when (result.status) {
                        HealthStatus.HEALTHY -> Color(0xFF4CAF50)
                        HealthStatus.DEGRADED -> Color(0xFFFFA500)
                        HealthStatus.UNAVAILABLE -> Color.Red
                        HealthStatus.UNKNOWN -> Color.Gray
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

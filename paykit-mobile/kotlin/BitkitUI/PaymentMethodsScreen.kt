package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.paykit.mobile.paykit_mobile.PaykitClient
import com.paykit.mobile.paykit_mobile.PaymentMethodInfo
import com.paykit.mobile.paykit_mobile.HealthCheckResult

/**
 * Payment Methods view model for Bitkit integration
 */
class BitkitPaymentMethodsViewModel(private val paykitClient: PaykitClient) {
    var methods by mutableStateOf<List<PaymentMethodInfo>>(emptyList())
        private set
    var healthResults by mutableStateOf<List<HealthCheckResult>>(emptyList())
        private set
    var isLoading by mutableStateOf(false)
        private set
    
    fun loadMethods() {
        isLoading = true
        methods = try {
            paykitClient.listMethods()
        } catch (e: Exception) {
            emptyList()
        }
        healthResults = paykitClient.checkHealth()
        isLoading = false
    }
    
    fun validateEndpoint(methodId: String, endpoint: String): Boolean {
        return try {
            paykitClient.validateEndpoint(methodId = methodId, endpoint = endpoint)
        } catch (e: Exception) {
            false
        }
    }
}

/**
 * Payment Methods screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitPaymentMethodsScreen(viewModel: BitkitPaymentMethodsViewModel) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Payment Methods") },
                actions = {
                    IconButton(onClick = { viewModel.loadMethods() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                }
            )
        }
    ) { padding ->
        when {
            viewModel.isLoading -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            viewModel.methods.isEmpty() -> {
                EmptyPaymentMethodsView(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                )
            }
            else -> {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacing(8.dp)
                ) {
                    items(viewModel.methods) { method ->
                        PaymentMethodRow(
                            method = method,
                            health = viewModel.healthResults.firstOrNull { it.methodId == method.methodId }
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun PaymentMethodRow(
    method: PaymentMethodInfo,
    health: HealthCheckResult?
) {
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
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = method.methodId,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium
                )
                health?.let {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Box(
                            modifier = Modifier
                                .size(8.dp)
                                .background(
                                    if (it.isHealthy) MaterialTheme.colorScheme.primary
                                    else MaterialTheme.colorScheme.error,
                                    shape = androidx.compose.foundation.shape.CircleShape
                                )
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(
                            text = if (it.isHealthy) "Healthy" else "Unhealthy",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
            if (health != null && !health.isHealthy) {
                Icon(
                    Icons.Default.Warning,
                    contentDescription = "Warning",
                    tint = MaterialTheme.colorScheme.error
                )
            }
        }
    }
}

@Composable
fun EmptyPaymentMethodsView(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "No Payment Methods",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Semibold
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Payment methods will appear here once configured",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

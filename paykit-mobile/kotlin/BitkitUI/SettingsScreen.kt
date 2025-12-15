package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.paykit.mobile.paykit_mobile.BitcoinNetworkFfi
import com.paykit.mobile.paykit_mobile.LightningNetworkFfi

/**
 * Settings view model for Bitkit integration
 */
class BitkitSettingsViewModel {
    var appVersion by mutableStateOf("1.0.0")
    var selectedBitcoinNetwork by mutableStateOf(BitcoinNetworkFfi.MAINNET)
    var selectedLightningNetwork by mutableStateOf(LightningNetworkFfi.MAINNET)
    var autoPayAlerts by mutableStateOf(true)
    
    // Navigation callbacks
    var onNavigateToAutoPay: (() -> Unit)? = null
    var onNavigateToSubscriptions: (() -> Unit)? = null
    var onNavigateToPaymentRequests: (() -> Unit)? = null
    var onNavigateToPaymentMethods: (() -> Unit)? = null
    var onNavigateToIdentityManagement: (() -> Unit)? = null
}

/**
 * Settings screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitSettingsScreen(viewModel: BitkitSettingsViewModel) {
    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Settings") })
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Quick Access Section
            item {
                Text(
                    "Quick Access",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(16.dp, 8.dp)
                )
            }
            
            viewModel.onNavigateToAutoPay?.let { onAutoPay ->
                item {
                    SettingsNavigationItem(
                        title = "Auto-Pay",
                        icon = Icons.Default.Repeat,
                        onClick = onAutoPay
                    )
                }
            }
            
            viewModel.onNavigateToSubscriptions?.let { onSubscriptions ->
                item {
                    SettingsNavigationItem(
                        title = "Subscriptions",
                        icon = Icons.Default.Repeat,
                        onClick = onSubscriptions
                    )
                }
            }
            
            viewModel.onNavigateToPaymentRequests?.let { onPaymentRequests ->
                item {
                    SettingsNavigationItem(
                        title = "Payment Requests",
                        icon = Icons.Default.Mail,
                        onClick = onPaymentRequests
                    )
                }
            }
            
            viewModel.onNavigateToPaymentMethods?.let { onPaymentMethods ->
                item {
                    SettingsNavigationItem(
                        title = "Payment Methods",
                        icon = Icons.Default.CreditCard,
                        onClick = onPaymentMethods
                    )
                }
            }
            
            // Network Section
            item {
                Text(
                    "Network",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(16.dp, 8.dp)
                )
            }
            
            item {
                var bitcoinExpanded by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(
                    expanded = bitcoinExpanded,
                    onExpandedChange = { bitcoinExpanded = !bitcoinExpanded }
                ) {
                    OutlinedTextField(
                        value = viewModel.selectedBitcoinNetwork.name,
                        onValueChange = {},
                        readOnly = true,
                        label = { Text("Bitcoin Network") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = bitcoinExpanded) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp)
                            .menuAnchor()
                    )
                    ExposedDropdownMenu(
                        expanded = bitcoinExpanded,
                        onDismissRequest = { bitcoinExpanded = false }
                    ) {
                        BitcoinNetworkFfi.values().forEach { network ->
                            DropdownMenuItem(
                                text = { Text(network.name) },
                                onClick = {
                                    viewModel.selectedBitcoinNetwork = network
                                    bitcoinExpanded = false
                                }
                            )
                        }
                    }
                }
            }
            
            item {
                var lightningExpanded by remember { mutableStateOf(false) }
                ExposedDropdownMenuBox(
                    expanded = lightningExpanded,
                    onExpandedChange = { lightningExpanded = !lightningExpanded }
                ) {
                    OutlinedTextField(
                        value = viewModel.selectedLightningNetwork.name,
                        onValueChange = {},
                        readOnly = true,
                        label = { Text("Lightning Network") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = lightningExpanded) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp)
                            .menuAnchor()
                    )
                    ExposedDropdownMenu(
                        expanded = lightningExpanded,
                        onDismissRequest = { lightningExpanded = false }
                    ) {
                        LightningNetworkFfi.values().forEach { network ->
                            DropdownMenuItem(
                                text = { Text(network.name) },
                                onClick = {
                                    viewModel.selectedLightningNetwork = network
                                    lightningExpanded = false
                                }
                            )
                        }
                    }
                }
            }
            
            // Notifications Section
            item {
                Text(
                    "Notifications",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(16.dp, 8.dp)
                )
            }
            
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Auto-Pay Alerts")
                    Switch(
                        checked = viewModel.autoPayAlerts,
                        onCheckedChange = { viewModel.autoPayAlerts = it }
                    )
                }
            }
            
            // App Info Section
            item {
                Text(
                    "App Information",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(16.dp, 8.dp)
                )
            }
            
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Version")
                    Text(
                        viewModel.appVersion,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            
            // Identity Management
            viewModel.onNavigateToIdentityManagement?.let { onIdentityManagement ->
                item {
                    Text(
                        "Identity",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.padding(16.dp, 8.dp)
                    )
                }
                
                item {
                    SettingsNavigationItem(
                        title = "Manage Identities",
                        icon = Icons.Default.Person,
                        onClick = onIdentityManagement
                    )
                }
            }
        }
    }
}

@Composable
fun SettingsNavigationItem(
    title: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 4.dp),
        onClick = onClick
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(icon, contentDescription = null)
            Text(title, style = MaterialTheme.typography.bodyLarge)
        }
    }
}

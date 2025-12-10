package com.paykit.demo.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import com.paykit.demo.PaykitDemoApp

/**
 * Settings Screen
 *
 * Application settings including:
 * - App info and version
 * - Network selection
 * - Security settings
 * - Notification preferences
 * - Advanced/developer options
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen() {
    var selectedNetwork by remember { mutableStateOf("Mainnet") }
    var useTestnet by remember { mutableStateOf(false) }
    var requireBiometric by remember { mutableStateOf(false) }
    var lockOnBackground by remember { mutableStateOf(true) }
    var paymentNotifications by remember { mutableStateOf(true) }
    var subscriptionReminders by remember { mutableStateOf(true) }
    var autoPayAlerts by remember { mutableStateOf(true) }
    var limitWarnings by remember { mutableStateOf(true) }
    var showResetDialog by remember { mutableStateOf(false) }

    val uriHandler = LocalUriHandler.current

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") }
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            // About Section
            item {
                SettingsSectionHeader("About")
            }

            item {
                SettingsItem(
                    title = "Version",
                    subtitle = "1.0.0"
                )
            }

            item {
                SettingsItem(
                    title = "Paykit Library",
                    subtitle = "0.0.1"
                )
            }

            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text("Client Status", style = MaterialTheme.typography.bodyLarge)
                    }
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Surface(
                            color = Color(0xFF4CAF50),
                            shape = MaterialTheme.shapes.small,
                            modifier = Modifier.size(8.dp)
                        ) {}
                        Text(
                            "Connected",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }

            item { Divider() }

            // Network Section
            item {
                SettingsSectionHeader("Network")
            }

            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text("Network", style = MaterialTheme.typography.bodyLarge)
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        listOf("Mainnet", "Testnet", "Regtest").forEach { network ->
                            FilterChip(
                                selected = selectedNetwork == network,
                                onClick = { selectedNetwork = network },
                                label = { Text(network) }
                            )
                        }
                    }
                }
            }

            item {
                SettingsSwitch(
                    title = "Use Testnet for Demo",
                    checked = useTestnet,
                    onCheckedChange = { useTestnet = it }
                )
            }

            item { Divider() }

            // Security Section
            item {
                SettingsSectionHeader("Security")
            }

            item {
                SettingsSwitch(
                    title = "Require Biometric",
                    checked = requireBiometric,
                    onCheckedChange = { requireBiometric = it }
                )
            }

            item {
                SettingsSwitch(
                    title = "Lock on Background",
                    checked = lockOnBackground,
                    onCheckedChange = { lockOnBackground = it }
                )
            }

            item {
                SettingsItem(
                    title = "Manage Keys",
                    subtitle = "View and export your keys",
                    onClick = { /* Navigate to key management */ }
                )
            }

            item { Divider() }

            // Notifications Section
            item {
                SettingsSectionHeader("Notifications")
            }

            item {
                SettingsSwitch(
                    title = "Payment Notifications",
                    checked = paymentNotifications,
                    onCheckedChange = { paymentNotifications = it }
                )
            }

            item {
                SettingsSwitch(
                    title = "Subscription Reminders",
                    checked = subscriptionReminders,
                    onCheckedChange = { subscriptionReminders = it }
                )
            }

            item {
                SettingsSwitch(
                    title = "Auto-Pay Alerts",
                    checked = autoPayAlerts,
                    onCheckedChange = { autoPayAlerts = it }
                )
            }

            item {
                SettingsSwitch(
                    title = "Limit Warnings",
                    checked = limitWarnings,
                    onCheckedChange = { limitWarnings = it }
                )
            }

            item { Divider() }

            // Advanced Section
            item {
                SettingsSectionHeader("Advanced")
            }

            item {
                SettingsItem(
                    title = "Developer Options",
                    subtitle = "Debug logging, test payments",
                    onClick = { /* Navigate to developer options */ }
                )
            }

            item {
                SettingsItem(
                    title = "Clear Cache",
                    subtitle = null,
                    titleColor = Color(0xFFFFA500),
                    onClick = { /* Clear cache */ }
                )
            }

            item {
                SettingsItem(
                    title = "Reset All Settings",
                    subtitle = null,
                    titleColor = Color.Red,
                    onClick = { showResetDialog = true }
                )
            }

            item { Divider() }

            // Help Section
            item {
                SettingsSectionHeader("Help & Support")
            }

            item {
                SettingsItem(
                    title = "Documentation",
                    subtitle = null,
                    onClick = { uriHandler.openUri("https://paykit.dev/docs") }
                )
            }

            item {
                SettingsItem(
                    title = "GitHub Repository",
                    subtitle = null,
                    onClick = { uriHandler.openUri("https://github.com/paykit") }
                )
            }

            item {
                SettingsItem(
                    title = "Report Issue",
                    subtitle = null,
                    onClick = { uriHandler.openUri("https://github.com/paykit/issues") }
                )
            }

            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
    }

    // Reset Confirmation Dialog
    if (showResetDialog) {
        AlertDialog(
            onDismissRequest = { showResetDialog = false },
            title = { Text("Reset Settings") },
            text = { Text("This will reset all settings to their defaults. This cannot be undone.") },
            confirmButton = {
                TextButton(
                    onClick = {
                        PaykitDemoApp.instance.clearAllData()
                        showResetDialog = false
                    },
                    colors = ButtonDefaults.textButtonColors(
                        contentColor = Color.Red
                    )
                ) {
                    Text("Reset")
                }
            },
            dismissButton = {
                TextButton(onClick = { showResetDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@Composable
fun SettingsSectionHeader(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp)
    )
}

@Composable
fun SettingsItem(
    title: String,
    subtitle: String?,
    titleColor: Color = MaterialTheme.colorScheme.onSurface,
    onClick: (() -> Unit)? = null
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable { onClick() } else Modifier)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge,
                color = titleColor
            )
            subtitle?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
        if (onClick != null) {
            Icon(
                Icons.Default.ChevronRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun SettingsSwitch(
    title: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.bodyLarge
        )
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange
        )
    }
}

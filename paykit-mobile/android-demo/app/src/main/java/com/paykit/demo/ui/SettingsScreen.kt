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
import androidx.compose.ui.unit.dp

/**
 * Settings Screen
 *
 * Application settings demo placeholder.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen() {
    var selectedNetwork by remember { mutableStateOf("Mainnet") }
    var requireBiometric by remember { mutableStateOf(false) }
    var lockOnBackground by remember { mutableStateOf(true) }
    var paymentNotifications by remember { mutableStateOf(true) }
    var showResetDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Settings") })
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            // About Section
            item { SettingsSectionHeader("About") }

            item {
                SettingsItem(
                    title = "Version",
                    subtitle = "1.0.0 (Demo)"
                )
            }

            item {
                SettingsItem(
                    title = "Paykit Library",
                    subtitle = "0.0.1"
                )
            }

            item { HorizontalDivider() }

            // Network Section
            item { SettingsSectionHeader("Network") }

            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Network", style = MaterialTheme.typography.bodyLarge)
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        listOf("Mainnet", "Testnet").forEach { network ->
                            FilterChip(
                                selected = selectedNetwork == network,
                                onClick = { selectedNetwork = network },
                                label = { Text(network) }
                            )
                        }
                    }
                }
            }

            item { HorizontalDivider() }

            // Security Section
            item { SettingsSectionHeader("Security") }

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

            item { HorizontalDivider() }

            // Notifications Section
            item { SettingsSectionHeader("Notifications") }

            item {
                SettingsSwitch(
                    title = "Payment Notifications",
                    checked = paymentNotifications,
                    onCheckedChange = { paymentNotifications = it }
                )
            }

            item { HorizontalDivider() }

            // Advanced Section
            item { SettingsSectionHeader("Advanced") }

            item {
                SettingsItem(
                    title = "Reset All Settings",
                    subtitle = null,
                    titleColor = Color.Red,
                    onClick = { showResetDialog = true }
                )
            }

            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
    }

    if (showResetDialog) {
        AlertDialog(
            onDismissRequest = { showResetDialog = false },
            title = { Text("Reset Settings") },
            text = { Text("This will reset all settings to their defaults.") },
            confirmButton = {
                TextButton(
                    onClick = { showResetDialog = false },
                    colors = ButtonDefaults.textButtonColors(contentColor = Color.Red)
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
        Text(text = title, style = MaterialTheme.typography.bodyLarge)
        Switch(checked = checked, onCheckedChange = onCheckedChange)
    }
}

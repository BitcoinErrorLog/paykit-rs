package com.paykit.demo.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import java.util.UUID

/**
 * Settings Screen
 *
 * Application settings including key management and developer options.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen() {
    var selectedNetwork by remember { mutableStateOf("Mainnet") }
    var requireBiometric by remember { mutableStateOf(false) }
    var lockOnBackground by remember { mutableStateOf(true) }
    var paymentNotifications by remember { mutableStateOf(true) }
    var subscriptionReminders by remember { mutableStateOf(true) }
    var autoPayAlerts by remember { mutableStateOf(true) }
    var limitWarnings by remember { mutableStateOf(true) }
    var showResetDialog by remember { mutableStateOf(false) }
    var showKeyManagement by remember { mutableStateOf(false) }
    var showDeveloperOptions by remember { mutableStateOf(false) }

    if (showKeyManagement) {
        KeyManagementScreen(onBack = { showKeyManagement = false })
    } else if (showDeveloperOptions) {
        DeveloperOptionsScreen(onBack = { showDeveloperOptions = false })
    } else {
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

                item {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Client Status", style = MaterialTheme.typography.bodyLarge)
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

                item {
                    SettingsItem(
                        title = "Manage Keys",
                        subtitle = "View and export your keys",
                        onClick = { showKeyManagement = true }
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

                item { HorizontalDivider() }

                // Advanced Section
                item { SettingsSectionHeader("Advanced") }

                item {
                    SettingsItem(
                        title = "Developer Options",
                        subtitle = "Debug logging, test payments",
                        onClick = { showDeveloperOptions = true }
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

                item { Spacer(modifier = Modifier.height(16.dp)) }
            }
        }

        if (showResetDialog) {
            AlertDialog(
                onDismissRequest = { showResetDialog = false },
                title = { Text("Reset Settings") },
                text = { Text("This will reset all settings to their defaults. This cannot be undone.") },
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
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KeyManagementScreen(onBack: () -> Unit) {
    var publicKey by remember { mutableStateOf("pk1abc123def456ghi789jkl012mno345") }
    var showGenerateDialog by remember { mutableStateOf(false) }
    val clipboardManager = LocalClipboardManager.current

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Key Management") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            item { SettingsSectionHeader("Your Keys") }

            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Public Key", style = MaterialTheme.typography.bodyLarge)
                    Text(
                        text = publicKey.take(16) + "...",
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            item {
                SettingsItem(
                    title = "Copy Public Key",
                    subtitle = null,
                    onClick = {
                        clipboardManager.setText(AnnotatedString(publicKey))
                    }
                )
            }

            item {
                SettingsItem(
                    title = "Export Keys",
                    subtitle = null,
                    onClick = { /* Show export options */ }
                )
            }

            item { HorizontalDivider() }

            item { SettingsSectionHeader("Key Management") }

            item {
                SettingsItem(
                    title = "Generate New Keypair",
                    subtitle = null,
                    titleColor = Color(0xFFFFA500),
                    onClick = { showGenerateDialog = true }
                )
            }

            item {
                SettingsItem(
                    title = "Import from Backup",
                    subtitle = null,
                    onClick = { /* Show import dialog */ }
                )
            }

            item {
                Text(
                    text = "Warning: Generating a new keypair will change your identity",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
                )
            }
        }
    }

    if (showGenerateDialog) {
        AlertDialog(
            onDismissRequest = { showGenerateDialog = false },
            title = { Text("Generate New Keys?") },
            text = { Text("This will replace your current keys. Make sure you have a backup!") },
            confirmButton = {
                TextButton(
                    onClick = {
                        publicKey = "pk1new${UUID.randomUUID().toString().take(20)}"
                        showGenerateDialog = false
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = Color.Red)
                ) {
                    Text("Generate")
                }
            },
            dismissButton = {
                TextButton(onClick = { showGenerateDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DeveloperOptionsScreen(onBack: () -> Unit) {
    var debugLogging by remember { mutableStateOf(false) }
    var showRequestResponse by remember { mutableStateOf(false) }
    var mockPayments by remember { mutableStateOf(true) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Developer Options") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            item { SettingsSectionHeader("Debug") }

            item {
                SettingsSwitch(
                    title = "Debug Logging",
                    checked = debugLogging,
                    onCheckedChange = { debugLogging = it }
                )
            }

            item {
                SettingsSwitch(
                    title = "Show Request/Response",
                    checked = showRequestResponse,
                    onCheckedChange = { showRequestResponse = it }
                )
            }

            item {
                SettingsSwitch(
                    title = "Mock Payments",
                    checked = mockPayments,
                    onCheckedChange = { mockPayments = it }
                )
            }

            item { HorizontalDivider() }

            item { SettingsSectionHeader("Testing") }

            item {
                SettingsItem(
                    title = "Trigger Test Payment",
                    subtitle = null,
                    onClick = { /* Trigger test payment */ }
                )
            }

            item {
                SettingsItem(
                    title = "Simulate Auto-Pay",
                    subtitle = null,
                    onClick = { /* Simulate auto-pay */ }
                )
            }

            item {
                SettingsItem(
                    title = "Test Notification",
                    subtitle = null,
                    onClick = { /* Send test notification */ }
                )
            }

            item { HorizontalDivider() }

            item { SettingsSectionHeader("Stats") }

            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Pending Payments", style = MaterialTheme.typography.bodyLarge)
                    Text("0", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }

            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Cache Size", style = MaterialTheme.typography.bodyLarge)
                    Text("1.2 MB", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }

            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Last Sync", style = MaterialTheme.typography.bodyLarge)
                    Text("Just now", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }

            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
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

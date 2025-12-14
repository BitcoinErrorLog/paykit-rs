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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.paykit.mobile.KeyManager
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
    var showIdentityList by remember { mutableStateOf(false) }

    if (showIdentityList) {
        IdentityListScreen(onNavigateBack = { showIdentityList = false })
        return
    }
    
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

                // Identity Section
                item { SettingsSectionHeader("Identity") }
                
                item {
                    val context = LocalContext.current
                    val keyManager = remember(context) { KeyManager(context) }
                    val currentIdentityName by keyManager.currentIdentityName.collectAsState()
                    
                    SettingsItem(
                        title = "Current Identity",
                        subtitle = currentIdentityName ?: "None"
                    )
                }
                
                item {
                    SettingsItem(
                        title = "Manage Identities",
                        subtitle = "Switch or create identities",
                        onClick = { showIdentityList = true }
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
    val context = LocalContext.current
    val keyManager = remember { com.paykit.mobile.KeyManager(context) }
    
    val hasIdentity by keyManager.hasIdentity.collectAsState()
    val publicKeyZ32 by keyManager.publicKeyZ32.collectAsState()
    val publicKeyHex by keyManager.publicKeyHex.collectAsState()
    
    var showGenerateDialog by remember { mutableStateOf(false) }
    var showExportDialog by remember { mutableStateOf(false) }
    var showImportDialog by remember { mutableStateOf(false) }
    var showExportResult by remember { mutableStateOf(false) }
    var exportPassword by remember { mutableStateOf("") }
    var importPassword by remember { mutableStateOf("") }
    var importBackupText by remember { mutableStateOf("") }
    var exportedBackup by remember { mutableStateOf("") }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    
    val clipboardManager = LocalClipboardManager.current

    // Auto-create identity if none exists
    LaunchedEffect(hasIdentity) {
        if (!hasIdentity) {
            try {
                keyManager.getOrCreateIdentity()
            } catch (e: Exception) {
                errorMessage = "Failed to create identity: ${e.message}"
            }
        }
    }

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
            item { SettingsSectionHeader("Your Identity") }

            if (hasIdentity) {
                item {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 12.dp)
                    ) {
                        Text(
                            "pkarr Identity (z-base32)",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = publicKeyZ32,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            maxLines = 2
                        )
                    }
                }

                item {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 12.dp)
                    ) {
                        Text(
                            "Hex Format",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = publicKeyHex.take(32) + "...",
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace
                        )
                    }
                }

                item {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Device ID", style = MaterialTheme.typography.bodyLarge)
                        Text(
                            text = keyManager.getDeviceId().take(12) + "...",
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
                            val keyToCopy = if (publicKeyZ32.isNotEmpty()) publicKeyZ32 else publicKeyHex
                            clipboardManager.setText(AnnotatedString(keyToCopy))
                        }
                    )
                }

                item { HorizontalDivider() }

                item { SettingsSectionHeader("Backup") }

                item {
                    SettingsItem(
                        title = "Export Encrypted Backup",
                        subtitle = "Save your keys with password protection",
                        onClick = { showExportDialog = true }
                    )
                }
            } else {
                item {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Icon(
                            Icons.Default.Key,
                            contentDescription = null,
                            modifier = Modifier.size(48.dp),
                            tint = Color(0xFFF7931A)
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Text("No Identity", style = MaterialTheme.typography.headlineSmall)
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            "Generate a new keypair or import from backup",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Button(
                            onClick = {
                                try {
                                    keyManager.getOrCreateIdentity()
                                } catch (e: Exception) {
                                    errorMessage = "Failed to create identity: ${e.message}"
                                }
                            },
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFFF7931A))
                        ) {
                            Text("Create Identity")
                        }
                    }
                }
            }

            item { HorizontalDivider() }

            item { SettingsSectionHeader("Restore") }

            item {
                SettingsItem(
                    title = "Import from Backup",
                    subtitle = "Restore keys from encrypted backup",
                    onClick = { showImportDialog = true }
                )
            }

            item { HorizontalDivider() }

            item { SettingsSectionHeader("Advanced") }

            item {
                SettingsItem(
                    title = "Generate New Keypair",
                    subtitle = null,
                    titleColor = Color(0xFFFFA500),
                    onClick = { showGenerateDialog = true }
                )
            }

            item {
                Text(
                    text = "⚠️ Generating a new keypair will replace your current identity. Make sure you have a backup first!",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
                )
            }

            // Error display
            errorMessage?.let { error ->
                item {
                    Text(
                        text = error,
                        style = MaterialTheme.typography.bodySmall,
                        color = Color.Red,
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
                    )
                }
            }
        }
    }

    // Generate confirmation dialog
    if (showGenerateDialog) {
        AlertDialog(
            onDismissRequest = { showGenerateDialog = false },
            title = { Text("Generate New Keys?") },
            text = { Text("This will replace your current keys. Make sure you have a backup!") },
            confirmButton = {
                TextButton(
                    onClick = {
                        try {
                            keyManager.generateNewIdentity()
                            errorMessage = null
                        } catch (e: Exception) {
                            errorMessage = "Failed to generate keypair: ${e.message}"
                        }
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

    // Export dialog
    if (showExportDialog) {
        AlertDialog(
            onDismissRequest = { showExportDialog = false; exportPassword = "" },
            title = { Text("Export Backup") },
            text = {
                Column {
                    Text("Enter a password to encrypt your backup:")
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = exportPassword,
                        onValueChange = { exportPassword = it },
                        label = { Text("Password") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        if (exportPassword.isNotEmpty()) {
                            try {
                                val backup = keyManager.exportBackup(exportPassword)
                                exportedBackup = keyManager.backupToString(backup)
                                showExportDialog = false
                                showExportResult = true
                                exportPassword = ""
                                errorMessage = null
                            } catch (e: Exception) {
                                errorMessage = "Export failed: ${e.message}"
                            }
                        }
                    }
                ) {
                    Text("Export")
                }
            },
            dismissButton = {
                TextButton(onClick = { showExportDialog = false; exportPassword = "" }) {
                    Text("Cancel")
                }
            }
        )
    }

    // Export result dialog
    if (showExportResult) {
        AlertDialog(
            onDismissRequest = { showExportResult = false },
            title = { Text("Backup Created!") },
            text = {
                Column {
                    Text("Copy this encrypted backup and store it safely:")
                    Spacer(modifier = Modifier.height(8.dp))
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant,
                        shape = MaterialTheme.shapes.small
                    ) {
                        Text(
                            text = exportedBackup,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            modifier = Modifier.padding(8.dp)
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        clipboardManager.setText(AnnotatedString(exportedBackup))
                        showExportResult = false
                    }
                ) {
                    Text("Copy & Close")
                }
            },
            dismissButton = {
                TextButton(onClick = { showExportResult = false }) {
                    Text("Close")
                }
            }
        )
    }

    // Import dialog
    if (showImportDialog) {
        AlertDialog(
            onDismissRequest = { 
                showImportDialog = false
                importPassword = ""
                importBackupText = ""
            },
            title = { Text("Import Backup") },
            text = {
                Column {
                    Text("Paste your backup JSON:")
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = importBackupText,
                        onValueChange = { importBackupText = it },
                        label = { Text("Backup JSON") },
                        maxLines = 5
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = importPassword,
                        onValueChange = { importPassword = it },
                        label = { Text("Password") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        if (importBackupText.isNotEmpty() && importPassword.isNotEmpty()) {
                            try {
                                val backup = keyManager.backupFromString(importBackupText)
                                keyManager.importBackup(backup, importPassword)
                                showImportDialog = false
                                importBackupText = ""
                                importPassword = ""
                                errorMessage = null
                            } catch (e: Exception) {
                                errorMessage = "Import failed: ${e.message}"
                            }
                        }
                    }
                ) {
                    Text("Import")
                }
            },
            dismissButton = {
                TextButton(onClick = { 
                    showImportDialog = false
                    importBackupText = ""
                    importPassword = ""
                }) {
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

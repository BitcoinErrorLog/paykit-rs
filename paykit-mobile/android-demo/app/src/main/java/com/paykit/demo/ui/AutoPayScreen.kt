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
import com.paykit.demo.viewmodel.AutoPayViewModel
import com.paykit.storage.AutoPayRule
import com.paykit.storage.PeerSpendingLimit
import com.paykit.storage.RecentAutoPayment
import com.paykit.storage.SpendingPeriod
import java.text.SimpleDateFormat
import java.util.*

/**
 * Auto-Pay Settings Screen
 *
 * Displays and manages auto-pay configuration including:
 * - Global enable/disable
 * - Daily spending limits
 * - Per-peer limits
 * - Auto-pay rules
 * - Recent auto-payments
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AutoPayScreen(
    viewModel: AutoPayViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    var showAddPeerDialog by remember { mutableStateOf(false) }
    var showAddRuleDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Auto-Pay") },
                actions = {
                    IconButton(onClick = { viewModel.resetToDefaults() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Reset")
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
            // Status Section
            item {
                AutoPayStatusCard(
                    isEnabled = uiState.isEnabled,
                    onEnabledChange = { viewModel.setEnabled(it) }
                )
            }

            if (uiState.isEnabled) {
                // Global Spending Limit Section
                item {
                    SpendingLimitCard(
                        dailyLimit = uiState.dailyLimit,
                        usedToday = uiState.usedToday,
                        onLimitChange = { viewModel.setDailyLimit(it) }
                    )
                }

                // Per-Peer Limits Section
                item {
                    SectionHeader(
                        title = "Per-Peer Limits",
                        onAdd = { showAddPeerDialog = true }
                    )
                }

                items(uiState.peerLimits) { peerLimit ->
                    PeerLimitCard(
                        peerLimit = peerLimit,
                        onDelete = { viewModel.removePeerLimit(peerLimit.peerPubkey) }
                    )
                }

                if (uiState.peerLimits.isEmpty()) {
                    item {
                        EmptyState("No peer limits configured")
                    }
                }

                // Auto-Pay Rules Section
                item {
                    SectionHeader(
                        title = "Auto-Pay Rules",
                        onAdd = { showAddRuleDialog = true }
                    )
                }

                items(uiState.rules) { rule ->
                    RuleCard(
                        rule = rule,
                        onToggle = { viewModel.toggleRule(rule.id) },
                        onDelete = { viewModel.removeRule(rule.id) }
                    )
                }

                if (uiState.rules.isEmpty()) {
                    item {
                        EmptyState("No auto-pay rules configured")
                    }
                }

                // Recent Payments Section
                item {
                    Text(
                        text = "Recent Auto-Payments",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.padding(top = 16.dp, bottom = 8.dp)
                    )
                }

                items(uiState.recentPayments.take(5)) { payment ->
                    RecentPaymentCard(payment = payment)
                }

                if (uiState.recentPayments.isEmpty()) {
                    item {
                        EmptyState("No recent auto-payments")
                    }
                }

                item { Spacer(modifier = Modifier.height(16.dp)) }
            }
        }
    }

    // Add Peer Limit Dialog
    if (showAddPeerDialog) {
        AddPeerLimitDialog(
            onDismiss = { showAddPeerDialog = false },
            onConfirm = { peerLimit ->
                viewModel.addPeerLimit(peerLimit)
                showAddPeerDialog = false
            }
        )
    }

    // Add Rule Dialog
    if (showAddRuleDialog) {
        AddRuleDialog(
            onDismiss = { showAddRuleDialog = false },
            onConfirm = { rule ->
                viewModel.addRule(rule)
                showAddRuleDialog = false
            }
        )
    }
}

@Composable
fun AutoPayStatusCard(
    isEnabled: Boolean,
    onEnabledChange: (Boolean) -> Unit
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
            Column {
                Text(
                    text = "Auto-Pay",
                    style = MaterialTheme.typography.titleMedium
                )
                Text(
                    text = if (isEnabled) "Enabled" else "Disabled",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Switch(
                checked = isEnabled,
                onCheckedChange = onEnabledChange
            )
        }
    }
}

@Composable
fun SpendingLimitCard(
    dailyLimit: Long,
    usedToday: Long,
    onLimitChange: (Long) -> Unit
) {
    val progress = if (dailyLimit > 0) usedToday.toFloat() / dailyLimit else 0f
    val progressColor = when {
        progress > 0.9f -> Color.Red
        progress > 0.7f -> Color(0xFFFFA500) // Orange
        else -> Color(0xFF4CAF50) // Green
    }

    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            Text(
                text = "Global Spending Limit",
                style = MaterialTheme.typography.titleMedium
            )

            Spacer(modifier = Modifier.height(16.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Daily Limit")
                Text("${dailyLimit.formatSats()} sats")
            }

            Slider(
                value = dailyLimit.toFloat(),
                onValueChange = { onLimitChange(it.toLong()) },
                valueRange = 1000f..1000000f,
                steps = 99,
                modifier = Modifier.fillMaxWidth()
            )

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Used Today")
                Text(
                    text = "${usedToday.formatSats()} sats",
                    color = progressColor
                )
            }

            LinearProgressIndicator(
                progress = progress.coerceIn(0f, 1f),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(8.dp),
                color = progressColor
            )
        }
    }
}

@Composable
fun SectionHeader(
    title: String,
    onAdd: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 16.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium
        )
        IconButton(onClick = onAdd) {
            Icon(Icons.Default.Add, contentDescription = "Add")
        }
    }
}

@Composable
fun PeerLimitCard(
    peerLimit: PeerSpendingLimit,
    onDelete: () -> Unit
) {
    val progress = peerLimit.percentUsed

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
                        text = peerLimit.peerName,
                        style = MaterialTheme.typography.titleSmall
                    )
                    Text(
                        text = peerLimit.peerPubkey.take(16) + "...",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        text = "${peerLimit.limit.formatSats()} sats",
                        style = MaterialTheme.typography.bodyMedium
                    )
                    Text(
                        text = peerLimit.period.name,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                LinearProgressIndicator(
                    progress = progress.coerceIn(0f, 1f),
                    modifier = Modifier
                        .weight(1f)
                        .height(6.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "${peerLimit.used.formatSats()}/${peerLimit.limit.formatSats()}",
                    style = MaterialTheme.typography.bodySmall
                )
                IconButton(onClick = onDelete) {
                    Icon(
                        Icons.Default.Delete,
                        contentDescription = "Delete",
                        tint = MaterialTheme.colorScheme.error
                    )
                }
            }
        }
    }
}

@Composable
fun RuleCard(
    rule: AutoPayRule,
    onToggle: () -> Unit,
    onDelete: () -> Unit
) {
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
                imageVector = if (rule.isEnabled) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                contentDescription = null,
                tint = if (rule.isEnabled) Color(0xFF4CAF50) else Color.Gray,
                modifier = Modifier.size(24.dp)
            )

            Spacer(modifier = Modifier.width(12.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = rule.name,
                    style = MaterialTheme.typography.titleSmall
                )
                Text(
                    text = rule.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            rule.maxAmount?.let { max ->
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    shape = MaterialTheme.shapes.small
                ) {
                    Text(
                        text = "≤ ${max.formatSats()} sats",
                        style = MaterialTheme.typography.labelSmall,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }

            IconButton(onClick = onToggle) {
                Icon(
                    imageVector = if (rule.isEnabled) Icons.Default.ToggleOn else Icons.Default.ToggleOff,
                    contentDescription = "Toggle"
                )
            }

            IconButton(onClick = onDelete) {
                Icon(
                    Icons.Default.Delete,
                    contentDescription = "Delete",
                    tint = MaterialTheme.colorScheme.error
                )
            }
        }
    }
}

@Composable
fun RecentPaymentCard(payment: RecentAutoPayment) {
    val dateFormat = remember { SimpleDateFormat("MMM d, HH:mm", Locale.getDefault()) }

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
            Column {
                Text(
                    text = payment.peerName,
                    style = MaterialTheme.typography.titleSmall
                )
                Text(
                    text = payment.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = "${payment.amount.formatSats()} sats",
                    style = MaterialTheme.typography.bodyMedium,
                    color = Color(0xFF4CAF50)
                )
                Text(
                    text = dateFormat.format(Date(payment.timestamp)),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
fun EmptyState(message: String) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(32.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddPeerLimitDialog(
    onDismiss: () -> Unit,
    onConfirm: (PeerSpendingLimit) -> Unit
) {
    var peerPubkey by remember { mutableStateOf("") }
    var peerName by remember { mutableStateOf("") }
    var limit by remember { mutableStateOf(10000L) }
    var period by remember { mutableStateOf(SpendingPeriod.DAILY) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add Peer Limit") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = peerPubkey,
                    onValueChange = { peerPubkey = it },
                    label = { Text("Peer Public Key") },
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = peerName,
                    onValueChange = { peerName = it },
                    label = { Text("Display Name") },
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = limit.toString(),
                    onValueChange = { it.toLongOrNull()?.let { l -> limit = l } },
                    label = { Text("Limit (sats)") },
                    modifier = Modifier.fillMaxWidth()
                )
                // Period dropdown would go here
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    onConfirm(
                        PeerSpendingLimit(
                            peerPubkey = peerPubkey,
                            peerName = peerName.ifEmpty { "Unknown" },
                            limit = limit,
                            used = 0,
                            period = period,
                            periodStart = System.currentTimeMillis()
                        )
                    )
                },
                enabled = peerPubkey.isNotEmpty()
            ) {
                Text("Add")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddRuleDialog(
    onDismiss: () -> Unit,
    onConfirm: (AutoPayRule) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var maxAmount by remember { mutableStateOf(1000L) }
    var hasMaxAmount by remember { mutableStateOf(true) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add Auto-Pay Rule") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Rule Name") },
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description") },
                    modifier = Modifier.fillMaxWidth()
                )
                Row(
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Checkbox(
                        checked = hasMaxAmount,
                        onCheckedChange = { hasMaxAmount = it }
                    )
                    Text("Maximum Amount")
                }
                if (hasMaxAmount) {
                    OutlinedTextField(
                        value = maxAmount.toString(),
                        onValueChange = { it.toLongOrNull()?.let { a -> maxAmount = a } },
                        label = { Text("Max Amount (sats)") },
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    onConfirm(
                        AutoPayRule(
                            id = UUID.randomUUID().toString(),
                            name = name,
                            description = description,
                            isEnabled = true,
                            maxAmount = if (hasMaxAmount) maxAmount else null,
                            methodFilter = null,
                            peerFilter = null
                        )
                    )
                },
                enabled = name.isNotEmpty()
            ) {
                Text("Add")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

// Helper extension function
fun Long.formatSats(): String {
    return when {
        this >= 1_000_000 -> String.format("%.1fM", this / 1_000_000.0)
        this >= 1_000 -> String.format("%.1fK", this / 1_000.0)
        else -> this.toString()
    }
}

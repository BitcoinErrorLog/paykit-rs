package com.paykit.demo.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.layout.Box
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.paykit.demo.model.Receipt
import com.paykit.demo.viewmodel.AutoPayViewModel
import java.text.SimpleDateFormat
import java.util.*

/**
 * Auto-Pay Settings Screen
 *
 * Demo placeholder for auto-pay configuration.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AutoPayScreen(
    viewModel: AutoPayViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()

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

                // Recent Auto-Payments Section
                if (uiState.recentAutoPayments.isNotEmpty()) {
                    item {
                        Text(
                            "Recent Payments",
                            style = MaterialTheme.typography.titleMedium,
                            modifier = Modifier.padding(top = 8.dp)
                        )
                    }
                    
                    items(uiState.recentAutoPayments.size) { index ->
                        val receipt = uiState.recentAutoPayments[index]
                        RecentPaymentCard(receipt = receipt)
                    }
                }

                item {
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(
                            modifier = Modifier.padding(16.dp),
                            horizontalAlignment = Alignment.CenterHorizontally
                        ) {
                            Icon(
                                Icons.Default.Build,
                                contentDescription = null,
                                modifier = Modifier.size(48.dp),
                                tint = MaterialTheme.colorScheme.primary
                            )
                            Spacer(modifier = Modifier.height(8.dp))
                            Text(
                                "Full Auto-Pay Coming Soon",
                                style = MaterialTheme.typography.titleMedium
                            )
                            Text(
                                "Per-peer limits, rules, and payment history will be available when the SDK is fully integrated.",
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
fun AutoPayStatusCard(
    isEnabled: Boolean,
    onEnabledChange: (Boolean) -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
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
        progress > 0.7f -> Color(0xFFFFA500)
        else -> Color(0xFF4CAF50)
    }

    Card(modifier = Modifier.fillMaxWidth()) {
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
                progress = { progress.coerceIn(0f, 1f) },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(8.dp),
                color = progressColor
            )
        }
    }
}

fun Long.formatSats(): String {
    return when {
        this >= 1_000_000 -> String.format("%.1fM", this / 1_000_000.0)
        this >= 1_000 -> String.format("%.1fK", this / 1_000.0)
        else -> this.toString()
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

@Composable
fun RecentPaymentCard(receipt: Receipt) {
    val dateFormat = SimpleDateFormat("MMM d, h:mm a", Locale.getDefault())
    val dateStr = dateFormat.format(Date(receipt.timestamp))
    
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    Icons.Default.ArrowUpward,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary
                )
                Column {
                    Text(
                        text = receipt.displayName,
                        style = MaterialTheme.typography.bodyMedium
                    )
                    Text(
                        text = dateStr,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            Text(
                text = "${receipt.amount.formatSats()} sats",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary
            )
        }
    }
}

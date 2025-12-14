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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.paykit.demo.PaykitApplication
import com.paykit.demo.storage.MethodRotationSettings
import com.paykit.demo.storage.RotationEvent
import com.paykit.demo.storage.RotationPolicy
import com.paykit.demo.storage.RotationSettingsStorage
import java.text.SimpleDateFormat
import java.util.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RotationSettingsScreen(
    onNavigateBack: () -> Unit
) {
    val context = LocalContext.current
    val app = context.applicationContext as PaykitApplication
    val identityName = app.currentIdentityName
    
    var autoRotateEnabled by remember { mutableStateOf(true) }
    var defaultPolicy by remember { mutableStateOf(RotationPolicy.ON_USE) }
    var defaultThreshold by remember { mutableStateOf(5) }
    var methodSettings by remember { mutableStateOf<Map<String, MethodRotationSettings>>(emptyMap()) }
    var history by remember { mutableStateOf<List<RotationEvent>>(emptyList()) }
    var totalRotations by remember { mutableStateOf(0) }
    
    var showMethodPicker by remember { mutableStateOf<String?>(null) }
    var showClearHistoryDialog by remember { mutableStateOf(false) }
    
    val storage = remember(identityName) {
        identityName?.let { RotationSettingsStorage(context, it) }
    }
    
    fun load() {
        storage?.let {
            val settings = it.loadSettings()
            autoRotateEnabled = settings.autoRotateEnabled
            defaultPolicy = try { RotationPolicy.valueOf(settings.defaultPolicy) } catch (e: Exception) { RotationPolicy.ON_USE }
            defaultThreshold = settings.defaultThreshold
            methodSettings = settings.methodSettings
            history = it.loadHistory()
            totalRotations = it.totalRotations()
        }
    }
    
    fun save() {
        storage?.let {
            val settings = it.loadSettings().copy(
                autoRotateEnabled = autoRotateEnabled,
                defaultPolicy = defaultPolicy.name,
                defaultThreshold = defaultThreshold
            )
            it.saveSettings(settings)
        }
    }
    
    LaunchedEffect(identityName) {
        load()
    }
    
    // Clear History Dialog
    if (showClearHistoryDialog) {
        AlertDialog(
            onDismissRequest = { showClearHistoryDialog = false },
            title = { Text("Clear History") },
            text = { Text("Are you sure you want to clear the rotation history?") },
            confirmButton = {
                TextButton(
                    onClick = {
                        storage?.clearHistory()
                        history = emptyList()
                        showClearHistoryDialog = false
                    }
                ) {
                    Text("Clear", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearHistoryDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
    
    // Method Policy Picker Dialog
    showMethodPicker?.let { methodId ->
        val currentSettings = methodSettings[methodId] ?: MethodRotationSettings()
        MethodPolicyDialog(
            methodId = methodId,
            settings = currentSettings,
            onDismiss = { showMethodPicker = null },
            onSave = { newSettings ->
                storage?.updateMethodSettings(methodId, newSettings)
                load()
                showMethodPicker = null
            }
        )
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Rotation Settings") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Global Settings Section
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(
                            "Global Settings",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold
                        )
                        
                        Spacer(modifier = Modifier.height(16.dp))
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text("Auto-Rotate After Payments")
                                Text(
                                    "Automatically rotate endpoints after use",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Switch(
                                checked = autoRotateEnabled,
                                onCheckedChange = {
                                    autoRotateEnabled = it
                                    save()
                                }
                            )
                        }
                        
                        Spacer(modifier = Modifier.height(16.dp))
                        
                        Text("Default Policy", style = MaterialTheme.typography.bodyMedium)
                        Spacer(modifier = Modifier.height(8.dp))
                        
                        RotationPolicy.values().forEach { policy ->
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                RadioButton(
                                    selected = defaultPolicy == policy,
                                    onClick = {
                                        defaultPolicy = policy
                                        save()
                                    }
                                )
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(policy.displayName)
                                    Text(
                                        policy.description,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            }
                        }
                        
                        if (defaultPolicy == RotationPolicy.AFTER_USES) {
                            Spacer(modifier = Modifier.height(8.dp))
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(16.dp)
                            ) {
                                Text("Threshold: $defaultThreshold uses")
                                Slider(
                                    value = defaultThreshold.toFloat(),
                                    onValueChange = { defaultThreshold = it.toInt() },
                                    valueRange = 1f..100f,
                                    steps = 98,
                                    onValueChangeFinished = { save() },
                                    modifier = Modifier.weight(1f)
                                )
                            }
                        }
                    }
                }
            }
            
            // Per-Method Settings Section
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(
                            "Per-Method Policies",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold
                        )
                        
                        Spacer(modifier = Modifier.height(16.dp))
                        
                        listOf("lightning", "onchain").forEach { methodId ->
                            val settings = methodSettings[methodId] ?: MethodRotationSettings()
                            MethodPolicyRow(
                                methodId = methodId,
                                settings = settings,
                                onClick = { showMethodPicker = methodId }
                            )
                            if (methodId != "onchain") {
                                Divider(modifier = Modifier.padding(vertical = 8.dp))
                            }
                        }
                    }
                }
            }
            
            // Rotation Status Section
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(
                            "Rotation Status",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold
                        )
                        
                        Spacer(modifier = Modifier.height(16.dp))
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text("Total Rotations")
                            Text(
                                totalRotations.toString(),
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }
                        
                        methodSettings.forEach { (methodId, settings) ->
                            if (settings.rotationCount > 0) {
                                Spacer(modifier = Modifier.height(8.dp))
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Row(
                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        MethodIcon(methodId)
                                        Text(methodId.capitalize())
                                    }
                                    Column(horizontalAlignment = Alignment.End) {
                                        Text("${settings.rotationCount} rotations")
                                        settings.lastRotated?.let { ts ->
                                            Text(
                                                formatRelativeTime(ts),
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // History Section
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                "Rotation History",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold
                            )
                            if (history.isNotEmpty()) {
                                TextButton(onClick = { showClearHistoryDialog = true }) {
                                    Text("Clear", color = MaterialTheme.colorScheme.error)
                                }
                            }
                        }
                        
                        Spacer(modifier = Modifier.height(8.dp))
                        
                        if (history.isEmpty()) {
                            Text(
                                "No rotation history yet",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        } else {
                            history.take(10).forEach { event ->
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(vertical = 4.dp),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Row(
                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        MethodIcon(event.methodId)
                                        Column {
                                            Text(event.methodId.capitalize())
                                            Text(
                                                event.reason,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant
                                            )
                                        }
                                    }
                                    Text(
                                        formatRelativeTime(event.timestamp),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            }
                            
                            if (history.size > 10) {
                                Spacer(modifier = Modifier.height(8.dp))
                                Text(
                                    "+ ${history.size - 10} more events",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun MethodPolicyRow(
    methodId: String,
    settings: MethodRotationSettings,
    onClick: () -> Unit
) {
    val policy = try { RotationPolicy.valueOf(settings.policy) } catch (e: Exception) { RotationPolicy.ON_USE }
    
    Surface(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier.padding(vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                MethodIcon(methodId)
                Column {
                    Text(methodId.capitalize(), fontWeight = FontWeight.Medium)
                    Text(
                        policy.displayName,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    "Uses: ${settings.useCount}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Icon(
                    Icons.Default.ChevronRight,
                    contentDescription = "Edit",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MethodPolicyDialog(
    methodId: String,
    settings: MethodRotationSettings,
    onDismiss: () -> Unit,
    onSave: (MethodRotationSettings) -> Unit
) {
    var policy by remember { mutableStateOf(
        try { RotationPolicy.valueOf(settings.policy) } catch (e: Exception) { RotationPolicy.ON_USE }
    )}
    var threshold by remember { mutableStateOf(settings.threshold) }
    
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("${methodId.capitalize()} Policy") },
        text = {
            Column {
                RotationPolicy.values().forEach { p ->
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        RadioButton(
                            selected = policy == p,
                            onClick = { policy = p }
                        )
                        Column(modifier = Modifier.weight(1f)) {
                            Text(p.displayName)
                            Text(
                                p.description,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
                
                if (policy == RotationPolicy.AFTER_USES) {
                    Spacer(modifier = Modifier.height(16.dp))
                    Text("Threshold: $threshold uses")
                    Slider(
                        value = threshold.toFloat(),
                        onValueChange = { threshold = it.toInt() },
                        valueRange = 1f..100f,
                        steps = 98,
                        modifier = Modifier.fillMaxWidth()
                    )
                }
                
                Spacer(modifier = Modifier.height(16.dp))
                Divider()
                Spacer(modifier = Modifier.height(16.dp))
                
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Current uses", style = MaterialTheme.typography.bodyMedium)
                    Text(settings.useCount.toString())
                }
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Total rotations", style = MaterialTheme.typography.bodyMedium)
                    Text(settings.rotationCount.toString())
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    onSave(settings.copy(
                        policy = policy.name,
                        threshold = threshold
                    ))
                }
            ) {
                Text("Save")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

@Composable
private fun MethodIcon(methodId: String) {
    val (color, icon) = when (methodId.lowercase()) {
        "lightning" -> Color(0xFFFF9800) to Icons.Default.Bolt
        "onchain" -> Color(0xFFFFC107) to Icons.Default.CurrencyBitcoin
        else -> MaterialTheme.colorScheme.primary to Icons.Default.CreditCard
    }
    
    Icon(icon, contentDescription = null, tint = color)
}

private fun formatRelativeTime(timestamp: Long): String {
    val now = System.currentTimeMillis()
    val diff = now - timestamp
    
    return when {
        diff < 60_000 -> "Just now"
        diff < 3600_000 -> "${diff / 60_000}m ago"
        diff < 86400_000 -> "${diff / 3600_000}h ago"
        diff < 604800_000 -> "${diff / 86400_000}d ago"
        else -> SimpleDateFormat("MMM d", Locale.getDefault()).format(Date(timestamp))
    }
}

private fun String.capitalize(): String {
    return this.replaceFirstChar { if (it.isLowerCase()) it.titlecase(Locale.getDefault()) else it.toString() }
}


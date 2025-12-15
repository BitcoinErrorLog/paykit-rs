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
import com.paykit.mobile.bitkit.PeerSpendingLimit
import com.paykit.mobile.bitkit.AutoPayRule

/**
 * Auto-Pay view model for Bitkit integration
 */
class BitkitAutoPayViewModel(
    private val identityName: String,
    private val autoPayStorage: AutoPayStorageProtocol
) {
    var isEnabled by mutableStateOf(false)
        private set
    var dailyLimit by mutableLongStateOf(100000L)
        private set
    var usedToday by mutableLongStateOf(0L)
        private set
    var peerLimits by mutableStateOf<List<PeerSpendingLimit>>(emptyList())
        private set
    var autoPayRules by mutableStateOf<List<AutoPayRule>>(emptyList())
        private set
    var recentPayments by mutableStateOf<List<RecentAutoPayment>>(emptyList())
        private set
    var showingAddPeer by mutableStateOf(false)
    var showingAddRule by mutableStateOf(false)
    
    init {
        loadFromStorage()
    }
    
    fun loadFromStorage() {
        val settings = autoPayStorage.getSettings()
        isEnabled = settings.isEnabled
        dailyLimit = settings.globalDailyLimitSats
        usedToday = 0L // Tracked separately in view model
        
        // Load peer limits
        peerLimits = autoPayStorage.getPeerLimits().map { storedLimit ->
            PeerSpendingLimit(
                id = storedLimit.id,
                peerPubkey = storedLimit.peerPubkey,
                peerName = storedLimit.peerName,
                limitSats = storedLimit.limitSats,
                spentSats = storedLimit.spentSats,
                period = storedLimit.period,
                lastResetDate = storedLimit.lastResetDate
            )
        }
        
        // Load auto-pay rules
        autoPayRules = autoPayStorage.getRules().map { storedRule ->
            AutoPayRule(
                id = storedRule.id,
                name = storedRule.name,
                description = "Max: ${storedRule.maxAmountSats ?: 0} sats",
                isEnabled = storedRule.isEnabled,
                maxAmountSats = storedRule.maxAmountSats,
                allowedMethods = storedRule.allowedMethods,
                allowedPeers = storedRule.allowedPeers
            )
        }
    }
    
    fun setEnabled(enabled: Boolean) {
        isEnabled = enabled
        saveSettings()
    }
    
    fun setDailyLimit(limit: Long) {
        dailyLimit = limit
        saveSettings()
    }
    
    fun addPeerLimit(limit: PeerSpendingLimit) {
        peerLimits = peerLimits + limit
        savePeerLimitToStorage(limit)
    }
    
    fun updatePeerLimit(limit: PeerSpendingLimit) {
        peerLimits = peerLimits.map { if (it.id == limit.id) limit else it }
        savePeerLimitToStorage(limit)
    }
    
    fun removePeerLimit(id: String) {
        peerLimits = peerLimits.filter { it.id != id }
        autoPayStorage.deletePeerLimit(id)
    }
    
    fun addRule(rule: AutoPayRule) {
        autoPayRules = autoPayRules + rule
        saveRuleToStorage(rule)
    }
    
    fun updateRule(rule: AutoPayRule) {
        autoPayRules = autoPayRules.map { if (it.id == rule.id) rule else it }
        saveRuleToStorage(rule)
    }
    
    fun removeRule(id: String) {
        autoPayRules = autoPayRules.filter { it.id != id }
        autoPayStorage.deleteRule(id)
    }
    
    fun shouldAutoApprove(peerPubkey: String, amount: Long, methodId: String): AutoApprovalResult {
        if (!isEnabled) {
            return AutoApprovalResult.Denied("Auto-pay is disabled")
        }
        
        if (usedToday + amount > dailyLimit) {
            return AutoApprovalResult.Denied("Would exceed daily limit")
        }
        
        peerLimits.firstOrNull { it.peerPubkey == peerPubkey }?.let { limit ->
            if (limit.spentSats + amount > limit.limitSats) {
                return AutoApprovalResult.Denied("Would exceed peer limit")
            }
        }
        
        autoPayRules.filter { it.isEnabled }.forEach { rule ->
            if (rule.matches(amount, methodId, peerPubkey)) {
                return AutoApprovalResult.Approved(rule.id, rule.name)
            }
        }
        
        return AutoApprovalResult.NeedsApproval
    }
    
    fun recordPayment(
        peerPubkey: String,
        peerName: String,
        amount: Long,
        description: String,
        ruleId: String?
    ) {
        usedToday += amount
        
        peerLimits.firstOrNull { it.peerPubkey == peerPubkey }?.let { limit ->
            val updated = limit.copy(spentSats = limit.spentSats + amount)
            updatePeerLimit(updated)
        }
        
        val payment = RecentAutoPayment(
            id = java.util.UUID.randomUUID().toString(),
            peerPubkey = peerPubkey,
            peerName = peerName,
            amount = amount,
            description = description,
            timestamp = System.currentTimeMillis(),
            status = PaymentExecutionStatus.COMPLETED,
            ruleId = ruleId
        )
        recentPayments = (listOf(payment) + recentPayments).take(50)
    }
    
    fun resetToDefaults() {
        isEnabled = false
        dailyLimit = 100000L
        usedToday = 0L
        peerLimits = emptyList()
        autoPayRules = emptyList()
        recentPayments = emptyList()
        
        val settings = autoPayStorage.getSettings()
        autoPayStorage.saveSettings(settings.copy(isEnabled = false, globalDailyLimitSats = 100000L))
        
        autoPayStorage.getPeerLimits().forEach { autoPayStorage.deletePeerLimit(it.id) }
        autoPayStorage.getRules().forEach { autoPayStorage.deleteRule(it.id) }
    }
    
    private fun saveSettings() {
        val settings = autoPayStorage.getSettings()
        autoPayStorage.saveSettings(settings.copy(isEnabled = isEnabled, globalDailyLimitSats = dailyLimit))
    }
    
    private fun savePeerLimitToStorage(limit: PeerSpendingLimit) {
        val storedLimit = StoredPeerLimit(
            id = limit.id,
            peerPubkey = limit.peerPubkey,
            peerName = limit.peerName,
            limitSats = limit.limitSats,
            spentSats = limit.spentSats,
            period = limit.period,
            lastResetDate = limit.lastResetDate
        )
        autoPayStorage.savePeerLimit(storedLimit)
    }
    
    private fun saveRuleToStorage(rule: AutoPayRule) {
        val storedRule = StoredAutoPayRule(
            id = rule.id,
            name = rule.name,
            isEnabled = rule.isEnabled,
            maxAmountSats = rule.maxAmountSats,
            allowedMethods = rule.allowedMethods,
            allowedPeers = rule.allowedPeers,
            requireConfirmation = false,
            createdAt = System.currentTimeMillis()
        )
        autoPayStorage.saveRule(storedRule)
    }
}

/**
 * Auto-approval result
 */
sealed class AutoApprovalResult {
    data class Approved(val ruleId: String, val ruleName: String) : AutoApprovalResult()
    data class Denied(val reason: String) : AutoApprovalResult()
    object NeedsApproval : AutoApprovalResult()
    
    val isApproved: Boolean
        get() = this is Approved
}

/**
 * Payment execution status
 */
enum class PaymentExecutionStatus {
    PENDING, PROCESSING, COMPLETED, FAILED
}

/**
 * Recent auto-payment
 */
data class RecentAutoPayment(
    val id: String,
    val peerPubkey: String,
    val peerName: String,
    val amount: Long,
    val description: String,
    val timestamp: Long,
    val status: PaymentExecutionStatus,
    val ruleId: String?
)

/**
 * Peer spending limit model
 */
data class PeerSpendingLimit(
    val id: String,
    val peerPubkey: String,
    val peerName: String,
    val limitSats: Long,
    val spentSats: Long,
    val period: String,
    val lastResetDate: Long
) {
    fun resetIfNeeded(): PeerSpendingLimit {
        val calendar = java.util.Calendar.getInstance()
        val calendarReset = java.util.Calendar.getInstance().apply { timeInMillis = lastResetDate }
        
        val shouldReset = when (period.lowercase()) {
            "daily" -> calendar.get(java.util.Calendar.DAY_OF_YEAR) != calendarReset.get(java.util.Calendar.DAY_OF_YEAR)
            "weekly" -> calendar.get(java.util.Calendar.WEEK_OF_YEAR) != calendarReset.get(java.util.Calendar.WEEK_OF_YEAR)
            "monthly" -> calendar.get(java.util.Calendar.MONTH) != calendarReset.get(java.util.Calendar.MONTH)
            else -> false
        }
        
        return if (shouldReset) {
            copy(spentSats = 0L, lastResetDate = System.currentTimeMillis())
        } else {
            this
        }
    }
}

/**
 * Auto-pay rule model
 */
data class AutoPayRule(
    val id: String,
    val name: String,
    val description: String,
    val isEnabled: Boolean,
    val maxAmountSats: Long?,
    val allowedMethods: List<String>,
    val allowedPeers: List<String>
) {
    fun matches(amount: Long, method: String, peer: String): Boolean {
        if (!isEnabled) return false
        
        maxAmountSats?.let { if (amount > it) return false }
        if (allowedMethods.isNotEmpty() && !allowedMethods.contains(method)) return false
        if (allowedPeers.isNotEmpty() && !allowedPeers.contains(peer)) return false
        
        return true
    }
}

/**
 * Stored peer limit
 */
data class StoredPeerLimit(
    val id: String,
    val peerPubkey: String,
    val peerName: String,
    val limitSats: Long,
    val spentSats: Long,
    val period: String,
    val lastResetDate: Long
)

/**
 * Stored auto-pay rule
 */
data class StoredAutoPayRule(
    val id: String,
    val name: String,
    val isEnabled: Boolean,
    val maxAmountSats: Long?,
    val allowedMethods: List<String>,
    val allowedPeers: List<String>,
    val requireConfirmation: Boolean,
    val createdAt: Long
)

/**
 * Auto-pay storage protocol
 */
interface AutoPayStorageProtocol {
    fun getSettings(): AutoPaySettings
    fun saveSettings(settings: AutoPaySettings)
    fun getPeerLimits(): List<StoredPeerLimit>
    fun savePeerLimit(limit: StoredPeerLimit)
    fun deletePeerLimit(id: String)
    fun getRules(): List<StoredAutoPayRule>
    fun saveRule(rule: StoredAutoPayRule)
    fun deleteRule(id: String)
}

/**
 * Auto-Pay screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitAutoPayScreen(viewModel: BitkitAutoPayViewModel) {
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
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Status Section
            item {
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
                                text = if (viewModel.isEnabled) "Enabled" else "Disabled",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        Switch(
                            checked = viewModel.isEnabled,
                            onCheckedChange = { viewModel.setEnabled(it) }
                        )
                    }
                }
            }
            
            if (viewModel.isEnabled) {
                // Global Spending Limit
                item {
                    SpendingLimitCard(
                        dailyLimit = viewModel.dailyLimit,
                        usedToday = viewModel.usedToday,
                        onLimitChange = { viewModel.setDailyLimit(it) }
                    )
                }
                
                // Peer Limits
                if (viewModel.peerLimits.isNotEmpty()) {
                    item {
                        Text(
                            "Per-Peer Limits",
                            style = MaterialTheme.typography.titleMedium
                        )
                    }
                    items(viewModel.peerLimits) { limit ->
                        PeerLimitCard(limit = limit)
                    }
                }
                
                // Auto-Pay Rules
                if (viewModel.autoPayRules.isNotEmpty()) {
                    item {
                        Text(
                            "Auto-Pay Rules",
                            style = MaterialTheme.typography.titleMedium
                        )
                    }
                    items(viewModel.autoPayRules) { rule ->
                        AutoPayRuleCard(rule = rule)
                    }
                }
                
                // Recent Payments
                if (viewModel.recentPayments.isNotEmpty()) {
                    item {
                        Text(
                            "Recent Auto-Payments",
                            style = MaterialTheme.typography.titleMedium
                        )
                    }
                    items(viewModel.recentPayments) { payment ->
                        RecentPaymentCard(payment = payment)
                    }
                }
            }
        }
    }
}

@Composable
fun SpendingLimitCard(
    dailyLimit: Long,
    usedToday: Long,
    onLimitChange: (Long) -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacing(12.dp)
        ) {
            Text(
                "Global Spending Limit",
                style = MaterialTheme.typography.titleMedium
            )
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Daily Limit")
                Text("$dailyLimit sats", style = MaterialTheme.typography.bodyMedium)
            }
            
            Slider(
                value = dailyLimit.toFloat(),
                onValueChange = { onLimitChange(it.toLong()) },
                valueRange = 1000f..1000000f,
                steps = 999
            )
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Used Today")
                Text(
                    "$usedToday sats",
                    color = if (usedToday > dailyLimit) MaterialTheme.colorScheme.error
                    else MaterialTheme.colorScheme.primary
                )
            }
            
            LinearProgressIndicator(
                progress = { (usedToday.toFloat() / dailyLimit.toFloat()).coerceIn(0f, 1f) },
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}

@Composable
fun PeerLimitCard(limit: PeerSpendingLimit) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacing(4.dp)
        ) {
            Text(limit.peerName, style = MaterialTheme.typography.bodyLarge)
            Text("Limit: ${limit.limitSats} sats", style = MaterialTheme.typography.bodySmall)
            Text("Used: ${limit.spentSats} sats", style = MaterialTheme.typography.bodySmall)
        }
    }
}

@Composable
fun AutoPayRuleCard(rule: AutoPayRule) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text(rule.name, style = MaterialTheme.typography.bodyLarge)
                Text(rule.description, style = MaterialTheme.typography.bodySmall)
            }
            Icon(
                if (rule.isEnabled) Icons.Default.CheckCircle else Icons.Default.Cancel,
                contentDescription = null,
                tint = if (rule.isEnabled) MaterialTheme.colorScheme.primary
                else MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun RecentPaymentCard(payment: RecentAutoPayment) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Column {
                Text(payment.peerName, style = MaterialTheme.typography.bodyMedium)
                Text(payment.description, style = MaterialTheme.typography.bodySmall)
            }
            Text("${payment.amount} sats", style = MaterialTheme.typography.bodyMedium)
        }
    }
}

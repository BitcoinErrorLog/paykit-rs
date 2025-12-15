package com.paykit.mobile.bitkit

import com.paykit.mobile.bitkit.AutopayEvaluationResult

/**
 * Auto-Pay view model for Bitkit integration (Android)
 * This implements AutopayEvaluator interface
 */
class BitkitAutoPayViewModel(
    private val identityName: String,
    private val autoPayStorage: AutoPayStorageProtocol
) : AutopayEvaluator {
    
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
    
    init {
        loadFromStorage()
    }
    
    fun loadFromStorage() {
        val settings = autoPayStorage.getSettings()
        isEnabled = settings.isEnabled
        dailyLimit = settings.globalDailyLimitSats
        usedToday = 0L
        
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
    
    override fun evaluate(peerPubkey: String, amount: Long, methodId: String): AutopayEvaluationResult {
        val result = shouldAutoApprove(peerPubkey, amount, methodId)
        
        return when (result) {
            is AutoApprovalResult.Approved -> {
                AutopayEvaluationResult.Approved(result.ruleId, result.ruleName)
            }
            is AutoApprovalResult.Denied -> {
                AutopayEvaluationResult.Denied(result.reason)
            }
            is AutoApprovalResult.NeedsApproval -> {
                AutopayEvaluationResult.NeedsApproval
            }
        }
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

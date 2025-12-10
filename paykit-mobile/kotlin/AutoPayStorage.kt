// AutoPayStorage.kt
// Paykit Android Auto-Pay Storage
//
// This file provides specialized storage for auto-pay settings,
// spending limits, and payment rules using EncryptedSharedPreferences.
//
// USAGE:
//   val storage = EncryptedPreferencesStorage.create(context)
//   val autoPayStorage = AutoPayStorage(storage)
//   
//   // Enable auto-pay with limits
//   autoPayStorage.setEnabled(true)
//   autoPayStorage.setGlobalDailyLimit(100_000)
//   
//   // Add peer-specific limit
//   autoPayStorage.addPeerLimit(PeerSpendingLimit(...))

package com.paykit.storage

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.util.Date

/**
 * Specialized storage for auto-pay settings and spending limits.
 *
 * This class provides a high-level API for managing auto-pay configuration,
 * including global settings, per-peer limits, and auto-pay rules.
 *
 * All data is stored encrypted using the underlying EncryptedPreferencesStorage.
 *
 * @param storage The underlying encrypted storage
 */
class AutoPayStorage(
    private val storage: EncryptedPreferencesStorage
) {
    companion object {
        private const val KEY_ENABLED = "autopay.enabled"
        private const val KEY_GLOBAL_DAILY_LIMIT = "autopay.global_daily_limit"
        private const val KEY_USED_TODAY = "autopay.used_today"
        private const val KEY_LAST_RESET_DATE = "autopay.last_reset_date"
        private const val KEY_PEER_LIMITS = "autopay.peer_limits"
        private const val KEY_RULES = "autopay.rules"
        private const val KEY_RECENT_PAYMENTS = "autopay.recent_payments"
        private const val KEY_REQUIRE_BIOMETRIC_ABOVE = "autopay.require_biometric_above"
        private const val KEY_NOTIFY_ON_AUTOPAY = "autopay.notify_on_autopay"
        private const val KEY_NOTIFY_ON_LIMIT = "autopay.notify_on_limit"

        private const val DEFAULT_DAILY_LIMIT = 100_000L
        private const val MAX_RECENT_PAYMENTS = 100
    }

    // MARK: - Global Settings

    /**
     * Check if auto-pay is enabled.
     */
    fun isEnabled(): Boolean {
        return storage.retrieveString(KEY_ENABLED)?.toBoolean() ?: false
    }

    /**
     * Enable or disable auto-pay.
     */
    fun setEnabled(enabled: Boolean) {
        storage.store(KEY_ENABLED, enabled.toString())
    }

    /**
     * Get the global daily spending limit in satoshis.
     */
    fun getGlobalDailyLimit(): Long {
        return storage.retrieveString(KEY_GLOBAL_DAILY_LIMIT)?.toLongOrNull()
            ?: DEFAULT_DAILY_LIMIT
    }

    /**
     * Set the global daily spending limit.
     *
     * @param limit Limit in satoshis
     */
    fun setGlobalDailyLimit(limit: Long) {
        storage.store(KEY_GLOBAL_DAILY_LIMIT, limit.toString())
    }

    /**
     * Get the amount used today in satoshis.
     */
    fun getUsedToday(): Long {
        checkAndResetDaily()
        return storage.retrieveString(KEY_USED_TODAY)?.toLongOrNull() ?: 0L
    }

    /**
     * Add to today's usage.
     *
     * @param amount Amount in satoshis
     * @return New total for today
     */
    fun addUsage(amount: Long): Long {
        checkAndResetDaily()
        val current = getUsedToday()
        val newTotal = current + amount
        storage.store(KEY_USED_TODAY, newTotal.toString())
        return newTotal
    }

    /**
     * Reset today's usage to zero.
     */
    fun resetDailyUsage() {
        storage.store(KEY_USED_TODAY, "0")
        storage.store(KEY_LAST_RESET_DATE, todayString())
    }

    /**
     * Get remaining daily limit.
     */
    fun getRemainingDailyLimit(): Long {
        val limit = getGlobalDailyLimit()
        val used = getUsedToday()
        return maxOf(0, limit - used)
    }

    /**
     * Check if amount would exceed daily limit.
     */
    fun wouldExceedDailyLimit(amount: Long): Boolean {
        return getUsedToday() + amount > getGlobalDailyLimit()
    }

    // MARK: - Peer Limits

    /**
     * Get all peer spending limits.
     */
    fun getPeerLimits(): List<PeerSpendingLimit> {
        val json = storage.retrieveString(KEY_PEER_LIMITS) ?: return emptyList()
        return try {
            val array = JSONArray(json)
            (0 until array.length()).map { i ->
                PeerSpendingLimit.fromJson(array.getJSONObject(i))
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    /**
     * Get limit for a specific peer.
     */
    fun getPeerLimit(peerPubkey: String): PeerSpendingLimit? {
        return getPeerLimits().find { it.peerPubkey == peerPubkey }
    }

    /**
     * Add or update a peer spending limit.
     */
    fun savePeerLimit(limit: PeerSpendingLimit) {
        val limits = getPeerLimits().toMutableList()
        val existingIndex = limits.indexOfFirst { it.peerPubkey == limit.peerPubkey }
        
        if (existingIndex >= 0) {
            limits[existingIndex] = limit
        } else {
            limits.add(limit)
        }
        
        savePeerLimits(limits)
    }

    /**
     * Remove a peer spending limit.
     */
    fun removePeerLimit(peerPubkey: String) {
        val limits = getPeerLimits().filter { it.peerPubkey != peerPubkey }
        savePeerLimits(limits)
    }

    /**
     * Add usage for a specific peer.
     *
     * @return Updated peer limit, or null if no limit exists for this peer
     */
    fun addPeerUsage(peerPubkey: String, amount: Long): PeerSpendingLimit? {
        val limit = getPeerLimit(peerPubkey) ?: return null
        val updated = limit.copy(used = limit.used + amount)
        savePeerLimit(updated)
        return updated
    }

    /**
     * Reset usage for a specific peer.
     */
    fun resetPeerUsage(peerPubkey: String) {
        val limit = getPeerLimit(peerPubkey) ?: return
        savePeerLimit(limit.copy(used = 0, periodStart = System.currentTimeMillis()))
    }

    /**
     * Reset all peer usages that have expired.
     */
    fun resetExpiredPeerLimits() {
        val now = System.currentTimeMillis()
        val limits = getPeerLimits().map { limit ->
            if (limit.shouldReset(now)) {
                limit.copy(used = 0, periodStart = now)
            } else {
                limit
            }
        }
        savePeerLimits(limits)
    }

    private fun savePeerLimits(limits: List<PeerSpendingLimit>) {
        val array = JSONArray()
        limits.forEach { array.put(it.toJson()) }
        storage.store(KEY_PEER_LIMITS, array.toString())
    }

    // MARK: - Auto-Pay Rules

    /**
     * Get all auto-pay rules.
     */
    fun getRules(): List<AutoPayRule> {
        val json = storage.retrieveString(KEY_RULES) ?: return emptyList()
        return try {
            val array = JSONArray(json)
            (0 until array.length()).map { i ->
                AutoPayRule.fromJson(array.getJSONObject(i))
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    /**
     * Add or update a rule.
     */
    fun saveRule(rule: AutoPayRule) {
        val rules = getRules().toMutableList()
        val existingIndex = rules.indexOfFirst { it.id == rule.id }
        
        if (existingIndex >= 0) {
            rules[existingIndex] = rule
        } else {
            rules.add(rule)
        }
        
        saveRules(rules)
    }

    /**
     * Remove a rule.
     */
    fun removeRule(ruleId: String) {
        val rules = getRules().filter { it.id != ruleId }
        saveRules(rules)
    }

    /**
     * Enable or disable a rule.
     */
    fun setRuleEnabled(ruleId: String, enabled: Boolean) {
        val rule = getRules().find { it.id == ruleId } ?: return
        saveRule(rule.copy(isEnabled = enabled))
    }

    private fun saveRules(rules: List<AutoPayRule>) {
        val array = JSONArray()
        rules.forEach { array.put(it.toJson()) }
        storage.store(KEY_RULES, array.toString())
    }

    // MARK: - Auto-Approval Check

    /**
     * Check if a payment should be auto-approved.
     *
     * @param peerPubkey The peer requesting payment
     * @param amount Amount in satoshis
     * @param methodId Payment method ID
     * @return Result indicating approval status
     */
    fun checkAutoApproval(
        peerPubkey: String,
        amount: Long,
        methodId: String
    ): AutoApprovalResult {
        // Check if auto-pay is enabled
        if (!isEnabled()) {
            return AutoApprovalResult.Denied("Auto-pay is disabled")
        }

        // Check global daily limit
        if (wouldExceedDailyLimit(amount)) {
            return AutoApprovalResult.Denied("Would exceed daily limit")
        }

        // Check peer-specific limit
        val peerLimit = getPeerLimit(peerPubkey)
        if (peerLimit != null) {
            // Reset if period expired
            if (peerLimit.shouldReset(System.currentTimeMillis())) {
                resetPeerUsage(peerPubkey)
            }
            
            val currentLimit = getPeerLimit(peerPubkey)!!
            if (currentLimit.used + amount > currentLimit.limit) {
                return AutoApprovalResult.Denied("Would exceed peer limit")
            }
        }

        // Check rules
        val rules = getRules().filter { it.isEnabled }
        for (rule in rules) {
            if (rule.matches(peerPubkey, amount, methodId)) {
                return AutoApprovalResult.Approved(rule.id, rule.name)
            }
        }

        return AutoApprovalResult.NeedsApproval
    }

    /**
     * Record an auto-approved payment.
     */
    fun recordAutoPayment(
        peerPubkey: String,
        peerName: String,
        amount: Long,
        description: String,
        ruleId: String?
    ) {
        // Update global usage
        addUsage(amount)

        // Update peer usage
        addPeerUsage(peerPubkey, amount)

        // Add to recent payments
        val payment = RecentAutoPayment(
            id = java.util.UUID.randomUUID().toString(),
            peerPubkey = peerPubkey,
            peerName = peerName,
            amount = amount,
            description = description,
            timestamp = System.currentTimeMillis(),
            status = PaymentStatus.COMPLETED,
            ruleId = ruleId
        )
        addRecentPayment(payment)
    }

    // MARK: - Recent Payments

    /**
     * Get recent auto-payments.
     */
    fun getRecentPayments(): List<RecentAutoPayment> {
        val json = storage.retrieveString(KEY_RECENT_PAYMENTS) ?: return emptyList()
        return try {
            val array = JSONArray(json)
            (0 until array.length()).map { i ->
                RecentAutoPayment.fromJson(array.getJSONObject(i))
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    /**
     * Add a recent payment to history.
     */
    fun addRecentPayment(payment: RecentAutoPayment) {
        val payments = getRecentPayments().toMutableList()
        payments.add(0, payment)
        
        // Keep only the most recent payments
        val trimmed = payments.take(MAX_RECENT_PAYMENTS)
        saveRecentPayments(trimmed)
    }

    /**
     * Clear recent payment history.
     */
    fun clearRecentPayments() {
        storage.delete(KEY_RECENT_PAYMENTS)
    }

    private fun saveRecentPayments(payments: List<RecentAutoPayment>) {
        val array = JSONArray()
        payments.forEach { array.put(it.toJson()) }
        storage.store(KEY_RECENT_PAYMENTS, array.toString())
    }

    // MARK: - Notification Settings

    /**
     * Get threshold above which biometric is required.
     */
    fun getBiometricThreshold(): Long? {
        return storage.retrieveString(KEY_REQUIRE_BIOMETRIC_ABOVE)?.toLongOrNull()
    }

    /**
     * Set threshold above which biometric is required.
     */
    fun setBiometricThreshold(threshold: Long?) {
        if (threshold != null) {
            storage.store(KEY_REQUIRE_BIOMETRIC_ABOVE, threshold.toString())
        } else {
            storage.delete(KEY_REQUIRE_BIOMETRIC_ABOVE)
        }
    }

    /**
     * Check if notifications on auto-pay are enabled.
     */
    fun isNotifyOnAutoPay(): Boolean {
        return storage.retrieveString(KEY_NOTIFY_ON_AUTOPAY)?.toBoolean() ?: true
    }

    /**
     * Set notification preference for auto-pay.
     */
    fun setNotifyOnAutoPay(enabled: Boolean) {
        storage.store(KEY_NOTIFY_ON_AUTOPAY, enabled.toString())
    }

    /**
     * Check if notifications on limit reached are enabled.
     */
    fun isNotifyOnLimit(): Boolean {
        return storage.retrieveString(KEY_NOTIFY_ON_LIMIT)?.toBoolean() ?: true
    }

    /**
     * Set notification preference for limit warnings.
     */
    fun setNotifyOnLimit(enabled: Boolean) {
        storage.store(KEY_NOTIFY_ON_LIMIT, enabled.toString())
    }

    // MARK: - Reset

    /**
     * Reset all auto-pay settings to defaults.
     */
    fun resetToDefaults() {
        setEnabled(false)
        setGlobalDailyLimit(DEFAULT_DAILY_LIMIT)
        resetDailyUsage()
        savePeerLimits(emptyList())
        saveRules(emptyList())
        clearRecentPayments()
        setBiometricThreshold(null)
        setNotifyOnAutoPay(true)
        setNotifyOnLimit(true)
    }

    // MARK: - Private Helpers

    private fun checkAndResetDaily() {
        val lastReset = storage.retrieveString(KEY_LAST_RESET_DATE)
        val today = todayString()
        
        if (lastReset != today) {
            storage.store(KEY_USED_TODAY, "0")
            storage.store(KEY_LAST_RESET_DATE, today)
        }
    }

    private fun todayString(): String {
        val cal = java.util.Calendar.getInstance()
        return "${cal.get(java.util.Calendar.YEAR)}-${cal.get(java.util.Calendar.MONTH)}-${cal.get(java.util.Calendar.DAY_OF_MONTH)}"
    }
}

// MARK: - Data Classes

/**
 * Spending period for limits.
 */
enum class SpendingPeriod(val milliseconds: Long) {
    HOURLY(3600_000L),
    DAILY(86400_000L),
    WEEKLY(604800_000L),
    MONTHLY(2592000_000L);

    companion object {
        fun fromString(value: String): SpendingPeriod {
            return values().find { it.name == value } ?: DAILY
        }
    }
}

/**
 * Per-peer spending limit.
 */
data class PeerSpendingLimit(
    val peerPubkey: String,
    val peerName: String,
    val limit: Long,
    val used: Long,
    val period: SpendingPeriod,
    val periodStart: Long
) {
    val remaining: Long get() = maxOf(0, limit - used)
    val percentUsed: Float get() = if (limit > 0) used.toFloat() / limit else 0f
    val isExhausted: Boolean get() = used >= limit

    fun shouldReset(now: Long): Boolean {
        return now - periodStart >= period.milliseconds
    }

    fun toJson(): JSONObject {
        return JSONObject().apply {
            put("peerPubkey", peerPubkey)
            put("peerName", peerName)
            put("limit", limit)
            put("used", used)
            put("period", period.name)
            put("periodStart", periodStart)
        }
    }

    companion object {
        fun fromJson(json: JSONObject): PeerSpendingLimit {
            return PeerSpendingLimit(
                peerPubkey = json.getString("peerPubkey"),
                peerName = json.getString("peerName"),
                limit = json.getLong("limit"),
                used = json.getLong("used"),
                period = SpendingPeriod.fromString(json.getString("period")),
                periodStart = json.getLong("periodStart")
            )
        }
    }
}

/**
 * Auto-pay rule.
 */
data class AutoPayRule(
    val id: String,
    val name: String,
    val description: String,
    val isEnabled: Boolean,
    val maxAmount: Long?,
    val methodFilter: String?,
    val peerFilter: String?
) {
    fun matches(peerPubkey: String, amount: Long, methodId: String): Boolean {
        // Check amount
        if (maxAmount != null && amount > maxAmount) {
            return false
        }

        // Check method
        if (methodFilter != null && methodFilter != methodId) {
            return false
        }

        // Check peer
        if (peerFilter != null && peerFilter != peerPubkey) {
            return false
        }

        return isEnabled
    }

    fun toJson(): JSONObject {
        return JSONObject().apply {
            put("id", id)
            put("name", name)
            put("description", description)
            put("isEnabled", isEnabled)
            maxAmount?.let { put("maxAmount", it) }
            methodFilter?.let { put("methodFilter", it) }
            peerFilter?.let { put("peerFilter", it) }
        }
    }

    companion object {
        fun fromJson(json: JSONObject): AutoPayRule {
            return AutoPayRule(
                id = json.getString("id"),
                name = json.getString("name"),
                description = json.getString("description"),
                isEnabled = json.getBoolean("isEnabled"),
                maxAmount = if (json.has("maxAmount")) json.getLong("maxAmount") else null,
                methodFilter = if (json.has("methodFilter")) json.getString("methodFilter") else null,
                peerFilter = if (json.has("peerFilter")) json.getString("peerFilter") else null
            )
        }
    }
}

/**
 * Payment status.
 */
enum class PaymentStatus {
    PENDING,
    PROCESSING,
    COMPLETED,
    FAILED;

    companion object {
        fun fromString(value: String): PaymentStatus {
            return values().find { it.name == value } ?: PENDING
        }
    }
}

/**
 * Recent auto-payment record.
 */
data class RecentAutoPayment(
    val id: String,
    val peerPubkey: String,
    val peerName: String,
    val amount: Long,
    val description: String,
    val timestamp: Long,
    val status: PaymentStatus,
    val ruleId: String?
) {
    fun toJson(): JSONObject {
        return JSONObject().apply {
            put("id", id)
            put("peerPubkey", peerPubkey)
            put("peerName", peerName)
            put("amount", amount)
            put("description", description)
            put("timestamp", timestamp)
            put("status", status.name)
            ruleId?.let { put("ruleId", it) }
        }
    }

    companion object {
        fun fromJson(json: JSONObject): RecentAutoPayment {
            return RecentAutoPayment(
                id = json.getString("id"),
                peerPubkey = json.getString("peerPubkey"),
                peerName = json.getString("peerName"),
                amount = json.getLong("amount"),
                description = json.getString("description"),
                timestamp = json.getLong("timestamp"),
                status = PaymentStatus.fromString(json.getString("status")),
                ruleId = if (json.has("ruleId")) json.getString("ruleId") else null
            )
        }
    }
}

/**
 * Result of auto-approval check.
 */
sealed class AutoApprovalResult {
    data class Approved(val ruleId: String, val ruleName: String) : AutoApprovalResult()
    data class Denied(val reason: String) : AutoApprovalResult()
    object NeedsApproval : AutoApprovalResult()

    val isApproved: Boolean get() = this is Approved
}

// MARK: - Coroutine Extensions

/**
 * Coroutine-friendly extensions for async auto-pay operations.
 */

suspend fun AutoPayStorage.checkAutoApprovalAsync(
    peerPubkey: String,
    amount: Long,
    methodId: String
): AutoApprovalResult = withContext(Dispatchers.IO) {
    checkAutoApproval(peerPubkey, amount, methodId)
}

suspend fun AutoPayStorage.recordAutoPaymentAsync(
    peerPubkey: String,
    peerName: String,
    amount: Long,
    description: String,
    ruleId: String?
) = withContext(Dispatchers.IO) {
    recordAutoPayment(peerPubkey, peerName, amount, description, ruleId)
}

/**
 * Extension function to create AutoPayStorage from EncryptedPreferencesStorage.
 */
fun EncryptedPreferencesStorage.asAutoPayStorage(): AutoPayStorage {
    return AutoPayStorage(this)
}

package com.paykit.mobile

import com.paykit.mobile.paykit_mobile.PaykitClient
import com.paykit.mobile.paykit_mobile.PaymentRequest
import com.paykit.mobile.paykit_mobile.PaymentExecutionResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Result of autopay evaluation
 */
sealed class AutopayEvaluationResult {
    data class Approved(val ruleId: String?, val ruleName: String?) : AutopayEvaluationResult()
    data class Denied(val reason: String) : AutopayEvaluationResult()
    object NeedsApproval : AutopayEvaluationResult()
    
    val isApproved: Boolean
        get() = this is Approved
}

/**
 * Result of payment request processing
 */
sealed class PaymentRequestProcessingResult {
    data class AutoPaid(val paymentResult: PaymentExecutionResult) : PaymentRequestProcessingResult()
    data class NeedsApproval(val request: PaymentRequest) : PaymentRequestProcessingResult()
    data class Denied(val reason: String) : PaymentRequestProcessingResult()
    data class Error(val error: Throwable) : PaymentRequestProcessingResult()
}

// PaymentExecutionResult and PaymentRequest are already defined in paykit_mobile.kt
// We use type aliases for clarity
typealias PaymentExecutionResult = com.paykit.mobile.paykit_mobile.PaymentExecutionResult
typealias PaymentRequest = com.paykit.mobile.paykit_mobile.PaymentRequest

/**
 * Service for handling payment requests with autopay support.
 * Designed for Bitkit integration.
 */
class PaymentRequestService(
    private val paykitClient: PaykitClient,
    private val autopayEvaluator: AutopayEvaluator
) {
    
    /**
     * Handle an incoming payment request
     * @param requestId Payment request ID
     * @param fromPubkey Requester's public key
     * @return Result with processing result
     */
    suspend fun handleIncomingRequest(
        requestId: String,
        fromPubkey: String
    ): Result<PaymentRequestProcessingResult> = withContext(Dispatchers.IO) {
        try {
            // Fetch payment request details
            // This is a placeholder - Bitkit should implement actual request fetching
            val request = fetchPaymentRequest(requestId, fromPubkey)
            
            // Evaluate autopay
            val evaluation = autopayEvaluator.evaluate(
                peerPubkey = fromPubkey,
                amount = request.amountSats,
                methodId = request.methodId
            )
            
            when (evaluation) {
                is AutopayEvaluationResult.Approved -> {
                    // Execute payment automatically
                    // Note: Endpoint resolution must be implemented by Bitkit
                    try {
                        val endpoint = resolveEndpoint(request)
                        val paymentResult = executePayment(request, endpoint, null)
                        Result.success(
                            PaymentRequestProcessingResult.AutoPaid(paymentResult)
                        )
                    } catch (e: Exception) {
                        Result.success(
                            PaymentRequestProcessingResult.Error(e)
                        )
                    }
                }
                is AutopayEvaluationResult.Denied -> {
                    Result.success(
                        PaymentRequestProcessingResult.Denied(evaluation.reason)
                    )
                }
                is AutopayEvaluationResult.NeedsApproval -> {
                    Result.success(
                        PaymentRequestProcessingResult.NeedsApproval(request)
                    )
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Evaluate autopay for a payment request
     */
    fun evaluateAutopay(
        peerPubkey: String,
        amount: Long,
        methodId: String
    ): AutopayEvaluationResult {
        return autopayEvaluator.evaluate(peerPubkey, amount, methodId)
    }
    
    /**
     * Execute a payment request
     * Note: This requires the endpoint to be resolved from the payment request.
     * Bitkit should implement endpoint resolution logic.
     */
    suspend fun executePayment(
        request: PaymentRequest,
        endpoint: String,
        metadataJson: String?
    ): PaymentExecutionResult = withContext(Dispatchers.IO) {
        // Execute payment via PaykitClient
        paykitClient.executePayment(
            methodId = request.methodId,
            endpoint = endpoint,
            amountSats = request.amountSats.toULong(),
            metadataJson = metadataJson
        )
    }
    
    // MARK: - Private Helpers
    
    /**
     * Fetch payment request details (to be implemented by Bitkit)
     */
    private suspend fun fetchPaymentRequest(requestId: String, fromPubkey: String): PaymentRequest {
        // This is a placeholder implementation
        // Bitkit should implement this to fetch from their storage/network
        throw IllegalStateException(
            "Payment request fetching not implemented. " +
            "Bitkit should implement this method to fetch from storage/network."
        )
    }
    
    /**
     * Resolve payment endpoint from request (to be implemented by Bitkit)
     */
    private suspend fun resolveEndpoint(request: PaymentRequest): String {
        // This is a placeholder implementation
        // Bitkit should implement this to resolve the endpoint from:
        // - The payment method (methodId)
        // - The recipient's directory (fromPubkey)
        // - Payment method discovery
        throw IllegalStateException(
            "Endpoint resolution not implemented. " +
            "Bitkit should implement this method to resolve payment endpoints."
        )
    }
}

/**
 * Protocol for autopay evaluation
 */
interface AutopayEvaluator {
    /**
     * Evaluate if a payment should be auto-approved
     */
    fun evaluate(peerPubkey: String, amount: Long, methodId: String): AutopayEvaluationResult
}

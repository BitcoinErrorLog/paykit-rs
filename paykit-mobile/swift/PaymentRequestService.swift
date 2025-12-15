//
//  PaymentRequestService.swift
//  PaykitMobile
//
//  Service for handling incoming payment requests with autopay evaluation.
//  Designed for Bitkit integration.
//

import Foundation
import PaykitMobile

/// Result of autopay evaluation
public enum AutopayEvaluationResult {
    case approved(ruleId: String?, ruleName: String?)
    case denied(reason: String)
    case needsApproval
    
    public var isApproved: Bool {
        if case .approved = self { return true }
        return false
    }
}

/// Result of payment request processing
public enum PaymentRequestProcessingResult {
    case autoPaid(paymentResult: PaymentExecutionResult)
    case needsApproval(request: PaymentRequest)
    case denied(reason: String)
    case error(Error)
}

// PaymentExecutionResult is already defined in PaykitMobile.swift
// We'll use a typealias for clarity
public typealias PaymentExecutionResult = PaykitMobile.PaymentExecutionResult

/// Service for handling payment requests with autopay support
public class PaymentRequestService {
    
    private let paykitClient: PaykitClient
    private let autopayEvaluator: AutopayEvaluator
    
    /// Initialize with PaykitClient and autopay evaluator
    public init(
        paykitClient: PaykitClient,
        autopayEvaluator: AutopayEvaluator
    ) {
        self.paykitClient = paykitClient
        self.autopayEvaluator = autopayEvaluator
    }
    
    /// Handle an incoming payment request
    /// - Parameters:
    ///   - requestId: Payment request ID
    ///   - fromPubkey: Requester's public key
    ///   - completion: Completion handler with processing result
    public func handleIncomingRequest(
        requestId: String,
        fromPubkey: String,
        completion: @escaping (Result<PaymentRequestProcessingResult, Error>) -> Void
    ) {
        // In a real implementation, you would fetch the full payment request details
        // from the Paykit network or storage. For now, we'll use a placeholder.
        // This should be implemented by Bitkit to fetch from their storage/network.
        
        // Example: Fetch payment request details
        // let request = try await fetchPaymentRequest(requestId: requestId)
        
        // For now, we'll create a mock request structure
        // Bitkit should implement the actual fetching logic
        Task {
            do {
                // This is a placeholder - Bitkit should implement actual request fetching
                let request = try await self.fetchPaymentRequest(requestId: requestId, fromPubkey: fromPubkey)
                
                // Evaluate autopay
                let evaluation = autopayEvaluator.evaluate(
                    peerPubkey: fromPubkey,
                    amount: request.amountSats,
                    methodId: request.methodId
                )
                
                switch evaluation {
                case .approved(let ruleId, let ruleName):
                    // Execute payment automatically
                    // Note: Endpoint resolution must be implemented by Bitkit
                    do {
                        // This is a placeholder - Bitkit must implement endpoint resolution
                        let endpoint = try await self.resolveEndpoint(for: request)
                        let paymentResult = try await self.executePayment(
                            request: request,
                            endpoint: endpoint,
                            metadataJson: request.metadataJson
                        )
                        completion(.success(.autoPaid(paymentResult: paymentResult)))
                    } catch {
                        completion(.success(.error(error)))
                    }
                    
                case .denied(let reason):
                    completion(.success(.denied(reason: reason)))
                    
                case .needsApproval:
                    completion(.success(.needsApproval(request: request)))
                }
            } catch {
                completion(.failure(error))
            }
        }
    }
    
    /// Evaluate autopay for a payment request
    public func evaluateAutopay(
        peerPubkey: String,
        amount: Int64,
        methodId: String
    ) -> AutopayEvaluationResult {
        return autopayEvaluator.evaluate(
            peerPubkey: peerPubkey,
            amount: amount,
            methodId: methodId
        )
    }
    
    /// Execute a payment request
    /// Note: This requires the endpoint to be resolved from the payment request.
    /// Bitkit should implement endpoint resolution logic.
    public func executePayment(
        request: PaymentRequest,
        endpoint: String,
        metadataJson: String? = nil
    ) async throws -> PaymentExecutionResult {
        // Execute payment via PaykitClient
        let result = try paykitClient.executePayment(
            methodId: request.methodId,
            endpoint: endpoint,
            amountSats: UInt64(request.amountSats),
            metadataJson: metadataJson
        )
        
        return result
    }
    
    // MARK: - Private Helpers
    
    /// Fetch payment request details (to be implemented by Bitkit)
    private func fetchPaymentRequest(requestId: String, fromPubkey: String) async throws -> PaymentRequest {
        // This is a placeholder implementation
        // Bitkit should implement this to fetch from their storage/network
        // For now, we'll throw an error indicating this needs implementation
        
        throw NSError(
            domain: "PaymentRequestService",
            code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Payment request fetching not implemented. Bitkit should implement this method to fetch from storage/network."]
        )
    }
    
    /// Resolve payment endpoint from request (to be implemented by Bitkit)
    private func resolveEndpoint(for request: PaymentRequest) async throws -> String {
        // This is a placeholder implementation
        // Bitkit should implement this to resolve the endpoint from:
        // - The payment method (methodId)
        // - The recipient's directory (fromPubkey)
        // - Payment method discovery
        
        throw NSError(
            domain: "PaymentRequestService",
            code: 2,
            userInfo: [NSLocalizedDescriptionKey: "Endpoint resolution not implemented. Bitkit should implement this method to resolve payment endpoints."]
        )
    }
}

/// Protocol for autopay evaluation
public protocol AutopayEvaluator {
    /// Evaluate if a payment should be auto-approved
    func evaluate(peerPubkey: String, amount: Int64, methodId: String) -> AutopayEvaluationResult
}

/// Extension to make BitkitAutoPayViewModel conform to AutopayEvaluator
extension BitkitAutoPayViewModel: AutopayEvaluator {
    // Already implements evaluate() method
}

// PaymentRequest is already defined in PaykitMobile.swift
// We'll use a typealias for clarity
public typealias PaymentRequest = PaykitMobile.PaymentRequest

// Extension to add convenience properties for service use
extension PaymentRequest {
    /// Get payment endpoint (to be determined from request details)
    /// This is a placeholder - Bitkit should implement endpoint resolution
    public var endpoint: String {
        // In a real implementation, this would resolve the endpoint
        // from the payment method and recipient's directory
        // For now, return empty string as placeholder
        return ""
    }
    
    /// Get metadata JSON (optional)
    public var metadataJson: String? {
        // Can be constructed from request details if needed
        return nil
    }
}

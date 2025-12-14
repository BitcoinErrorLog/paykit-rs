//! E2E Test Helpers
//!
//! Common utilities, fixtures, and helpers for E2E testing across
//! iOS, Android, and cross-platform scenarios.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Test Configuration
// ============================================================================

/// Configuration constants for E2E tests
pub mod config {
    /// Default timeout for async operations (milliseconds)
    pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

    /// Short timeout for quick operations (milliseconds)
    pub const SHORT_TIMEOUT_MS: u64 = 5_000;

    /// Port range for test servers
    pub const TEST_PORT_MIN: u16 = 10_000;
    pub const TEST_PORT_MAX: u16 = 60_000;

    /// Generate a random test port
    pub fn random_port() -> u16 {
        use rand::Rng;
        rand::thread_rng().gen_range(TEST_PORT_MIN..=TEST_PORT_MAX)
    }

    /// Generate a unique test user ID
    pub fn test_user_id(prefix: &str) -> String {
        format!(
            "{}_{}",
            prefix,
            uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>()
        )
    }
}

// ============================================================================
// Mock Identity
// ============================================================================

/// A mock identity for testing
#[derive(Debug, Clone)]
pub struct MockIdentity {
    pub nickname: String,
    pub public_key_z32: String,
    pub secret_key: Vec<u8>,
    pub noise_keypair: Option<MockNoiseKeypair>,
}

/// A mock Noise keypair
#[derive(Debug, Clone)]
pub struct MockNoiseKeypair {
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

impl MockIdentity {
    /// Create a new mock identity
    pub fn new(nickname: &str) -> Self {
        Self {
            nickname: nickname.to_string(),
            public_key_z32: generate_z32_pubkey(),
            secret_key: generate_random_bytes(32),
            noise_keypair: Some(MockNoiseKeypair {
                public_key: generate_random_bytes(32),
                secret_key: generate_random_bytes(32),
            }),
        }
    }

    /// Get the Noise public key as hex string
    pub fn noise_pubkey_hex(&self) -> String {
        self.noise_keypair
            .as_ref()
            .map(|kp| bytes_to_hex(&kp.public_key))
            .unwrap_or_default()
    }
}

// ============================================================================
// Mock Receipt Store
// ============================================================================

/// A mock receipt for testing
#[derive(Debug, Clone)]
pub struct MockReceipt {
    pub id: String,
    pub payer_pubkey: String,
    pub payee_pubkey: String,
    pub amount_sats: u64,
    pub created_at: u64,
    pub status: String,
}

/// A mock receipt store
#[derive(Default)]
pub struct MockReceiptStore {
    receipts: Arc<Mutex<HashMap<String, MockReceipt>>>,
}

impl MockReceiptStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store(&self, receipt: MockReceipt) {
        let mut receipts = self.receipts.lock().unwrap();
        receipts.insert(receipt.id.clone(), receipt);
    }

    pub fn get(&self, id: &str) -> Option<MockReceipt> {
        let receipts = self.receipts.lock().unwrap();
        receipts.get(id).cloned()
    }

    pub fn get_all(&self) -> Vec<MockReceipt> {
        let receipts = self.receipts.lock().unwrap();
        receipts.values().cloned().collect()
    }

    pub fn clear(&self) {
        let mut receipts = self.receipts.lock().unwrap();
        receipts.clear();
    }

    pub fn count(&self) -> usize {
        let receipts = self.receipts.lock().unwrap();
        receipts.len()
    }
}

// ============================================================================
// Mock Directory Service
// ============================================================================

/// A mock endpoint for testing
#[derive(Debug, Clone)]
pub struct MockEndpoint {
    pub recipient_pubkey: String,
    pub host: String,
    pub port: u16,
    pub server_noise_pubkey: String,
    pub metadata: Option<String>,
}

impl MockEndpoint {
    /// Get the connection address
    pub fn connection_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// A mock directory service
#[derive(Default)]
pub struct MockDirectoryService {
    endpoints: Arc<Mutex<HashMap<String, MockEndpoint>>>,
}

impl MockDirectoryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish(&self, endpoint: MockEndpoint) {
        let mut endpoints = self.endpoints.lock().unwrap();
        endpoints.insert(endpoint.recipient_pubkey.clone(), endpoint);
    }

    pub fn discover(&self, pubkey: &str) -> Option<MockEndpoint> {
        let endpoints = self.endpoints.lock().unwrap();
        endpoints.get(pubkey).cloned()
    }

    pub fn remove(&self, pubkey: &str) {
        let mut endpoints = self.endpoints.lock().unwrap();
        endpoints.remove(pubkey);
    }

    pub fn clear(&self) {
        let mut endpoints = self.endpoints.lock().unwrap();
        endpoints.clear();
    }

    pub fn count(&self) -> usize {
        let endpoints = self.endpoints.lock().unwrap();
        endpoints.len()
    }
}

// ============================================================================
// Test Data Generators
// ============================================================================

/// Generate a unique receipt ID
pub fn generate_receipt_id() -> String {
    format!("rcpt_test_{}", uuid::Uuid::new_v4())
}

/// Generate a mock Z32 public key
pub fn generate_z32_pubkey() -> String {
    format!(
        "z32_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .replace("-", "")
            .chars()
            .take(52)
            .collect::<String>()
    )
}

/// Generate a mock Noise public key (hex string)
pub fn generate_noise_pubkey() -> String {
    bytes_to_hex(&generate_random_bytes(32))
}

/// Generate random bytes
pub fn generate_random_bytes(len: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Convert bytes to hex string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Get current timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Create a mock payment request
pub fn create_payment_request(
    payer: &str,
    payee: &str,
    amount: u64,
) -> (String, HashMap<String, String>) {
    let receipt_id = generate_receipt_id();
    let mut request = HashMap::new();
    request.insert("receipt_id".to_string(), receipt_id.clone());
    request.insert("payer_pubkey".to_string(), payer.to_string());
    request.insert("payee_pubkey".to_string(), payee.to_string());
    request.insert("method_id".to_string(), "lightning".to_string());
    request.insert("amount_sats".to_string(), amount.to_string());
    request.insert("created_at".to_string(), current_timestamp().to_string());
    (receipt_id, request)
}

/// Create a mock receipt confirmation
pub fn create_receipt_confirmation(receipt_id: &str, payee: &str) -> HashMap<String, String> {
    let mut confirmation = HashMap::new();
    confirmation.insert("receipt_id".to_string(), receipt_id.to_string());
    confirmation.insert("payee_pubkey".to_string(), payee.to_string());
    confirmation.insert("status".to_string(), "confirmed".to_string());
    confirmation.insert("confirmed_at".to_string(), current_timestamp().to_string());
    confirmation
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that a receipt is valid
pub fn assert_receipt_valid(receipt: &MockReceipt, expected_payer: &str, expected_payee: &str) {
    assert!(!receipt.id.is_empty(), "Receipt ID should not be empty");
    assert_eq!(
        receipt.payer_pubkey, expected_payer,
        "Payer should match expected"
    );
    assert_eq!(
        receipt.payee_pubkey, expected_payee,
        "Payee should match expected"
    );
    assert!(receipt.amount_sats > 0, "Amount should be positive");
    assert!(receipt.created_at > 0, "Created at should be set");
}

/// Assert that an endpoint is valid
pub fn assert_endpoint_valid(endpoint: &MockEndpoint) {
    assert!(
        !endpoint.recipient_pubkey.is_empty(),
        "Pubkey should not be empty"
    );
    assert!(!endpoint.host.is_empty(), "Host should not be empty");
    assert!(endpoint.port > 0, "Port should be positive");
    assert!(
        !endpoint.server_noise_pubkey.is_empty(),
        "Noise pubkey should not be empty"
    );
}

// ============================================================================
// Test Timeout Helpers
// ============================================================================

/// Run a closure with a timeout
pub fn with_timeout<F, T>(duration: Duration, f: F) -> Result<T, &'static str>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = f();
        let _ = tx.send(result);
    });

    rx.recv_timeout(duration).map_err(|_| "Operation timed out")
}

// ============================================================================
// Network Helpers
// ============================================================================

/// Check if a port is likely available
pub fn is_port_likely_available(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

/// Find an available port
pub fn find_available_port() -> u16 {
    for _ in 0..100 {
        let port = config::random_port();
        if is_port_likely_available(port) {
            return port;
        }
    }
    // Fallback
    config::random_port()
}

/// Create a loopback address
pub fn loopback_address(port: u16) -> String {
    format!("127.0.0.1:{}", port)
}

// ============================================================================
// Tests for Helpers
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_identity_creation() {
        let identity = MockIdentity::new("test_user");
        assert_eq!(identity.nickname, "test_user");
        assert!(identity.public_key_z32.starts_with("z32_"));
        assert_eq!(identity.secret_key.len(), 32);
        assert!(identity.noise_keypair.is_some());
    }

    #[test]
    fn test_mock_receipt_store() {
        let store = MockReceiptStore::new();

        let receipt = MockReceipt {
            id: "test_receipt".to_string(),
            payer_pubkey: "payer".to_string(),
            payee_pubkey: "payee".to_string(),
            amount_sats: 1000,
            created_at: current_timestamp(),
            status: "confirmed".to_string(),
        };

        store.store(receipt.clone());
        assert_eq!(store.count(), 1);

        let retrieved = store.get("test_receipt");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().amount_sats, 1000);

        store.clear();
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_mock_directory_service() {
        let service = MockDirectoryService::new();

        let endpoint = MockEndpoint {
            recipient_pubkey: "test_pubkey".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            server_noise_pubkey: generate_noise_pubkey(),
            metadata: Some("Test endpoint".to_string()),
        };

        service.publish(endpoint);
        assert_eq!(service.count(), 1);

        let discovered = service.discover("test_pubkey");
        assert!(discovered.is_some());
        assert_eq!(discovered.unwrap().port, 8080);

        service.remove("test_pubkey");
        assert_eq!(service.count(), 0);
    }

    #[test]
    fn test_generate_z32_pubkey() {
        let pubkey = generate_z32_pubkey();
        assert!(pubkey.starts_with("z32_"));
        assert!(pubkey.len() > 10);
    }

    #[test]
    fn test_generate_noise_pubkey() {
        let pubkey = generate_noise_pubkey();
        assert_eq!(pubkey.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_create_payment_request() {
        let (receipt_id, request) = create_payment_request("payer", "payee", 1000);
        assert!(receipt_id.starts_with("rcpt_test_"));
        assert_eq!(request.get("payer_pubkey"), Some(&"payer".to_string()));
        assert_eq!(request.get("payee_pubkey"), Some(&"payee".to_string()));
        assert_eq!(request.get("amount_sats"), Some(&"1000".to_string()));
    }

    #[test]
    fn test_assertion_helpers() {
        let receipt = MockReceipt {
            id: "test".to_string(),
            payer_pubkey: "payer".to_string(),
            payee_pubkey: "payee".to_string(),
            amount_sats: 100,
            created_at: current_timestamp(),
            status: "confirmed".to_string(),
        };

        // Should not panic
        assert_receipt_valid(&receipt, "payer", "payee");
    }
}

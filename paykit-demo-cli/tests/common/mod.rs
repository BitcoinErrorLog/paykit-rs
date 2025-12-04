//! Common test utilities for paykit-demo-cli integration tests

use paykit_demo_core::{Identity, IdentityManager};
use std::net::TcpListener;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test context with temporary storage and pre-created identities
#[allow(dead_code)]
pub struct TestContext {
    pub temp_dir: TempDir,
    pub storage_dir: PathBuf,
    pub alice: Identity,
    pub bob: Identity,
}

#[allow(dead_code)]
impl TestContext {
    /// Create a new test context with Alice and Bob identities
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        let id_manager = IdentityManager::new(storage_dir.join("identities"));

        let alice = id_manager.create("alice").unwrap();
        let bob = id_manager.create("bob").unwrap();

        Self {
            temp_dir,
            storage_dir,
            alice,
            bob,
        }
    }

    /// Get a free port for testing servers
    pub fn get_free_port() -> u16 {
        // Bind to port 0 to let the OS assign a free port
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to ephemeral port");
        let port = listener.local_addr().unwrap().port();
        drop(listener); // Release the port
        port
    }
}

/// Wait for a TCP server to become available on the given port
/// Returns true if the server is ready within the timeout
#[allow(dead_code)]
pub fn wait_for_server(port: u16, timeout_secs: u64) -> bool {
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    false
}

/// Create a test payment request
#[allow(dead_code)]
pub fn create_test_payment_request(
    from: &paykit_lib::PublicKey,
    to: &paykit_lib::PublicKey,
    amount_sats: i64,
) -> paykit_subscriptions::PaymentRequest {
    use paykit_lib::MethodId;
    use paykit_subscriptions::{Amount, PaymentRequest};

    PaymentRequest::new(
        from.clone(),
        to.clone(),
        Amount::from_sats(amount_sats),
        "SAT".to_string(),
        MethodId("lightning".to_string()),
    )
}

/// Create a test subscription
#[allow(dead_code)]
pub fn create_test_subscription(
    subscriber: &paykit_lib::PublicKey,
    provider: &paykit_lib::PublicKey,
    amount_sats: i64,
    frequency: paykit_subscriptions::PaymentFrequency,
) -> paykit_subscriptions::Subscription {
    use paykit_lib::MethodId;
    use paykit_subscriptions::{Amount, Subscription, SubscriptionTerms};

    let terms = SubscriptionTerms::new(
        Amount::from_sats(amount_sats),
        "SAT".to_string(),
        frequency,
        MethodId("lightning".to_string()),
        "Test subscription".to_string(),
    );

    Subscription::new(subscriber.clone(), provider.clone(), terms)
}

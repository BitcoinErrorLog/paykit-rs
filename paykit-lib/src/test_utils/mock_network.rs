//! Mock payment network for E2E testing.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::methods::{
    BitcoinExecutor, BitcoinTxResult, DecodedInvoice, LightningExecutor, LightningPaymentResult,
    LightningPaymentStatus,
};
use crate::{PaykitError, Result};
use async_trait::async_trait;

/// Configuration for the test network.
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// Base fee for Lightning payments in msat.
    pub base_fee_msat: u64,
    /// Fee rate for Lightning payments (parts per million).
    pub fee_rate_ppm: u64,
    /// Simulated block height.
    pub block_height: u64,
    /// Simulated fee rate in sat/vB.
    pub fee_rate_sat_vb: f64,
    /// Whether to simulate network failures.
    pub simulate_failures: bool,
    /// Failure probability (0.0 - 1.0).
    pub failure_probability: f64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            base_fee_msat: 1000,
            fee_rate_ppm: 100,
            block_height: 800_000,
            fee_rate_sat_vb: 10.0,
            simulate_failures: false,
            failure_probability: 0.0,
        }
    }
}

/// A simulated payment network for testing.
pub struct TestNetwork {
    config: NetworkConfig,
    wallets: RwLock<HashMap<String, Arc<TestWallet>>>,
    channels: RwLock<Vec<MockChannel>>,
    transactions: RwLock<Vec<MockTransaction>>,
    payments: RwLock<Vec<MockPayment>>,
}

impl TestNetwork {
    /// Create a new test network with default configuration.
    pub fn new() -> Arc<Self> {
        Self::with_config(NetworkConfig::default())
    }

    /// Create a new test network with custom configuration.
    pub fn with_config(config: NetworkConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            wallets: RwLock::new(HashMap::new()),
            channels: RwLock::new(Vec::new()),
            transactions: RwLock::new(Vec::new()),
            payments: RwLock::new(Vec::new()),
        })
    }

    /// Create a wallet in the network.
    pub fn create_wallet(self: &Arc<Self>, name: &str) -> Arc<TestWallet> {
        let wallet = Arc::new(TestWallet::new(name, self.clone()));
        self.wallets
            .write()
            .unwrap()
            .insert(name.to_string(), wallet.clone());
        wallet
    }

    /// Get a wallet by name.
    pub fn get_wallet(&self, name: &str) -> Option<Arc<TestWallet>> {
        self.wallets.read().unwrap().get(name).cloned()
    }

    /// Create a channel between two wallets.
    pub fn create_channel(&self, from: &str, to: &str, capacity_sats: u64) -> MockChannel {
        let channel = MockChannel {
            id: format!("{}x{}x0", from, to),
            from: from.to_string(),
            to: to.to_string(),
            capacity_sats,
            local_balance_sats: capacity_sats / 2,
            remote_balance_sats: capacity_sats / 2,
            active: true,
        };
        self.channels.write().unwrap().push(channel.clone());
        channel
    }

    /// Get the current block height.
    pub fn block_height(&self) -> u64 {
        self.config.block_height
    }

    /// Mine a block (increment block height).
    pub fn mine_block(&self) -> u64 {
        // Note: This is a simplified version - in a real impl we'd need interior mutability
        self.config.block_height + 1
    }

    /// Record a transaction in the network.
    pub fn record_transaction(&self, tx: MockTransaction) {
        self.transactions.write().unwrap().push(tx);
    }

    /// Record a payment in the network.
    pub fn record_payment(&self, payment: MockPayment) {
        self.payments.write().unwrap().push(payment);
    }

    /// Get a transaction by txid.
    pub fn get_transaction(&self, txid: &str) -> Option<MockTransaction> {
        self.transactions
            .read()
            .unwrap()
            .iter()
            .find(|tx| tx.txid == txid)
            .cloned()
    }

    /// Get a payment by hash.
    pub fn get_payment(&self, payment_hash: &str) -> Option<MockPayment> {
        self.payments
            .read()
            .unwrap()
            .iter()
            .find(|p| p.payment_hash == payment_hash)
            .cloned()
    }

    /// Calculate fee for a Lightning payment.
    pub fn calculate_ln_fee(&self, amount_msat: u64) -> u64 {
        self.config.base_fee_msat + (amount_msat * self.config.fee_rate_ppm / 1_000_000)
    }

    /// Check if we should simulate a failure.
    pub fn should_fail(&self) -> bool {
        if !self.config.simulate_failures {
            return false;
        }
        // Simple pseudo-random based on timestamp
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        (ts % 100) as f64 / 100.0 < self.config.failure_probability
    }
}

impl Default for TestNetwork {
    fn default() -> Self {
        Self {
            config: NetworkConfig::default(),
            wallets: RwLock::new(HashMap::new()),
            channels: RwLock::new(Vec::new()),
            transactions: RwLock::new(Vec::new()),
            payments: RwLock::new(Vec::new()),
        }
    }
}

/// A test wallet in the network.
pub struct TestWallet {
    name: String,
    network: Arc<TestNetwork>,
    balance_sats: RwLock<u64>,
    invoices: RwLock<HashMap<String, MockInvoice>>,
}

impl TestWallet {
    fn new(name: &str, network: Arc<TestNetwork>) -> Self {
        Self {
            name: name.to_string(),
            network,
            balance_sats: RwLock::new(0),
            invoices: RwLock::new(HashMap::new()),
        }
    }

    /// Get the wallet name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the wallet balance in satoshis.
    pub fn balance(&self) -> u64 {
        *self.balance_sats.read().unwrap()
    }

    /// Fund the wallet with satoshis.
    pub fn fund(&self, amount_sats: u64) {
        let mut balance = self.balance_sats.write().unwrap();
        *balance += amount_sats;
    }

    /// Create an invoice.
    pub fn create_invoice(&self, amount_msat: u64, description: &str) -> MockInvoice {
        let payment_hash = super::fixtures::random_payment_hash();
        let preimage = super::fixtures::random_preimage();

        let invoice = MockInvoice {
            payment_hash: payment_hash.clone(),
            preimage,
            amount_msat: Some(amount_msat),
            description: description.to_string(),
            payee: self.name.clone(),
            expiry_secs: 3600,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            paid: false,
        };

        self.invoices
            .write()
            .unwrap()
            .insert(payment_hash, invoice.clone());
        invoice
    }

    /// Get a public key for this wallet (mock).
    pub fn pubkey(&self) -> String {
        let (_, pk) = super::fixtures::create_test_keypair(&self.name);
        pk
    }

    /// Get an address for this wallet (mock).
    pub fn address(&self) -> String {
        format!("bc1q{}", &super::fixtures::random_payment_hash()[..38])
    }
}

#[async_trait]
impl LightningExecutor for TestWallet {
    async fn pay_invoice(
        &self,
        _invoice: &str,
        amount_msat: Option<u64>,
        _max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResult> {
        if self.network.should_fail() {
            return Err(PaykitError::Payment {
                payment_id: None,
                reason: "Simulated network failure".to_string(),
            });
        }

        let amount = amount_msat.unwrap_or(1000);
        let fee = self.network.calculate_ln_fee(amount);

        // Check balance
        let balance = self.balance();
        if balance * 1000 < amount + fee {
            return Err(PaykitError::InsufficientFunds {
                required: ((amount + fee) / 1000).to_string(),
                available: balance.to_string(),
                currency: "SAT".to_string(),
            });
        }

        let preimage = super::fixtures::random_preimage();
        let payment_hash = super::fixtures::random_payment_hash();

        // Deduct from balance
        {
            let mut bal = self.balance_sats.write().unwrap();
            *bal -= (amount + fee) / 1000;
        }

        // Record payment
        self.network.record_payment(MockPayment {
            payment_hash: payment_hash.clone(),
            preimage: preimage.clone(),
            amount_msat: amount,
            fee_msat: fee,
            status: LightningPaymentStatus::Succeeded,
            from: self.name.clone(),
            to: "unknown".to_string(),
        });

        Ok(LightningPaymentResult {
            preimage,
            payment_hash,
            amount_msat: amount,
            fee_msat: fee,
            hops: 1,
            status: LightningPaymentStatus::Succeeded,
        })
    }

    async fn decode_invoice(&self, invoice: &str) -> Result<DecodedInvoice> {
        Ok(DecodedInvoice {
            payment_hash: super::fixtures::random_payment_hash(),
            amount_msat: Some(1_000_000),
            description: Some(format!("Decoded: {}", &invoice[..20.min(invoice.len())])),
            description_hash: None,
            payee: self.pubkey(),
            expiry: 3600,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            expired: false,
        })
    }

    async fn estimate_fee(&self, _invoice: &str) -> Result<u64> {
        Ok(self.network.calculate_ln_fee(1_000_000))
    }

    async fn get_payment(&self, payment_hash: &str) -> Result<Option<LightningPaymentResult>> {
        Ok(self
            .network
            .get_payment(payment_hash)
            .map(|p| LightningPaymentResult {
                preimage: p.preimage,
                payment_hash: p.payment_hash,
                amount_msat: p.amount_msat,
                fee_msat: p.fee_msat,
                hops: 1,
                status: p.status,
            }))
    }
}

#[async_trait]
impl BitcoinExecutor for TestWallet {
    async fn send_to_address(
        &self,
        address: &str,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResult> {
        if self.network.should_fail() {
            return Err(PaykitError::Payment {
                payment_id: None,
                reason: "Simulated network failure".to_string(),
            });
        }

        let fee_rate = fee_rate.unwrap_or(self.network.config.fee_rate_sat_vb);
        let fee_sats = (fee_rate * 140.0) as u64; // ~140 vB for P2WPKH

        // Check balance
        let balance = self.balance();
        if balance < amount_sats + fee_sats {
            return Err(PaykitError::InsufficientFunds {
                required: (amount_sats + fee_sats).to_string(),
                available: balance.to_string(),
                currency: "SAT".to_string(),
            });
        }

        // Deduct from balance
        {
            let mut bal = self.balance_sats.write().unwrap();
            *bal -= amount_sats + fee_sats;
        }

        let txid = format!(
            "{:064x}",
            super::fixtures::simple_hash_u64(&format!(
                "{}:{}:{}",
                address,
                amount_sats,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ))
        );

        let tx = MockTransaction {
            txid: txid.clone(),
            from: self.name.clone(),
            to: address.to_string(),
            amount_sats,
            fee_sats,
            confirmations: 0,
            block_height: None,
        };

        self.network.record_transaction(tx);

        Ok(BitcoinTxResult {
            txid,
            raw_tx: None,
            vout: 0,
            fee_sats,
            fee_rate,
            block_height: None,
            confirmations: 0,
        })
    }

    async fn estimate_fee(
        &self,
        _address: &str,
        _amount_sats: u64,
        target_blocks: u32,
    ) -> Result<u64> {
        let fee_rate = match target_blocks {
            1 => 50.0,
            2..=3 => 25.0,
            4..=6 => 10.0,
            _ => 5.0,
        };
        Ok((fee_rate * 140.0) as u64)
    }

    async fn get_transaction(&self, txid: &str) -> Result<Option<BitcoinTxResult>> {
        Ok(self
            .network
            .get_transaction(txid)
            .map(|tx| BitcoinTxResult {
                txid: tx.txid,
                raw_tx: None,
                vout: 0,
                fee_sats: tx.fee_sats,
                fee_rate: tx.fee_sats as f64 / 140.0,
                block_height: tx.block_height,
                confirmations: tx.confirmations,
            }))
    }

    async fn verify_transaction(
        &self,
        txid: &str,
        address: &str,
        amount_sats: u64,
    ) -> Result<bool> {
        Ok(self
            .network
            .get_transaction(txid)
            .map(|tx| tx.to == address && tx.amount_sats == amount_sats)
            .unwrap_or(false))
    }
}

/// A mock Lightning channel.
#[derive(Clone, Debug)]
pub struct MockChannel {
    pub id: String,
    pub from: String,
    pub to: String,
    pub capacity_sats: u64,
    pub local_balance_sats: u64,
    pub remote_balance_sats: u64,
    pub active: bool,
}

/// A mock Lightning invoice.
#[derive(Clone, Debug)]
pub struct MockInvoice {
    pub payment_hash: String,
    pub preimage: String,
    pub amount_msat: Option<u64>,
    pub description: String,
    pub payee: String,
    pub expiry_secs: u64,
    pub created_at: u64,
    pub paid: bool,
}

/// A mock on-chain transaction.
#[derive(Clone, Debug)]
pub struct MockTransaction {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub confirmations: u64,
    pub block_height: Option<u64>,
}

/// A mock Lightning payment.
#[derive(Clone, Debug)]
pub struct MockPayment {
    pub payment_hash: String,
    pub preimage: String,
    pub amount_msat: u64,
    pub fee_msat: u64,
    pub status: LightningPaymentStatus,
    pub from: String,
    pub to: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_network() {
        let network = TestNetwork::new();
        let alice = network.create_wallet("alice");
        let bob = network.create_wallet("bob");

        assert_eq!(alice.name(), "alice");
        assert_eq!(bob.name(), "bob");
        assert_eq!(alice.balance(), 0);
    }

    #[test]
    fn test_fund_wallet() {
        let network = TestNetwork::new();
        let alice = network.create_wallet("alice");

        alice.fund(100_000);
        assert_eq!(alice.balance(), 100_000);

        alice.fund(50_000);
        assert_eq!(alice.balance(), 150_000);
    }

    #[test]
    fn test_create_channel() {
        let network = TestNetwork::new();
        network.create_wallet("alice");
        network.create_wallet("bob");

        let channel = network.create_channel("alice", "bob", 1_000_000);

        assert_eq!(channel.capacity_sats, 1_000_000);
        assert!(channel.active);
    }

    #[test]
    fn test_create_invoice() {
        let network = TestNetwork::new();
        let alice = network.create_wallet("alice");

        let invoice = alice.create_invoice(100_000_000, "test payment");

        assert!(invoice.amount_msat.is_some());
        assert_eq!(invoice.amount_msat.unwrap(), 100_000_000);
        assert_eq!(invoice.description, "test payment");
    }

    #[tokio::test]
    async fn test_lightning_payment() {
        let network = TestNetwork::new();
        let alice = network.create_wallet("alice");

        // Fund Alice
        alice.fund(100_000);

        // Make a payment
        let result = alice
            .pay_invoice("lnbc1...", Some(10_000_000), None)
            .await
            .unwrap();

        assert_eq!(result.status, LightningPaymentStatus::Succeeded);
        assert!(!result.preimage.is_empty());

        // Balance should be reduced
        assert!(alice.balance() < 100_000);
    }

    #[tokio::test]
    async fn test_bitcoin_payment() {
        let network = TestNetwork::new();
        let alice = network.create_wallet("alice");

        // Fund Alice
        alice.fund(100_000);

        // Make a payment
        let result = alice
            .send_to_address("bc1qtest...", 10_000, None)
            .await
            .unwrap();

        assert!(!result.txid.is_empty());
        assert!(result.fee_sats > 0);

        // Balance should be reduced
        assert!(alice.balance() < 100_000);
    }

    #[tokio::test]
    async fn test_insufficient_funds() {
        let network = TestNetwork::new();
        let alice = network.create_wallet("alice");

        alice.fund(1_000);

        // Try to send more than we have
        let result = alice.send_to_address("bc1qtest...", 10_000, None).await;

        assert!(result.is_err());
    }
}

//! Test assertions and verification helpers.

use crate::methods::{BitcoinTxResult, LightningPaymentResult, LightningPaymentStatus};
use crate::Result;

/// Assertion helper for payment verification.
pub struct PaymentAssertion;

impl PaymentAssertion {
    /// Assert that a Lightning payment result indicates success.
    pub fn lightning_succeeded(result: &LightningPaymentResult) -> bool {
        result.status == LightningPaymentStatus::Succeeded
            && !result.preimage.is_empty()
            && !result.payment_hash.is_empty()
    }

    /// Assert that a Bitcoin transaction has minimum confirmations.
    pub fn bitcoin_confirmed(result: &BitcoinTxResult, min_confirmations: u64) -> bool {
        result.confirmations >= min_confirmations
    }

    /// Assert that a Bitcoin transaction was broadcast (has txid).
    pub fn bitcoin_broadcast(result: &BitcoinTxResult) -> bool {
        !result.txid.is_empty()
    }
}

/// Assert that a payment result indicates success.
///
/// # Panics
/// Panics if the payment did not succeed.
pub fn assert_payment_succeeded(result: &Result<LightningPaymentResult>) {
    match result {
        Ok(payment) => {
            assert_eq!(
                payment.status,
                LightningPaymentStatus::Succeeded,
                "Payment status should be Succeeded, got {:?}",
                payment.status
            );
            assert!(
                !payment.preimage.is_empty(),
                "Payment preimage should not be empty"
            );
            assert!(
                !payment.payment_hash.is_empty(),
                "Payment hash should not be empty"
            );
        }
        Err(e) => {
            panic!("Payment failed with error: {}", e);
        }
    }
}

/// Assert that a payment result indicates failure.
///
/// # Panics
/// Panics if the payment succeeded.
pub fn assert_payment_failed(result: &Result<LightningPaymentResult>) {
    match result {
        Ok(payment) => {
            assert_ne!(
                payment.status,
                LightningPaymentStatus::Succeeded,
                "Expected payment to fail, but it succeeded"
            );
        }
        Err(_) => {
            // Expected
        }
    }
}

/// Assert that an invoice string appears valid.
///
/// # Panics
/// Panics if the invoice doesn't appear to be a valid Lightning invoice.
pub fn assert_invoice_valid(invoice: &str) {
    assert!(
        invoice.starts_with("lnbc") || invoice.starts_with("lntb") || invoice.starts_with("lnurl"),
        "Invoice should start with lnbc, lntb, or lnurl, got: {}",
        &invoice[..20.min(invoice.len())]
    );
    assert!(
        invoice.len() > 20,
        "Invoice should be longer than 20 characters"
    );
}

/// Assert that a Bitcoin address appears valid.
///
/// # Panics
/// Panics if the address doesn't appear to be a valid Bitcoin address.
pub fn assert_address_valid(address: &str, testnet: bool) {
    if testnet {
        assert!(
            address.starts_with("tb1")
                || address.starts_with("2")
                || address.starts_with("m")
                || address.starts_with("n"),
            "Testnet address should start with tb1, 2, m, or n, got: {}",
            address
        );
    } else {
        assert!(
            address.starts_with("bc1") || address.starts_with("1") || address.starts_with("3"),
            "Mainnet address should start with bc1, 1, or 3, got: {}",
            address
        );
    }
}

/// Builder for complex payment assertions.
pub struct PaymentAssertionBuilder<'a> {
    result: &'a LightningPaymentResult,
    checks: Vec<(&'static str, bool)>,
}

impl<'a> PaymentAssertionBuilder<'a> {
    /// Create a new assertion builder.
    pub fn new(result: &'a LightningPaymentResult) -> Self {
        Self {
            result,
            checks: Vec::new(),
        }
    }

    /// Assert the payment succeeded.
    pub fn succeeded(mut self) -> Self {
        self.checks.push((
            "status is Succeeded",
            self.result.status == LightningPaymentStatus::Succeeded,
        ));
        self
    }

    /// Assert the preimage is not empty.
    pub fn has_preimage(mut self) -> Self {
        self.checks
            .push(("has preimage", !self.result.preimage.is_empty()));
        self
    }

    /// Assert the amount matches.
    pub fn amount_msat(mut self, expected: u64) -> Self {
        self.checks
            .push(("amount matches", self.result.amount_msat == expected));
        self
    }

    /// Assert the fee is within bounds.
    pub fn fee_within(mut self, max_fee_msat: u64) -> Self {
        self.checks
            .push(("fee within bounds", self.result.fee_msat <= max_fee_msat));
        self
    }

    /// Assert the number of hops.
    pub fn hops(mut self, expected: u32) -> Self {
        self.checks
            .push(("hop count matches", self.result.hops == expected));
        self
    }

    /// Execute all assertions.
    ///
    /// # Panics
    /// Panics if any assertion fails.
    pub fn assert(self) {
        for (description, passed) in self.checks {
            assert!(passed, "Assertion failed: {}", description);
        }
    }

    /// Check if all assertions pass without panicking.
    pub fn check(self) -> bool {
        self.checks.iter().all(|(_, passed)| *passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_invoice_valid() {
        assert_invoice_valid(
            "lnbc1pvjluezsp5zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zygs9qrsgq",
        );
        assert_invoice_valid("lntb1000000000m1ptest");
        assert_invoice_valid("lnurl1dp68gurn8ghj7um9wfmxjcm99e3k7mf0");
    }

    #[test]
    fn test_assert_address_valid_mainnet() {
        assert_address_valid("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq", false);
        assert_address_valid("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2", false);
        assert_address_valid("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy", false);
    }

    #[test]
    fn test_assert_address_valid_testnet() {
        assert_address_valid("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx", true);
        assert_address_valid("2MzQwSSnBHWHqSAqtTVQ6v47XtaisrJa1Vc", true);
        assert_address_valid("mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn", true);
    }

    #[test]
    fn test_payment_assertion_builder() {
        let result = LightningPaymentResult {
            preimage: "abc123".to_string(),
            payment_hash: "hash123".to_string(),
            amount_msat: 1_000_000,
            fee_msat: 1000,
            hops: 2,
            status: LightningPaymentStatus::Succeeded,
        };

        let passed = PaymentAssertionBuilder::new(&result)
            .succeeded()
            .has_preimage()
            .amount_msat(1_000_000)
            .fee_within(5000)
            .hops(2)
            .check();

        assert!(passed);
    }

    #[test]
    fn test_payment_assertion_helper() {
        assert!(PaymentAssertion::lightning_succeeded(
            &LightningPaymentResult {
                preimage: "abc".to_string(),
                payment_hash: "hash".to_string(),
                amount_msat: 1000,
                fee_msat: 10,
                hops: 1,
                status: LightningPaymentStatus::Succeeded,
            }
        ));

        assert!(!PaymentAssertion::lightning_succeeded(
            &LightningPaymentResult {
                preimage: "abc".to_string(),
                payment_hash: "hash".to_string(),
                amount_msat: 1000,
                fee_msat: 10,
                hops: 1,
                status: LightningPaymentStatus::Failed,
            }
        ));
    }
}

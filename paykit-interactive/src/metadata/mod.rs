//! Payment Metadata Standards
//!
//! This module provides standardized metadata structures for payment receipts
//! and interactive payment flows. All metadata is serializable as JSON.
//!
//! # Standard Metadata Types
//!
//! - **OrderMetadata**: Order information (order ID, invoice number, items)
//! - **ShippingMetadata**: Shipping details (address, method, tracking)
//! - **TaxMetadata**: Tax information (rate, amount, jurisdiction)
//! - **CustomMetadata**: Extensible key-value pairs
//!
//! # Example
//!
//! ```ignore
//! use paykit_interactive::metadata::{OrderMetadata, PaymentMetadata};
//!
//! let order = OrderMetadata {
//!     order_id: Some("ORD-12345".into()),
//!     invoice_number: Some("INV-2024-001".into()),
//!     items: vec![],
//!     notes: None,
//! };
//!
//! let metadata = PaymentMetadata::new()
//!     .with_order(order);
//!
//! let json = metadata.to_json();
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Item in an order for metadata purposes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataItem {
    /// Item description.
    pub description: String,
    /// Quantity.
    pub quantity: u32,
    /// Unit price as string (preserves precision).
    pub unit_price: String,
    /// Currency code.
    pub currency: String,
    /// Optional SKU.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
}

impl MetadataItem {
    /// Create a new metadata item.
    pub fn new(
        description: impl Into<String>,
        quantity: u32,
        unit_price: impl Into<String>,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            quantity,
            unit_price: unit_price.into(),
            currency: currency.into(),
            sku: None,
        }
    }

    /// Set the SKU.
    pub fn with_sku(mut self, sku: impl Into<String>) -> Self {
        self.sku = Some(sku.into());
        self
    }
}

/// Order-related metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderMetadata {
    /// Unique order identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Invoice number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_number: Option<String>,
    /// Line items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<MetadataItem>,
    /// Order notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl OrderMetadata {
    /// Create empty order metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set order ID.
    pub fn with_order_id(mut self, id: impl Into<String>) -> Self {
        self.order_id = Some(id.into());
        self
    }

    /// Set invoice number.
    pub fn with_invoice_number(mut self, number: impl Into<String>) -> Self {
        self.invoice_number = Some(number.into());
        self
    }

    /// Add an item.
    pub fn add_item(mut self, item: MetadataItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Shipping address for metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataAddress {
    /// Recipient name.
    pub name: String,
    /// Street address.
    pub street: String,
    /// City.
    pub city: String,
    /// State/province.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Postal code.
    pub postal_code: String,
    /// Country code (ISO 3166-1 alpha-2).
    pub country: String,
}

impl MetadataAddress {
    /// Create a new address.
    pub fn new(
        name: impl Into<String>,
        street: impl Into<String>,
        city: impl Into<String>,
        postal_code: impl Into<String>,
        country: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            street: street.into(),
            city: city.into(),
            state: None,
            postal_code: postal_code.into(),
            country: country.into(),
        }
    }

    /// Set state.
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }
}

/// Shipping-related metadata.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShippingMetadata {
    /// Shipping address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<MetadataAddress>,
    /// Shipping method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Shipping cost as string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    /// Currency for cost.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_currency: Option<String>,
    /// Tracking number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking_number: Option<String>,
    /// Carrier name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
    /// Estimated delivery timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_delivery: Option<i64>,
}

impl ShippingMetadata {
    /// Create empty shipping metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set address.
    pub fn with_address(mut self, address: MetadataAddress) -> Self {
        self.address = Some(address);
        self
    }

    /// Set shipping method.
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Set shipping cost.
    pub fn with_cost(mut self, cost: impl Into<String>, currency: impl Into<String>) -> Self {
        self.cost = Some(cost.into());
        self.cost_currency = Some(currency.into());
        self
    }

    /// Set tracking information.
    pub fn with_tracking(mut self, number: impl Into<String>, carrier: impl Into<String>) -> Self {
        self.tracking_number = Some(number.into());
        self.carrier = Some(carrier.into());
        self
    }

    /// Set estimated delivery.
    pub fn with_estimated_delivery(mut self, timestamp: i64) -> Self {
        self.estimated_delivery = Some(timestamp);
        self
    }
}

/// Tax-related metadata.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TaxMetadata {
    /// Tax description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tax rate as percentage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    /// Tax amount as string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Currency for amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Jurisdiction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,
    /// Tax ID / VAT number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,
}

impl TaxMetadata {
    /// Create empty tax metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tax description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set tax rate.
    pub fn with_rate(mut self, rate: f64) -> Self {
        self.rate = Some(rate);
        self
    }

    /// Set tax amount.
    pub fn with_amount(mut self, amount: impl Into<String>, currency: impl Into<String>) -> Self {
        self.amount = Some(amount.into());
        self.currency = Some(currency.into());
        self
    }

    /// Set jurisdiction.
    pub fn with_jurisdiction(mut self, jurisdiction: impl Into<String>) -> Self {
        self.jurisdiction = Some(jurisdiction.into());
        self
    }

    /// Set tax ID.
    pub fn with_tax_id(mut self, id: impl Into<String>) -> Self {
        self.tax_id = Some(id.into());
        self
    }
}

/// Combined payment metadata.
///
/// This struct aggregates all standard metadata types plus custom data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PaymentMetadata {
    /// Order metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<OrderMetadata>,
    /// Shipping metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping: Option<ShippingMetadata>,
    /// Tax metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax: Option<TaxMetadata>,
    /// Custom key-value pairs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, Value>,
}

impl PaymentMetadata {
    /// Create empty payment metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set order metadata.
    pub fn with_order(mut self, order: OrderMetadata) -> Self {
        self.order = Some(order);
        self
    }

    /// Set shipping metadata.
    pub fn with_shipping(mut self, shipping: ShippingMetadata) -> Self {
        self.shipping = Some(shipping);
        self
    }

    /// Set tax metadata.
    pub fn with_tax(mut self, tax: TaxMetadata) -> Self {
        self.tax = Some(tax);
        self
    }

    /// Add a custom field.
    pub fn with_custom(mut self, key: impl Into<String>, value: Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Merge with another metadata instance.
    ///
    /// Values from `other` take precedence.
    pub fn merge(mut self, other: PaymentMetadata) -> Self {
        if other.order.is_some() {
            self.order = other.order;
        }
        if other.shipping.is_some() {
            self.shipping = other.shipping;
        }
        if other.tax.is_some() {
            self.tax = other.tax;
        }
        self.custom.extend(other.custom);
        self
    }

    /// Convert to JSON value.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// Parse from JSON value.
    pub fn from_json(value: &Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }

    /// Check if metadata is empty.
    pub fn is_empty(&self) -> bool {
        self.order.is_none()
            && self.shipping.is_none()
            && self.tax.is_none()
            && self.custom.is_empty()
    }
}

/// Validate metadata against Paykit standards.
#[derive(Clone, Debug, Default)]
pub struct MetadataValidator {
    /// Whether order ID is required.
    pub require_order_id: bool,
    /// Whether shipping address is required.
    pub require_shipping: bool,
    /// Whether tax info is required.
    pub require_tax: bool,
}

impl MetadataValidator {
    /// Create a new validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Require order ID.
    pub fn require_order_id(mut self) -> Self {
        self.require_order_id = true;
        self
    }

    /// Require shipping address.
    pub fn require_shipping(mut self) -> Self {
        self.require_shipping = true;
        self
    }

    /// Require tax info.
    pub fn require_tax(mut self) -> Self {
        self.require_tax = true;
        self
    }

    /// Validate metadata.
    pub fn validate(&self, metadata: &PaymentMetadata) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.require_order_id {
            if metadata
                .order
                .as_ref()
                .and_then(|o| o.order_id.as_ref())
                .is_none()
            {
                errors.push("Order ID is required".to_string());
            }
        }

        if self.require_shipping {
            if metadata
                .shipping
                .as_ref()
                .and_then(|s| s.address.as_ref())
                .is_none()
            {
                errors.push("Shipping address is required".to_string());
            }
        }

        if self.require_tax {
            if metadata.tax.is_none() {
                errors.push("Tax information is required".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_metadata() {
        let order = OrderMetadata::new()
            .with_order_id("ORD-123")
            .with_invoice_number("INV-456")
            .add_item(MetadataItem::new("Widget", 2, "1000", "SAT").with_sku("WGT-001"));

        assert_eq!(order.order_id, Some("ORD-123".to_string()));
        assert_eq!(order.invoice_number, Some("INV-456".to_string()));
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].sku, Some("WGT-001".to_string()));
    }

    #[test]
    fn test_shipping_metadata() {
        let shipping = ShippingMetadata::new()
            .with_address(MetadataAddress::new(
                "John Doe",
                "123 Main St",
                "Anytown",
                "12345",
                "US",
            ))
            .with_method("Express")
            .with_cost("500", "SAT")
            .with_tracking("1Z999AA10123456784", "UPS");

        assert!(shipping.address.is_some());
        assert_eq!(shipping.method, Some("Express".to_string()));
        assert_eq!(
            shipping.tracking_number,
            Some("1Z999AA10123456784".to_string())
        );
    }

    #[test]
    fn test_tax_metadata() {
        let tax = TaxMetadata::new()
            .with_description("Sales Tax")
            .with_rate(8.25)
            .with_amount("825", "SAT")
            .with_jurisdiction("CA");

        assert_eq!(tax.rate, Some(8.25));
        assert_eq!(tax.jurisdiction, Some("CA".to_string()));
    }

    #[test]
    fn test_payment_metadata() {
        let metadata = PaymentMetadata::new()
            .with_order(OrderMetadata::new().with_order_id("ORD-789"))
            .with_custom("source", serde_json::json!("web"));

        assert!(metadata.order.is_some());
        assert!(metadata.custom.contains_key("source"));
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata =
            PaymentMetadata::new().with_order(OrderMetadata::new().with_order_id("TEST"));

        let json = metadata.to_json();
        let parsed = PaymentMetadata::from_json(&json);

        assert!(parsed.is_some());
        assert_eq!(
            parsed.unwrap().order.unwrap().order_id,
            Some("TEST".to_string())
        );
    }

    #[test]
    fn test_metadata_merge() {
        let base = PaymentMetadata::new()
            .with_order(OrderMetadata::new().with_order_id("OLD"))
            .with_custom("key1", serde_json::json!("value1"));

        let overlay = PaymentMetadata::new()
            .with_order(OrderMetadata::new().with_order_id("NEW"))
            .with_custom("key2", serde_json::json!("value2"));

        let merged = base.merge(overlay);

        assert_eq!(merged.order.unwrap().order_id, Some("NEW".to_string()));
        assert!(merged.custom.contains_key("key1"));
        assert!(merged.custom.contains_key("key2"));
    }

    #[test]
    fn test_validator() {
        let validator = MetadataValidator::new().require_order_id();

        let valid = PaymentMetadata::new().with_order(OrderMetadata::new().with_order_id("ORD-1"));

        let invalid = PaymentMetadata::new();

        assert!(validator.validate(&valid).is_ok());
        assert!(validator.validate(&invalid).is_err());
    }
}

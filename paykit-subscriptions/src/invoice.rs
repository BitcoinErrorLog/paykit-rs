//! Invoice Types for Payment Requests
//!
//! This module provides standardized invoice structures for payment requests,
//! including line items, tax information, and shipping details.

use crate::Amount;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A line item in an invoice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceItem {
    /// Unique identifier for the item.
    pub item_id: Option<String>,
    /// Description of the item.
    pub description: String,
    /// Quantity of the item.
    pub quantity: u32,
    /// Unit price.
    pub unit_price: Amount,
    /// Total price for this line (quantity * unit_price).
    pub total: Amount,
    /// Optional SKU or product code.
    pub sku: Option<String>,
    /// Optional category.
    pub category: Option<String>,
}

impl InvoiceItem {
    /// Create a new invoice item.
    pub fn new(description: impl Into<String>, quantity: u32, unit_price: Amount) -> Self {
        let total = unit_price.multiply(quantity);
        Self {
            item_id: None,
            description: description.into(),
            quantity,
            unit_price,
            total,
            sku: None,
            category: None,
        }
    }

    /// Set the item ID.
    pub fn with_item_id(mut self, id: impl Into<String>) -> Self {
        self.item_id = Some(id.into());
        self
    }

    /// Set the SKU.
    pub fn with_sku(mut self, sku: impl Into<String>) -> Self {
        self.sku = Some(sku.into());
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// Tax information for an invoice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaxInfo {
    /// Tax description (e.g., "Sales Tax", "VAT").
    pub description: String,
    /// Tax rate as a percentage (e.g., dec!(8.25) for 8.25%).
    /// Uses Decimal for exact precision in financial calculations.
    pub rate: Decimal,
    /// Tax amount.
    pub amount: Amount,
    /// Jurisdiction (e.g., "CA", "EU", "UK").
    pub jurisdiction: Option<String>,
    /// Tax ID or registration number.
    pub tax_id: Option<String>,
}

impl TaxInfo {
    /// Create new tax info.
    pub fn new(description: impl Into<String>, rate: Decimal, amount: Amount) -> Self {
        Self {
            description: description.into(),
            rate,
            amount,
            jurisdiction: None,
            tax_id: None,
        }
    }

    /// Calculate tax from a subtotal.
    ///
    /// Uses Decimal for exact precision in financial calculations.
    pub fn from_subtotal(description: impl Into<String>, rate: Decimal, subtotal: &Amount) -> Self {
        let tax_amount = subtotal.percentage(rate);
        Self::new(description, rate, tax_amount)
    }
    
    /// Create new tax info from an f64 rate (convenience method).
    /// Note: Uses f64 for rate, which may introduce minor precision loss.
    /// For exact precision, use `new` with a Decimal rate.
    pub fn new_f64(description: impl Into<String>, rate: f64, amount: Amount) -> Self {
        Self::new(description, Decimal::from_f64_retain(rate).unwrap_or(Decimal::ZERO), amount)
    }

    /// Calculate tax from a subtotal using an f64 rate (convenience method).
    /// Note: Uses f64 for rate, which may introduce minor precision loss.
    /// For exact precision, use `from_subtotal` with a Decimal rate.
    pub fn from_subtotal_f64(description: impl Into<String>, rate: f64, subtotal: &Amount) -> Self {
        let rate_decimal = Decimal::from_f64_retain(rate).unwrap_or(Decimal::ZERO);
        Self::from_subtotal(description, rate_decimal, subtotal)
    }

    /// Set jurisdiction.
    pub fn with_jurisdiction(mut self, jurisdiction: impl Into<String>) -> Self {
        self.jurisdiction = Some(jurisdiction.into());
        self
    }

    /// Set tax ID.
    pub fn with_tax_id(mut self, tax_id: impl Into<String>) -> Self {
        self.tax_id = Some(tax_id.into());
        self
    }
}

/// Shipping method.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShippingMethod {
    /// Standard shipping.
    #[default]
    Standard,
    /// Express shipping.
    Express,
    /// Overnight shipping.
    Overnight,
    /// Digital delivery (no physical shipping).
    Digital,
    /// Local pickup.
    Pickup,
    /// Custom shipping method.
    Custom(String),
}

/// Shipping address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShippingAddress {
    /// Recipient name.
    pub name: String,
    /// Address line 1.
    pub line1: String,
    /// Address line 2 (optional).
    pub line2: Option<String>,
    /// City.
    pub city: String,
    /// State/province.
    pub state: Option<String>,
    /// Postal/ZIP code.
    pub postal_code: String,
    /// Country (ISO 3166-1 alpha-2).
    pub country: String,
    /// Phone number (optional).
    pub phone: Option<String>,
}

impl ShippingAddress {
    /// Create a new shipping address.
    pub fn new(
        name: impl Into<String>,
        line1: impl Into<String>,
        city: impl Into<String>,
        postal_code: impl Into<String>,
        country: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            line1: line1.into(),
            line2: None,
            city: city.into(),
            state: None,
            postal_code: postal_code.into(),
            country: country.into(),
            phone: None,
        }
    }

    /// Set address line 2.
    pub fn with_line2(mut self, line2: impl Into<String>) -> Self {
        self.line2 = Some(line2.into());
        self
    }

    /// Set state/province.
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Set phone number.
    pub fn with_phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = Some(phone.into());
        self
    }
}

/// Shipping information for an invoice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShippingInfo {
    /// Shipping address.
    pub address: ShippingAddress,
    /// Shipping method.
    pub method: ShippingMethod,
    /// Shipping cost.
    pub cost: Amount,
    /// Tracking number (if available).
    pub tracking_number: Option<String>,
    /// Carrier name.
    pub carrier: Option<String>,
    /// Estimated delivery date (unix timestamp).
    pub estimated_delivery: Option<i64>,
    /// Shipping instructions.
    pub instructions: Option<String>,
}

impl ShippingInfo {
    /// Create new shipping info.
    pub fn new(address: ShippingAddress, method: ShippingMethod, cost: Amount) -> Self {
        Self {
            address,
            method,
            cost,
            tracking_number: None,
            carrier: None,
            estimated_delivery: None,
            instructions: None,
        }
    }

    /// Create digital delivery (no shipping).
    pub fn digital() -> Self {
        Self {
            address: ShippingAddress::new("Digital", "N/A", "N/A", "00000", "XX"),
            method: ShippingMethod::Digital,
            cost: Amount::zero(),
            tracking_number: None,
            carrier: None,
            estimated_delivery: None,
            instructions: None,
        }
    }

    /// Set tracking number.
    pub fn with_tracking(mut self, number: impl Into<String>, carrier: impl Into<String>) -> Self {
        self.tracking_number = Some(number.into());
        self.carrier = Some(carrier.into());
        self
    }

    /// Set estimated delivery date.
    pub fn with_estimated_delivery(mut self, timestamp: i64) -> Self {
        self.estimated_delivery = Some(timestamp);
        self
    }

    /// Set shipping instructions.
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

/// Complete invoice data combining items, tax, and shipping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invoice {
    /// Invoice number.
    pub invoice_number: String,
    /// Line items.
    pub items: Vec<InvoiceItem>,
    /// Subtotal (sum of all items).
    pub subtotal: Amount,
    /// Tax information.
    pub tax: Option<TaxInfo>,
    /// Shipping information.
    pub shipping: Option<ShippingInfo>,
    /// Discount amount.
    pub discount: Option<Amount>,
    /// Total amount (subtotal + tax + shipping - discount).
    pub total: Amount,
    /// Invoice notes.
    pub notes: Option<String>,
    /// Invoice terms.
    pub terms: Option<String>,
    /// Invoice date.
    pub invoice_date: i64,
    /// Due date.
    pub due_date: Option<i64>,
}

impl Invoice {
    /// Create a new invoice from items.
    pub fn new(invoice_number: impl Into<String>, items: Vec<InvoiceItem>) -> Self {
        let subtotal = items
            .iter()
            .fold(Amount::zero(), |acc, item| acc.add(&item.total));
        let now = chrono::Utc::now().timestamp();

        Self {
            invoice_number: invoice_number.into(),
            items,
            subtotal,
            tax: None,
            shipping: None,
            discount: None,
            total: subtotal,
            notes: None,
            terms: None,
            invoice_date: now,
            due_date: None,
        }
    }

    /// Add tax to the invoice.
    pub fn with_tax(mut self, tax: TaxInfo) -> Self {
        self.tax = Some(tax);
        self.recalculate_total();
        self
    }

    /// Add shipping to the invoice.
    pub fn with_shipping(mut self, shipping: ShippingInfo) -> Self {
        self.shipping = Some(shipping);
        self.recalculate_total();
        self
    }

    /// Apply a discount.
    pub fn with_discount(mut self, discount: Amount) -> Self {
        self.discount = Some(discount);
        self.recalculate_total();
        self
    }

    /// Set invoice notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Set payment terms.
    pub fn with_terms(mut self, terms: impl Into<String>) -> Self {
        self.terms = Some(terms.into());
        self
    }

    /// Set due date.
    pub fn with_due_date(mut self, due_date: i64) -> Self {
        self.due_date = Some(due_date);
        self
    }

    /// Recalculate the total based on subtotal, tax, shipping, and discount.
    fn recalculate_total(&mut self) {
        let mut total = self.subtotal;

        if let Some(ref tax) = self.tax {
            total = total.add(&tax.amount);
        }

        if let Some(ref shipping) = self.shipping {
            total = total.add(&shipping.cost);
        }

        if let Some(ref discount) = self.discount {
            total = total.subtract(discount);
        }

        self.total = total;
    }

    /// Get the number of items.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Check if the invoice has tax.
    pub fn has_tax(&self) -> bool {
        self.tax.is_some()
    }

    /// Check if the invoice has shipping.
    pub fn has_shipping(&self) -> bool {
        self.shipping.is_some()
            && !matches!(
                self.shipping.as_ref().unwrap().method,
                ShippingMethod::Digital
            )
    }
}

/// Invoice format for export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoiceFormat {
    /// JSON format.
    Json,
    /// Plain text format.
    PlainText,
    /// HTML format for PDF generation.
    Html,
}

impl Invoice {
    /// Export the invoice in the specified format.
    pub fn export(&self, format: InvoiceFormat) -> String {
        match format {
            InvoiceFormat::Json => serde_json::to_string_pretty(self).unwrap_or_default(),
            InvoiceFormat::PlainText => self.to_plain_text(),
            InvoiceFormat::Html => self.to_html(),
        }
    }

    fn to_plain_text(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!("INVOICE #{}\n", self.invoice_number));
        text.push_str(&format!(
            "Date: {}\n\n",
            format_timestamp(self.invoice_date)
        ));

        text.push_str("ITEMS:\n");
        for item in &self.items {
            text.push_str(&format!(
                "  {} x {} @ {} = {}\n",
                item.quantity, item.description, item.unit_price, item.total
            ));
        }

        text.push_str(&format!("\nSubtotal: {}\n", self.subtotal));

        if let Some(ref tax) = self.tax {
            text.push_str(&format!(
                "{} ({}%): {}\n",
                tax.description, tax.rate, tax.amount
            ));
        }

        if let Some(ref shipping) = self.shipping {
            text.push_str(&format!(
                "Shipping ({:?}): {}\n",
                shipping.method, shipping.cost
            ));
        }

        if let Some(ref discount) = self.discount {
            text.push_str(&format!("Discount: -{}\n", discount));
        }

        text.push_str(&format!("\nTOTAL: {}\n", self.total));

        if let Some(ref notes) = self.notes {
            text.push_str(&format!("\nNotes: {}\n", notes));
        }

        text
    }

    fn to_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<div class=\"invoice\">\n");
        html.push_str(&format!("<h1>Invoice #{}</h1>\n", self.invoice_number));
        html.push_str(&format!(
            "<p>Date: {}</p>\n",
            format_timestamp(self.invoice_date)
        ));

        html.push_str("<table>\n<thead><tr><th>Item</th><th>Qty</th><th>Price</th><th>Total</th></tr></thead>\n<tbody>\n");
        for item in &self.items {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                item.description, item.quantity, item.unit_price, item.total
            ));
        }
        html.push_str("</tbody>\n</table>\n");

        html.push_str(&format!("<p>Subtotal: {}</p>\n", self.subtotal));

        if let Some(ref tax) = self.tax {
            html.push_str(&format!(
                "<p>{} ({}%): {}</p>\n",
                tax.description, tax.rate, tax.amount
            ));
        }

        if let Some(ref shipping) = self.shipping {
            html.push_str(&format!("<p>Shipping: {}</p>\n", shipping.cost));
        }

        if let Some(ref discount) = self.discount {
            html.push_str(&format!("<p>Discount: -{}</p>\n", discount));
        }

        html.push_str(&format!(
            "<p class=\"total\"><strong>Total: {}</strong></p>\n",
            self.total
        ));
        html.push_str("</div>\n");

        html
    }
}

fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| ts.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_item_creation() {
        let item = InvoiceItem::new("Widget", 2, Amount::from_sats(1000));
        assert_eq!(item.description, "Widget");
        assert_eq!(item.quantity, 2);
        assert_eq!(item.total, Amount::from_sats(2000));
    }

    #[test]
    fn test_tax_from_subtotal() {
        use rust_decimal_macros::dec;
        let subtotal = Amount::from_sats(10000);
        let tax = TaxInfo::from_subtotal("Sales Tax", dec!(8.25), &subtotal);
        assert_eq!(tax.rate, dec!(8.25));
        // 8.25% of 10000 = 825
        assert_eq!(tax.amount, Amount::from_sats(825));
    }

    #[test]
    fn test_shipping_address() {
        let addr = ShippingAddress::new("John Doe", "123 Main St", "Anytown", "12345", "US")
            .with_state("CA");

        assert_eq!(addr.name, "John Doe");
        assert_eq!(addr.state, Some("CA".to_string()));
    }

    #[test]
    fn test_invoice_creation() {
        let items = vec![
            InvoiceItem::new("Widget A", 2, Amount::from_sats(1000)),
            InvoiceItem::new("Widget B", 1, Amount::from_sats(5000)),
        ];

        let invoice = Invoice::new("INV-001", items);

        assert_eq!(invoice.invoice_number, "INV-001");
        assert_eq!(invoice.item_count(), 2);
        assert_eq!(invoice.subtotal, Amount::from_sats(7000)); // 2000 + 5000
        assert_eq!(invoice.total, Amount::from_sats(7000));
    }

    #[test]
    fn test_invoice_with_tax_and_shipping() {
        use rust_decimal_macros::dec;
        let items = vec![InvoiceItem::new("Product", 1, Amount::from_sats(10000))];

        let tax = TaxInfo::from_subtotal("Tax", dec!(10.0), &Amount::from_sats(10000));
        let shipping = ShippingInfo::new(
            ShippingAddress::new("Customer", "123 St", "City", "00000", "US"),
            ShippingMethod::Standard,
            Amount::from_sats(500),
        );

        let invoice = Invoice::new("INV-002", items)
            .with_tax(tax)
            .with_shipping(shipping);

        // 10000 + 1000 (tax) + 500 (shipping) = 11500
        assert_eq!(invoice.total, Amount::from_sats(11500));
    }

    #[test]
    fn test_invoice_with_discount() {
        let items = vec![InvoiceItem::new("Product", 1, Amount::from_sats(10000))];

        let invoice = Invoice::new("INV-003", items).with_discount(Amount::from_sats(1000));

        // 10000 - 1000 = 9000
        assert_eq!(invoice.total, Amount::from_sats(9000));
    }

    #[test]
    fn test_invoice_export_json() {
        let items = vec![InvoiceItem::new("Widget", 1, Amount::from_sats(1000))];
        let invoice = Invoice::new("INV-JSON", items);

        let json = invoice.export(InvoiceFormat::Json);
        assert!(json.contains("INV-JSON"));
        assert!(json.contains("Widget"));
    }

    #[test]
    fn test_invoice_export_plain_text() {
        let items = vec![InvoiceItem::new("Widget", 1, Amount::from_sats(1000))];
        let invoice = Invoice::new("INV-TXT", items);

        let text = invoice.export(InvoiceFormat::PlainText);
        assert!(text.contains("INVOICE #INV-TXT"));
        assert!(text.contains("Widget"));
    }

    #[test]
    fn test_digital_shipping() {
        let shipping = ShippingInfo::digital();
        assert!(matches!(shipping.method, ShippingMethod::Digital));
        assert_eq!(shipping.cost, Amount::zero());
    }
}

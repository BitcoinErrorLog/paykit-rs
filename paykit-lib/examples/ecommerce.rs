//! E-commerce Merchant Server Example
//!
//! This example demonstrates a complete e-commerce merchant server using Paykit:
//! - Product catalog with payment requests
//! - Receipt verification
//! - Order management
//!
//! # Usage
//!
//! ```bash
//! cargo run --example ecommerce
//! ```

use paykit_interactive::PaykitReceipt;
use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::{Amount, PaymentRequest, PaymentRequestResponse};
use std::collections::HashMap;
use std::str::FromStr;

/// Product in the catalog.
#[derive(Clone, Debug)]
struct Product {
    id: String,
    name: String,
    price: Amount,
    currency: String,
    description: String,
}

/// Order in the system.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Example demonstrating data model
struct Order {
    order_id: String,
    customer: PublicKey,
    product: Product,
    payment_request_id: String,
    status: OrderStatus,
    created_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)] // Example demonstrating status model
enum OrderStatus {
    Pending,
    Paid,
    Fulfilled,
    Cancelled,
}

/// Simple merchant server.
struct MerchantServer {
    products: Vec<Product>,
    orders: HashMap<String, Order>,
    merchant_key: PublicKey,
}

impl MerchantServer {
    fn new(merchant_key: PublicKey) -> Self {
        Self {
            products: vec![
                Product {
                    id: "prod_1".to_string(),
                    name: "Bitcoin T-Shirt".to_string(),
                    price: Amount::from_sats(50000),
                    currency: "SAT".to_string(),
                    description: "Cool Bitcoin-themed t-shirt".to_string(),
                },
                Product {
                    id: "prod_2".to_string(),
                    name: "Lightning Node".to_string(),
                    price: Amount::from_sats(100000),
                    currency: "SAT".to_string(),
                    description: "Raspberry Pi Lightning node".to_string(),
                },
            ],
            orders: HashMap::new(),
            merchant_key,
        }
    }

    /// List all products.
    fn list_products(&self) -> &[Product] {
        &self.products
    }

    /// Create a payment request for a product.
    fn create_payment_request(
        &self,
        customer: PublicKey,
        product_id: &str,
        method_id: MethodId,
    ) -> Result<PaymentRequest, String> {
        let product = self
            .products
            .iter()
            .find(|p| p.id == product_id)
            .ok_or_else(|| "Product not found".to_string())?;

        let expires_at = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
        let request = PaymentRequest::new(
            self.merchant_key.clone(),
            customer,
            product.price.clone(),
            product.currency.clone(),
            method_id,
        )
        .with_description(format!("Payment for {}", product.name))
        .with_expiration(expires_at);

        Ok(request)
    }

    /// Process a payment request response.
    fn process_payment_response(
        &mut self,
        response: PaymentRequestResponse,
    ) -> Result<String, String> {
        // Find the order for this request
        let request_id = match &response {
            PaymentRequestResponse::Accepted { request_id, .. } => request_id,
            PaymentRequestResponse::Declined { request_id, .. } => request_id,
            PaymentRequestResponse::Pending { request_id, .. } => request_id,
        };

        let order = self
            .orders
            .values()
            .find(|o| o.payment_request_id == *request_id)
            .ok_or_else(|| "Order not found".to_string())?;

        match response {
            PaymentRequestResponse::Accepted { .. } => {
                println!("Payment accepted for order {}", order.order_id);
                // In real implementation, verify payment and update order status
                Ok(format!("Order {} payment accepted", order.order_id))
            }
            PaymentRequestResponse::Declined { .. } => {
                println!("Payment declined for order {}", order.order_id);
                Ok(format!("Order {} payment declined", order.order_id))
            }
            PaymentRequestResponse::Pending { .. } => {
                println!("Payment pending for order {}", order.order_id);
                Ok(format!("Order {} payment pending", order.order_id))
            }
        }
    }

    /// Create an order.
    fn create_order(&mut self, customer: PublicKey, product_id: &str) -> Result<Order, String> {
        let product = self
            .products
            .iter()
            .find(|p| p.id == product_id)
            .cloned()
            .ok_or_else(|| "Product not found".to_string())?;

        let order_id = format!("order_{}", chrono::Utc::now().timestamp());
        let payment_request_id = format!("req_{}", chrono::Utc::now().timestamp());

        let order = Order {
            order_id: order_id.clone(),
            customer,
            product,
            payment_request_id,
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.orders.insert(order_id.clone(), order.clone());
        Ok(order)
    }

    /// Verify a receipt.
    fn verify_receipt(&self, receipt_id: &str) -> Result<bool, String> {
        // In a real implementation, this would verify the receipt signature
        // and check against the payment method's proof
        println!("Verifying receipt: {}", receipt_id);
        Ok(true)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Paykit E-commerce Merchant Server ===\n");

    // Setup merchant identity
    let merchant_key = PublicKey::from_str("merchant_pubkey_123").unwrap();
    let mut server = MerchantServer::new(merchant_key.clone());

    // Display catalog
    println!("Product Catalog:");
    for product in server.list_products() {
        println!(
            "  {}: {} - {} {} ({})",
            product.id, product.name, product.price, product.currency, product.description
        );
    }
    println!();

    // Simulate customer
    let customer_key = PublicKey::from_str("customer_pubkey_456").unwrap();
    println!("Customer: {:?}", customer_key);

    // Create order
    let order = server.create_order(customer_key.clone(), "prod_1")?;
    println!("Created order: {:?}\n", order);

    // Create payment request
    let payment_request = server.create_payment_request(
        customer_key.clone(),
        "prod_1",
        MethodId("lightning".to_string()),
    )?;
    println!("Payment Request:");
    println!("  ID: {}", payment_request.request_id);
    println!(
        "  Amount: {} {}",
        payment_request.amount, payment_request.currency
    );
    println!("  Method: {}", payment_request.method.0);
    if let Some(ref desc) = payment_request.description {
        println!("  Description: {}", desc);
    }
    println!();

    // Simulate payment response
    let response = PaymentRequestResponse::Accepted {
        request_id: payment_request.request_id.clone(),
        receipt: Box::new(PaykitReceipt::new(
            "receipt_123".to_string(),
            customer_key.clone(),
            merchant_key.clone(),
            MethodId("lightning".to_string()),
            Some("50000".to_string()),
            Some("SAT".to_string()),
            serde_json::json!({}),
        )),
    };
    server.process_payment_response(response)?;

    // Verify receipt (simulated)
    let receipt_id = "receipt_123";
    server.verify_receipt(receipt_id)?;

    println!("\n=== Example Complete ===");
    Ok(())
}

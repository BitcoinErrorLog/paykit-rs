# E-commerce Merchant Server Example

This example demonstrates a complete e-commerce merchant server using Paykit.

## Features

- Product catalog management
- Payment request creation
- Order processing
- Receipt verification

## Usage

```bash
cargo run --example ecommerce
```

## Architecture

The example shows:
1. **Product Catalog**: List of products with prices
2. **Order Management**: Create and track orders
3. **Payment Requests**: Generate payment requests for orders
4. **Receipt Verification**: Verify payment receipts

## Integration Points

- Uses `paykit-subscriptions::PaymentRequest` for payment requests
- Uses `paykit-lib` for endpoint discovery
- Demonstrates receipt verification workflow

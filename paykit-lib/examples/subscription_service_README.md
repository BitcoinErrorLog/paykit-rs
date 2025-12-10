# Subscription Service Example

This example demonstrates a subscription service provider using Paykit.

## Features

- Provider setup and configuration
- Subscriber enrollment
- Auto-pay configuration
- Billing cycle execution
- Subscription modifications (upgrade/downgrade)
- Prorated billing calculations

## Usage

```bash
cargo run --example subscription-service
```

## Architecture

The example shows:
1. **Subscriber Enrollment**: Create new subscriptions
2. **Billing Cycles**: Execute periodic payments
3. **Fallback Handling**: Automatic retry with alternative methods
4. **Modifications**: Upgrade/downgrade subscriptions
5. **Proration**: Calculate prorated amounts for mid-cycle changes

## Integration Points

- Uses `paykit-subscriptions` for subscription management
- Uses `FallbackHandler` for payment retry logic
- Uses `ProrationCalculator` for billing adjustments
- Demonstrates `ModificationRequest` processing

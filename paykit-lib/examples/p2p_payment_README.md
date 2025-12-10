# P2P Payment Example

This example demonstrates a peer-to-peer payment flow using Paykit.

## Features

- Two-party payment negotiation
- Private endpoint exchange
- Receipt exchange
- Interactive protocol usage

## Usage

```bash
cargo run --example p2p-payment
```

## Architecture

The example shows:
1. **Private Endpoint Exchange**: Alice offers a private endpoint to Bob
2. **Payment Initiation**: Bob uses the private endpoint for payment
3. **Receipt Exchange**: Both parties exchange payment receipts

## Integration Points

- Uses `paykit-lib::private_endpoints` for private endpoint management
- Uses `paykit-interactive` for payment negotiation
- Demonstrates secure peer-to-peer communication

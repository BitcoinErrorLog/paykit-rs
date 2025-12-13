# Testnet Setup Guide

This guide explains how to set up a local Bitcoin/Lightning testnet environment for developing and testing Paykit demos.

## Table of Contents

- [Quick Start with Polar](#quick-start-with-polar)
- [Environment Variables](#environment-variables)
- [Manual LND Setup](#manual-lnd-setup)
- [Using Public Testnets](#using-public-testnets)
- [Troubleshooting](#troubleshooting)

## Quick Start with Polar

[Polar](https://lightningpolar.com/) is the recommended tool for local development. It provides a GUI for managing a local Bitcoin and Lightning Network.

### 1. Install Polar

Download from [lightningpolar.com](https://lightningpolar.com/) for your platform.

### 2. Create a Network

1. Open Polar
2. Click "Create Network"
3. Add at least 2 LND nodes (e.g., Alice and Bob)
4. Click "Start"

### 3. Get Connection Details

For each LND node in Polar:

1. Click the node
2. Go to "Connect" tab
3. Copy:
   - **REST Host**: Usually `https://127.0.0.1:8081` (port varies by node)
   - **Admin Macaroon (Hex)**: Click "HEX" under Admin Macaroon

### 4. Configure Paykit

```bash
export PAYKIT_LND_URL=https://127.0.0.1:8081
export PAYKIT_LND_MACAROON=0201036c6e64...  # Your macaroon hex
export PAYKIT_NETWORK=regtest
```

### 5. Fund Channels

In Polar:
1. Right-click on a node → "Actions" → "Deposit"
2. Click "Mine" to confirm
3. Right-click on a channel → "Open Channel"
4. Mine blocks to confirm

### 6. Run Tests

```bash
# Run Paykit demo CLI
cargo run -p paykit-demo-cli --features http-executor -- wallet configure \
  --lnd-url $PAYKIT_LND_URL \
  --macaroon $PAYKIT_LND_MACAROON

# Run integration tests
cargo test -p paykit-lib --features http-executor --test executor_integration
```

## Environment Variables

All Paykit executors can be configured via environment variables:

### LND Configuration

| Variable | Required | Description | Example |
|----------|----------|-------------|---------|
| `PAYKIT_LND_URL` | Yes | LND REST API URL | `https://127.0.0.1:8081` |
| `PAYKIT_LND_MACAROON` | Yes | Admin macaroon (hex) | `0201036c6e6402...` |
| `PAYKIT_LND_TLS_CERT` | No | TLS certificate (PEM) | `-----BEGIN CERT...` |
| `PAYKIT_LND_TIMEOUT` | No | Request timeout (seconds) | `60` |
| `PAYKIT_LND_MAX_FEE_PERCENT` | No | Max fee as % of amount | `1.0` |

### Esplora Configuration

| Variable | Required | Description | Example |
|----------|----------|-------------|---------|
| `PAYKIT_ESPLORA_URL` | No | Esplora API URL | `https://mempool.space/testnet/api` |
| `PAYKIT_ESPLORA_TIMEOUT` | No | Request timeout (seconds) | `30` |

### General

| Variable | Required | Description | Values |
|----------|----------|-------------|--------|
| `PAYKIT_NETWORK` | No | Bitcoin network | `mainnet`, `testnet`, `signet`, `regtest` |

### Example .env file

```bash
# .env.testnet
PAYKIT_LND_URL=https://127.0.0.1:8081
PAYKIT_LND_MACAROON=0201036c6e6402f80165c69030...
PAYKIT_NETWORK=regtest
PAYKIT_ESPLORA_URL=http://localhost:3002/api
```

Load with: `source .env.testnet`

## Manual LND Setup

If you're not using Polar, here's how to set up LND manually:

### 1. Install Bitcoin Core and LND

```bash
# Bitcoin Core
brew install bitcoin  # macOS
# or download from bitcoin.org

# LND
brew install lnd  # macOS
# or download from github.com/lightningnetwork/lnd/releases
```

### 2. Configure Bitcoin Core for Regtest

Create `~/.bitcoin/bitcoin.conf`:

```ini
regtest=1
server=1
rpcuser=bitcoin
rpcpassword=bitcoin
zmqpubrawblock=tcp://127.0.0.1:28332
zmqpubrawtx=tcp://127.0.0.1:28333
```

### 3. Start Bitcoin Core

```bash
bitcoind -regtest -daemon
```

### 4. Configure LND

Create `~/.lnd/lnd.conf`:

```ini
[Bitcoin]
bitcoin.active=true
bitcoin.regtest=true
bitcoin.node=bitcoind

[Bitcoind]
bitcoind.rpcuser=bitcoin
bitcoind.rpcpass=bitcoin
bitcoind.zmqpubrawblock=tcp://127.0.0.1:28332
bitcoind.zmqpubrawtx=tcp://127.0.0.1:28333

[Application Options]
restlisten=0.0.0.0:8080
rpclisten=localhost:10009
```

### 5. Start LND

```bash
lnd --bitcoin.regtest
```

### 6. Initialize Wallet

```bash
lncli --network=regtest create
```

### 7. Get Macaroon

```bash
xxd -p ~/.lnd/data/chain/bitcoin/regtest/admin.macaroon | tr -d '\n'
```

## Using Public Testnets

For testing without local infrastructure, use public testnet APIs:

### Bitcoin Testnet3

```bash
export PAYKIT_NETWORK=testnet
export PAYKIT_ESPLORA_URL=https://blockstream.info/testnet/api
```

Or in Rust:

```rust
use paykit_lib::executors::EsploraExecutor;

let executor = EsploraExecutor::blockstream_testnet()?;
```

### Bitcoin Signet

Signet is a more stable testnet controlled by a small group of signers:

```bash
export PAYKIT_NETWORK=signet
export PAYKIT_ESPLORA_URL=https://mempool.space/signet/api
```

### Mutinynet

[Mutinynet](https://mutinynet.com) is a public signet with Lightning support:

```rust
use paykit_lib::executors::testnet::TestnetConfig;

let config = TestnetConfig::mutinynet();
```

### Get Testnet Coins

- **Testnet3 Faucet**: https://coinfaucet.eu/en/btc-testnet/
- **Signet Faucet**: https://signetfaucet.com/
- **Mutinynet Faucet**: https://faucet.mutinynet.com/

## Troubleshooting

### "Connection refused" errors

1. Check LND is running: `lncli --network=regtest getinfo`
2. Verify the REST port is correct
3. Check firewall settings

### "Permission denied" / Authentication errors

1. Verify macaroon is correct (copy entire hex string)
2. Check macaroon has required permissions
3. Try regenerating macaroon: `lncli bakemacaroon --save_to=test.macaroon`

### TLS Certificate Issues

For self-signed certs (like Polar):

```rust
let config = LndConfig::new(url, macaroon)
    .with_tls_cert(include_str!("path/to/tls.cert"));
```

Or the executor will accept self-signed certs by default when a cert is configured.

### "No route found" payment errors

1. Ensure channels are funded and active
2. Check both sides have inbound/outbound capacity
3. Verify invoice amount doesn't exceed channel capacity
4. Mine blocks to confirm channel: `bitcoin-cli -regtest generatetoaddress 6 <address>`

### Invoice decoding fails

1. Check invoice is valid: `lncli --network=regtest decodepayreq <invoice>`
2. Verify invoice hasn't expired
3. Ensure invoice is for the correct network (regtest vs testnet)

### Rust/Cargo issues

Ensure the `http-executor` feature is enabled:

```bash
cargo build -p paykit-lib --features http-executor
```

## Running the Full Test Suite

With a Polar network running:

```bash
# Set environment variables (see above)
source .env.testnet

# Run mock tests (no network needed)
cargo test -p paykit-lib --features http-executor --test executor_integration

# Run real testnet tests
cargo test -p paykit-lib --features http-executor --test executor_integration -- --ignored

# Run demo CLI
cargo run -p paykit-demo-cli -- wallet status
```

## Further Reading

- [LND Documentation](https://docs.lightning.engineering/)
- [Bitcoin Core RPC Reference](https://developer.bitcoin.org/reference/rpc/)
- [Polar Documentation](https://docs.lightning.engineering/lapps/guides/polar-lapps)
- [Esplora API Reference](https://github.com/Blockstream/esplora/blob/master/API.md)


# Paykit Library

Stateless Rust crate that implements the **public Paykit directory** surface.

This crate does **not** implement receipts, Locks, or Atomicity credit flows. It only covers the public supported payment methods layer and a minimal social helper, so that apps like Bitkit, Pubky App, and Paykit demos can:

- Publish per-method **payment endpoints** for a key
- Discover a payee's **supported payment methods**
- Resolve a single **endpoint** for a method
- Discover **known contacts** from Pubky follows

Everything is transport-agnostic behind traits, with optional adapters for the Pubky SDK.

## Auth & Dependency Injection

- Writes require an authenticated client. Instead of hard-coding `PubkySession`, public APIs accept an argument that implements a thin Paykit-defined trait (e.g., `AuthenticatedTransport`).  
- The crate provides adapters so callers can wrap [`pubky::PubkySession`](https://docs.rs/pubky/0.6.0-rc.6/pubky/struct.PubkySession.html) or provide mocks for tests.  
- Public reads only require the `UnauthenticatedTransportRead` trait, keeping unauthenticated flows lightweight. Session lifecycle, capability scoping, and key rotation stay outside this crate.
- The `pubky` feature flag (enabled by default) wires in Pubky adapters under `transport::pubky`. Disable it if you want to use custom transports only.

## Concepts

### PublicKey

`PublicKey` identifies the payee whose payment methods you are discovering.

- When the `pubky` feature is enabled (default), this is re-exported from the `pubky` crate.
- With the feature disabled, it is a simple `pub struct PublicKey(pub String)` to allow custom transports and tests.

### MethodId

`MethodId` is a string identifier for a payment method.

Examples (not enforced by the crate):

- `ln-btc`
- `onchain-btc`
- `cashapp`
- `paypal`
- `bank-sepa`

The string format is left to the higher-level Paykit spec and app conventions. The library just treats it as an opaque key.

### EndpointData

`EndpointData` is a string payload that encodes whatever the method requires:

- Lightning: lnurl, payreq, or a Paykit-specific descriptor
- On-chain: address or descriptor
- Fiat: payment link or account reference

The crate does not impose a format. It only stores and retrieves opaque strings.

### SupportedPayments

```rust
pub struct SupportedPayments {
    pub entries: HashMap<MethodId, EndpointData>,
}
```

Represents the full supported payment methods list for a given key.

## Public API

The crate exposes the following async helpers over trait-based transports:

```rust
pub async fn set_payment_endpoint<S>(
    client: &S,
    method: MethodId,
    data: EndpointData,
) -> Result<()>
where
    S: AuthenticatedTransport;
```

Store or update a payee-owned endpoint for a given method using an authenticated client, such as a Pubky session.

```rust
pub async fn remove_payment_endpoint<S>(
    client: &S,
    method: MethodId,
) -> Result<()>
where
    S: AuthenticatedTransport;
```

Remove a previously published endpoint for a given method.

```rust
pub async fn get_payment_list<R>(
    reader: &R,
    payee: &PublicKey,
) -> Result<SupportedPayments>
where
    R: UnauthenticatedTransportRead;
```

Fetch the list of supported payment methods for `payee`. Returns an empty `SupportedPayments` when no endpoints are published.

```rust
pub async fn get_payment_endpoint<R>(
    reader: &R,
    payee: &PublicKey,
    method: &MethodId,
) -> Result<Option<EndpointData>>
where
    R: UnauthenticatedTransportRead;
```

Convenience resolver for a single method. Returns `Ok(None)` if the endpoint is missing or empty.

```rust
pub async fn get_known_contacts<R>(
    reader: &R,
    key: &PublicKey,
) -> Result<Vec<PublicKey>>
where
    R: UnauthenticatedTransportRead;
```

Helper for discovering known contacts by listing Pubky follows for `key`. Returns an empty vector when no follows are stored.

Method/endpoint naming follows the PMIP consensus described in the repository root `README.md`. Each API returns well-typed structures (enums/structs) that mirror the protocol specification so downstream clients can share the same serialization layer.

When the `pubky` feature is enabled the crate exports:

- `transport::pubky::PAYKIT_PATH_PREFIX` (`/pub/paykit.app/v0/`) and `PUBKY_FOLLOWS_PATH` (`/pub/pubky.app/follows/`) to standardize path construction.  
- `PubkyAuthenticatedTransport` (wraps `PubkySession`) and `PubkyUnauthenticatedTransport` (wraps `pubky::PublicStorage`) as ready-to-use adapters that satisfy the traits above.

## Transport abstractions

Transports live in `paykit-lib/src/transport`.

### Traits

```rust
pub trait AuthenticatedTransport {
    async fn upsert_payment_endpoint(
        &self,
        method: MethodId,
        data: EndpointData,
    ) -> Result<()>;

    async fn remove_payment_endpoint(
        &self,
        method: MethodId,
    ) -> Result<()>;
}

pub trait UnauthenticatedTransportRead {
    async fn fetch_supported_payments(
        &self,
        payee: &PublicKey,
    ) -> Result<SupportedPayments>;

    async fn fetch_payment_endpoint(
        &self,
        payee: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<EndpointData>>;

    async fn fetch_known_contacts(
        &self,
        key: &PublicKey,
    ) -> Result<Vec<PublicKey>>;
}
```

Callers are expected to provide their own implementations or use the provided Pubky adapters.

### Pubky adapters

When the `pubky` feature is enabled, the crate provides:

- `transport::pubky::PAYKIT_PATH_PREFIX`
  - `"/pub/paykit.app/v0/"`
  - Public endpoints live under:
    - `pubky<user>/pub/paykit.app/v0/{methodId}`

- `transport::pubky::PUBKY_FOLLOWS_PATH`
  - `"/pub/pubky.app/follows/"`
  - Used for the `get_known_contacts` helper.

- `PubkyAuthenticatedTransport`
  - Wraps `pubky::PubkySession` to implement `AuthenticatedTransport`.

- `PubkyUnauthenticatedTransport`
  - Wraps `pubky::PublicStorage` to implement `UnauthenticatedTransportRead` and public readonly operations.

This matches the Pubky SDK addressing scheme, where:

- Session storage uses absolute paths such as `"/pub/app/file.txt"`.
- Public storage uses addresses such as `pubky<user>/pub/app/file.txt`.

## Non-goals

This crate does not:

- Define or serialize Paykit receipts or envelopes.
- Implement Locks such as paywalls, subscriptions, or puzzles.
- Implement Atomicity credit issuance, routing, or settlements.
- Decide any business rules about which method to prefer or how to select between methods.

It is deliberately narrow: a reusable, transport-agnostic public directory API for supported payment methods plus a minimal social helper.

## Example with Pubky

```rust
use paykit_lib::{
    get_payment_list,
    set_payment_endpoint,
    MethodId,
    EndpointData,
    transport::pubky::{PubkyAuthenticatedTransport, PubkyUnauthenticatedTransport},
};
use pubky::{PubkySession, PublicStorage, PublicKey};

async fn example(session: &PubkySession, public_storage: &PublicStorage) -> anyhow::Result<()> {
    // Wrap Pubky types in Paykit transports
    let auth = PubkyAuthenticatedTransport::new(session.clone());
    let reader = PubkyUnauthenticatedTransport::new(public_storage.clone());

    // Publish a Lightning endpoint
    let method = MethodId::new("ln-btc");
    let endpoint = EndpointData::new("lnurl1... or paykit descriptor");
    set_payment_endpoint(&auth, method.clone(), endpoint).await?;

    // Fetch supported methods for this key
    let my_key: PublicKey = session.public_key().clone();
    let supported = get_payment_list(&reader, &my_key).await?;

    if let Some(ep) = supported.entries.get(&method) {
        println!("My LNBTC endpoint: {}", ep.as_str());
    }

    Ok(())
}
```

## Related Components

This crate is the foundation for other Paykit components:

- **[paykit-interactive](../paykit-interactive/README.md)** - Uses transport traits for interactive payment protocol
- **[paykit-subscriptions](../paykit-subscriptions/README.md)** - May use directory features for subscription management
- **[paykit-demo-core](../paykit-demo-core/)** - Shared demo logic that uses this library
- **[paykit-demo-cli](../paykit-demo-cli/README.md)** - CLI demo using this library
- **[paykit-demo-web](../paykit-demo-web/README.md)** - Web demo using this library

## Testing

### Unit Tests

Run unit tests (no network required):

```bash
cargo test -p paykit-lib
```

### Integration Tests

Integration tests require network access to connect to the Pubky testnet. They are gated behind the `integration-tests` feature and marked with `#[ignore]` by default.

To run integration tests:

```bash
cargo test -p paykit-lib --features integration-tests -- --ignored
```

**Note**: These tests may fail if network is unavailable or the Pubky testnet is unreachable.

## Payment Executors

The crate also includes payment executor implementations for actually sending payments via Lightning or on-chain Bitcoin.

### Feature Flags

Enable the `http-executor` feature for real HTTP requests:

```toml
[dependencies]
paykit-lib = { version = "1.0", features = ["http-executor"] }
```

### Lightning Payments (LND)

```rust
use paykit_lib::executors::{LndConfig, LndExecutor};
use paykit_lib::methods::LightningExecutor;

// Configure LND connection
let config = LndConfig::new("https://localhost:8080", "your_macaroon_hex")
    .with_timeout(60);
let executor = LndExecutor::new(config)?;

// Decode an invoice
let decoded = executor.decode_invoice("lnbc...").await?;
println!("Amount: {:?} msat", decoded.amount_msat);

// Pay an invoice
let result = executor.pay_invoice("lnbc...", None, None).await?;
println!("Preimage: {}", result.preimage);

// Estimate fees
let fee_msat = executor.estimate_fee("lnbc...").await?;
```

### On-chain Bitcoin (Esplora)

```rust
use paykit_lib::executors::EsploraExecutor;

// Use a preset configuration for testnet
let executor = EsploraExecutor::blockstream_testnet()?;

// Get fee estimates
let fees = executor.get_fee_estimates().await?;
println!("Next block: {} sat/vB", fees.get_rate_for_blocks(1));

// Check address balance
let info = executor.get_address_info("tb1q...").await?;
println!("Balance: {} sats", info.confirmed_balance());

// Broadcast a signed transaction
let txid = executor.broadcast_tx("0200000001...").await?;
```

### Testnet Configuration

Use the `testnet` module for easy development setup:

```rust
use paykit_lib::executors::testnet::{TestnetConfig, get_lnd_config_from_env};

// Preset for Polar regtest environment
let config = TestnetConfig::polar_alice("your_macaroon_hex");

// Or load from environment variables
// PAYKIT_LND_URL, PAYKIT_LND_MACAROON, PAYKIT_NETWORK
if let Some(lnd_config) = get_lnd_config_from_env() {
    let executor = LndExecutor::new(lnd_config)?;
}
```

See [docs/TESTNET_SETUP.md](../docs/TESTNET_SETUP.md) for detailed setup instructions.

## Status

- Public directory API and Pubky adapters in place.
- Integration tests using `pubky-testnet::EphemeralTestnet` (opt-in, requires network).
- LND and Esplora executors with `http-executor` feature.
- Testnet configuration helpers for development.

## Documentation

- [Build Instructions](BUILD.md)
- [Testnet Setup Guide](../docs/TESTNET_SETUP.md)
- [Repository Root README](../README.md)

# Migration Guide

This guide covers breaking changes and migration steps when upgrading Paykit dependencies.

## Upgrading from Pubky SDK 0.5.x to 0.6.0-rc.6

### Overview

Pubky SDK 0.6.0-rc.6 introduces significant API changes. This guide covers all breaking changes and how to update your code.

### Breaking Changes

#### 1. Keypair Generation

**Before (0.5.x):**
```rust
let keypair = pubky::generate_keypair();
```

**After (0.6.0-rc.6):**
```rust
let keypair = pubky::Keypair::random();
```

#### 2. PublicStorage Initialization

**Before (0.5.x):**
```rust
let storage = pubky::PublicStorage::new(&homeserver_url)?;
```

**After (0.6.0-rc.6):**
```rust
let storage = pubky::PublicStorage::new()?;
// Homeserver is now configured at the SDK level
```

#### 3. PubkyClient Removed

The `PubkyClient` type has been removed. Use the `Pubky` SDK directly:

**Before (0.5.x):**
```rust
let client = pubky::PubkyClient::new(homeserver_url)?;
let session = client.login(&keypair).await?;
```

**After (0.6.0-rc.6):**
```rust
use pubky_testnet::EphemeralTestnet;

let testnet = EphemeralTestnet::start().await?;
let sdk = testnet.sdk()?;
let signer = sdk.signer(keypair);
let session = signer.signup(&homeserver.public_key(), None).await?;
```

#### 4. Accessing Public Key from Session

**Before (0.5.x):**
```rust
let pubkey = session.public_key();
```

**After (0.6.0-rc.6):**
```rust
// Keep a reference to the keypair
let pubkey = keypair.public_key();
```

#### 5. Testnet Module

**Before (0.5.x):**
```rust
use pubky::PubkyTestnet;
```

**After (0.6.0-rc.6):**
```rust
use pubky_testnet::EphemeralTestnet;

// Add to Cargo.toml:
// [dev-dependencies]
// pubky-testnet = "0.6.0-rc.6"
```

### Migration Steps

#### Step 1: Update Cargo.toml

```toml
[dependencies]
pubky = "0.6.0-rc.6"

[dev-dependencies]
pubky-testnet = "0.6.0-rc.6"
```

#### Step 2: Update Imports

```rust
// Remove:
// use pubky::{PubkyClient, PubkyTestnet, generate_keypair};

// Add:
use pubky::{Keypair, PublicStorage, PubkySession};

// For tests:
use pubky_testnet::EphemeralTestnet;
```

#### Step 3: Update Keypair Generation

Search and replace all instances:

```bash
# Find all usages
grep -r "generate_keypair" --include="*.rs"

# Replace with
# pubky::generate_keypair() -> pubky::Keypair::random()
```

#### Step 4: Update PublicStorage

```rust
// Old
let storage = PublicStorage::new(&url)?;

// New
let storage = PublicStorage::new()?;
```

#### Step 5: Update Session Creation

For production code, consult Pubky SDK documentation for the latest session creation pattern.

For test code:

```rust
async fn setup_test() -> Result<(PubkySession, PublicKey)> {
    let testnet = EphemeralTestnet::start().await?;
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk()?;
    
    let keypair = Keypair::random();
    let signer = sdk.signer(keypair.clone());
    let session = signer.signup(&homeserver.public_key(), None).await?;
    
    Ok((session, keypair.public_key()))
}
```

#### Step 6: Update Public Key Access

If you were getting the public key from the session, you now need to keep a reference to the keypair:

```rust
// Old
async fn do_something(session: &PubkySession) {
    let pubkey = session.public_key();
}

// New
async fn do_something(session: &PubkySession, keypair: &Keypair) {
    let pubkey = keypair.public_key();
}
```

### Common Errors and Solutions

#### Error: `unresolved import pubky::PubkyClient`

**Solution:** Remove the import. Use `Pubky` SDK directly or `EphemeralTestnet` for tests.

#### Error: `cannot find function generate_keypair in crate pubky`

**Solution:** Replace `pubky::generate_keypair()` with `pubky::Keypair::random()`.

#### Error: `this function takes 0 arguments but 1 argument was supplied` for `PublicStorage::new()`

**Solution:** Remove the homeserver URL argument: `PublicStorage::new()`.

#### Error: `no method named public_key found for reference &PubkySession`

**Solution:** Keep a reference to the `Keypair` and call `keypair.public_key()` instead.

### Testing After Migration

Run the full test suite to verify migration:

```bash
# Run all tests
cargo test --all

# Run integration tests specifically
cargo test --test pubky_sdk_compliance

# Check for clippy warnings
cargo clippy --all-targets
```

### Rollback Plan

If you encounter issues, you can temporarily pin to the old version:

```toml
[dependencies]
pubky = "=0.5.x"  # Pin to specific old version
```

However, this is not recommended for production as 0.5.x will not receive updates.

---

## Upgrading Pubky-Noise 0.9.x to 1.0.0

### Breaking Changes

#### 1. Feature Renaming

**Before:**
```toml
pubky-noise = { version = "0.9", features = ["pubky"] }
```

**After:**
```toml
pubky-noise = { version = "1.0", features = ["pubky-sdk"] }
```

#### 2. Constructor Renaming

**Before:**
```rust
let client = NoiseClient::new("kid", device_id, ring);
let server = NoiseServer::new("kid", device_id, ring);
```

**After:**
```rust
let client = NoiseClient::new_direct("kid", device_id, ring);
let server = NoiseServer::new_direct("kid", device_id, ring);
```

### Migration Steps

1. Update feature flag in Cargo.toml
2. Search and replace constructor calls
3. Run tests to verify

---

## Need Help?

If you encounter issues not covered in this guide:

1. Check the [COMPATIBILITY.md](COMPATIBILITY.md) for version requirements
2. Review the [INTEGRATION_AUDIT_REPORT.md](INTEGRATION_AUDIT_REPORT.md) for known issues
3. Open an issue on GitHub with:
   - Your Cargo.toml dependencies
   - The error message
   - Steps to reproduce


# Dependency Compatibility Matrix

This document tracks version compatibility between Paykit and its key dependencies.

## Current Compatibility

| Paykit Version | Pubky SDK | Pubky-Noise | Rust Edition | Status |
|----------------|-----------|-------------|--------------|--------|
| 0.2.x          | 0.6.0-rc.6| 1.0.0       | 2021         | Current |
| 0.1.x          | 0.5.x     | 0.9.x       | 2021         | Legacy  |

## Component Version Requirements

### paykit-lib

```toml
[dependencies]
pubky = "0.6.0-rc.6"
```

### paykit-interactive

```toml
[dependencies]
pubky-noise = { version = "1.0.0", features = ["pubky-sdk"] }
```

### paykit-demo-core

```toml
[dependencies]
pubky = "0.6.0-rc.6"
pubky-noise = { features = ["pubky-sdk"] }
```

## Breaking Changes

### Pubky SDK 0.5.x → 0.6.0-rc.6

| Old API | New API | Notes |
|---------|---------|-------|
| `pubky::generate_keypair()` | `pubky::Keypair::random()` | Method renamed |
| `PublicStorage::new(&url)` | `PublicStorage::new()` | No longer takes homeserver URL |
| `PubkyClient` | Removed | Use `Pubky` SDK directly |
| `session.public_key()` | `keypair.public_key()` | Public key accessed via keypair |
| `PubkyTestnet` | `pubky_testnet::EphemeralTestnet` | Moved to separate crate |

### Pubky-Noise 0.9.x → 1.0.0

| Old API | New API | Notes |
|---------|---------|-------|
| `NoiseClient::new()` | `NoiseClient::new_direct()` | Renamed for clarity |
| `NoiseServer::new()` | `NoiseServer::new_direct()` | Renamed for clarity |
| Feature `pubky` | Feature `pubky-sdk` | Feature renamed |

## Feature Flags

### paykit-lib

| Feature | Description | Default |
|---------|-------------|---------|
| `pubky` | Enable Pubky SDK integration | ✅ Yes |
| `test-utils` | Enable test utilities | ❌ No |

### pubky-noise

| Feature | Description | Default |
|---------|-------------|---------|
| `pubky-sdk` | Enable Pubky SDK integration | ❌ No |
| `storage-queue` | Enable storage-backed messaging | ❌ No |
| `secure-mem` | Enable secure memory handling | ❌ No |
| `trace` | Enable tracing support | ❌ No |

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x86_64 | ✅ Full | Primary development platform |
| macOS ARM64 | ✅ Full | Tested on Apple Silicon |
| macOS x86_64 | ✅ Full | Intel Macs |
| Windows x86_64 | ⚠️ Partial | Credential Manager integration |
| iOS | ✅ Full | Via UniFFI bindings |
| Android | ✅ Full | Via UniFFI bindings |
| WebAssembly | ⚠️ Partial | Limited async support |

## Minimum Supported Rust Version (MSRV)

- **Paykit**: Rust 1.70+
- **Pubky-Noise**: Rust 1.70+
- **Pubky SDK**: Rust 1.70+

## Testing Dependencies

For integration testing, you need:

```toml
[dev-dependencies]
pubky-testnet = "0.6.0-rc.6"
tokio = { version = "1", features = ["full"] }
```

## Known Issues

1. **Pubky SDK API Stability**: The Pubky SDK 0.6.x is still in release candidate stage. API may change in patch releases.

2. **Storage Queue Feature**: Requires `tokio` runtime for async sleep operations.

3. **WASM Limitations**: Some async operations may not work in WASM environments due to browser limitations.

## Upgrade Checklist

When upgrading dependencies:

- [ ] Check this compatibility matrix
- [ ] Read MIGRATION.md for breaking changes
- [ ] Run full test suite: `cargo test --all`
- [ ] Run clippy: `cargo clippy --all-targets`
- [ ] Update documentation if needed


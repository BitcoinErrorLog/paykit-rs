# Changelog

All notable changes to the Paykit project are documented in this file.

## [Unreleased]

### Bitkit Executor FFI Integration

Added comprehensive support for integrating external wallet implementations (like Bitkit iOS/Android) through UniFFI callback interfaces.

#### New Features

- **Executor FFI Bindings** (`paykit-mobile/src/executor_ffi.rs`)
  - `BitcoinExecutorFFI` interface for on-chain wallet operations
  - `LightningExecutorFFI` interface for Lightning node operations
  - FFI result types: `BitcoinTxResultFFI`, `LightningPaymentResultFFI`, `DecodedInvoiceFFI`
  - Network configuration enums: `BitcoinNetworkFFI`, `LightningNetworkFFI`
  - Executor bridges for Rust trait adaptation

- **PaykitClient Extensions** (`paykit-mobile/src/lib.rs`)
  - `new_with_network()` constructor for testnet/regtest support
  - `register_bitcoin_executor()` and `register_lightning_executor()` methods
  - `execute_payment()` for real payment execution
  - `generate_payment_proof()` for proof generation
  - Network accessor methods

- **Example Implementations**
  - `swift/BitkitExecutorExample.swift` - Complete Swift example
  - `kotlin/BitkitExecutorExample.kt` - Complete Kotlin example

- **Documentation**
  - `BITKIT_INTEGRATION_GUIDE.md` - Step-by-step integration guide
  - `API_REFERENCE.md` - Complete API reference
  - `CHANGELOG.md` - Mobile-specific changelog

- **Integration Tests** (`tests/executor_integration.rs`)
  - 30 comprehensive tests covering all integration scenarios
  - Network configuration, executor registration, payment execution
  - Proof generation, error handling, thread safety

#### Test Results
- 151 tests passing (121 unit + 30 integration)
- All builds passing
- Clippy clean

---

## [1.0.1] - 2025-12-12

### Production Audit Remediation

Comprehensive production readiness improvements based on security audit.

#### Critical Fixes
- **paykit-lib**: Fixed `Box<PaykitReceipt>` type mismatch in ecommerce example

#### Security Improvements
- **paykit-subscriptions**: Changed `Amount::percentage()` to accept `Decimal` instead of `f64` for exact financial arithmetic
- **paykit-subscriptions**: Added `percentage_f64()` convenience method with precision warning
- **paykit-mobile**: Added comprehensive `block_on()` documentation for FFI safety
- **paykit-subscriptions**: Added RFC 8032 Ed25519 test vectors (3 official vectors)

#### Rate Limiting
- **paykit-interactive**: Added optional global rate limit (`global_max_attempts`)
- **paykit-interactive**: Added `RateLimitConfig::with_global_limit()` constructor
- **paykit-interactive**: Added `RateLimitConfig::strict_with_global()` preset
- **paykit-interactive**: Added `global_count()` for monitoring

#### Documentation
- **docs/SECURITY_HARDENING.md**: Comprehensive security hardening guide
- **docs/DEMO_VS_PRODUCTION.md**: Demo vs production code boundaries
- **docs/CONCURRENCY.md**: Lock poisoning policy and thread safety
- **paykit-subscriptions/docs/NONCE_CLEANUP_GUIDE.md**: Nonce cleanup automation
- **paykit-interactive/examples/rate_limited_server.rs**: Rate limiter integration example
- Updated `docs/README.md` with new documentation links

#### Testing
- **paykit-demo-cli/tests/smoke_test.rs**: Basic CLI smoke tests
- Added unit tests for core types (MethodId, EndpointData, Amount)

#### Code Quality
- Fixed unused imports in `integration_noise.rs`
- Fixed unused variables in `e2e_payment_flows.rs`
- Removed unused `Duration` import

## [1.0.0] - 2025-12-11

### Production Readiness Release

This release marks the first stable production-ready version of paykit-lib.

### Added

#### Error Handling
- **PaykitError**: Comprehensive error enum with 21 variants
- **PaykitErrorCode**: Numeric codes for FFI compatibility
- `is_retryable()` and `retry_after_ms()` helpers
- Specific errors: `InsufficientFunds`, `InvoiceExpired`, `PaymentRejected`

#### Secure Storage
- **SecureKeyStorage Trait**: Platform-agnostic secure key storage
- **InMemoryKeyStorage**: Testing implementation
- Platform stubs: iOS Keychain, Android Keystore, WebCrypto, Desktop
- FFI bridge signatures and integration examples

#### Payment Executors
- **LndExecutor**: Lightning payments via LND REST API
- **EsploraExecutor**: On-chain verification via Esplora API
- **BitcoinNetwork**: Mainnet, Testnet, Signet, Regtest support
- Configuration structs: `LndConfig`, `EsploraConfig`, `ElectrumConfig`

#### Testing Infrastructure
- **TestNetwork**: Simulated payment network for E2E testing
- **TestWallet**: Mock wallet with LightningExecutor and BitcoinExecutor
- Test fixtures: addresses, amounts, keypairs, invoices
- Assertion helpers and PaymentAssertionBuilder

#### API Improvements
- **Prelude Module**: Convenient imports via `use paykit_lib::prelude::*`
- **MethodId**: `new()`, `as_str()`, `onchain()`, `lightning()` helpers
- **EndpointData**: `new()`, `as_str()`, `is_empty()`, `len()` helpers
- Well-known constants: `MethodId::ONCHAIN`, `MethodId::LIGHTNING`

#### Documentation
- **Integration Guide**: Complete usage documentation
- **Production Deployment Guide**: Configuration, security, monitoring

### Changed
- Improved fee estimate tie-breaking (prefer lower block counts)

## [0.9.0] - 2024

### Core Components

#### paykit-lib
- Transport trait abstractions for authenticated and unauthenticated operations
- Pubky homeserver integration
- Public directory operations for payment method discovery
- Support for multiple payment methods (onchain, lightning, custom)

#### paykit-interactive
- Interactive payment protocol using Noise encryption
- Receipt negotiation and exchange
- Private endpoint sharing
- Payment coordination over encrypted channels

#### paykit-subscriptions
- Subscription agreements with cryptographic signatures
- Payment requests with expiration and metadata
- Auto-pay rules with configurable spending limits
- Thread-safe nonce tracking and spending limit enforcement
- Safe arithmetic with overflow protection

### Demo Applications

#### paykit-demo-cli
- Complete command-line interface for all Paykit features
- Identity management (Ed25519/X25519 keypairs)
- Payment method publishing and discovery
- Contact management
- Subscription management
- Auto-pay configuration
- Receipt viewing and tracking
- Rich terminal UI with colors and QR codes
- Comprehensive documentation

#### paykit-demo-web
- Full WebAssembly browser application
- Identity management with localStorage persistence
- Interactive dashboard with real-time statistics
- Contact management
- Payment method configuration
- Subscription and auto-pay management
- Receipt tracking and filtering
- Complete API reference documentation
- ~103 comprehensive tests

#### paykit-demo-core
- Shared business logic for demo applications
- Identity management abstractions
- Directory client wrapper
- Payment coordinator
- File-based storage
- Contact management utilities

### Security
- Ed25519 signatures for all cryptographic operations
- SHA-256 hashing for message integrity
- Replay protection with unique nonces
- Domain separation constants for signature security
- Thread-safe operations for concurrent access

### Documentation
- Component-specific READMEs for all crates
- API reference documentation
- Architecture guides
- Testing guides
- Deployment guides
- Build instructions

### Testing
- Comprehensive test suites for all components
- Integration tests for payment flows
- Property-based testing for subscriptions
- Edge case coverage
- Cross-feature integration tests

## Development History

### Phase 0: Protocol Testing
- Integration tests for Noise protocol handshakes
- Pubky SDK compliance tests
- Fixed pubky-noise compilation issues

### Phase 1: Core Library
- Identity management with Ed25519/X25519
- Directory operations wrapper
- Payment flow coordination
- File-based storage
- Data models

### Phase 2: CLI Demo
- Complete command structure
- All core commands implemented
- Rich terminal UI with colors and QR codes
- Contact management
- Comprehensive documentation

### Phase 3: Subscriptions
- Subscription agreement implementation
- Payment request system
- Auto-pay automation
- Spending limits

### Phase 4: Web Demo
- WebAssembly compilation
- Browser-based UI
- localStorage persistence
- Interactive dashboard
- Complete feature parity with CLI

### Phase 5: Documentation & Polish
- Comprehensive documentation cleanup
- Cross-component consistency
- API reference completion
- Testing documentation

---

For detailed component-specific changelogs, see:
- [paykit-demo-web/CHANGELOG.md](paykit-demo-web/CHANGELOG.md)


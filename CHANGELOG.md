# Changelog

All notable changes to the Paykit project are documented in this file.

## [Unreleased]

## [2.0.0] - 2025-12-04

### Added
- **Cold Key Pattern Support** for hardware wallet / Bitkit integration
  - New `NoiseRawClientHelper` with `connect_ik_raw`, `connect_anonymous`, `connect_ephemeral`, `connect_xx` methods
  - New `NoiseServerHelper` pattern-aware accept methods: `accept_ik_raw`, `accept_n`, `accept_nn`, `accept_xx`
  - New `AcceptedConnection` enum for pattern-specific connection handling
  - New `NoisePattern` enum: `IK`, `IKRaw`, `N`, `NN`, `XX`
- **XX Pattern (Trust-On-First-Use)**
  - Full 3-message XX handshake implementation
  - Returns server's static key for caching (upgrade to IK on subsequent connections)
  - E2E tests for TOFU payment scenarios
- **NN Post-Handshake Attestation with Full Verification**
  - `attestation` module in `paykit-demo-core` for Ed25519 identity binding
  - `create_attestation()` and `verify_attestation()` functions using both ephemeral keys
  - `PaykitNoiseMessage::Attestation` variant for over-the-wire attestation
  - CLI `pay` and `receive` commands enforce NN attestation automatically
  - Updated `connect_ephemeral()` returns 3-tuple `(channel, server_ephemeral, client_ephemeral)`
- **Pattern Negotiation Unification**
  - `NoiseClientHelper::connect_to_recipient_with_negotiation()` for IK with pattern byte
  - All patterns now supported by pattern-aware server
- **CLI Pattern Selection**
  - `--pattern` flag for `receive` command (ik, ik-raw, n, nn, xx)
  - `--pattern` flag for `pay` command
  - `--connect` flag for direct connections bypassing discovery
- **Demo Scripts**
  - `03-cold-key-payment.sh` - Demonstrates IK-raw pattern for cold key scenarios
  - `04-anonymous-payment.sh` - Demonstrates N pattern for donation boxes
- **Documentation**
  - `docs/PATTERN_SELECTION.md` - Comprehensive pattern selection guide
  - `docs/NOISE_PATTERN_NEGOTIATION.md` - Wire protocol documentation
  - Updated `paykit-demo-cli/demos/README.md` with pattern reference
- Cold key integration tests (`tests/cold_key_integration.rs`)
- Pattern E2E tests (`tests/pattern_e2e.rs`)

### Changed
- **BREAKING**: Upgraded to pubky-noise v0.8.0
  - Removed `epoch` parameter from all Noise API calls
  - Removed `()` phantom type parameter from `NoiseClient<R>` and `NoiseServer<R>`
  - Updated `derive_x25519_for_device_epoch()` to `derive_x25519_static()`
  - Updated return types: `client_start_ik_direct()` now returns 2-tuple `(hs, msg)`
  - Updated return types: `server_accept_ik()` now returns 2-tuple `(hs, response)`
- Fixed pubky-noise dependency paths (now points to `../../pubky-noise`)
- Updated `.gitignore` to match pubky-noise patterns
- Added `zeroize` and `sha2` dependencies to `paykit-demo-core`

### Documentation
- Comprehensive documentation cleanup and consolidation
- Component relationship documentation
- Cross-component consistency improvements

## [1.0.0] - 2024

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


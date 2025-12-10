# Changelog

All notable changes to the Paykit project are documented in this file.

## [Unreleased]

### Added
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


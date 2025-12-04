# Paykit Documentation Index

This directory contains comprehensive guides for building, deploying, and integrating Paykit.

## Integration Guides

- **[BITKIT_INTEGRATION.md](BITKIT_INTEGRATION.md)** - Complete guide for integrating Paykit into Bitkit
  - React Native bridge examples (iOS Swift + Android Kotlin)
  - Cold key architecture for hardware wallets
  - One-time setup flow with Ed25519/X25519 derivation
  - Runtime payment flows
  - pkarr-based key discovery
  - Pattern selection for Bitkit use cases

- **[PATTERN_SELECTION.md](PATTERN_SELECTION.md)** - Noise pattern selection guide
  - When to use each pattern (IK, IK-raw, N, NN, XX)
  - Security comparison table
  - Best practices and recommendations
  - Code examples for each pattern
  - Pattern selection flowchart

- **[NOISE_PATTERN_NEGOTIATION.md](NOISE_PATTERN_NEGOTIATION.md)** - Pattern negotiation protocol
  - Wire format specification
  - Pattern byte mapping (0x00-0x04)
  - Client and server implementation
  - Security considerations

## Deployment Guides

- **[PRODUCTION_DEPLOYMENT.md](PRODUCTION_DEPLOYMENT.md)** - Production deployment checklist
  - Security configuration
  - Environment setup
  - Monitoring and logging
  - Incident response

## Related Documentation

### pubky-noise Library
- [pubky-noise README](../../pubky-noise/README.md) - Noise library overview
- [Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md) - pkarr-based cold key design
- [Mobile Integration](../../pubky-noise/docs/MOBILE_INTEGRATION.md) - Mobile app integration guide
- [FFI Guide](../../pubky-noise/docs/FFI_GUIDE.md) - UniFFI bindings for iOS/Android

### Component Documentation
- [paykit-lib README](../paykit-lib/README.md) - Core library API
- [paykit-interactive README](../paykit-interactive/README.md) - Interactive payment protocol
- [paykit-subscriptions README](../paykit-subscriptions/README.md) - Subscription management
- [paykit-demo-cli README](../paykit-demo-cli/README.md) - CLI demo user guide
- [paykit-demo-web README](../paykit-demo-web/README.md) - Web demo user guide

### Project Documentation
- [Main README](../README.md) - Project overview
- [BUILD.md](../BUILD.md) - Build and development setup
- [CHANGELOG.md](../CHANGELOG.md) - Version history
- [SECURITY.md](../SECURITY.md) - Security considerations
- [DEPLOYMENT.md](../DEPLOYMENT.md) - Deployment instructions

## Recommended Reading Order

### For New Developers
1. [Main README](../README.md) - Start here for project overview
2. [BUILD.md](../BUILD.md) - Set up your development environment
3. [PATTERN_SELECTION.md](PATTERN_SELECTION.md) - Understand Noise patterns
4. Component READMEs - Dive into specific crates

### For Bitkit Integration
1. [BITKIT_INTEGRATION.md](BITKIT_INTEGRATION.md) - Primary integration guide
2. [Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md) - Understand cold key design
3. [PATTERN_SELECTION.md](PATTERN_SELECTION.md) - Choose the right patterns
4. [Mobile Integration](../../pubky-noise/docs/MOBILE_INTEGRATION.md) - Mobile-specific considerations

### For Security Review
1. [SECURITY.md](../SECURITY.md) - Security best practices
2. [pubky-noise Threat Model](../../pubky-noise/THREAT_MODEL.md) - Threat analysis
3. [pubky-noise Audit Report](../../pubky-noise/PUBKY_NOISE_AUDIT_REPORT.md) - Audit findings
4. [PATTERN_SELECTION.md](PATTERN_SELECTION.md) - Pattern security comparison

## Quick Links

- **Demo Scripts**: [paykit-demo-cli/demos/](../paykit-demo-cli/demos/README.md)
- **API Reference**: [paykit-demo-web/API_REFERENCE.md](../paykit-demo-web/API_REFERENCE.md)
- **Architecture**: [paykit-demo-web/ARCHITECTURE.md](../paykit-demo-web/ARCHITECTURE.md)
- **Testing**: [paykit-demo-cli/TESTING.md](../paykit-demo-cli/TESTING.md)

---

**Last Updated**: December 2025 (Paykit v2.0.0 + pubky-noise v0.8.0)


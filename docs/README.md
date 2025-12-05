# Paykit Documentation

Complete documentation for the Paykit payment protocol and its integration with Pubky and Noise.

## Table of Contents

### Getting Started
- [Main README](../README.md) - Project overview and quick start
- [Build Instructions](../BUILD.md) - How to build all components
- [Deployment Guide](../DEPLOYMENT.md) - Deployment considerations

### Architecture & Design
- [Pattern Selection Guide](PATTERN_SELECTION.md) - Choose the right Noise pattern for your use case
- [Noise Pattern Negotiation](NOISE_PATTERN_NEGOTIATION.md) - Wire protocol for pattern negotiation
- [Key Caching Strategy](KEY_CACHING_STRATEGY.md) - How to cache and rotate Noise keys
- [Key Rotation](KEY_ROTATION.md) - Strategies for rotating X25519 keys
- [Threat Model](THREAT_MODEL.md) - Security analysis and threat vectors

### Integration Guides
- [Bitkit Integration](BITKIT_INTEGRATION.md) - Mobile wallet integration with cold keys
- [Production Deployment](PRODUCTION_DEPLOYMENT.md) - Production deployment checklist
- [Production Checklist](PRODUCTION_CHECKLIST.md) - Pre-launch verification

### Security
- [Security Policy](../SECURITY.md) - Vulnerability reporting and security practices
- [Threat Model](THREAT_MODEL.md) - Comprehensive security analysis

### Development
- [Changelog](../CHANGELOG.md) - Version history and release notes
- [Release Process](../RELEASING.md) - How to cut a new release
- [Agents Guide](../AGENTS.md) - AI agent conventions for this codebase

### External References
- [pubky-noise](../../pubky-noise/README.md) - Noise protocol implementation
- [pubky-noise Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md) - Detailed cold key design
- [pubky-noise Mobile Integration](../../pubky-noise/docs/MOBILE_INTEGRATION.md) - Mobile-specific considerations

## Quick Navigation

### By Use Case

**Building a payment application:**
1. Start with [Main README](../README.md)
2. Review [Pattern Selection Guide](PATTERN_SELECTION.md)
3. Check [Production Checklist](PRODUCTION_CHECKLIST.md)

**Integrating with Bitkit/mobile:**
1. Read [Bitkit Integration](BITKIT_INTEGRATION.md)
2. Review [pubky-noise Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md)
3. Run [cold_key_workflow example](../paykit-demo-core/examples/README.md)

**Understanding security:**
1. Read [Threat Model](THREAT_MODEL.md)
2. Review [Security Policy](../SECURITY.md)
3. Check [Production Deployment](PRODUCTION_DEPLOYMENT.md)

### By Role

**Developers:**
- [Build Instructions](../BUILD.md)
- [Pattern Selection Guide](PATTERN_SELECTION.md)
- [Examples](../paykit-demo-core/examples/README.md)

**Security Reviewers:**
- [Threat Model](THREAT_MODEL.md)
- [Security Policy](../SECURITY.md)
- [Key Rotation](KEY_ROTATION.md)

**Operators:**
- [Production Deployment](PRODUCTION_DEPLOYMENT.md)
- [Production Checklist](PRODUCTION_CHECKLIST.md)
- [Deployment Guide](../DEPLOYMENT.md)

## Documentation Conventions

### Code Examples
All code examples in documentation are tested against the current API. If you find outdated examples, please open an issue.

### Pattern References
When referring to Noise patterns, we use:
- **IK** - Interactive Key (standard authenticated)
- **IK-raw** - Interactive Key without handshake signing (cold key scenario)
- **N** - Anonymous client, authenticated server
- **NN** - Fully anonymous (requires post-handshake attestation)
- **XX** - Trust-on-first-use

### Security Notes
Documents marked with 🔒 contain security-critical information that must be understood before production deployment.

## Contributing to Documentation

When updating docs:
1. Keep examples current with the API
2. Test all code snippets
3. Use clear, concise language
4. Link to related documentation
5. Mark security-critical sections clearly

## Archived Documentation

Historical documentation has been moved to [`archive/`](../archive/README.md). These docs are preserved for reference but may be outdated.

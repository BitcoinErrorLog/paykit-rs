# Paykit Documentation

This directory contains additional documentation for the Paykit project.

## Documentation Structure

### Root Documentation
- **[README.md](../README.md)** - Main project overview and quick start
- **[CHANGELOG.md](../CHANGELOG.md)** - Project changelog and version history
- **[SECURITY.md](../SECURITY.md)** - Security considerations and best practices
- **[DEPLOYMENT.md](../DEPLOYMENT.md)** - Deployment instructions
- **[RELEASING.md](../RELEASING.md)** - Release process documentation
- **[BUILD.md](../BUILD.md)** - Build and development setup guide

### Architecture & Design
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - System architecture and component relationships

### Production & Security
- **[PRODUCTION_DEPLOYMENT.md](./PRODUCTION_DEPLOYMENT.md)** - Production deployment guide
- **[SECURITY_HARDENING.md](./SECURITY_HARDENING.md)** - Security hardening recommendations
- **[DEMO_VS_PRODUCTION.md](./DEMO_VS_PRODUCTION.md)** - Demo vs production code boundaries
- **[CONCURRENCY.md](./CONCURRENCY.md)** - Lock poisoning policy and thread safety

### Feature Guides
- **[autopay-guide.md](./autopay-guide.md)** - Auto-pay rules and spending limits
- **[mobile-integration.md](./mobile-integration.md)** - iOS and Android integration guide
- **[bip-compliance.md](./bip-compliance.md)** - BIP-0000 specification compliance

### Component Documentation

#### Core Libraries
- **[paykit-lib/README.md](../paykit-lib/README.md)** - Core library API reference
- **[paykit-interactive/README.md](../paykit-interactive/README.md)** - Interactive payment protocol
- **[paykit-subscriptions/README.md](../paykit-subscriptions/README.md)** - Subscription management
- **[paykit-subscriptions/docs/NONCE_CLEANUP_GUIDE.md](../paykit-subscriptions/docs/NONCE_CLEANUP_GUIDE.md)** - Nonce cleanup automation

#### Demo Applications
- **[paykit-demo-core/README.md](../paykit-demo-core/README.md)** - Shared demo logic
- **[paykit-demo-cli/README.md](../paykit-demo-cli/README.md)** - CLI demo user guide
- **[paykit-demo-web/README.md](../paykit-demo-web/README.md)** - Web demo user guide
- **[paykit-mobile/README.md](../paykit-mobile/README.md)** - Mobile FFI bindings

#### Mobile Demo Apps
- **[paykit-mobile/ios-demo/README.md](../paykit-mobile/ios-demo/README.md)** - iOS demo app (SwiftUI)
- **[paykit-mobile/android-demo/README.md](../paykit-mobile/android-demo/README.md)** - Android demo app (Jetpack Compose)

### Build Documentation
- **[BUILD.md](../BUILD.md)** - Workspace build guide
- **[paykit-lib/BUILD.md](../paykit-lib/BUILD.md)** - Core library build
- **[paykit-subscriptions/BUILD.md](../paykit-subscriptions/BUILD.md)** - Subscriptions build
- **[paykit-demo-core/BUILD.md](../paykit-demo-core/BUILD.md)** - Demo core build
- **[paykit-demo-cli/BUILD.md](../paykit-demo-cli/BUILD.md)** - CLI demo build
- **[paykit-demo-web/BUILD_INSTRUCTIONS.md](../paykit-demo-web/BUILD_INSTRUCTIONS.md)** - Web demo WASM build

### Web Demo Feature Documentation
- **[paykit-demo-web/API_REFERENCE.md](../paykit-demo-web/API_REFERENCE.md)** - WASM API reference
- **[paykit-demo-web/ARCHITECTURE.md](../paykit-demo-web/ARCHITECTURE.md)** - Web demo architecture
- **[paykit-demo-web/DASHBOARD.md](../paykit-demo-web/DASHBOARD.md)** - Dashboard feature guide
- **[paykit-demo-web/CONTACTS_FEATURE.md](../paykit-demo-web/CONTACTS_FEATURE.md)** - Contacts feature guide
- **[paykit-demo-web/PAYMENT_METHODS.md](../paykit-demo-web/PAYMENT_METHODS.md)** - Payment methods guide
- **[paykit-demo-web/PAYMENTS.md](../paykit-demo-web/PAYMENTS.md)** - Payments feature guide
- **[paykit-demo-web/RECEIPTS.md](../paykit-demo-web/RECEIPTS.md)** - Receipts feature guide
- **[paykit-demo-web/SUBSCRIPTION_IMPLEMENTATION.md](../paykit-demo-web/SUBSCRIPTION_IMPLEMENTATION.md)** - Subscriptions guide
- **[paykit-demo-web/TESTING.md](../paykit-demo-web/TESTING.md)** - Testing guide
- **[paykit-demo-web/DEPLOYMENT.md](../paykit-demo-web/DEPLOYMENT.md)** - Deployment guide

### CLI Demo Documentation
- **[paykit-demo-cli/QUICKSTART.md](../paykit-demo-cli/QUICKSTART.md)** - Quick start guide
- **[paykit-demo-cli/TESTING.md](../paykit-demo-cli/TESTING.md)** - Testing guide
- **[paykit-demo-cli/TROUBLESHOOTING.md](../paykit-demo-cli/TROUBLESHOOTING.md)** - Troubleshooting guide

## Archive

Historical development artifacts are preserved in:
- **[archive/status-reports/](./archive/status-reports/)** - Status and completion reports
- **[archive/implementation-reports/](./archive/implementation-reports/)** - Phase completion reports
- **[archive/audit-reports/](./archive/audit-reports/)** - Audit documents

## Contributing Documentation

When adding new documentation:

1. **Component READMEs**: Place in the component directory (e.g., `paykit-lib/README.md`)
2. **Feature Documentation**: Place in the relevant demo directory (e.g., `paykit-demo-web/FEATURE.md`)
3. **Architecture Docs**: Place in `docs/` directory
4. **Build Docs**: Use `BUILD.md` naming convention
5. **Update this index**: Add links to new documentation

## Documentation Standards

- Use Markdown format
- Include code examples where applicable
- Cross-reference related components
- Keep examples current and working
- Update when APIs change
- Follow the structure outlined in component READMEs

## Quick Links

- [Getting Started](../README.md#quick-start)
- [Architecture Overview](./ARCHITECTURE.md)
- [Component Relationships](./ARCHITECTURE.md#component-dependencies)
- [Auto-Pay Guide](./autopay-guide.md)
- [Mobile Integration](./mobile-integration.md)
- [Security Best Practices](../SECURITY.md)
- [Build Instructions](../BUILD.md)


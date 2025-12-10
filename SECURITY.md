# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| 0.2.x   | :x:                |
| < 0.2   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to: [INSERT SECURITY EMAIL]

You should receive a response within 48 hours. If for some reason you do not, please follow up via email to ensure we received your original message.

Please include the following information:

- Type of issue (e.g. buffer overflow, signature bypass, etc.)
- Full paths of source file(s) related to the issue
- The location of the affected source code (tag/branch/commit or direct URL)
- Any special configuration required to reproduce the issue
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

This information will help us triage your report more quickly.

## Disclosure Policy

When we receive a security bug report, we will:

1. Confirm the problem and determine the affected versions
2. Audit code to find any similar problems
3. Prepare fixes for all supported releases
4. Release patches as soon as possible

## Security Best Practices

### For Users

1. **Keep Dependencies Updated**: Regularly run `cargo update` and `cargo audit`
2. **Verify Signatures**: Always verify Ed25519 signatures before accepting subscription data
3. **Use Strong Nonces**: Generate cryptographically secure random nonces for each signature
4. **Rotate Keys**: Rotate signing keys periodically
5. **Monitor Spending Limits**: Regularly check spending limits aren't being exceeded

### For Developers

1. **No Unsafe Code**: Avoid `unsafe` blocks unless absolutely necessary and thoroughly audited
2. **Use Checked Arithmetic**: Always use `checked_add`, `checked_sub`, etc. for Amount operations
3. **Validate All Inputs**: Sanitize and validate all external inputs
4. **Constant-Time Operations**: Use constant-time comparisons for cryptographic operations
5. **Zeroize Secrets**: Use `zeroize` crate to clear sensitive data from memory

## Cryptographic Implementation

### Algorithms Used

- **Signature**: Ed25519 (RFC 8032)
- **Key Exchange**: X25519
- **Encryption**: ChaCha20-Poly1305
- **Hashing**: BLAKE2s, SHA-256
- **Key Derivation**: HKDF

### Security Considerations

1. **Nonce Reuse**: NEVER reuse nonces - this completely breaks signature security
2. **Replay Attacks**: Always check nonces against the NonceStore
3. **Timestamp Validation**: Verify signature timestamps are within acceptable range
4. **Domain Separation**: Use `PAYKIT_SUBSCRIPTION_V2` domain constant
5. **Deterministic Serialization**: Use `postcard` for canonical serialization

## Component-Specific Security Considerations

### paykit-lib
- **Transport Security**: All network operations should use TLS/HTTPS
- **Key Validation**: Always validate public keys before use
- **Path Traversal**: Ensure payment method IDs don't allow path traversal attacks

### paykit-interactive
- **Noise Protocol**: Use Noise_IK handshake pattern correctly
- **Key Rotation**: Rotate keys periodically to prevent long-term compromise
- **Session Management**: Properly manage Noise channel sessions
- **Replay Protection**: Verify message nonces and timestamps

### paykit-subscriptions
- **Nonce Management**: Never reuse nonces - use cryptographically secure random generation
- **NonceStore Cleanup**: Call `cleanup_expired()` periodically to prevent memory growth
- **Spending Limits**: File locks may not work on network filesystems (NFS, SMB) - use local storage
- **Signature Verification**: Always verify Ed25519 signatures before processing
- **Timestamp Validation**: Check signature timestamps are within acceptable range

### paykit-demo-core
- **Key Storage**: ⚠️ Demo code stores keys in plaintext JSON files - NOT for production
- **Secure Storage**: Production implementations should use OS keychain, HSMs, or secure enclaves
- **Error Handling**: Don't leak sensitive information in error messages

### paykit-demo-cli
- **File Permissions**: Ensure identity files have restrictive permissions (600)
- **Key Storage**: ⚠️ Keys stored in `~/.paykit/` as plaintext - demo only
- **Input Validation**: Validate all user inputs before processing

### paykit-demo-web
- **localStorage Security**: ⚠️ Data stored in browser localStorage is NOT encrypted
- **XSS Protection**: Sanitize all user inputs to prevent XSS attacks
- **CSP Headers**: Use Content Security Policy headers
- **WASM Security**: WebAssembly code is sandboxed but still requires careful validation
- **Key Storage**: ⚠️ Private keys in localStorage are accessible to any JavaScript on the page
- **HTTPS Required**: Always serve over HTTPS in production
- **No Encryption at Rest**: Demo code doesn't encrypt data at rest - NOT for production

## Known Security Considerations

### 1. NonceStore Memory Growth

The NonceStore accumulates nonces over time. Call `cleanup_expired()` periodically to prevent unbounded memory growth.

```rust
// Recommended: Run hourly
nonce_store.cleanup_expired(chrono::Utc::now().timestamp())?;
```

### 2. File Lock Limitations

Spending limits use file-level locks which may not work correctly on network filesystems (NFS, SMB). Use local storage for production deployments.

### 3. Pubky Noise Handshake

Interactive payments use the Noise_IK handshake pattern. Ensure proper key rotation and session management to prevent long-term key compromise.

### 4. Demo Application Limitations

**All demo applications (CLI and Web) are for development and testing only:**

- Private keys stored in plaintext
- No encryption at rest
- No OS-level secure storage
- Simplified error handling
- No rate limiting
- No DDoS protection

**For production use**, implement:
- Secure key storage (HSMs, secure enclaves, OS keychain)
- Encryption at rest
- Proper authentication and authorization
- Rate limiting and DDoS protection
- Comprehensive audit logging
- Input validation and sanitization

## Security Audit History

Historical audit reports are archived in [docs/archive/audit-reports/](docs/archive/audit-reports/):

- **2025-11**: Internal audit completed. Grade: A (Strong)
- See [PAYKIT_SECURITY_AUDIT_REPORT.md](docs/archive/audit-reports/PAYKIT_SECURITY_AUDIT_REPORT.md) for details
- **Next audit**: Scheduled for 2026-Q1

## Security Updates

Security updates will be released with:

- Detailed changelog
- CVE identifier (if applicable)
- Affected versions
- Mitigation steps
- Credit to reporter (if desired)

## Bug Bounty

[To be determined - consider setting up a bug bounty program]

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [Noise Protocol Framework](https://noiseprotocol.org/)
- [RFC 8032 - Ed25519](https://tools.ietf.org/html/rfc8032)

## Contact

- Security Email: [INSERT SECURITY EMAIL]
- GPG Key: [INSERT GPG KEY FINGERPRINT]

Last Updated: November 21, 2025


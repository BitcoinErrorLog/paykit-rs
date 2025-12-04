# Production Deployment Checklist

Use this checklist before deploying pubky-noise + paykit-rs to production.

## Pre-Deployment Verification

### Security Configuration

- [ ] **Secret keys stored securely**
  - iOS: Keychain with `.whenUnlocked` accessibility
  - Android: EncryptedSharedPreferences or Keystore
  - Server: Environment variables or HSM
  - Never in code, logs, or version control

- [ ] **pkarr timestamp verification enabled**
  - `NoiseKeyConfig::max_age_seconds` set to 30 days
  - `discover_noise_key_with_config()` used instead of unverified discovery
  - Timestamp freshness checked on every pkarr lookup

- [ ] **Pattern selection appropriate for use case**
  - IK for hot key scenarios (homeserver sessions)
  - IK-raw for cold key scenarios (hardware wallets, Bitkit)
  - NN with attestation for anonymous but authenticated scenarios
  - N only for one-way donations (client cannot receive responses)
  - XX only for first contact, then upgrade to IK/IK-raw

- [ ] **Weak key rejection enabled**
  - pubky-noise v0.8.0+ automatically rejects weak keys
  - Verify by testing with all-zero peer public key (should fail)

- [ ] **Session state persistence**
  - `NoiseManager` state saved before app suspension
  - Counters persisted for `StorageBackedMessaging`
  - Session IDs tracked for reconnection

### Code Quality

- [ ] **All tests passing**
  ```bash
  cd pubky-noise && cargo test --all-features
  cd paykit-rs && cargo test --all --all-features
  ```

- [ ] **No clippy warnings**
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

- [ ] **Code formatted**
  ```bash
  cargo fmt --all -- --check
  ```

- [ ] **Documentation builds**
  ```bash
  cargo doc --no-deps --all-features
  ```

### Dependency Audit

- [ ] **No known vulnerabilities**
  ```bash
  cargo audit
  ```

- [ ] **Dependencies up to date**
  - `snow = "0.10"` (latest)
  - `ed25519-dalek = "2"` (latest)
  - `x25519-dalek = "2"` (latest)
  - `pubky = "0.6.0-rc.6"` (latest RC)

- [ ] **Feature flags minimal**
  - Only enable features actually needed
  - Disable `trace` in production builds
  - Enable `secure-mem` for server deployments

### Network Configuration

- [ ] **TLS for all socket connections**
  - WebSocket connections use `wss://` not `ws://`
  - TCP connections wrapped in TLS
  - Certificate validation enabled

- [ ] **Connection timeouts configured**
  - Handshake timeout: 30 seconds
  - Message timeout: 60 seconds
  - Reconnection backoff: exponential

- [ ] **Rate limiting implemented**
  - Connection attempts: 10/minute per IP
  - Message size limits: 16MB
  - Handshake message limit: 4KB

### Mobile-Specific (iOS/Android)

- [ ] **FFI builds successful**
  ```bash
  cd pubky-noise
  ./build-ios.sh
  ./build-android.sh
  ```

- [ ] **UniFFI bindings generated**
  - Swift bindings in `target/uniffi/`
  - Kotlin bindings in `target/uniffi/`
  - No missing exports

- [ ] **Platform-specific key storage**
  - iOS: SecureEnclave for Ed25519 (if available)
  - Android: StrongBox/TEE for Ed25519 (if available)
  - Fallback to Keychain/Keystore

- [ ] **Background task handling**
  - Sessions saved before app backgrounding
  - Reconnection logic for app resume
  - Battery-efficient retry intervals

- [ ] **Memory management**
  - `Zeroizing` used for all secret keys
  - Keys cleared after use
  - No caching of secrets in UI state

## Deployment Configuration

### Environment Variables

```bash
# Bitkit Example
PUBKY_HOMESERVER_URL=https://pubky.example.com
NOISE_MAX_KEY_AGE_SECONDS=2592000  # 30 days
NOISE_DEFAULT_PATTERN=ik-raw
ENABLE_PATTERN_NEGOTIATION=true
MAX_RECONNECT_ATTEMPTS=3
```

### Logging Configuration

- [ ] **Sensitive data not logged**
  - No secret keys in logs
  - No full peer identities in logs
  - Truncate public keys to first 8 bytes for logs

- [ ] **Structured logging enabled**
  ```rust
  tracing::info!(
      session_id = %session_id,
      pattern = %pattern,
      "Noise handshake complete"
  );
  ```

- [ ] **Log levels appropriate**
  - Production: INFO or WARN
  - Development: DEBUG or TRACE
  - Never: Secret keys at any level

### Monitoring

- [ ] **Metrics tracked**
  - Handshake success/failure rates
  - Connection latency (p50, p95, p99)
  - Pattern usage distribution
  - pkarr lookup success rate
  - Key rotation events

- [ ] **Alerts configured**
  - Handshake failure rate > 5%
  - pkarr lookup failures > 10%
  - Connection latency > 1 second
  - Unexpected pattern usage

## Testing in Production-Like Environment

### Integration Testing

- [ ] **End-to-end payment flow**
  - Identity creation
  - Key derivation and publication
  - pkarr discovery
  - Noise handshake (all patterns)
  - Payment request/response
  - Receipt exchange

- [ ] **Network resilience**
  - Connection interruption recovery
  - Timeout handling
  - Retry logic verification
  - Background/foreground transitions (mobile)

- [ ] **Error handling**
  - Invalid peer keys
  - Expired pkarr records
  - Network failures
  - Decrypt failures (corrupted data)

### Load Testing (Server-Side)

- [ ] **Concurrent connections**
  - Verify 100+ simultaneous sessions
  - Memory usage stays constant
  - No mutex deadlocks

- [ ] **Long-running sessions**
  - 24+ hour sessions stable
  - Counter overflow handling (u64)
  - Memory leak detection

## Post-Deployment Monitoring

### First 24 Hours

- [ ] Monitor handshake success rates
- [ ] Check for unexpected errors
- [ ] Verify pattern distribution matches expectations
- [ ] Confirm key caching working (IK-raw usage increasing)

### First Week

- [ ] Review all error logs
- [ ] Analyze performance metrics
- [ ] User feedback collection
- [ ] Connection success rate > 95%

### Ongoing

- [ ] Weekly: Review security logs
- [ ] Monthly: Dependency updates
- [ ] Quarterly: X25519 key rotation
- [ ] Annually: Security audit

## Rollback Plan

If critical issues are discovered:

1. **Identify the issue**
   - Check error logs
   - Review metrics
   - Isolate affected pattern or feature

2. **Immediate mitigation**
   - Disable affected pattern (feature flag)
   - Fallback to known-good pattern (IK)
   - Notify users of degraded functionality

3. **Deploy fix or rollback**
   - Hot-patch if possible
   - Full rollback if necessary
   - Communicate timeline to users

## Incident Response

### Key Compromise

If X25519 key is compromised:
1. Immediately derive new key
2. Publish revocation to pkarr (`revoked=true`)
3. Publish new key to pkarr
4. Notify affected peers
5. Audit recent sessions
6. Update incident log

### Service Outage

If pkarr/pubky storage unavailable:
1. Use cached keys (if available)
2. Allow connections with extended timeout
3. Queue pkarr updates for retry
4. Notify users of reduced functionality

## Sign-Off

Before going to production, ensure sign-off from:

- [ ] **Security Team** - Cryptographic review complete
- [ ] **DevOps Team** - Infrastructure ready
- [ ] **QA Team** - All tests passing
- [ ] **Product Team** - Features complete
- [ ] **Legal Team** - Compliance verified (if applicable)

## Production Readiness Criteria

### Must Have
- ✅ All critical security improvements implemented
- ✅ pkarr timestamp verification enabled
- ✅ Pattern warnings documented
- ✅ All tests passing
- ✅ FFI builds working (for mobile)
- ✅ Documentation current

### Should Have
- ✅ Key caching implemented
- ✅ Key rotation procedure documented
- ✅ Monitoring dashboards
- ✅ Incident response plan
- ✅ Rollback procedure tested

### Nice to Have
- HSM integration for Ed25519 keys
- Automated key rotation
- Advanced monitoring (distributed tracing)
- A/B testing for pattern selection

---

**Status**: Ready for production deployment once all checkboxes are ✅

**Last Updated**: December 2025


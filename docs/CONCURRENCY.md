# Concurrency and Lock Poisoning Policy

This document describes the project's approach to handling lock poisoning and concurrent access patterns.

## Lock Types Used

| Type | Location | Purpose |
|------|----------|---------|
| `Mutex` | Rate limiter | IP tracking for DoS protection |
| `RwLock` | Nonce store | Replay attack prevention |
| `RwLock` | Private endpoint storage | Endpoint caching |
| `Arc<Mutex>` | Rotation manager | Endpoint rotation callbacks |

## Lock Poisoning Strategies

Rust's `Mutex` and `RwLock` become "poisoned" when a thread panics while holding the lock. This project uses two strategies depending on the security context.

### Strategy 1: Fail-Open (Availability Priority)

Used when blocking requests would be worse than potentially incorrect state.

**Implementation Pattern:**
```rust
fn check_rate_limit(&self, ip: IpAddr) -> bool {
    let records = match self.records.lock() {
        Ok(r) => r,
        Err(_) => return true, // Fail open - allow the request
    };
    // ... check rate limit
}
```

**Used In:**
- `paykit-interactive/src/rate_limit.rs` - `HandshakeRateLimiter`
  - Rationale: Blocking legitimate users is worse than potentially allowing a few extra requests during edge cases

**Behavior:**
- On lock poisoning, returns a "safe default" that allows the operation
- Logs should record the poisoning event for monitoring
- Service remains available to legitimate users

### Strategy 2: Fail-Closed (Security Priority)

Used when incorrect behavior could compromise security.

**Implementation Pattern:**
```rust
fn check_nonce(&self, nonce: &[u8; 32]) -> Result<bool> {
    let nonces = self.used_nonces.write()
        .map_err(|e| Error::Other(format!("Lock poisoned: {}", e)))?;
    // ... check and mark nonce
    Ok(true)
}
```

**Used In:**
- `paykit-subscriptions/src/nonce_store.rs` - `NonceStore`
  - Rationale: Allowing a replayed signature is a security violation; better to reject the request

**Behavior:**
- On lock poisoning, returns an error
- Caller should handle the error appropriately (retry, fail request, etc.)
- Security properties are maintained even during failures

### Strategy 3: Panic (Critical Internal State)

Used only where continued operation is impossible and the lock should never be contested.

**Implementation Pattern:**
```rust
fn get_tracker(&self, method: &MethodId) -> Option<&RotationTracker> {
    let callbacks = self.callbacks.read()
        .expect("RotationManager lock should never be poisoned");
    // ...
}
```

**Used In:**
- Internal state that should never fail
- Test utilities and development helpers

**Behavior:**
- Panics immediately on lock poisoning
- Only used where poisoning indicates a bug that needs investigation
- Should be accompanied by logging/alerting in production

## Deadlock Prevention

### Rules

1. **No nested locks**: Never acquire a lock while holding another lock
2. **Short critical sections**: Release locks as quickly as possible
3. **No I/O under lock**: Never perform network or disk operations while holding a lock

### Lock Ordering

When multiple locks must be acquired (rare), follow this order:

1. Rate limiter locks
2. Nonce store locks
3. Storage locks

This order is hypothetical - currently no code path requires multiple locks.

## Thread Safety Markers

All lock-protected types implement or require:
- `Send` - Can be transferred between threads
- `Sync` - Can be shared between threads via `&T`

### FFI Considerations

When crossing FFI boundaries (Swift/Kotlin):
- Locks must be held by Rust code only
- FFI callbacks should not attempt to acquire locks
- Use `Arc<T>` for shared ownership across FFI boundaries

## Testing Concurrent Access

### Nonce Store Concurrent Test

```rust
#[test]
fn test_concurrent_nonce_checks() {
    let store = Arc::new(NonceStore::new());
    let nonce = [42u8; 32];
    
    let mut handles = vec![];
    for _ in 0..10 {
        let store_clone = store.clone();
        handles.push(thread::spawn(move || {
            store_clone.check_and_mark(&nonce, expires_at).unwrap()
        }));
    }
    
    let successes = handles.into_iter()
        .filter_map(|h| h.join().ok())
        .filter(|&r| r)
        .count();
    
    assert_eq!(successes, 1, "Only one concurrent attempt should succeed");
}
```

### Rate Limiter Thread Safety

The rate limiter uses `Mutex` which guarantees mutual exclusion. Tests verify:
- Multiple IPs can be tracked concurrently
- Same IP from different threads is properly rate-limited
- Lock contention doesn't cause deadlocks

## Monitoring Recommendations

### Metrics to Track

1. **Lock contention time**: How long threads wait for locks
2. **Poisoning events**: Any lock poisoning should be alerted on
3. **Nonce store size**: Growing too large indicates cleanup issues

### Alerting

| Event | Severity | Action |
|-------|----------|--------|
| Lock poisoning (any) | Critical | Investigate immediately |
| Lock contention > 100ms | Warning | Review load patterns |
| Nonce store > 100k entries | Warning | Verify cleanup is running |

## Implementation Checklist

When adding new lock-protected state:

- [ ] Choose fail-open or fail-closed based on security requirements
- [ ] Document the strategy in code comments
- [ ] Add thread safety test
- [ ] Keep critical sections minimal
- [ ] Consider whether RwLock (many readers) or Mutex (exclusive) is appropriate
- [ ] Verify no nested lock acquisition


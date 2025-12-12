# Nonce Cleanup Guide

This guide explains how to properly manage nonce cleanup in production deployments to prevent memory exhaustion while maintaining replay attack protection.

## Overview

The `NonceStore` tracks used nonces to prevent signature replay attacks. Each nonce is stored with its expiration timestamp. Without periodic cleanup, the store will grow unbounded as new signatures are processed.

## Why Cleanup Matters

1. **Memory Management**: Each tracked nonce uses ~40 bytes. At 1000 requests/hour, that's ~1MB/day without cleanup.
2. **Performance**: Lookup performance degrades as the HashMap grows unbounded.
3. **Security**: Expired nonces no longer need protection - they can't be replayed after expiration anyway.

## Cleanup Strategies

### Strategy 1: Periodic Cleanup (Recommended for Servers)

Call `cleanup_expired()` on a regular interval:

```rust
use paykit_subscriptions::NonceStore;
use std::sync::Arc;
use tokio::time::{interval, Duration};

async fn run_nonce_cleanup(store: Arc<NonceStore>) {
    let mut cleanup_interval = interval(Duration::from_secs(3600)); // Every hour
    
    loop {
        cleanup_interval.tick().await;
        let now = chrono::Utc::now().timestamp();
        
        if let Err(e) = store.cleanup_expired(now) {
            tracing::warn!("Nonce cleanup failed: {}", e);
        } else {
            tracing::debug!("Nonce cleanup completed, {} nonces tracked", store.count());
        }
    }
}
```

### Strategy 2: Lazy Cleanup (Resource-Constrained Environments)

Perform cleanup when the store exceeds a threshold:

```rust
use paykit_subscriptions::NonceStore;

const MAX_NONCES: usize = 10_000;

fn check_and_mark_with_cleanup(
    store: &NonceStore,
    nonce: &[u8; 32],
    expires_at: i64,
) -> Result<bool, Error> {
    // Clean up if we're getting too large
    if store.count() > MAX_NONCES {
        let now = chrono::Utc::now().timestamp();
        store.cleanup_expired(now)?;
    }
    
    store.check_and_mark(nonce, expires_at)
}
```

### Strategy 3: Request-Based Cleanup (Low-Volume Services)

Clean up every N requests:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
const CLEANUP_INTERVAL: u64 = 1000;

fn maybe_cleanup(store: &NonceStore) {
    let count = REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    
    if count % CLEANUP_INTERVAL == 0 {
        let now = chrono::Utc::now().timestamp();
        let _ = store.cleanup_expired(now);
    }
}
```

## Integration Patterns

### With Axum/Tower Middleware

```rust
use axum::{middleware::Next, response::Response, http::Request};
use std::sync::Arc;
use paykit_subscriptions::NonceStore;

pub async fn nonce_cleanup_middleware<B>(
    State(store): State<Arc<NonceStore>>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    // Cleanup on every 100th request (lightweight check)
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    if COUNTER.fetch_add(1, Ordering::Relaxed) % 100 == 0 {
        let now = chrono::Utc::now().timestamp();
        let _ = store.cleanup_expired(now);
    }
    
    next.run(request).await
}
```

### With Background Task Manager

```rust
use tokio::task::JoinHandle;
use std::sync::Arc;
use paykit_subscriptions::NonceStore;

pub struct NonceManager {
    store: Arc<NonceStore>,
    cleanup_task: Option<JoinHandle<()>>,
}

impl NonceManager {
    pub fn new() -> Self {
        let store = Arc::new(NonceStore::new());
        Self {
            store,
            cleanup_task: None,
        }
    }
    
    pub fn start_cleanup(&mut self, interval_secs: u64) {
        let store = self.store.clone();
        
        self.cleanup_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(interval_secs)
            );
            
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp();
                let _ = store.cleanup_expired(now);
            }
        }));
    }
    
    pub fn store(&self) -> &Arc<NonceStore> {
        &self.store
    }
    
    pub async fn shutdown(self) {
        if let Some(task) = self.cleanup_task {
            task.abort();
            let _ = task.await;
        }
    }
}
```

## Recommended Settings

| Environment | Cleanup Interval | Max Nonces | Strategy |
|-------------|------------------|------------|----------|
| High-traffic server | 5-15 minutes | 100,000 | Periodic |
| Standard server | 1 hour | 50,000 | Periodic |
| Mobile/embedded | 1 hour or on-demand | 10,000 | Lazy |
| Development | Manual | 1,000 | Lazy |

## Monitoring

Track these metrics for production systems:

```rust
// Example with tracing/metrics
fn report_nonce_metrics(store: &NonceStore) {
    let count = store.count();
    tracing::info!(nonce_count = count, "NonceStore status");
    
    // If using metrics crate:
    // metrics::gauge!("paykit.nonce_store.count", count as f64);
}
```

## Troubleshooting

### Memory Growing Despite Cleanup

1. **Check signature lifetimes**: If signatures have very long expiration times (days/weeks), nonces accumulate.
2. **Verify cleanup is running**: Add logging to confirm `cleanup_expired()` is being called.
3. **Check clock sync**: Ensure server time is synchronized (NTP) - clock skew affects expiration.

### Replay Attacks After Cleanup

This should NOT happen if implemented correctly:
- Cleanup only removes nonces whose signatures have expired
- An expired signature cannot be replayed anyway (expiration is checked first)
- If you see replay attacks, check that signature verification includes expiration check

## Security Considerations

1. **Never skip nonce checking**: Even under memory pressure, rejecting requests is safer than allowing replays.
2. **Fail-closed on errors**: If cleanup fails, log the error but continue enforcing nonce checks.
3. **Monitor for anomalies**: Sudden spikes in nonce count may indicate an attack.


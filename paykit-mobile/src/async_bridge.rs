//! Async Bridge for Mobile Platforms
//!
//! This module provides callback-based async patterns for mobile platforms
//! that may not natively support Rust futures.
//!
//! # Overview
//!
//! Mobile platforms have different async models:
//! - iOS: Uses completion handlers and async/await (Swift 5.5+)
//! - Android: Uses callbacks, coroutines, or RxJava
//!
//! This module bridges Rust's async/await to callback-style APIs.
//!
//! # Example (Callback Style)
//!
//! ```ignore
//! // From Swift/Kotlin
//! client.fetchEndpointsAsync(pubkey) { result in
//!     switch result {
//!     case .success(let endpoints):
//!         // Handle endpoints
//!     case .failure(let error):
//!         // Handle error
//!     }
//! }
//! ```

use std::sync::Arc;
use tokio::sync::oneshot;

use crate::transport_ffi::{AuthenticatedTransportFFI, UnauthenticatedTransportFFI};
use crate::{PaykitMobileError, PaymentMethod};

/// Result callback interface for mobile.
///
/// This trait is implemented by mobile code to receive async results.
pub trait ResultCallback<T>: Send + Sync {
    fn on_success(&self, value: T);
    fn on_error(&self, error: String);
}

/// Async operation handle.
///
/// Can be used to cancel pending operations.
pub struct AsyncHandle {
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl AsyncHandle {
    /// Create a new handle with a cancellation sender.
    pub fn new(cancel_tx: oneshot::Sender<()>) -> Self {
        Self {
            cancel_tx: Some(cancel_tx),
        }
    }

    /// Cancel the operation.
    pub fn cancel(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }
    }

    /// Check if the operation was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_tx.is_none()
    }
}

/// Async runtime wrapper for mobile.
///
/// Manages the Tokio runtime and provides async execution utilities.
pub struct AsyncRuntime {
    runtime: tokio::runtime::Runtime,
}

impl AsyncRuntime {
    /// Create a new async runtime.
    pub fn new() -> Result<Self, String> {
        tokio::runtime::Runtime::new()
            .map(|runtime| Self { runtime })
            .map_err(|e| format!("Failed to create runtime: {}", e))
    }

    /// Create with custom configuration.
    pub fn with_threads(num_threads: usize) -> Result<Self, String> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(num_threads)
            .enable_all()
            .build()
            .map(|runtime| Self { runtime })
            .map_err(|e| format!("Failed to create runtime: {}", e))
    }

    /// Run a future to completion (blocking).
    pub fn block_on<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        self.runtime.block_on(future)
    }

    /// Spawn an async task with callback.
    ///
    /// Returns a handle that can be used to cancel the operation.
    pub fn spawn_with_callback<F, T, C>(
        &self,
        future: F,
        callback: Arc<C>,
    ) -> AsyncHandle
    where
        F: std::future::Future<Output = Result<T, String>> + Send + 'static,
        T: Send + 'static,
        C: ResultCallback<T> + 'static,
    {
        let (cancel_tx, cancel_rx) = oneshot::channel();

        self.runtime.spawn(async move {
            tokio::select! {
                _ = cancel_rx => {
                    // Cancelled
                }
                result = future => {
                    match result {
                        Ok(value) => callback.on_success(value),
                        Err(error) => callback.on_error(error),
                    }
                }
            }
        });

        AsyncHandle::new(cancel_tx)
    }

    /// Spawn an async task (fire and forget).
    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(future);
    }
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default runtime")
    }
}

/// Callback wrapper for FFI.
///
/// This provides a simple way to bridge callbacks across FFI boundaries.
#[derive(Clone)]
pub struct FfiCallback<T> {
    success_fn: Arc<dyn Fn(T) + Send + Sync>,
    error_fn: Arc<dyn Fn(String) + Send + Sync>,
}

impl<T> FfiCallback<T> {
    /// Create a new FFI callback.
    pub fn new<S, E>(success_fn: S, error_fn: E) -> Self
    where
        S: Fn(T) + Send + Sync + 'static,
        E: Fn(String) + Send + Sync + 'static,
    {
        Self {
            success_fn: Arc::new(success_fn),
            error_fn: Arc::new(error_fn),
        }
    }
}

impl<T: Send + Sync + 'static> ResultCallback<T> for FfiCallback<T> {
    fn on_success(&self, value: T) {
        (self.success_fn)(value);
    }

    fn on_error(&self, error: String) {
        (self.error_fn)(error);
    }
}

/// Debouncer for rate-limiting async operations.
///
/// Useful for search-as-you-type scenarios.
pub struct Debouncer {
    delay_ms: u64,
    pending: std::sync::Mutex<Option<oneshot::Sender<()>>>,
}

impl Debouncer {
    /// Create a new debouncer with the given delay.
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay_ms,
            pending: std::sync::Mutex::new(None),
        }
    }

    /// Cancel any pending operation.
    pub fn cancel(&self) {
        let mut pending = self.pending.lock().unwrap();
        if let Some(tx) = pending.take() {
            let _ = tx.send(());
        }
    }

    /// Schedule an operation with debouncing.
    ///
    /// If called again before the delay expires, the previous operation is cancelled.
    pub fn schedule<F>(&self, runtime: &AsyncRuntime, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cancel();

        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            *pending = Some(cancel_tx);
        }

        let delay_ms = self.delay_ms;
        runtime.spawn(async move {
            tokio::select! {
                _ = cancel_rx => {
                    // Cancelled
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)) => {
                    f();
                }
            }
        });
    }
}

/// Retry configuration for async operations.
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Initial delay between retries in milliseconds.
    pub initial_delay_ms: u64,
    /// Backoff multiplier (delay *= multiplier after each retry).
    pub backoff_multiplier: f64,
    /// Maximum delay between retries.
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_delay_ms: 10000,
        }
    }
}

/// Execute an async operation with retry.
pub async fn with_retry<F, Fut, T, E>(config: &RetryConfig, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay_ms = config.initial_delay_ms;
    let mut attempts = 0;

    loop {
        attempts += 1;
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) if attempts >= config.max_attempts => return Err(e),
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                delay_ms = ((delay_ms as f64) * config.backoff_multiplier) as u64;
                if delay_ms > config.max_delay_ms {
                    delay_ms = config.max_delay_ms;
                }
            }
        }
    }
}

// ============================================================================
// Directory Operation Async Wrappers
// ============================================================================

/// Async directory operations manager.
///
/// Provides non-blocking directory operations with callback support.
#[derive(uniffi::Object)]
pub struct DirectoryOperationsAsync {
    runtime: tokio::runtime::Runtime,
}

#[uniffi::export]
impl DirectoryOperationsAsync {
    /// Create a new async directory operations manager.
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, PaykitMobileError> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PaykitMobileError::Internal { message: format!("Failed to create runtime: {}", e) })?;
        Ok(Arc::new(Self { runtime }))
    }

    /// Publish a payment endpoint asynchronously.
    ///
    /// This is a blocking call that wraps the async operation.
    /// For true non-blocking behavior, use the callback-based methods from mobile SDKs.
    pub fn publish_payment_endpoint(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        method_id: String,
        endpoint_data: String,
    ) -> Result<(), PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::publish_payment_endpoint(&transport, &method_id, &endpoint_data)
        })
    }

    /// Remove a payment endpoint asynchronously.
    pub fn remove_payment_endpoint(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        method_id: String,
    ) -> Result<(), PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::remove_payment_endpoint(&transport, &method_id)
        })
    }

    /// Fetch all supported payment methods asynchronously.
    pub fn fetch_supported_payments(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        owner_pubkey: String,
    ) -> Result<Vec<PaymentMethod>, PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::fetch_supported_payments(&transport, &owner_pubkey)
        })
    }

    /// Fetch a specific payment endpoint asynchronously.
    pub fn fetch_payment_endpoint(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        owner_pubkey: String,
        method_id: String,
    ) -> Result<Option<String>, PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::fetch_payment_endpoint(&transport, &owner_pubkey, &method_id)
        })
    }

    /// Fetch known contacts asynchronously.
    pub fn fetch_known_contacts(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        owner_pubkey: String,
    ) -> Result<Vec<String>, PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::fetch_known_contacts(&transport, &owner_pubkey)
        })
    }

    /// Add a contact asynchronously.
    pub fn add_contact(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        contact_pubkey: String,
    ) -> Result<(), PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::add_contact(&transport, &contact_pubkey)
        })
    }

    /// Remove a contact asynchronously.
    pub fn remove_contact(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        contact_pubkey: String,
    ) -> Result<(), PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::remove_contact(&transport, &contact_pubkey)
        })
    }

    /// List all contacts asynchronously.
    pub fn list_contacts(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
    ) -> Result<Vec<String>, PaykitMobileError> {
        self.runtime.block_on(async {
            crate::transport_ffi::list_contacts(&transport)
        })
    }
}

/// Create a new async directory operations manager.
#[uniffi::export]
pub fn create_directory_operations_async() -> Result<Arc<DirectoryOperationsAsync>, PaykitMobileError> {
    DirectoryOperationsAsync::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    #[test]
    fn test_async_runtime_creation() {
        let runtime = AsyncRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_block_on() {
        let runtime = AsyncRuntime::new().unwrap();
        let result = runtime.block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_callback() {
        let success_called = Arc::new(AtomicBool::new(false));
        let success_called_clone = success_called.clone();

        let callback = FfiCallback::new(
            move |value: i32| {
                assert_eq!(value, 42);
                success_called_clone.store(true, Ordering::SeqCst);
            },
            |_| panic!("Should not be called"),
        );

        callback.on_success(42);
        assert!(success_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_async_handle_cancel() {
        let (tx, _rx) = oneshot::channel();
        let mut handle = AsyncHandle::new(tx);
        
        assert!(!handle.is_cancelled());
        handle.cancel();
        assert!(handle.is_cancelled());
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 100);
    }

    #[tokio::test]
    async fn test_with_retry_success() {
        let attempts = AtomicU32::new(0);
        let config = RetryConfig::default();

        let result: Result<i32, &str> = with_retry(&config, || {
            attempts.fetch_add(1, Ordering::SeqCst);
            async { Ok(42) }
        }).await;

        assert_eq!(result, Ok(42));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_failure_then_success() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
            backoff_multiplier: 1.0,
            max_delay_ms: 10,
        };

        let result: Result<i32, &str> = with_retry(&config, || {
            let current = attempts_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if current < 2 {
                    Err("not yet")
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result, Ok(42));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}

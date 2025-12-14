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

    /// Run a future to completion, blocking the current thread until complete.
    ///
    /// # Safety and Usage Restrictions
    ///
    /// **CRITICAL**: This function MUST NOT be called from within an existing Tokio
    /// runtime context. Doing so will cause a panic or deadlock.
    ///
    /// ## Correct Usage
    ///
    /// Call from mobile platform threads that are NOT managed by Tokio:
    /// - iOS: Main thread, Grand Central Dispatch queues
    /// - Android: Main thread, background threads from ExecutorService
    /// - FFI callbacks from Swift/Kotlin
    ///
    /// ## Incorrect Usage (Will Deadlock/Panic)
    ///
    /// - Inside `async fn` or `async` blocks
    /// - Inside Tokio tasks spawned with `spawn()` or `spawn_blocking()`
    /// - Inside any code that's already running on a Tokio worker thread
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // CORRECT: Called from Swift/Kotlin synchronous context
    /// let runtime = AsyncRuntime::new().unwrap();
    /// let result = runtime.block_on(async {
    ///     fetch_payment_methods().await
    /// });
    ///
    /// // INCORRECT: Called from async Rust code - WILL DEADLOCK!
    /// async fn bad_example() {
    ///     let runtime = AsyncRuntime::new().unwrap();
    ///     // This will deadlock because we're already in an async context
    ///     runtime.block_on(async { /* ... */ });
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if called from within a Tokio runtime context.
    pub fn block_on<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        self.runtime.block_on(future)
    }

    /// Spawn an async task with callback.
    ///
    /// Returns a handle that can be used to cancel the operation.
    pub fn spawn_with_callback<F, T, C>(&self, future: F, callback: Arc<C>) -> AsyncHandle
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
        let runtime = tokio::runtime::Runtime::new().map_err(|e| PaykitMobileError::Internal {
            message: format!("Failed to create runtime: {}", e),
        })?;
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
        self.runtime
            .block_on(async { crate::transport_ffi::add_contact(&transport, &contact_pubkey) })
    }

    /// Remove a contact asynchronously.
    pub fn remove_contact(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        contact_pubkey: String,
    ) -> Result<(), PaykitMobileError> {
        self.runtime
            .block_on(async { crate::transport_ffi::remove_contact(&transport, &contact_pubkey) })
    }

    /// List all contacts asynchronously.
    pub fn list_contacts(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
    ) -> Result<Vec<String>, PaykitMobileError> {
        self.runtime
            .block_on(async { crate::transport_ffi::list_contacts(&transport) })
    }
}

/// Create a new async directory operations manager.
#[uniffi::export]
pub fn create_directory_operations_async(
) -> Result<Arc<DirectoryOperationsAsync>, PaykitMobileError> {
    DirectoryOperationsAsync::new()
}

// ============================================================================
// Executor Async Bridge
// ============================================================================

/// Async bridge for executor operations.
///
/// Provides async wrappers for Bitcoin and Lightning executor operations
/// with timeout handling and cancellation support. This is useful when
/// you need to wrap synchronous wallet operations with timeout handling.
///
/// # Usage
///
/// ```ignore
/// let bridge = ExecutorAsyncBridge::new()?;
///
/// // Execute with default 30s timeout
/// let result = bridge.execute_bitcoin_operation(|| {
///     // Your wallet operation here
///     wallet.send_to_address(address, amount)
/// }, None)?;
///
/// // Execute with custom 60s timeout
/// let result = bridge.execute_lightning_operation(|| {
///     // Your node operation here
///     node.pay_invoice(invoice)
/// }, Some(60000))?;
/// ```
///
/// # Timeout Handling
///
/// If an operation exceeds the timeout, a `PaykitMobileError::Transport`
/// error is returned with message "Bitcoin/Lightning operation timed out".
///
/// # Thread Safety
///
/// The bridge manages its own Tokio runtime and is safe to use from any thread.
/// Operations are executed on the runtime's thread pool.
///
/// # Cancellation
///
/// Use `execute_with_cancellation()` to get an `AsyncHandle` that can be used
/// to cancel long-running operations.
#[derive(uniffi::Object)]
pub struct ExecutorAsyncBridge {
    runtime: tokio::runtime::Runtime,
    /// Default timeout for executor operations in milliseconds.
    default_timeout_ms: u64,
}

// UniFFI-exported constructors and simple methods
#[uniffi::export]
impl ExecutorAsyncBridge {
    /// Create a new executor async bridge.
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, PaykitMobileError> {
        Self::with_timeout_internal(30000) // 30 second default timeout
    }

    /// Create with custom timeout.
    #[uniffi::constructor]
    pub fn with_timeout(timeout_ms: u64) -> Result<Arc<Self>, PaykitMobileError> {
        Self::with_timeout_internal(timeout_ms)
    }

    /// Get the default timeout in milliseconds.
    pub fn default_timeout_ms(&self) -> u64 {
        self.default_timeout_ms
    }
}

// Non-exported generic methods (used internally by Rust code)
impl ExecutorAsyncBridge {
    fn with_timeout_internal(timeout_ms: u64) -> Result<Arc<Self>, PaykitMobileError> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| PaykitMobileError::Internal {
            message: format!("Failed to create runtime: {}", e),
        })?;
        Ok(Arc::new(Self {
            runtime,
            default_timeout_ms: timeout_ms,
        }))
    }

    /// Execute a Bitcoin operation with timeout.
    ///
    /// Wraps a Bitcoin executor operation with timeout handling.
    /// Returns an error if the operation times out.
    pub fn execute_bitcoin_operation<F, T>(
        &self,
        operation: F,
        timeout_ms: Option<u64>,
    ) -> Result<T, PaykitMobileError>
    where
        F: FnOnce() -> Result<T, PaykitMobileError> + Send + 'static,
        T: Send + 'static,
    {
        let timeout =
            std::time::Duration::from_millis(timeout_ms.unwrap_or(self.default_timeout_ms));

        self.runtime.block_on(async {
            tokio::time::timeout(timeout, async move {
                tokio::task::spawn_blocking(operation).await
            })
            .await
            .map_err(|_| PaykitMobileError::Transport {
                message: "Bitcoin operation timed out".to_string(),
            })?
            .map_err(|e| PaykitMobileError::Internal {
                message: format!("Task join error: {}", e),
            })?
        })
    }

    /// Execute a Lightning operation with timeout.
    ///
    /// Wraps a Lightning executor operation with timeout handling.
    /// Returns an error if the operation times out.
    pub fn execute_lightning_operation<F, T>(
        &self,
        operation: F,
        timeout_ms: Option<u64>,
    ) -> Result<T, PaykitMobileError>
    where
        F: FnOnce() -> Result<T, PaykitMobileError> + Send + 'static,
        T: Send + 'static,
    {
        let timeout =
            std::time::Duration::from_millis(timeout_ms.unwrap_or(self.default_timeout_ms));

        self.runtime.block_on(async {
            tokio::time::timeout(timeout, async move {
                tokio::task::spawn_blocking(operation).await
            })
            .await
            .map_err(|_| PaykitMobileError::Transport {
                message: "Lightning operation timed out".to_string(),
            })?
            .map_err(|e| PaykitMobileError::Internal {
                message: format!("Task join error: {}", e),
            })?
        })
    }

    /// Execute an operation with cancellation support.
    ///
    /// Returns an AsyncHandle that can be used to cancel the operation.
    pub fn execute_with_cancellation<F, T, C>(
        &self,
        operation: F,
        callback: Arc<C>,
        timeout_ms: Option<u64>,
    ) -> AsyncHandle
    where
        F: FnOnce() -> Result<T, PaykitMobileError> + Send + 'static,
        T: Send + 'static,
        C: ResultCallback<T> + 'static,
    {
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let timeout =
            std::time::Duration::from_millis(timeout_ms.unwrap_or(self.default_timeout_ms));

        self.runtime.spawn(async move {
            tokio::select! {
                _ = cancel_rx => {
                    // Operation was cancelled
                }
                result = async {
                    tokio::time::timeout(timeout, async move {
                        tokio::task::spawn_blocking(operation).await
                    }).await
                } => {
                    match result {
                        Ok(Ok(Ok(value))) => callback.on_success(value),
                        Ok(Ok(Err(e))) => callback.on_error(e.to_string()),
                        Ok(Err(e)) => callback.on_error(format!("Task join error: {}", e)),
                        Err(_) => callback.on_error("Operation timed out".to_string()),
                    }
                }
            }
        });

        AsyncHandle::new(cancel_tx)
    }
}

/// Create a new executor async bridge.
#[uniffi::export]
pub fn create_executor_async_bridge() -> Result<Arc<ExecutorAsyncBridge>, PaykitMobileError> {
    ExecutorAsyncBridge::new()
}

/// Create an executor async bridge with custom timeout.
#[uniffi::export]
pub fn create_executor_async_bridge_with_timeout(
    timeout_ms: u64,
) -> Result<Arc<ExecutorAsyncBridge>, PaykitMobileError> {
    ExecutorAsyncBridge::with_timeout(timeout_ms)
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
        })
        .await;

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
        })
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_executor_async_bridge_creation() {
        let bridge = ExecutorAsyncBridge::new();
        assert!(bridge.is_ok());
        let bridge = bridge.unwrap();
        assert_eq!(bridge.default_timeout_ms(), 30000);
    }

    #[test]
    fn test_executor_async_bridge_with_timeout() {
        let bridge = ExecutorAsyncBridge::with_timeout(5000);
        assert!(bridge.is_ok());
        let bridge = bridge.unwrap();
        assert_eq!(bridge.default_timeout_ms(), 5000);
    }

    #[test]
    fn test_execute_bitcoin_operation_success() {
        let bridge = ExecutorAsyncBridge::new().unwrap();
        let result = bridge.execute_bitcoin_operation(|| Ok(42i32), None);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_execute_lightning_operation_success() {
        let bridge = ExecutorAsyncBridge::new().unwrap();
        let result = bridge.execute_lightning_operation(|| Ok("preimage".to_string()), None);
        assert_eq!(result.unwrap(), "preimage");
    }

    #[test]
    fn test_execute_bitcoin_operation_error() {
        let bridge = ExecutorAsyncBridge::new().unwrap();
        let result: Result<i32, _> = bridge.execute_bitcoin_operation(
            || {
                Err(PaykitMobileError::Transport {
                    message: "Network error".to_string(),
                })
            },
            None,
        );
        assert!(result.is_err());
        match result {
            Err(PaykitMobileError::Transport { message }) => {
                assert!(message.contains("Network error"));
            }
            _ => panic!("Expected Transport error"),
        }
    }

    #[test]
    fn test_execute_lightning_operation_timeout() {
        let bridge = ExecutorAsyncBridge::with_timeout(10).unwrap(); // 10ms timeout
        let result: Result<i32, _> = bridge.execute_lightning_operation(
            || {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(42)
            },
            None,
        );
        assert!(result.is_err());
        match result {
            Err(PaykitMobileError::Transport { message }) => {
                assert!(message.contains("timed out"));
            }
            _ => panic!("Expected timeout error"),
        }
    }
}

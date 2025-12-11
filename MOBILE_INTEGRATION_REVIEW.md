# Paykit Mobile Integration Review for Bitkit

## Executive Summary

This document reviews the `paykit-rs-master` repository to identify what's needed to facilitate mobile implementation into Bitkit for iOS and Android. The review compares the current mobile FFI bindings with the full Paykit API surface and identifies gaps.

## Current State

### ✅ What's Already Implemented

1. **Core Client Structure**
   - `PaykitClient` with UniFFI bindings
   - Payment method selection and validation
   - Health monitoring
   - Subscription management (create, proration)
   - Payment requests
   - Receipt creation
   - QR code scanning/parsing

2. **Storage Abstraction**
   - `SecureStorage` trait for platform-specific storage
   - iOS Keychain adapter (Swift)
   - Android EncryptedSharedPreferences adapter (Kotlin)
   - In-memory storage for testing

3. **Async Bridge**
   - Callback-based async patterns
   - Retry logic
   - Debouncing utilities

4. **Demo Applications**
   - Complete iOS demo (SwiftUI)
   - Complete Android demo (Jetpack Compose)

## Critical Gaps for Bitkit Integration

### 1. **Directory Operations (HIGH PRIORITY)**

**Missing FFI Bindings:**
- `publish_payment_endpoint(method_id, endpoint_data)` - Publish payment methods
- `fetch_supported_payments(public_key)` - Discover payment methods
- `fetch_payment_endpoint(public_key, method_id)` - Get specific endpoint
- `remove_payment_endpoint(method_id)` - Remove published methods
- `fetch_known_contacts(public_key)` - List contacts

**Impact:** Without these, Bitkit cannot:
- Publish user's payment methods to the directory
- Discover payment methods for other users
- Build contact lists
- Update or remove published methods

**Recommendation:** Add these methods to `PaykitClient` with async bridge support.

### 2. **Transport Integration (HIGH PRIORITY)**

**Current Issue:** `PaykitClient` is stateless but doesn't accept transport instances. The core `paykit-lib` requires:
- `AuthenticatedTransport` for publishing operations
- `UnauthenticatedTransportRead` for discovery operations

**Missing:**
- FFI wrapper for `PubkyAuthenticatedTransport`
- FFI wrapper for `PubkyUnauthenticatedTransport`
- Session management helpers

**Impact:** Cannot perform actual network operations without transport instances.

**Recommendation:** 
- Add `PaykitClient::with_transport()` constructor that accepts transport instances
- Or create separate `PaykitDirectoryClient` that wraps transport operations
- Provide session creation helpers (or document that apps use Pubky SDK directly)

### 3. **Interactive Payment Protocol (MEDIUM PRIORITY)**

**Missing FFI Bindings:**
- `PaykitInteractiveManager` - No FFI exposure
- `initiate_payment()` - Start payment negotiation
- `handle_message()` - Process payment messages
- `PaykitNoiseChannel` - Encrypted channel management

**Impact:** Cannot perform interactive payment flows with Noise encryption.

**Recommendation:** Expose `PaykitInteractiveManager` through FFI with simplified API.

### 4. **Private Endpoint Exchange (MEDIUM PRIORITY)**

**Missing:**
- FFI for private endpoint storage/retrieval
- Integration with Noise channels for secure exchange

**Impact:** Cannot exchange private payment endpoints securely.

**Recommendation:** Add private endpoint management to FFI.

### 5. **Contact Management (MEDIUM PRIORITY)**

**Missing:**
- `add_contact(public_key)` - Add to contacts
- `remove_contact(public_key)` - Remove from contacts
- `list_contacts()` - List all contacts
- Contact storage integration

**Impact:** Cannot build and manage contact lists in Bitkit.

**Recommendation:** Add contact management methods to `PaykitClient`.

### 6. **Configuration & Build (LOW PRIORITY)**

**Missing:**
- `uniffi.toml` configuration file for `paykit-mobile`
- Package name/module name configuration
- Build scripts for automated binding generation

**Impact:** Manual binding generation, potential naming inconsistencies.

**Recommendation:** Add `uniffi.toml` with proper configuration.

### 7. **Error Handling Enhancements (LOW PRIORITY)**

**Current State:** Basic error types exist but may need expansion for:
- Network errors (timeout, connection refused)
- Authentication errors
- Validation errors with more context

**Recommendation:** Review error types and ensure all failure modes are covered.

## Detailed Recommendations

### Priority 1: Directory Operations

Add to `paykit-mobile/src/lib.rs`:

```rust
#[uniffi::export]
impl PaykitClient {
    /// Publish a payment endpoint to the directory.
    /// Requires an authenticated transport (Pubky session).
    pub async fn publish_payment_endpoint(
        &self,
        transport: Arc<dyn AuthenticatedTransportFFI>,
        method_id: String,
        endpoint_data: String,
    ) -> Result<()> {
        // Implementation
    }

    /// Discover payment methods for a public key.
    pub async fn fetch_supported_payments(
        &self,
        transport: Arc<dyn UnauthenticatedTransportFFI>,
        public_key: String,
    ) -> Result<Vec<PaymentMethod>> {
        // Implementation
    }

    /// Fetch a specific payment endpoint.
    pub async fn fetch_payment_endpoint(
        &self,
        transport: Arc<dyn UnauthenticatedTransportFFI>,
        public_key: String,
        method_id: String,
    ) -> Result<Option<EndpointData>> {
        // Implementation
    }

    /// Remove a published payment endpoint.
    pub async fn remove_payment_endpoint(
        &self,
        transport: Arc<dyn AuthenticatedTransportFFI>,
        method_id: String,
    ) -> Result<()> {
        // Implementation
    }

    /// Fetch known contacts for a public key.
    pub async fn fetch_known_contacts(
        &self,
        transport: Arc<dyn UnauthenticatedTransportFFI>,
        public_key: String,
    ) -> Result<Vec<String>> {
        // Implementation
    }
}
```

### Priority 2: Transport Wrappers

Create `paykit-mobile/src/transport_ffi.rs`:

```rust
/// FFI wrapper for authenticated transport.
#[derive(uniffi::Object)]
pub struct AuthenticatedTransportFFI {
    inner: Arc<dyn paykit_lib::AuthenticatedTransport>,
}

#[uniffi::export]
impl AuthenticatedTransportFFI {
    #[uniffi::constructor]
    pub fn from_pubky_session(session: PubkySessionFFI) -> Arc<Self> {
        // Wrap PubkyAuthenticatedTransport
    }
}

/// FFI wrapper for unauthenticated transport.
#[derive(uniffi::Object)]
pub struct UnauthenticatedTransportFFI {
    inner: Arc<dyn paykit_lib::UnauthenticatedTransportRead>,
}

#[uniffi::export]
impl UnauthenticatedTransportFFI {
    #[uniffi::constructor]
    pub fn from_pubky_storage(storage: PubkyStorageFFI) -> Arc<Self> {
        // Wrap PubkyUnauthenticatedTransport
    }
}
```

### Priority 3: Interactive Protocol

Add to `paykit-mobile/src/lib.rs`:

```rust
/// Interactive payment manager for mobile.
#[derive(uniffi::Object)]
pub struct PaykitInteractiveManagerFFI {
    inner: Arc<PaykitInteractiveManager>,
}

#[uniffi::export]
impl PaykitInteractiveManagerFFI {
    #[uniffi::constructor]
    pub fn new(
        storage: Arc<dyn SecureStorage>,
        receipt_generator: Option<Arc<dyn ReceiptGeneratorFFI>>,
    ) -> Result<Arc<Self>> {
        // Implementation
    }

    /// Initiate a payment over a Noise channel.
    pub async fn initiate_payment(
        &self,
        channel: Arc<NoiseChannelFFI>,
        provisional_receipt: Receipt,
    ) -> Result<Receipt> {
        // Implementation
    }

    /// Handle an incoming payment message.
    pub async fn handle_message(
        &self,
        message: Vec<u8>,
        payer: String,
        payee: String,
    ) -> Result<PaymentResponse> {
        // Implementation
    }
}
```

### Priority 4: Contact Management

Add to `paykit-mobile/src/lib.rs`:

```rust
#[uniffi::export]
impl PaykitClient {
    /// Add a contact to the local contact list.
    pub async fn add_contact(
        &self,
        transport: Arc<dyn AuthenticatedTransportFFI>,
        public_key: String,
    ) -> Result<()> {
        // Implementation using PUBKY_FOLLOWS_PATH
    }

    /// Remove a contact from the local contact list.
    pub async fn remove_contact(
        &self,
        transport: Arc<dyn AuthenticatedTransportFFI>,
        public_key: String,
    ) -> Result<()> {
        // Implementation
    }

    /// List all contacts.
    pub async fn list_contacts(
        &self,
        transport: Arc<dyn AuthenticatedTransportFFI>,
    ) -> Result<Vec<String>> {
        // Implementation
    }
}
```

## Implementation Checklist

### Phase 1: Core Directory Operations
- [ ] Add `publish_payment_endpoint` to `PaykitClient`
- [ ] Add `fetch_supported_payments` to `PaykitClient`
- [ ] Add `fetch_payment_endpoint` to `PaykitClient`
- [ ] Add `remove_payment_endpoint` to `PaykitClient`
- [ ] Add `fetch_known_contacts` to `PaykitClient`
- [ ] Add async bridge methods for all directory operations
- [ ] Add tests for directory operations

### Phase 2: Transport Integration
- [ ] Create `AuthenticatedTransportFFI` wrapper
- [ ] Create `UnauthenticatedTransportFFI` wrapper
- [ ] Add `PubkySessionFFI` wrapper (or document direct Pubky SDK usage)
- [ ] Add `PubkyStorageFFI` wrapper
- [ ] Update `PaykitClient` to accept transport instances
- [ ] Add examples for session creation

### Phase 3: Interactive Protocol
- [ ] Create `PaykitInteractiveManagerFFI`
- [ ] Add `NoiseChannelFFI` wrapper
- [ ] Expose `initiate_payment` and `handle_message`
- [ ] Add receipt generation helpers
- [ ] Add tests for interactive protocol

### Phase 4: Contact Management
- [ ] Add `add_contact` method
- [ ] Add `remove_contact` method
- [ ] Add `list_contacts` method
- [ ] Integrate with storage for local contact list
- [ ] Add tests for contact management

### Phase 5: Configuration & Polish
- [ ] Add `uniffi.toml` configuration
- [ ] Create build scripts for binding generation
- [ ] Update documentation with complete examples
- [ ] Add error handling improvements
- [ ] Performance testing and optimization

## Comparison with pubky-noise-main

The `pubky-noise-main` project provides a good reference for mobile FFI patterns:

1. **Session Management**: `FfiNoiseManager` provides a complete example of wrapping async operations
2. **State Management**: `save_state` and `restore_state` methods for session persistence
3. **Error Handling**: Comprehensive `FfiNoiseError` enum
4. **Configuration**: `FfiMobileConfig` for platform-specific settings

**Recommendation:** Follow similar patterns for Paykit mobile bindings.

## Testing Recommendations

1. **Unit Tests**: Test all FFI methods with mock transports
2. **Integration Tests**: Test with real Pubky testnet
3. **Mobile Tests**: Test on actual iOS/Android devices
4. **Performance Tests**: Measure async operation latency
5. **Error Path Tests**: Test all error conditions

## Documentation Needs

1. **Complete API Reference**: Document all FFI methods
2. **Integration Guide**: Step-by-step Bitkit integration guide
3. **Session Management Guide**: How to create and manage Pubky sessions
4. **Example Code**: Complete working examples for common use cases
5. **Troubleshooting Guide**: Common issues and solutions

## Conclusion

The current `paykit-mobile` implementation provides a solid foundation with payment method selection, subscriptions, and QR scanning. However, **critical directory operations and transport integration are missing**, which are essential for Bitkit integration.

**Immediate Action Items:**
1. Implement directory operations (publish, fetch, remove)
2. Add transport wrappers for Pubky integration
3. Create `uniffi.toml` configuration
4. Add comprehensive examples and documentation

Once these are implemented, Paykit will be ready for full Bitkit mobile integration.



# Paykit Mobile Integration - Critical Gaps Summary

## 🚨 Critical Missing Features for Bitkit Integration

### 1. Directory Operations (BLOCKER)
**Status:** ❌ Not Implemented

**Missing Methods:**
- `publish_payment_endpoint()` - Cannot publish user's payment methods
- `fetch_supported_payments()` - Cannot discover payment methods for contacts
- `fetch_payment_endpoint()` - Cannot fetch specific payment endpoints
- `remove_payment_endpoint()` - Cannot remove published methods
- `fetch_known_contacts()` - Cannot list contacts

**Impact:** **CRITICAL** - Without these, Bitkit cannot:
- Publish payment methods to the directory
- Discover payment methods for other users
- Build contact lists
- Update or remove published methods

**Required For:** Core payment discovery and publishing functionality

---

### 2. Transport Integration (BLOCKER)
**Status:** ❌ Not Implemented

**Missing:**
- FFI wrappers for `AuthenticatedTransport` and `UnauthenticatedTransportRead`
- Integration with Pubky sessions for authenticated operations
- Integration with Pubky storage for unauthenticated reads

**Impact:** **CRITICAL** - Cannot perform actual network operations. The current `PaykitClient` is stateless but doesn't accept transport instances needed for directory operations.

**Required For:** All network-based operations (publishing, discovery)

---

### 3. Interactive Payment Protocol (HIGH PRIORITY)
**Status:** ❌ Not Implemented

**Missing:**
- `PaykitInteractiveManager` FFI exposure
- `initiate_payment()` - Start payment negotiation
- `handle_message()` - Process payment messages
- Noise channel integration

**Impact:** **HIGH** - Cannot perform interactive payment flows with encrypted channels

**Required For:** Secure payment negotiation and receipt exchange

---

### 4. Contact Management (MEDIUM PRIORITY)
**Status:** ❌ Not Implemented

**Missing:**
- `add_contact()` - Add contacts to local list
- `remove_contact()` - Remove contacts
- `list_contacts()` - List all contacts

**Impact:** **MEDIUM** - Cannot build and manage contact lists in Bitkit

**Required For:** Contact management UI and features

---

### 5. Configuration (LOW PRIORITY)
**Status:** ❌ Missing

**Missing:**
- `uniffi.toml` configuration file
- Package/module name configuration
- Build automation scripts

**Impact:** **LOW** - Manual binding generation, potential inconsistencies

---

## ✅ What's Already Working

1. ✅ Payment method selection and validation
2. ✅ Subscription management (create, proration)
3. ✅ Payment requests
4. ✅ Receipt creation
5. ✅ QR code scanning/parsing
6. ✅ Health monitoring
7. ✅ Secure storage abstraction (iOS Keychain, Android EncryptedSharedPreferences)
8. ✅ Async bridge utilities
9. ✅ Demo applications (iOS and Android)

## Implementation Priority

### Phase 1: Critical (Must Have)
1. **Directory Operations** - Publish, fetch, remove payment endpoints
2. **Transport Integration** - Wrap Pubky transports for FFI

### Phase 2: High Priority
3. **Interactive Protocol** - Payment negotiation and receipt exchange
4. **Contact Management** - Add, remove, list contacts

### Phase 3: Nice to Have
5. **Configuration** - uniffi.toml and build scripts
6. **Error Handling** - Enhanced error types
7. **Documentation** - Complete integration guide

## Quick Start for Implementation

### Step 1: Add Directory Operations
```rust
// In paykit-mobile/src/lib.rs
#[uniffi::export]
impl PaykitClient {
    pub async fn publish_payment_endpoint(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        method_id: String,
        endpoint_data: String,
    ) -> Result<()> {
        // Use paykit_lib::set_payment_endpoint
    }
    
    pub async fn fetch_supported_payments(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        public_key: String,
    ) -> Result<Vec<PaymentMethod>> {
        // Use paykit_lib::get_payment_list
    }
}
```

### Step 2: Create Transport Wrappers
```rust
// In paykit-mobile/src/transport_ffi.rs
#[derive(uniffi::Object)]
pub struct AuthenticatedTransportFFI {
    inner: Arc<dyn paykit_lib::AuthenticatedTransport>,
}

#[uniffi::export]
impl AuthenticatedTransportFFI {
    #[uniffi::constructor]
    pub fn from_pubky_session(session_bytes: Vec<u8>) -> Result<Arc<Self>> {
        // Deserialize PubkySession and wrap PubkyAuthenticatedTransport
    }
}
```

### Step 3: Add uniffi.toml
```toml
# In paykit-mobile/uniffi.toml
[bindings.kotlin]
package_name = "com.paykit.mobile"

[bindings.swift]
module_name = "PaykitMobile"
```

## Testing Checklist

- [ ] Unit tests for all directory operations
- [ ] Integration tests with Pubky testnet
- [ ] Mobile device tests (iOS and Android)
- [ ] Error path testing
- [ ] Performance testing

## Documentation Needs

- [ ] Complete API reference
- [ ] Bitkit integration guide
- [ ] Session management examples
- [ ] Troubleshooting guide
- [ ] Migration guide from current implementation

## Estimated Effort

- **Phase 1 (Critical):** 2-3 days
- **Phase 2 (High Priority):** 2-3 days
- **Phase 3 (Nice to Have):** 1 day

**Total:** ~5-7 days for complete mobile integration readiness

---

**Next Steps:** Start with Phase 1 (Directory Operations + Transport Integration) as these are blockers for Bitkit integration.



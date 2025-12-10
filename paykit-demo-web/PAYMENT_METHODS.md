# Payment Methods Feature

## Overview

The Payment Methods feature allows you to configure and manage your payment endpoints in the Paykit Demo Web application. This is a key part of the Paykit protocol, enabling discovery of how others can pay you.

## Features

### ✅ Implemented

- **Add Payment Methods**: Configure Lightning, Bitcoin Onchain, or custom payment methods
- **Priority Ordering**: Set priority levels (1 = highest) for your preferred methods
- **Preferred Methods**: Mark specific methods as preferred for incoming payments
- **Public/Private Visibility**: Control whether methods are visible in the public directory
- **Method Management**: Edit priorities, toggle preferences, and delete methods
- **Local Persistence**: All methods are stored in browser localStorage

### ⚠️ Demo Limitations

This is a **demonstration implementation** with important limitations:

1. **No Real Publishing**: The "Mock Publish" button simulates publishing but does NOT actually publish methods to Pubky homeservers
2. **No Homeserver Integration**: Methods are saved locally only and won't be discoverable by others
3. **No Authentication**: No capability tokens or authenticated requests
4. **No Encryption**: Methods stored in plaintext in browser localStorage
5. **No Synchronization**: Methods are per-browser, not synced across devices

For production use, you would need to integrate with Pubky's authenticated PUT operations to publish methods to the directory.

## Usage

### Adding a Payment Method

1. Navigate to the **Settings** tab
2. In the "Add Payment Method" section:
   - Select method type (Lightning, Bitcoin Onchain, or Custom)
   - Enter the payment endpoint:
     - **Lightning**: LNURL, Lightning Address (e.g., `user@domain.com`), or node pubkey
     - **Onchain**: Bitcoin address (e.g., `bc1q...`)
     - **Custom**: Any custom payment endpoint
   - Toggle **Public** (visible in directory) or Private
   - Toggle **Preferred** if this is your preferred method
   - Set **Priority** (1 = highest priority)
3. Click **Add Payment Method**

### Managing Methods

#### Reorder Priorities

Use the **↑** and **↓** buttons to adjust method priority. Lower numbers = higher priority (1 is highest).

Methods with higher priority are recommended first when others query your payment methods.

#### Toggle Preferred Status

Click the **Set ⭐** / **Unset ⭐** button to mark a method as preferred. Preferred methods are highlighted and recommended to payers.

#### Delete Methods

Click **Delete** on any method card to remove it. This action cannot be undone.

### Mock Publishing

Click the **Mock Publish** button to simulate publishing your public methods to a Pubky homeserver. 

**Important**: This is demo-only and does NOT actually publish to a real homeserver. In production, this would:
- Use authenticated Pubky PUT requests
- Publish to your Pubky homeserver at `/pub/paykit.app/v0/{method_id}`
- Require valid capability tokens
- Make methods discoverable by others via the Directory

## Payment Method Types

### Lightning ⚡

Lightning Network payment endpoints. Supported formats:
- **LNURL**: `lnurl1dp68gurn8ghj7um5v93kketj9ehx2amn9uh8wetvdskkkmn0wahz7mrww4excup0...`
- **Lightning Address**: `user@domain.com`
- **Node Pubkey**: `02abc123...`

### Bitcoin Onchain ₿

Bitcoin blockchain addresses:
- **Bech32**: `bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh`
- **P2SH**: `3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy`
- **Legacy**: `1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa`

### Custom 💳

Any custom payment method ID and endpoint. Examples:
- **Liquid Network**: `liquid:VJL...`
- **Ecash**: `cashu:token123...`
- **Other protocols**: Define your own

## Storage Schema

Payment methods are stored in browser localStorage with the following keys:

```
paykit_payment_method:{method_id} → JSON method config
paykit_mock_publish_status → Mock publish timestamp
```

Each method config contains:
```json
{
  "method_id": "lightning",
  "endpoint": "user@domain.com",
  "is_public": true,
  "is_preferred": true,
  "priority": 1
}
```

## Security Considerations

⚠️ **Demo Security Warnings:**

1. **Unencrypted Storage**: All methods stored in plaintext in localStorage
2. **No Access Control**: Anyone with browser access can see/modify methods
3. **localStorage Limits**: Subject to ~5-10MB browser limits
4. **No Backup**: Can be cleared by browser or user at any time
5. **No Validation**: Endpoints are not validated against the actual payment system

### Production Requirements

For production deployment, implement:

1. **Encryption at Rest**: Encrypt method configs before storage
2. **Authenticated Publishing**: Use Pubky capability tokens for PUT requests
3. **Endpoint Validation**: Verify endpoints are valid before saving
4. **Backup & Sync**: Server-side storage with cross-device sync
5. **Rate Limiting**: Protect against abuse of mock/real publishing
6. **Key Rotation**: Support updating keys without losing method configs

## API Reference

### WasmPaymentMethodConfig

```rust
pub struct WasmPaymentMethodConfig {
    method_id: String,
    endpoint: String,
    is_public: bool,
    is_preferred: bool,
    priority: u32,
}
```

**Constructor:**
```javascript
new WasmPaymentMethodConfig(
    methodId,    // String: "lightning", "onchain", etc.
    endpoint,    // String: payment endpoint
    isPublic,    // Boolean: visible in directory
    isPreferred, // Boolean: mark as preferred
    priority     // Number: 1 = highest priority
)
```

### WasmPaymentMethodStorage

**Methods:**

- `save_method(method)` - Save a payment method
- `get_method(method_id)` - Retrieve a method by ID
- `list_methods()` - List all methods sorted by priority
- `delete_method(method_id)` - Delete a method
- `set_preferred(method_id, preferred)` - Update preferred status
- `update_priority(method_id, priority)` - Change priority
- `get_preferred_methods()` - Get only preferred methods
- `mock_publish()` - Mock publish (demo only)

## Examples

### Adding a Lightning Method

```javascript
const storage = new WasmPaymentMethodStorage();

const lightningMethod = new WasmPaymentMethodConfig(
    "lightning",
    "alice@getalby.com",
    true,  // public
    true,  // preferred
    1      // highest priority
);

await storage.save_method(lightningMethod);
```

### Listing Methods

```javascript
const methods = await storage.list_methods();
// Returns array sorted by priority (lowest first)

methods.forEach(method => {
    console.log(`${method.method_id}: ${method.endpoint}`);
    console.log(`Priority: ${method.priority}, Preferred: ${method.is_preferred}`);
});
```

### Updating Priority

```javascript
// Move a method to highest priority
await storage.update_priority("lightning", 1);

// Move another method down
await storage.update_priority("onchain", 2);
```

## Future Enhancements

Phase 2 provides the foundation. Future phases will add:

- **Phase 6**: Integration with payment flow (method compatibility checking)
- **Phase 8**: Method matching algorithm (find compatible methods with contacts)
- **Production**: Real Pubky homeserver publishing with authentication

## Related Documentation

- [Paykit Protocol Specification](../README.md)
- [Contacts Feature](./CONTACTS_FEATURE.md)
- [Directory Client](./ARCHITECTURE.md#directory-client)
- [Pubky Protocol](https://pubky.org)

## Troubleshooting

### Methods Not Appearing

**Symptom**: Added methods don't appear in the list.

**Solutions**:
1. Click the "Refresh List" button
2. Check browser console for errors
3. Verify localStorage is enabled
4. Check browser localStorage quota

### Mock Publish Warning

**Symptom**: Mock publish shows warning message.

**Expected**: This is correct behavior. The demo does NOT publish to real homeservers. The warning reminds you this is demo-only functionality.

### Priority Conflicts

**Symptom**: Multiple methods have the same priority.

**Expected**: The system allows duplicate priorities. Methods with the same priority are sorted by insertion order. For best results, assign unique priorities (1, 2, 3, etc.).

### Methods Not Discoverable

**Symptom**: Others can't see my methods in Directory queries.

**Expected**: This is a demo limitation. Methods are stored locally only and not published to real Pubky homeservers. In production, you would use authenticated publishing to make methods discoverable.

## Support

For issues or questions:
- Check browser console for error messages
- Verify WASM module loaded successfully
- Review [Testing Guide](./TESTING.md)
- Open an issue on GitHub

---

**Status**: Phase 2 Complete ✅
**Last Updated**: November 2024


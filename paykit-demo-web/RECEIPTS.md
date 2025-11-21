# Receipt Management Feature

## Overview

The Receipt Management feature allows you to view, filter, and manage your payment transaction history in the Paykit Demo Web application. This provides a complete audit trail of all payment coordinations.

## Features

### ✅ Implemented

- **View Receipts**: See all payment receipts with details
- **Direction Filtering**: Filter by sent or received payments
- **Method Filtering**: Filter by payment method (Lightning, Onchain, Custom)
- **Contact Filtering**: View transactions with specific contacts
- **Statistics Dashboard**: Overview of total, sent, and received payments
- **Export Functionality**: Export all receipts as JSON
- **Delete Operations**: Remove individual receipts or clear all
- **Local Persistence**: All receipts stored in browser localStorage
- **Visual Timeline**: Receipts sorted by timestamp (newest first)

### 📊 Statistics

The receipt dashboard shows:
- **Total**: Total number of receipts
- **Sent**: Number of outgoing payments
- **Received**: Number of incoming payments

## Usage

### Viewing Receipts

1. Navigate to the **Receipts** tab
2. See your receipt statistics at the top
3. View all receipts in chronological order (newest first)
4. Each receipt card shows:
   - Direction (📤 Sent or 📥 Received)
   - Amount and currency
   - Payment method with icon
   - Contact (payer or payee)
   - Timestamp
   - Receipt ID

### Filtering Receipts

Use the filter controls to narrow down receipts:

#### By Direction
- **All**: Show all receipts
- **Sent**: Show only outgoing payments
- **Received**: Show only incoming payments

#### By Method
- **All Methods**: Show all payment methods
- **Lightning**: Show only Lightning payments
- **Bitcoin Onchain**: Show only onchain payments
- **Custom**: Show only custom methods

#### By Contact
Select a contact from the dropdown to see all transactions with that person.

**Steps:**
1. Select your filters
2. Click **Apply Filters**
3. Click **Reset** to clear filters

### Receipt Actions

#### View Details
Click **View Details** on any receipt to see full information including:
- Complete receipt ID
- Full public keys (payer and payee)
- Exact timestamp
- All metadata

#### Delete Receipt
Click **Delete** to remove a specific receipt. This action requires confirmation.

#### Export Receipts
Click **Export JSON** to download all receipts as a JSON file. The file is named:
```
paykit-receipts-[timestamp].json
```

#### Clear All
Click **Clear All** to delete all receipts. This action requires confirmation and cannot be undone.

## Receipt Format

Receipts are stored with the following structure:

```json
{
  "receipt_id": "unique_receipt_id",
  "payer": "payer_public_key",
  "payee": "payee_public_key",
  "amount": "1000",
  "currency": "SAT",
  "method": "lightning",
  "timestamp": 1700000000
}
```

### Fields

- **receipt_id**: Unique identifier for the receipt
- **payer**: Public key of the person who paid
- **payee**: Public key of the person who received
- **amount**: Payment amount as string
- **currency**: Currency code (e.g., "SAT", "USD")
- **method**: Payment method used (e.g., "lightning", "onchain")
- **timestamp**: Unix timestamp in seconds

## Storage Schema

Receipts are stored in browser localStorage with keys:
```
paykit_receipts:{receipt_id} → JSON receipt data
```

### Storage Limits

Browser localStorage typically has a 5-10MB limit. With average receipt size of ~300 bytes, you can store approximately:
- **5MB limit**: ~17,000 receipts
- **10MB limit**: ~34,000 receipts

## API Reference

### WasmReceiptStorage

Main class for managing receipts in WASM.

**Constructor:**
```javascript
const storage = new WasmReceiptStorage();
```

**Methods:**

#### Basic Operations
- `save_receipt(receipt_id, receipt_json)` - Save a receipt
- `get_receipt(receipt_id)` - Retrieve a receipt by ID
- `list_receipts()` - List all receipts
- `delete_receipt(receipt_id)` - Delete a specific receipt

#### Filtering
- `filter_by_direction(direction, current_pubkey)` - Filter by "sent" or "received"
- `filter_by_method(method)` - Filter by payment method
- `filter_by_contact(contact_pubkey, current_pubkey)` - Filter by contact

#### Analytics
- `get_statistics(current_pubkey)` - Get receipt statistics
- `export_as_json()` - Export all receipts as JSON string

#### Bulk Operations
- `clear_all()` - Delete all receipts

## Examples

### JavaScript Usage

```javascript
import { WasmReceiptStorage } from './pkg/paykit_demo_web.js';

const storage = new WasmReceiptStorage();

// Save a receipt
const receipt = JSON.stringify({
    receipt_id: "receipt_123",
    payer: "alice_pubkey",
    payee: "bob_pubkey",
    amount: "1000",
    currency: "SAT",
    method: "lightning",
    timestamp: Date.now() / 1000
});

await storage.save_receipt("receipt_123", receipt);

// List all receipts
const receipts = await storage.list_receipts();
console.log(`Total receipts: ${receipts.length}`);

// Filter sent payments
const sent = await storage.filter_by_direction("sent", myPublicKey);

// Filter by method
const lightning = await storage.filter_by_method("lightning");

// Get statistics
const stats = await storage.get_statistics(myPublicKey);
console.log(`Sent: ${stats.sent}, Received: ${stats.received}`);

// Export receipts
const json = await storage.export_as_json();
// Download or process json
```

### Rust Usage (Testing)

```rust
use paykit_demo_web::WasmReceiptStorage;

#[wasm_bindgen_test]
async fn test_receipts() {
    let storage = WasmReceiptStorage::new();
    
    // Save receipt
    let receipt_json = r#"{"receipt_id":"test","payer":"alice","payee":"bob","amount":"1000","currency":"SAT","method":"lightning","timestamp":1700000000}"#;
    storage.save_receipt("test", receipt_json).await.unwrap();
    
    // Retrieve
    let retrieved = storage.get_receipt("test").await.unwrap();
    assert!(retrieved.is_some());
}
```

## Security Considerations

⚠️ **Demo Security Warnings:**

1. **Unencrypted Storage**: Receipts stored in plaintext in localStorage
2. **No Access Control**: Anyone with browser access can see/modify receipts
3. **localStorage Limits**: Subject to ~5-10MB browser limits
4. **No Backup**: Can be cleared by browser or user at any time
5. **No Server Sync**: Receipts are local-only, not synced across devices

### Production Requirements

For production deployment, implement:

1. **Encryption at Rest**: Encrypt receipts before storage
2. **Server-Side Storage**: Backend database with proper backups
3. **Authentication**: Secure access control for receipt viewing
4. **Audit Logging**: Track receipt access and modifications
5. **Cross-Device Sync**: Sync receipts across user's devices
6. **Data Retention**: Implement retention policies and archiving
7. **Export Security**: Add authentication to export functionality

## Troubleshooting

### Receipts Not Appearing

**Symptom**: Added receipts don't appear in the list.

**Solutions**:
1. Click the "Refresh List" button
2. Check browser console for errors
3. Verify localStorage is enabled
4. Check browser localStorage quota

### Statistics Not Updating

**Symptom**: Statistics don't reflect current receipts.

**Solutions**:
1. Refresh the receipts list
2. Switch tabs and return to Receipts tab
3. Check that currentIdentity is set

### Filters Not Working

**Symptom**: Filters don't show expected results.

**Solutions**:
1. Click "Reset" then reapply filters
2. Verify receipts have the expected fields
3. Check that contact filter is populated with contacts

### Export Fails

**Symptom**: Export button doesn't download file.

**Solutions**:
1. Check browser popup blocker settings
2. Verify download permissions
3. Try a different browser
4. Check browser console for errors

### localStorage Full

**Symptom**: Error saving receipts ("QuotaExceededError").

**Solutions**:
1. Clear old receipts using "Clear All"
2. Export receipts before clearing
3. Clear other website data
4. Increase browser storage limits (browser-dependent)

## Integration with Other Features

### With Contacts
- Filter receipts by contact to see transaction history
- Receipt count could be shown in contact details (future enhancement)

### With Payment Methods
- Receipts show which payment method was used
- Filter by method to analyze usage patterns

### With Payment Flow
- Receipts are automatically created during payment coordination
- Both payer and payee receive receipts

## Future Enhancements

Phase 3 provides the foundation. Future phases could add:

- **Visual Timeline**: Calendar or timeline view of receipts
- **Receipt Search**: Full-text search across receipt fields
- **CSV Export**: Export receipts in CSV format for accounting software
- **Receipt Notes**: Add custom notes to receipts
- **Receipt Categories**: Tag receipts with categories
- **Amount Summaries**: Total amounts by period, method, or contact
- **Charts & Graphs**: Visual analytics of payment patterns
- **Receipt Verification**: Cryptographic verification of receipt authenticity
- **Print Functionality**: Print-friendly receipt format

## Related Documentation

- [Payment Methods Feature](./PAYMENT_METHODS.md)
- [Contacts Feature](./CONTACTS_FEATURE.md)
- [Payment Coordinator](./README.md#interactive-payments)
- [Testing Guide](./TESTING.md)

## Technical Details

### Performance

- **List Performance**: O(n) where n = number of receipts in localStorage
- **Filter Performance**: O(n) single pass over receipts
- **Storage Access**: Async operations, non-blocking
- **Memory Usage**: Receipts loaded on-demand, not kept in memory

### Browser Compatibility

Tested and working on:
- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Opera 76+

Requires:
- WebAssembly support
- localStorage API
- JSON support
- ES6+ JavaScript

### Limitations

- **Demo Only**: Not suitable for production use without encryption
- **No Real-Time**: Changes not reflected across tabs/windows
- **No Pagination**: All receipts loaded at once (may be slow with thousands)
- **No Backup**: localStorage can be cleared at any time
- **Single User**: No multi-user support

## Support

For issues or questions:
- Check browser console for error messages
- Verify WASM module loaded successfully
- Review [Testing Guide](./TESTING.md)
- Open an issue on GitHub

---

**Status**: Phase 3 Complete ✅  
**Last Updated**: November 2024


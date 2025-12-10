# Contact Management Feature

## Overview

The Contact Management feature provides a browser-based address book for managing Pubky peers. Contacts are stored in browser localStorage and can be enriched with notes and payment history.

## Features

- **Add Contacts**: Create contacts with Pubky public keys and human-readable names
- **Notes**: Add optional notes about each contact
- **Search**: Case-insensitive search by contact name
- **Payment History**: Track payment receipts associated with each contact
- **Persistence**: Automatic browser localStorage persistence

## Storage Schema

Contacts are stored in browser localStorage with the following keys:

- `paykit_contact:{pubkey}` - Individual contact data (JSON)

### Contact Data Structure

```json
{
  "public_key": "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
  "name": "Alice Smith",
  "notes": "Met at Bitcoin conference 2025",
  "added_at": 1700000000,
  "payment_history": ["receipt_001", "receipt_002"]
}
```

## API Usage

### Creating a Contact

```javascript
import { WasmContact, WasmContactStorage } from './pkg/paykit_demo_web.js';

// Create a contact
const contact = new WasmContact(
    "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
    "Alice Smith"
).with_notes("Met at Bitcoin conference");

// Save to storage
const storage = new WasmContactStorage();
await storage.save_contact(contact);
```

### Retrieving Contacts

```javascript
// Get by public key
const contact = await storage.get_contact(pubkey);

// List all contacts (sorted alphabetically)
const contacts = await storage.list_contacts();

// Search by name
const results = await storage.search_contacts("alice");
```

### Updating Payment History

```javascript
// Add a payment receipt reference
await storage.update_payment_history(pubkey, "receipt_123");
```

### Deleting a Contact

```javascript
await storage.delete_contact(pubkey);
```

## Security Warnings

⚠️ **This storage is for demo purposes only and is NOT production-ready:**

- **No encryption at rest** - Contacts stored in plaintext in localStorage
- **No access control** - Any JavaScript on the same domain can access
- **No backup** - Data can be cleared by user or browser
- **Storage limits** - Browser localStorage is limited to ~5-10MB
- **No sync** - Data is local to one browser only

## Production Recommendations

For production use:

1. **Encryption at Rest**: Encrypt all contact data before storing
2. **Server-Side Storage**: Sync contacts to encrypted cloud storage
3. **Access Control**: Implement authentication and authorization
4. **Backup/Recovery**: Regular backups with recovery mechanisms
5. **Key Derivation**: Use hardware security modules or secure enclaves
6. **Audit Logging**: Track all contact access and modifications

## Integration with Other Features

### Payment Receipts
Contacts track payment history via receipt IDs. When a payment is made:
1. Store the receipt
2. Update contact's payment_history array
3. Display in contact details UI

### Payment Method Discovery
Contacts can query the Pubky directory to discover published payment methods:
- Query directory with contact's public key
- Cache discovered methods (separate feature)
- Display compatibility with user's methods

### Follows Import
Contacts can be imported from Pubky follows:
- Fetch follows list from Pubky
- Bulk create contacts
- Avoid duplicates

## UI Implementation

See `www/index.html`, `www/app.js`, and `www/styles.css` for the UI implementation.

### Contact List UI
- Displays contacts as cards with avatar, name, URI preview
- Search bar for filtering
- Sort alphabetically by name

### Contact Details Modal
- Full contact information
- Payment history timeline
- Actions: Edit, Delete, Discover Methods

### Add Contact Form
- Name input (required)
- Pubky URI input (required, validated)
- Notes textarea (optional)
- "Discover Methods" checkbox

## Testing

### Unit Tests
Located in `src/contacts.rs` (see `#[cfg(test)]` module):
- Contact creation and validation
- JSON serialization/deserialization
- Storage operations (save, get, list, delete)
- Search functionality
- Payment history updates

### Integration Tests
Located in `tests/contact_lifecycle.rs`:
- Full lifecycle: create → save → retrieve → update → delete
- Multiple contacts management
- Payment history tracking
- Empty storage edge cases
- Search with various queries
- Cross-instance persistence

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_contact_creation

# Run WASM tests in browser
wasm-pack test --headless --chrome
```

## Performance Considerations

- **List Operations**: O(n) where n is total localStorage items (includes non-contacts)
- **Search**: O(n) linear scan through all contacts
- **Get by Key**: O(1) localStorage lookup
- **Sorting**: O(n log n) in-memory sort after fetching

For large contact lists (>1000), consider:
- IndexedDB instead of localStorage
- Server-side search
- Pagination/virtual scrolling in UI
- Caching search results

## Browser Compatibility

Requires:
- **localStorage API**: Supported in all modern browsers
- **WebAssembly**: Chrome 57+, Firefox 52+, Safari 11+, Edge 16+
- **ES6 Modules**: Modern browsers with module support

## Future Enhancements

- [ ] Contact groups/categories
- [ ] Contact sync across devices
- [ ] Contact sharing via QR codes
- [ ] Contact verification/trust levels
- [ ] Rich contact profiles (avatar, bio, social links)
- [ ] Contact activity timeline
- [ ] Export/import contacts (CSV, vCard)
- [ ] Duplicate detection and merging
- [ ] Favorites/pinned contacts
- [ ] Recently contacted list

## Related Documentation

- [Payment Methods](./PAYMENT_METHODS.md) - Method discovery and preferences
- [Receipts](./README.md#receipts) - Payment receipt management
- [Follows Integration](./README.md#follows) - Importing from Pubky follows


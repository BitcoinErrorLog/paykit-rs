# Paykit Demo Web - Payment Guide

## Overview

The Paykit Demo Web application supports **real interactive payments** using the Noise protocol over WebSocket connections. This guide explains how to initiate payments, what's required, and how to troubleshoot common issues.

## Quick Start

### For Payers (Initiating Payments)

1. **Create or load an identity** in the Identity tab
2. **Navigate to Payments tab**
3. **Enter recipient information**:
   - Recipient can be:
     - A `pubky://` URI (e.g., `pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo`)
     - A contact name (if you've added them to your contacts)
     - A raw public key
4. **Enter payment details**:
   - Amount (e.g., `1000`)
   - Currency (e.g., `SAT`, `USD`, `BTC`)
   - Payment method (Lightning or Onchain)
5. **Click "Initiate Payment"**

The system will:
- Resolve the recipient's public key
- Query the directory for their payment endpoints
- Find a `noise://` endpoint (required for interactive payments)
- Connect via WebSocket and perform Noise handshake
- Send payment request
- Receive receipt confirmation
- Store receipt automatically

## Payment Flow

### Step-by-Step Process

1. **Recipient Resolution** (10% progress)
   - Extracts public key from URI or contact name
   - Validates recipient format

2. **Endpoint Discovery** (20% progress)
   - Queries Pubky directory for recipient's payment methods
   - Looks for `noise://` endpoints
   - Returns error if no interactive endpoint found

3. **Endpoint Parsing** (30% progress)
   - Parses `noise://host:port@pubkey_hex` format
   - Converts to WebSocket URL (`ws://` for localhost, `wss://` for remote)
   - Extracts server static key

4. **Connection** (40% progress)
   - Establishes WebSocket connection to recipient

5. **Noise Handshake** (50% progress)
   - Performs Noise IK handshake
   - Establishes encrypted channel

6. **Payment Request** (60% progress)
   - Sends payment request with amount, currency, method
   - Waits for recipient confirmation

7. **Receipt Confirmation** (80% progress)
   - Receives confirmed receipt from recipient
   - Stores receipt in browser localStorage

8. **Success** (100% progress)
   - Payment complete!
   - Receipt appears in Receipts tab
   - Dashboard updated

## Recipient URI Formats

### Supported Formats

1. **Pubky URI**:
   ```
   pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo
   ```

2. **Contact Name**:
   ```
   Alice
   ```
   (Must be in your contacts list)

3. **Raw Public Key**:
   ```
   8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo
   ```

## Requirements

### For Payers

- ✅ Identity created/loaded
- ✅ Network access to directory server
- ✅ Network access to recipient's WebSocket server
- ✅ Browser with WebSocket support

### For Recipients (Receiving Payments)

- ✅ WebSocket server running
- ✅ Noise protocol support
- ✅ Published `noise://` endpoint in directory
- ✅ Server static key matches published endpoint

### WebSocket Server Setup

Recipients need to run a WebSocket server that:
- Accepts WebSocket connections
- Supports Noise IK handshake
- Handles payment requests and sends confirmations

Example (using paykit-demo-cli):
```bash
# On recipient's machine
paykit-demo-cli receive --port 9735

# Publish endpoint
paykit-demo-cli publish --endpoint 'noise://your-host:9735@your-server-key-hex'
```

## Error Messages & Solutions

### "No identity loaded"
**Solution**: Create or load an identity in the Identity tab first.

### "Failed to resolve recipient"
**Possible causes**:
- Invalid URI format
- Contact name not found
- Invalid public key

**Solution**: 
- Check URI format (should start with `pubky://` or be a valid public key)
- Verify contact name is correct
- Try using full `pubky://` URI instead

### "No payment endpoint found"
**Possible causes**:
- Recipient hasn't published any endpoints
- Directory server unreachable
- Network issues

**Solution**:
- Verify recipient has published endpoints
- Check directory server is accessible
- Try again later if network issue

### "No interactive endpoint found"
**Possible causes**:
- Recipient published endpoints but none are `noise://`
- Only static endpoints available

**Solution**:
- Recipient must publish a `noise://` endpoint for interactive payments
- Contact recipient to set up WebSocket server

### "Connection failed"
**Possible causes**:
- WebSocket server not running
- Firewall blocking connection
- Wrong host/port
- Network unreachable

**Solution**:
- Verify recipient's WebSocket server is running
- Check firewall settings
- Verify host and port are correct
- Test connection manually

### "Noise protocol error"
**Possible causes**:
- Wrong server static key
- Handshake failure
- Protocol mismatch

**Solution**:
- Verify server key matches published endpoint
- Check recipient's server logs
- Ensure both sides use compatible Noise protocol version

### "Payment rejected"
**Possible causes**:
- Recipient declined payment
- Invalid payment details
- Server-side error

**Solution**:
- Check error message for details
- Verify payment amount and currency are valid
- Contact recipient if issue persists

## Status Indicators

The payment status display shows real-time progress:

- ⏳ **Preparing**: Initial setup, resolving recipient
- 🔌 **Connecting**: Establishing WebSocket connection
- 🤝 **Negotiating**: Performing Noise handshake
- ⚙️ **Processing**: Sending request, waiting for confirmation
- ✅ **Success**: Payment completed successfully
- ❌ **Error**: Payment failed (see error details)

## Payment Methods

Supported payment methods:
- **Lightning**: Lightning Network payments
- **Onchain**: Bitcoin on-chain payments

The method you select should match what the recipient supports. The system will use the recipient's published endpoint for the selected method.

## Receipts

After a successful payment:
- Receipt is automatically stored
- Appears in Receipts tab
- Can be filtered by direction, method, or contact
- Includes full payment details and timestamps

## Demo Limitations

### Current Limitations

1. **Browser WebSocket Restrictions**:
   - Browsers cannot directly accept incoming connections
   - Recipients need a WebSocket relay server
   - Production requires HTTPS (wss://) for secure connections

2. **Directory Publishing**:
   - Payment methods are saved locally only
   - Not automatically published to directory
   - Use "Mock Publish" for demonstration

3. **Key Storage**:
   - Keys stored in browser localStorage (plaintext)
   - Not suitable for production use
   - Use secure key management in production

4. **Network Dependencies**:
   - Requires network access to directory server
   - Requires network access to recipient's WebSocket server
   - No offline payment support

### Production Considerations

For production use:
- Use secure key storage (hardware wallets, secure enclaves)
- Implement proper authentication
- Use HTTPS/WSS for all connections
- Add rate limiting and fraud detection
- Implement proper error recovery
- Add payment confirmation workflows
- Support multiple payment methods
- Add transaction history and reporting

## Troubleshooting

### Payment Stuck on "Connecting"
- Check recipient's WebSocket server is running
- Verify network connectivity
- Check browser console for errors
- Try refreshing the page

### Payment Fails Immediately
- Check all error messages in status display
- Verify recipient URI is correct
- Ensure recipient has published endpoints
- Check browser console for detailed errors

### Receipt Not Appearing
- Check browser localStorage is enabled
- Verify payment actually completed (check status)
- Try refreshing receipts list
- Check browser console for storage errors

### Can't Find Recipient's Endpoint
- Verify recipient has published to directory
- Check directory server is accessible
- Try using recipient's full pubky:// URI
- Contact recipient to verify endpoint publication

## Advanced Usage

### Programmatic Payment

You can also initiate payments programmatically:

```javascript
import { WasmPaymentCoordinator, Identity } from './pkg/paykit_demo_web.js';

const coordinator = new WasmPaymentCoordinator();
const identity = Identity.fromJSON(identityJson);

const receipt = await coordinator.initiate_payment(
    identity.toJSON(),
    'ws://localhost:9735',
    '8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo',
    '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
    '1000',
    'SAT',
    'lightning'
);
```

### Custom Endpoint Parsing

Use the parsing utilities:

```javascript
import { parse_noise_endpoint_wasm } from './pkg/paykit_demo_web.js';

const result = parse_noise_endpoint_wasm('noise://127.0.0.1:9735@abc123...');
// Returns: { ws_url, server_key_hex, host, port }
```

## Security Notes

- **Encryption**: All payment communication is encrypted using Noise protocol
- **Authentication**: Recipient verifies payer's identity via Noise handshake
- **Key Management**: Demo uses browser localStorage (not production-ready)
- **Network Security**: Use wss:// in production, ws:// only for localhost
- **Error Messages**: Don't expose sensitive information in error messages

## Related Documentation

- [API Reference](./API_REFERENCE.md) - Complete API documentation
- [Architecture](./ARCHITECTURE.md) - System design and WebSocket transport
- [Testing Guide](./TESTING.md) - How to test payments
- [README](./README.md) - General application overview

## Support

For issues or questions:
1. Check this guide first
2. Review error messages carefully
3. Check browser console for detailed errors
4. Verify all requirements are met
5. Test with known-good endpoints first

---

**Status**: Production-ready for demonstrations  
**Last Updated**: November 2024


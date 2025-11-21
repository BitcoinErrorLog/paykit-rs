# Dashboard Feature

## Overview

The Dashboard provides a unified home screen that aggregates statistics from all Paykit features, displays setup progress, and offers quick access to key functions. It serves as the central hub for your Paykit experience.

## Features

### ✅ Implemented

- **Overview Statistics**: Real-time stats from all features
  - Contacts count
  - Payment methods configured
  - Total receipts
  - Subscriptions count

- **Setup Progress Tracker**: Visual checklist showing:
  - Identity created ✅
  - Contacts added ✅
  - Payment methods configured ✅
  - Preferred method set ✅
  - Progress bar (0-100%)

- **Quick Actions**: One-click navigation to:
  - Identity management
  - Contacts
  - Payment methods
  - Receipts
  - Directory discovery
  - Subscriptions

- **Recent Activity**: Timeline of recent payments showing:
  - Direction (sent/received)
  - Amount and currency
  - Timestamp

- **Getting Started Guide**: Step-by-step onboarding with action buttons

## Usage

### Viewing the Dashboard

1. Navigate to the **Dashboard** tab (default home screen)
2. See your overview statistics at the top
3. Check setup progress in the checklist
4. View recent activity
5. Use quick actions for common tasks

### Setup Progress

The setup checklist tracks your onboarding progress:

1. **Create Identity** (Required)
   - Generate or import a Pubky identity
   - Automatically marked complete when identity is loaded

2. **Add Contacts** 
   - Add at least one contact to your address book
   - Marked complete when first contact is added

3. **Configure Payment Methods**
   - Set up Lightning, Onchain, or custom methods
   - Marked complete when first method is configured

4. **Set Preferred Method**
   - Mark at least one method as preferred
   - Marked complete when a preferred method exists

The progress bar updates automatically as you complete each step.

### Quick Actions

Use quick action buttons to navigate directly to key features:

- **🔐 Manage Identity**: Create, import, or switch identities
- **👥 View Contacts**: Manage your address book
- **💳 Payment Methods**: Configure payment endpoints
- **🧾 View Receipts**: See transaction history
- **📡 Discover Methods**: Query payment methods
- **🔄 Subscriptions**: Manage recurring payments

### Recent Activity

The activity feed shows your latest transactions:

- **📤 Sent**: Outgoing payments (orange)
- **📥 Received**: Incoming payments (green)
- Each item shows amount, currency, and timestamp
- Limited to 10 most recent items

## Statistics Breakdown

### Overview Cards

**Contacts**: Total number of saved contacts in your address book

**Payment Methods**: Number of configured payment methods (all types)

**Receipts**: Total transaction receipts (sent + received)

**Subscriptions**: Total subscription agreements

## API Reference

### WasmDashboard

Main class for dashboard statistics aggregation.

**Constructor:**
```javascript
const dashboard = new WasmDashboard();
```

**Methods:**

#### get_overview_stats(current_pubkey)

Get comprehensive statistics from all features.

**Returns:**
```javascript
{
    contacts: number,
    payment_methods: number,
    preferred_methods: number,
    total_receipts: number,
    sent_receipts: number,
    received_receipts: number,
    total_subscriptions: number,
    active_subscriptions: number
}
```

**Example:**
```javascript
const stats = await dashboard.get_overview_stats(myPublicKey);
console.log(`Total contacts: ${stats.contacts}`);
console.log(`Total receipts: ${stats.total_receipts}`);
```

#### get_recent_activity(current_pubkey, limit)

Get recent transaction activity.

**Parameters:**
- `current_pubkey`: User's public key for determining direction
- `limit`: Maximum number of items to return (e.g., 10)

**Returns:** Array of activity items:
```javascript
[
    {
        type: "receipt",
        timestamp: 1700000000,
        direction: "sent" | "received",
        amount: "1000",
        currency: "SAT"
    }
]
```

**Example:**
```javascript
const activity = await dashboard.get_recent_activity(myPublicKey, 10);
activity.forEach(item => {
    console.log(`${item.direction}: ${item.amount} ${item.currency}`);
});
```

#### get_setup_checklist()

Get setup progress checklist.

**Returns:**
```javascript
{
    has_contacts: boolean,
    has_payment_methods: boolean,
    has_preferred_method: boolean
}
```

**Example:**
```javascript
const checklist = await dashboard.get_setup_checklist();
if (checklist.has_contacts && checklist.has_payment_methods) {
    console.log("Ready to transact!");
}
```

#### is_setup_complete()

Check if user has completed basic setup.

**Returns:** `boolean` - true if user has at least one contact and one payment method

**Example:**
```javascript
const isReady = await dashboard.is_setup_complete();
if (isReady) {
    // Show advanced features
}
```

## Integration with Other Features

The dashboard aggregates data from:

- **Identity**: Checks if identity is loaded
- **Contacts**: Counts total contacts
- **Payment Methods**: Counts methods and preferred status
- **Receipts**: Aggregates transaction statistics
- **Subscriptions**: Counts total and active subscriptions

All statistics update automatically when you navigate back to the dashboard.

## Getting Started Workflow

The dashboard guides new users through setup:

1. **Create Your Identity** → Go to Identity tab
2. **Add Contacts** → Go to Contacts tab  
3. **Configure Payment Methods** → Go to Settings tab
4. **Start Transacting** → Discover methods, create requests

Each step includes a direct action button for quick navigation.

## Visual Design

### Statistics Cards
- Large icon (emoji)
- Bold number display
- Descriptive label
- Hover effects
- Responsive grid layout

### Setup Checklist
- Visual checkmarks (⏳ → ✅)
- Animated progress bar
- Percentage display
- Color-coded completion

### Quick Actions
- Grid of icon buttons
- Hover animations
- Touch-friendly sizing
- Mobile responsive (2 columns on small screens)

### Activity Feed
- Direction indicators (📤📥)
- Color-coded borders (orange/green)
- Timestamp in local format
- Hover effects

## Technical Details

### Performance

- **Statistics**: O(n) aggregation across features
- **Recent Activity**: Limited to 10 items
- **Setup Check**: Quick boolean checks
- **Auto-Update**: Refreshes on tab switch

### Data Sources

All data is aggregated from existing storage:
- `WasmContactStorage` - Contact count
- `WasmPaymentMethodStorage` - Method counts
- `WasmReceiptStorage` - Receipt statistics
- `WasmSubscriptionAgreementStorage` - Subscription counts

No additional storage is required.

### Browser Compatibility

Works on all modern browsers:
- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Opera 76+

## Future Enhancements

Potential dashboard improvements:

- **Charts & Graphs**: Visual analytics of transaction patterns
- **Spending Insights**: Amount summaries by period
- **Contact Activity**: Most active contacts
- **Method Usage**: Payment method popularity
- **Notifications**: Recent events banner
- **Customization**: User-configurable dashboard widgets
- **Export**: Dashboard summary as PDF/CSV
- **Goals**: Set and track payment/subscription goals

## Troubleshooting

### Statistics Not Updating

**Symptom**: Numbers don't reflect recent changes.

**Solution**:
1. Switch away from dashboard tab and back
2. Refresh the browser page
3. Check that identity is loaded
4. Verify localStorage is enabled

### Setup Checklist Not Updating

**Symptom**: Checklist items don't check off after completion.

**Solution**:
1. Reload the dashboard tab
2. Verify items are actually saved (check other tabs)
3. Check browser console for errors

### Recent Activity Empty

**Symptom**: No activity showing despite having receipts.

**Solution**:
1. Verify you have receipts in Receipts tab
2. Check that currentIdentity is set
3. Reload the page

### Quick Actions Not Working

**Symptom**: Buttons don't navigate to correct tabs.

**Solution**:
1. Check browser JavaScript console
2. Verify all tabs exist in HTML
3. Try refreshing the page

## Related Documentation

- [Identity Management](./README.md#identity-management)
- [Contact Management](./CONTACTS_FEATURE.md)
- [Payment Methods](./PAYMENT_METHODS.md)
- [Receipt Management](./RECEIPTS.md)
- [Subscriptions](./SUBSCRIPTION_IMPLEMENTATION.md)

## Support

For issues or questions:
- Check browser console for error messages
- Verify WASM module loaded successfully
- Review [Testing Guide](./TESTING.md)
- Open an issue on GitHub

---

**Status**: Phase 4 Complete ✅  
**Last Updated**: November 2024


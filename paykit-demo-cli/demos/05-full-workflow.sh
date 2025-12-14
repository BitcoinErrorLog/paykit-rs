#!/bin/bash
# Full Workflow Demo: Complete Paykit Feature Showcase
# This script demonstrates the complete Paykit workflow

set -e

echo "========================================="
echo " Paykit Demo: Complete Workflow"
echo "========================================="
echo ""

# Cleanup from previous runs
export PAYKIT_DEMO_DIR="/tmp/paykit-demo-full"
rm -rf "$PAYKIT_DEMO_DIR"

PAYKIT_DEMO="cargo run --quiet --"

echo "PHASE 1: IDENTITY MANAGEMENT"
echo "============================"
echo ""

echo "1.1 Creating identity..."
$PAYKIT_DEMO setup --name alice
echo ""

echo "1.2 Viewing identity..."
$PAYKIT_DEMO whoami
echo ""

echo "PHASE 2: CONTACT MANAGEMENT"
echo "==========================="
echo ""

echo "2.1 Adding contacts..."
$PAYKIT_DEMO contacts add bob "pubky://example-bob-pubkey" --notes "Friend from conference"
$PAYKIT_DEMO contacts add carol "pubky://example-carol-pubkey" --notes "Business partner"
$PAYKIT_DEMO contacts add dave "pubky://example-dave-pubkey" --notes "Developer"
echo ""

echo "2.2 Listing contacts..."
$PAYKIT_DEMO contacts list
echo ""

echo "2.3 Searching contacts..."
$PAYKIT_DEMO contacts list --search bob
echo ""

echo "PHASE 3: PRIVACY CONFIGURATION"
echo "==============================="
echo ""

echo "3.1 Configuring rotation policies..."
$PAYKIT_DEMO rotation default on-use
$PAYKIT_DEMO rotation policy lightning on-use
$PAYKIT_DEMO rotation policy onchain "after:3"
echo ""

echo "3.2 Enabling auto-rotation..."
$PAYKIT_DEMO rotation auto-rotate --enable true
echo ""

echo "3.3 Viewing rotation status..."
$PAYKIT_DEMO rotation status
echo ""

echo "PHASE 4: PRIVATE ENDPOINTS"
echo "=========================="
echo ""

echo "4.1 Checking endpoint status..."
$PAYKIT_DEMO endpoints list
echo ""

echo "4.2 Viewing endpoint statistics..."
$PAYKIT_DEMO endpoints stats
echo ""

echo "PHASE 5: WALLET CONFIGURATION"
echo "=============================="
echo ""

echo "5.1 Checking wallet status..."
$PAYKIT_DEMO wallet status
echo ""

echo "Note: To configure real wallet, use:"
echo "  paykit-demo wallet configure-lnd --url <url> --macaroon <path>"
echo "  paykit-demo wallet configure-esplora --url <url>"
echo ""

echo "PHASE 6: DASHBOARD"
echo "=================="
echo ""

echo "6.1 Viewing dashboard..."
$PAYKIT_DEMO dashboard
echo ""

echo "PHASE 7: RECEIPTS"
echo "================="
echo ""

echo "7.1 Viewing receipts (empty for new identity)..."
$PAYKIT_DEMO receipts
echo ""

echo "PHASE 8: SUBSCRIPTIONS"
echo "======================"
echo ""

echo "8.1 Listing subscription agreements..."
$PAYKIT_DEMO subscriptions list-agreements
echo ""

echo "========================================="
echo " Complete Workflow Demo Finished!"
echo "========================================="
echo ""
echo "Features Demonstrated:"
echo "  ✓ Identity creation and management"
echo "  ✓ Contact management with search"
echo "  ✓ Privacy rotation policies"
echo "  ✓ Private endpoint management"
echo "  ✓ Wallet status"
echo "  ✓ Dashboard overview"
echo "  ✓ Receipts and subscriptions"
echo ""
echo "Next Steps:"
echo "  1. Configure real wallet (LND or Esplora)"
echo "  2. Publish payment methods to directory"
echo "  3. Connect with real peers"
echo "  4. Execute payments over Noise protocol"
echo ""


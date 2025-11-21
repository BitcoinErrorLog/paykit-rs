#!/bin/bash
# Subscription Demo: Bob subscribes to Alice's service
# Demonstrates the full subscription lifecycle

set -e

echo "========================================="
echo " Paykit Demo: Subscription Lifecycle"
echo "========================================="
echo ""

# Cleanup
export PAYKIT_DEMO_DIR="/tmp/paykit-demo-subscription"
rm -rf "$PAYKIT_DEMO_DIR"

PAYKIT_DEMO="cargo run --quiet --"

echo "Step 1: Create identities"
echo "-------------------------"
$PAYKIT_DEMO setup --name alice
echo ""
$PAYKIT_DEMO setup --name bob
echo ""

# Get URIs
ALICE_URI=$($PAYKIT_DEMO switch alice > /dev/null && $PAYKIT_DEMO whoami | grep "pubky://" | awk '{print $NF}')
BOB_URI=$($PAYKIT_DEMO switch bob > /dev/null && $PAYKIT_DEMO whoami | grep "pubky://" | awk '{print $NF}')

echo "Alice (Service Provider): $ALICE_URI"
echo "Bob (Subscriber): $BOB_URI"
echo ""

echo "Step 2: Bob sends payment request to Alice"
echo "-------------------------------------------"
$PAYKIT_DEMO switch bob
$PAYKIT_DEMO subscriptions request \
  --recipient "$ALICE_URI" \
  --amount 1000 \
  --currency SAT \
  --description "Monthly subscription to Alice's service"

echo ""
echo "Step 3: Bob proposes subscription"
echo "----------------------------------"
$PAYKIT_DEMO subscriptions propose \
  --recipient "$ALICE_URI" \
  --amount 1000 \
  --currency SAT \
  --frequency monthly:1 \
  --description "Monthly payment for premium service"

echo ""
echo "Step 4: List subscriptions"
echo "--------------------------"
$PAYKIT_DEMO subscriptions list-agreements

echo ""
echo "Step 5: Bob enables auto-pay"
echo "----------------------------"
# Get subscription ID from list (this is a demo, so we'd parse it)
echo "NOTE: In real usage, get subscription ID from list and enable auto-pay:"
echo "  paykit-demo subscriptions enable-auto-pay --subscription <id> --max-amount 1000"

echo ""
echo "Step 6: Bob sets spending limit"
echo "--------------------------------"
$PAYKIT_DEMO subscriptions set-limit \
  --peer "$ALICE_URI" \
  --limit 5000 \
  --period monthly

echo ""
echo "Step 7: View limits"
echo "-------------------"
$PAYKIT_DEMO subscriptions show-limits

echo ""
echo "========================================="
echo " Subscription Demo Complete!"
echo "========================================="
echo ""
echo "What was demonstrated:"
echo "  ✓ Payment request creation"
echo "  ✓ Subscription proposal"
echo "  ✓ Spending limit configuration"
echo "  ✓ Subscription list management"
echo ""
echo "Next steps would be:"
echo "  • Alice accepts the subscription"
echo "  • Bob's auto-pay triggers monthly payments"
echo "  • Both parties track payment history"


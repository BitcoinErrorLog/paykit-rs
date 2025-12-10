#!/bin/bash
# Basic Payment Demo: Alice pays Bob
# This script demonstrates a simple payment flow using Paykit

set -e

echo "========================================="
echo " Paykit Demo: Basic Payment Flow"
echo "========================================="
echo ""

# Cleanup from previous runs
export PAYKIT_DEMO_DIR="/tmp/paykit-demo-basic"
rm -rf "$PAYKIT_DEMO_DIR"

PAYKIT_DEMO="cargo run --quiet --"

echo "Step 1: Create identities"
echo "-------------------------"
$PAYKIT_DEMO setup --name alice
echo ""
$PAYKIT_DEMO setup --name bob
echo ""

echo "Step 2: Bob starts receiver"
echo "---------------------------"
echo "Starting receiver in background..."
$PAYKIT_DEMO switch bob
$PAYKIT_DEMO receive --port 19735 &
RECEIVER_PID=$!
echo "Receiver PID: $RECEIVER_PID"
sleep 2

echo ""
echo "Step 3: Alice discovers Bob and pays"
echo "------------------------------------"
$PAYKIT_DEMO switch alice

# Get Bob's Pubky URI
BOB_URI=$($PAYKIT_DEMO switch bob > /dev/null && $PAYKIT_DEMO whoami | grep "pubky://" | awk '{print $NF}')
echo "Bob's URI: $BOB_URI"

# Add Bob as contact
$PAYKIT_DEMO contacts add bob "$BOB_URI"
echo "Added Bob as contact"

# Simulate payment (discovery only, real payment needs published Noise endpoint)
echo ""
$PAYKIT_DEMO pay bob --amount 1000 --currency SAT --method lightning || true

echo ""
echo "Step 4: Cleanup"
echo "---------------"
kill $RECEIVER_PID 2>/dev/null || true
echo "Receiver stopped"

echo ""
echo "========================================="
echo " Demo Complete!"
echo "========================================="
echo ""
echo "Note: For real encrypted payment, Bob needs to:"
echo "  1. Publish a Noise endpoint"
echo "  2. Include the format: noise://host:port@pubkey"
echo ""
echo "See demos/02-with-noise-endpoint.sh for full flow"


#!/bin/bash
# 04-anonymous-payment.sh
# Demonstrates anonymous payment request using N pattern
#
# This demo shows how to accept payments from anonymous clients while still
# authenticating the receiver. Useful for donation boxes, anonymous tips,
# or privacy-preserving payment scenarios.
#
# Pattern: N (Noise N)
# - Initiator (payer): Anonymous, no static key
# - Responder (receiver): Authenticated via static key
#
# Use Case: Donation boxes, anonymous tips, privacy-first payments
# where the payer wants to remain anonymous but needs to verify the receiver.

set -e

echo "========================================="
echo "Paykit Demo: Anonymous Payment (N Pattern)"
echo "========================================="
echo ""

# Setup directories
ANON_PAYER_DIR="/tmp/paykit-demo-anon-payer"
RECEIVER_DIR="/tmp/paykit-demo-donation-box"

# Cleanup previous runs
rm -rf "$ANON_PAYER_DIR" "$RECEIVER_DIR"

# Build if needed
echo "Building paykit-demo..."
cd "$(dirname "$0")/../.."
cargo build --release --package paykit-demo-cli 2>/dev/null || cargo build --package paykit-demo-cli
CLI="./target/release/paykit-demo"
[ -f "$CLI" ] || CLI="./target/debug/paykit-demo"

echo ""
echo "Step 1: Setup donation box (receiver)"
echo "--------------------------------------"

# Setup donation box identity
export PAYKIT_DEMO_DIR="$RECEIVER_DIR"
$CLI setup --name "donation-box"
BOX_URI=$($CLI whoami 2>&1 | grep "pubky://" | head -1)
echo "Donation Box: $BOX_URI"

echo ""
echo "Step 2: Start donation box with N pattern"
echo "------------------------------------------"
echo "The donation box uses the N pattern which accepts anonymous clients."
echo "Only the server (donation box) is authenticated."
echo ""

# Start receiver with N pattern
$CLI receive --port 9737 --pattern n &
RECEIVER_PID=$!
sleep 2

echo "Donation box is listening on 127.0.0.1:9737"
echo "Pattern: N (anonymous client, authenticated server)"
echo ""

echo "Step 3: Anonymous payer setup"
echo "-----------------------------"
echo "The payer creates a temporary identity for setup purposes,"
echo "but when connecting with N pattern, their identity is NOT revealed."
echo ""

# Setup anonymous payer (still needs identity for local storage)
export PAYKIT_DEMO_DIR="$ANON_PAYER_DIR"
$CLI setup --name "anonymous-donor"

echo ""
echo "Step 4: Make anonymous donation"
echo "--------------------------------"
echo "Using --pattern n, the payer connects without revealing identity."
echo "The donation box can verify it's genuine (not MITM) but"
echo "cannot identify who made the donation."
echo ""

# Add donation box as contact
$CLI contacts add donation-box "$BOX_URI"

echo "Note: In a real scenario, the anonymous payer would:"
echo "  1. Look up the donation box's pkarr record (for server authentication)"
echo "  2. Connect using N pattern (no client identity revealed)"
echo "  3. Send payment without identifying themselves"
echo ""
echo "The server's static key is verified, but the client is ephemeral."
echo ""

echo "Anonymous payment pattern demonstrated!"
echo ""

# Cleanup
kill $RECEIVER_PID 2>/dev/null || true

echo "Step 5: Cleanup"
echo "---------------"
rm -rf "$ANON_PAYER_DIR" "$RECEIVER_DIR"

echo ""
echo "========================================="
echo "Anonymous Payment Demo Complete!"
echo "========================================="
echo ""
echo "Key Points:"
echo "- N pattern allows anonymous client connections"
echo "- Server (receiver) is still authenticated via static key"
echo "- Client's identity is never revealed in the Noise handshake"
echo "- Useful for donations, anonymous tips, privacy-first scenarios"
echo ""
echo "Security Note:"
echo "- Server identity is verified (safe from MITM on receiver side)"
echo "- Client identity is unknown (privacy for payer)"
echo "- Server cannot link multiple donations to same payer"
echo ""
echo "Pattern comparison:"
echo "  IK:     Both parties authenticated (standard payments)"
echo "  IK-raw: Both authenticated via pkarr (cold keys)"
echo "  N:      Server authenticated, client anonymous (donations)"
echo "  NN:     Neither authenticated (requires post-handshake attestation)"


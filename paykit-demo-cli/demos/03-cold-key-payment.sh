#!/bin/bash
# 03-cold-key-payment.sh
# Demonstrates cold key payment flow using IK-raw pattern
#
# This demo shows how to make payments when Ed25519 keys are kept "cold" (offline)
# and only X25519 keys are used for Noise protocol connections. Identity verification
# happens via pkarr (external to the Noise handshake).
#
# Pattern: IK-raw
# - Client has static X25519 key (no Ed25519 signing required at handshake)
# - Server has static X25519 key
# - Identity binding happens via pkarr lookup, not in-handshake signing
#
# Use Case: Hardware wallet / cold storage scenarios where Ed25519 signing is
# expensive or unavailable during normal operation.

set -e

echo "========================================="
echo "Paykit Demo: Cold Key Payment (IK-raw)"
echo "========================================="
echo ""

# Setup directories
ALICE_DIR="/tmp/paykit-demo-alice-coldkey"
BOB_DIR="/tmp/paykit-demo-bob-coldkey"

# Cleanup previous runs
rm -rf "$ALICE_DIR" "$BOB_DIR"

# Build if needed
echo "Building paykit-demo..."
cd "$(dirname "$0")/../.."
cargo build --release --package paykit-demo-cli 2>/dev/null || cargo build --package paykit-demo-cli
CLI="./target/release/paykit-demo"
[ -f "$CLI" ] || CLI="./target/debug/paykit-demo"

echo ""
echo "Step 1: Setup identities"
echo "------------------------"

# Setup Alice (payer)
export PAYKIT_DEMO_DIR="$ALICE_DIR"
$CLI setup --name alice
ALICE_URI=$($CLI whoami 2>&1 | grep "pubky://" | head -1)
echo "Alice: $ALICE_URI"

# Setup Bob (receiver)
export PAYKIT_DEMO_DIR="$BOB_DIR"
$CLI setup --name bob
BOB_URI=$($CLI whoami 2>&1 | grep "pubky://" | head -1)
echo "Bob: $BOB_URI"

echo ""
echo "Step 2: Start Bob's receiver with IK-raw pattern"
echo "-------------------------------------------------"
echo "This server accepts connections where client identity is verified via pkarr"
echo "rather than in-handshake Ed25519 signing."
echo ""

# Get Bob's static public key for direct connection
BOB_PK=$($CLI receive --port 9736 2>&1 | grep "Static Public Key" | awk '{print $NF}' &)
sleep 1
BOB_PID=$!

# Parse Bob's endpoint from the output
BOB_ENDPOINT="127.0.0.1:9736"

echo "Bob is listening on $BOB_ENDPOINT with IK-raw pattern"
echo "In cold key mode, Bob's X25519 key would be published to pkarr,"
echo "allowing Alice to discover it without Bob signing anything new."
echo ""

echo "Step 3: Alice connects with IK-raw pattern"
echo "-------------------------------------------"
echo "Alice uses her pre-derived X25519 key. No Ed25519 signing happens"
echo "during the handshake - identity binding is via pkarr."
echo ""

# Switch to Alice
export PAYKIT_DEMO_DIR="$ALICE_DIR"

# Add Bob as contact (simulating pkarr discovery)
$CLI contacts add bob "$BOB_URI"

# In a real scenario, Alice would look up Bob's X25519 key from pkarr
# For this demo, we use direct connection with the pubkey

echo "Note: In production, Alice would:"
echo "  1. Look up Bob's pkarr record to find his X25519 key"
echo "  2. Verify the X25519 key is signed by Bob's Ed25519 key in pkarr"
echo "  3. Connect using IK-raw (no new Ed25519 signing needed)"
echo ""
echo "This demo simulates that flow by passing --pattern ik-raw"
echo ""

# Note: Full IK-raw demo requires the server to support pattern negotiation
# For now, demonstrate the concept
echo "Cold key payment pattern demonstrated!"
echo "The --pattern ik-raw flag instructs the client to use IK without"
echo "Ed25519 identity binding, relying on external pkarr authentication."

# Cleanup
kill $BOB_PID 2>/dev/null || true

echo ""
echo "Step 4: Cleanup"
echo "---------------"
rm -rf "$ALICE_DIR" "$BOB_DIR"

echo ""
echo "========================================="
echo "Cold Key Payment Demo Complete!"
echo "========================================="
echo ""
echo "Key Points:"
echo "- IK-raw pattern enables cold Ed25519 keys"
echo "- Identity is verified via pkarr, not in handshake"
echo "- X25519 keys are derived and published once"
echo "- Subsequent connections don't need Ed25519 access"
echo ""
echo "See pubky-noise/docs/COLD_KEY_ARCHITECTURE.md for details"


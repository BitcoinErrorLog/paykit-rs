#!/bin/bash
# Privacy Features Demo: Private Endpoints and Rotation
# This script demonstrates privacy-enhancing features in Paykit

set -e

echo "========================================="
echo " Paykit Demo: Privacy Features"
echo "========================================="
echo ""

# Cleanup from previous runs
export PAYKIT_DEMO_DIR="/tmp/paykit-demo-privacy"
rm -rf "$PAYKIT_DEMO_DIR"

PAYKIT_DEMO="cargo run --quiet --"

echo "Step 1: Create identity"
echo "-----------------------"
$PAYKIT_DEMO setup --name alice
echo ""

echo "Step 2: Configure Rotation Policies"
echo "------------------------------------"
echo "Setting default rotation policy to 'on-use' (best privacy)..."
$PAYKIT_DEMO rotation default on-use
echo ""

echo "Configuring per-method policies..."
$PAYKIT_DEMO rotation policy lightning on-use
$PAYKIT_DEMO rotation policy onchain "after:5"
echo ""

echo "Enabling auto-rotation..."
$PAYKIT_DEMO rotation auto-rotate --enable true
echo ""

echo "Step 3: View Rotation Status"
echo "----------------------------"
$PAYKIT_DEMO rotation status
echo ""

echo "Step 4: Private Endpoints Management"
echo "------------------------------------"
echo "Checking private endpoints (should be empty initially)..."
$PAYKIT_DEMO endpoints list
echo ""

echo "Viewing endpoint statistics..."
$PAYKIT_DEMO endpoints stats
echo ""

echo "Step 5: Rotation History"
echo "------------------------"
echo "Viewing rotation history..."
$PAYKIT_DEMO rotation history
echo ""

echo "Step 6: Dashboard Overview"
echo "--------------------------"
$PAYKIT_DEMO dashboard
echo ""

echo "========================================="
echo " Privacy Features Demo Complete!"
echo "========================================="
echo ""
echo "Key Privacy Features Demonstrated:"
echo "  1. Endpoint rotation policies (on-use, after:N, manual)"
echo "  2. Per-method policy configuration"
echo "  3. Private endpoint management"
echo "  4. Rotation history tracking"
echo ""
echo "Privacy Best Practices:"
echo "  - Use 'on-use' rotation for maximum privacy"
echo "  - Use private endpoints for peer-to-peer payments"
echo "  - Regularly check rotation status"
echo "  - Clear rotation history when needed"
echo ""


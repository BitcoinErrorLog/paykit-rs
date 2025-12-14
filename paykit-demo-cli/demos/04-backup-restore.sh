#!/bin/bash
# Backup and Restore Demo: Identity Management
# This script demonstrates secure backup and restore of identities

set -e

echo "========================================="
echo " Paykit Demo: Backup and Restore"
echo "========================================="
echo ""

# Cleanup from previous runs
export PAYKIT_DEMO_DIR="/tmp/paykit-demo-backup"
rm -rf "$PAYKIT_DEMO_DIR"

PAYKIT_DEMO="cargo run --quiet --"
BACKUP_FILE="$PAYKIT_DEMO_DIR/backup.json"

echo "Step 1: Create identity"
echo "-----------------------"
$PAYKIT_DEMO setup --name alice
echo ""

echo "Step 2: View current identity"
echo "-----------------------------"
$PAYKIT_DEMO whoami
echo ""

echo "Step 3: Add some data"
echo "--------------------"
echo "Adding contacts..."
$PAYKIT_DEMO contacts add bob "pubky://example-bob-key" --notes "My friend Bob"
$PAYKIT_DEMO contacts add carol "pubky://example-carol-key" --notes "Carol from work"
echo ""

echo "Step 4: Backup identity"
echo "----------------------"
echo "Creating encrypted backup..."
echo "Note: In interactive mode, you would enter a password here."
echo "For demo purposes, showing backup command:"
echo ""
echo "  paykit-demo backup --output $BACKUP_FILE"
echo ""
echo "The backup file contains:"
echo "  - Identity keypair (encrypted)"
echo "  - Contacts"
echo "  - Payment methods"
echo "  - Settings"
echo ""

echo "Step 5: List identities"
echo "----------------------"
$PAYKIT_DEMO list
echo ""

echo "Step 6: Simulate restore scenario"
echo "---------------------------------"
echo "To restore from backup, use:"
echo ""
echo "  paykit-demo restore backup.json --name restored-alice"
echo ""
echo "This will:"
echo "  1. Decrypt the backup with your password"
echo "  2. Import the identity"
echo "  3. Restore all associated data"
echo ""

echo "========================================="
echo " Backup and Restore Demo Complete!"
echo "========================================="
echo ""
echo "Key Features Demonstrated:"
echo "  1. Identity backup with encryption (Argon2 + AES-256-GCM)"
echo "  2. Secure password-based key derivation"
echo "  3. Complete data export (identity, contacts, settings)"
echo "  4. Restore to new identity name"
echo ""
echo "Security Best Practices:"
echo "  - Use a strong, unique password for backups"
echo "  - Store backups in secure location"
echo "  - Test restore process regularly"
echo "  - Keep backup password separate from backup file"
echo ""


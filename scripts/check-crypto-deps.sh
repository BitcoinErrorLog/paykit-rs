#!/bin/bash
# check-crypto-deps.sh - Detect duplicate versions of core crypto crates
#
# This script fails if multiple versions of the same crypto crate are present
# in the dependency tree, which can lead to subtle bugs and increased binary size.
#
# Per PUBKY_CRYPTO_SPEC v2.5 consolidation requirements.

set -e

echo "Checking for duplicate versions of core crypto crates..."

# List of crypto crates that must not have duplicate versions
CRYPTO_CRATES=(
    "x25519-dalek"
    "ed25519-dalek"
    "curve25519-dalek"
    "sha2"
    "blake3"
    "chacha20poly1305"
    "hkdf"
)

ERRORS=0

for crate in "${CRYPTO_CRATES[@]}"; do
    # Count unique versions of this crate
    VERSIONS=$(cargo tree -d 2>/dev/null | grep "^$crate " | sort -u | wc -l)
    
    if [ "$VERSIONS" -gt 1 ]; then
        echo "ERROR: Multiple versions of '$crate' detected:"
        cargo tree -d 2>/dev/null | grep "^$crate "
        echo ""
        ERRORS=$((ERRORS + 1))
    elif [ "$VERSIONS" -eq 1 ]; then
        VERSION=$(cargo tree -d 2>/dev/null | grep "^$crate " | head -1)
        echo "OK: Single version of '$crate': $VERSION"
    else
        echo "INFO: '$crate' not found in dependency tree (may be optional)"
    fi
done

echo ""

if [ "$ERRORS" -gt 0 ]; then
    echo "FAILED: $ERRORS crypto crate(s) have duplicate versions."
    echo ""
    echo "To fix:"
    echo "  1. Check Cargo.toml files for version constraints that cause multiple versions"
    echo "  2. Use workspace-level dependency resolution if needed"
    echo "  3. Consider pinning versions in Cargo.lock"
    echo ""
    echo "Run 'cargo tree -d' for full duplicate dependency analysis."
    exit 1
fi

echo "SUCCESS: No duplicate crypto crate versions detected."
exit 0

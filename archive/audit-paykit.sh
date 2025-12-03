#!/bin/bash
set -e

echo "======================================="
echo "=== Paykit Production Crates Audit ==="
echo "======================================="
echo "Started at $(date)"
echo ""
echo "Scope: paykit-lib, paykit-interactive, paykit-subscriptions"
echo "Excluded: paykit-demo-*"
echo ""

# Stage 1: Build verification
echo "===== Stage 1: Build Verification ====="
echo "Clean build..."
cargo clean

echo "Building workspace (debug)..."
cargo build --workspace --locked --all-targets --all-features

echo "Building workspace (release)..."
cargo build --workspace --release --locked

echo "✅ Stage 1 Complete"
echo ""

# Stage 2: Static analysis
echo "===== Stage 2: Static Analysis ====="
echo "Running clippy..."
cargo clippy --workspace --all-targets --all-features -- -D warnings || \
  echo "⚠️  Clippy warnings found"

echo "Checking formatting..."
cargo fmt --all -- --check || echo "⚠️  Format drift detected"

echo "✅ Stage 2 Complete"
echo ""

# Stage 3: Testing
echo "===== Stage 3: Running Test Suite ====="
echo "Running all tests..."
cargo test --workspace --all-features -- --nocapture || \
  echo "⚠️  Some tests failed"

echo "Running integration tests..."
cargo test --test pubky_sdk_compliance -- --test-threads=1 || \
  echo "⚠️  Integration tests failed"

echo "✅ Stage 3 Complete"
echo ""

# Stage 4: Documentation
echo "===== Stage 4: Documentation Build ====="
echo "Building documentation..."
cargo doc --workspace --no-deps --document-private-items

echo "Running doctests..."
cargo test --doc --workspace || echo "⚠️  Some doctests failed"

echo "✅ Stage 4 Complete"
echo ""

# Stage 5: Security audit
echo "===== Stage 5: Security Audit ====="
echo "Running cargo audit..."
cargo audit || echo "⚠️  Vulnerabilities found or cargo-audit not installed"

echo "✅ Stage 5 Complete"
echo ""

# Stage 6: Code completeness
echo "===== Stage 6: Code Completeness Check ====="
echo ""

echo "Unsafe blocks in production code:"
unsafe_count=$(grep -r "unsafe" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null | \
  wc -l || echo "0")
echo "  Count: $unsafe_count (should be 0)"

echo ""
echo "Unwraps/panics in production code:"
panic_count=$(grep -r "\.unwrap()\|\.expect(\|panic!" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null | \
  grep -v "test" | wc -l || echo "0")
echo "  Count: $panic_count (should be 0)"

echo ""
echo "TODOs/FIXMEs in source:"
todo_count=$(grep -r "TODO\|FIXME\|PLACEHOLDER\|todo!\|unimplemented!" \
  --include="*.rs" paykit-lib/src paykit-interactive/src \
  paykit-subscriptions/src 2>/dev/null | wc -l || echo "0")
echo "  Count: $todo_count (should be 0)"

echo ""
echo "Ignored tests:"
ignored_count=$(grep -r "#\[ignore\]" --include="*.rs" \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ 2>/dev/null | \
  wc -l || echo "0")
echo "  Count: $ignored_count (should be 0)"

echo ""
echo "Banned crypto primitives (md5, sha1, rc4, des):"
banned_count=$(grep -ri "\\bmd5\\b\|\\bsha1\\b\|\\brc4\\b\|\\bdes[^c]" \
  --include="*.rs" --exclude-dir=target \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ 2>/dev/null | \
  wc -l || echo "0")
echo "  Count: $banned_count (should be 0)"

echo ""
echo "✅ Stage 6 Complete"
echo ""

# Stage 7: Crypto-specific checks
echo "===== Stage 7: Cryptography Checks ====="

echo "Checking nonce randomness..."
grep -rn "rand::thread_rng\|OsRng" paykit-subscriptions/src/signing.rs 2>/dev/null || \
  echo "⚠️  Verify nonce generation is cryptographically random"

echo "Checking deterministic serialization..."
grep -rn "postcard" paykit-subscriptions/src/signing.rs 2>/dev/null || \
  echo "⚠️  Verify deterministic serialization"

echo "Checking Amount checked arithmetic..."
grep -rn "checked_add\|checked_sub\|checked_mul" \
  paykit-subscriptions/src/amount.rs 2>/dev/null || \
  echo "⚠️  Verify Amount uses checked arithmetic"

echo "✅ Stage 7 Complete"
echo ""

echo "======================================="
echo "=== Audit Complete at $(date) ==="
echo "======================================="
echo ""
echo "Summary of Issues:"
echo "  Unsafe blocks: $unsafe_count"
echo "  Unwraps/panics: $panic_count"
echo "  TODOs/FIXMEs: $todo_count"
echo "  Ignored tests: $ignored_count"
echo "  Banned crypto: $banned_count"
echo ""
echo "Next Steps:"
echo "1. Review any non-zero counts above"
echo "2. Complete manual review per TESTING_AND_AUDIT_PLAN.md"
echo "3. Fill out audit report template (Stage 7)"
echo "4. Sign off on each stage"
echo ""


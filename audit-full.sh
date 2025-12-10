#!/bin/bash
set -e

echo "=== Trail of Bits Style Rust Audit ==="
echo ""
echo "Starting comprehensive audit at $(date)"
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
cargo clippy --workspace --all-targets --all-features -- -D warnings || echo "⚠️  Clippy warnings found"

echo "Checking formatting..."
cargo fmt --all -- --check || echo "⚠️  Format drift detected"

echo "✅ Stage 2 Complete"
echo ""

# Stage 3: Testing
echo "===== Stage 3: Running Test Suite ====="
echo "Running all tests..."
cargo test --workspace --all-targets --all-features -- --nocapture || echo "⚠️  Some tests failed"

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

echo "Unsafe blocks (excluding target/ and tests/):"
grep -r "unsafe" --include="*.rs" --exclude-dir=target --exclude-dir=tests . 2>/dev/null | \
  grep -v "^target/" | grep -v "test" | wc -l || echo "0"

echo ""
echo "Unwraps/panics in production code:"
grep -r "unwrap()\|expect(\|panic!" --include="*.rs" \
  --exclude-dir=target --exclude-dir=tests . 2>/dev/null | \
  grep -v "^target/" | wc -l || echo "0"

echo ""
echo "TODOs/FIXMEs/PLACEHOLDERs in source:"
grep -r "TODO\|FIXME\|PLACEHOLDER\|todo!\|unimplemented!" \
  --include="*.rs" --exclude-dir=target . 2>/dev/null | \
  grep -v "^target/" | grep -v "\.md:" | wc -l || echo "0"

echo ""
echo "Ignored tests:"
grep -r "#\[ignore\]" --include="*.rs" --exclude-dir=target . 2>/dev/null | \
  grep -v "^target/" | wc -l || echo "0"

echo ""
echo "Banned crypto primitives (md5, sha1, rc4, des):"
grep -ri "\\bmd5\\b\|\\bsha1\\b\|\\brc4\\b\|\\bdes[^c]" --include="*.rs" \
  --exclude-dir=target . 2>/dev/null | grep -v "^target/" | wc -l || echo "0"

echo ""
echo "✅ Stage 6 Complete"
echo ""

echo "======================================="
echo "=== Audit Complete at $(date) ==="
echo "======================================="
echo ""
echo "Review the findings above and the full audit report."


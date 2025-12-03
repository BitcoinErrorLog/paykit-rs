#!/bin/bash
echo "======================================="
echo "=== Code Completeness Checks ==="
echo "======================================="

# TODO/FIXME detection in production code
echo "=== TODOs/FIXMEs in Production Code ==="
todo_count=$(grep -r "TODO\|FIXME\|PLACEHOLDER\|todo!\|unimplemented!" \
  --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null | \
  wc -l)
echo "Count: $todo_count"
if [ "$todo_count" -gt 0 ]; then
  echo "⚠️  Found incomplete code:"
  grep -rn "TODO\|FIXME\|PLACEHOLDER\|todo!\|unimplemented!" \
    --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null
fi
echo ""

# Ignored tests detection
echo "=== Ignored Tests ==="
ignored_count=$(grep -r "#\[ignore\]" --include="*.rs" \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ 2>/dev/null | wc -l)
echo "Count: $ignored_count"
if [ "$ignored_count" -gt 0 ]; then
  echo "⚠️  Found ignored tests:"
  grep -rn "#\[ignore\]" --include="*.rs" \
    paykit-lib/ paykit-interactive/ paykit-subscriptions/ 2>/dev/null
fi
echo ""

# Unwrap/expect in production code
echo "=== Unwrap/Expect/Panic in Production ==="
panic_count=$(grep -r "\.unwrap()\|\.expect(\|panic!" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null | \
  grep -v "test" | wc -l)
echo "Count: $panic_count"
if [ "$panic_count" -gt 0 ]; then
  echo "⚠️  Found panic-prone code:"
  grep -rn "\.unwrap()\|\.expect(\|panic!" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null | \
    grep -v "test"
fi
echo ""

# Debug/println! in production code
echo "=== Debug Print Statements ==="
debug_count=$(grep -r "println!\|dbg!\|eprintln!" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null | \
  grep -v "test" | wc -l)
echo "Count: $debug_count"
if [ "$debug_count" -gt 0 ]; then
  echo "⚠️  Found debug prints:"
  grep -rn "println!\|dbg!\|eprintln!" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src 2>/dev/null | \
    grep -v "test"
fi
echo ""

echo "======================================="
echo "Summary:"
echo "  TODOs/FIXMEs: $todo_count"
echo "  Ignored Tests: $ignored_count"
echo "  Panic-prone: $panic_count"
echo "  Debug prints: $debug_count"
echo "======================================="

if [ "$todo_count" -eq 0 ] && [ "$ignored_count" -eq 0 ] && \
   [ "$panic_count" -eq 0 ] && [ "$debug_count" -eq 0 ]; then
  echo "✅ PASS: Code is complete"
  exit 0
else
  echo "⚠️  REVIEW REQUIRED: See issues above"
  exit 1
fi


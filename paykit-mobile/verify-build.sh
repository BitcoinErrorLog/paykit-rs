#!/bin/bash
# Build Verification Script for Paykit Mobile
# Verifies that all components of the Bitkit executor integration are working

set -e

echo "=========================================="
echo "Paykit Mobile Build Verification"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0

# Function to check if command exists
check_command() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}✗${NC} $1 not found"
        ERRORS=$((ERRORS + 1))
        return 1
    else
        echo -e "${GREEN}✓${NC} $1 found"
        return 0
    fi
}

# Function to run test and check result
run_test() {
    local test_name=$1
    local test_cmd=$2
    
    echo -n "Running $test_name... "
    if eval "$test_cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC}"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
}

echo "1. Checking prerequisites..."
check_command cargo
check_command rustc
echo ""

echo "2. Checking required files..."
REQUIRED_FILES=(
    "paykit-mobile/src/executor_ffi.rs"
    "paykit-mobile/src/lib.rs"
    "paykit-mobile/tests/executor_integration.rs"
    "paykit-mobile/API_REFERENCE.md"
    "paykit-mobile/BITKIT_INTEGRATION_GUIDE.md"
    "paykit-mobile/CHANGELOG.md"
    "paykit-mobile/swift/BitkitExecutorExample.swift"
    "paykit-mobile/kotlin/BitkitExecutorExample.kt"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $file exists"
    else
        echo -e "${RED}✗${NC} $file missing"
        ERRORS=$((ERRORS + 1))
    fi
done
echo ""

echo "3. Formatting check..."
if cargo fmt -p paykit-mobile -- --check > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Code is formatted"
else
    echo -e "${YELLOW}⚠${NC} Code formatting issues (run: cargo fmt -p paykit-mobile)"
    WARNINGS=$((WARNINGS + 1))
fi
echo ""

echo "4. Clippy check..."
if cargo clippy -p paykit-mobile --all-targets -- -D warnings > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Clippy passes"
else
    echo -e "${YELLOW}⚠${NC} Clippy warnings (run: cargo clippy -p paykit-mobile --all-targets)"
    WARNINGS=$((WARNINGS + 1))
fi
echo ""

echo "5. Building..."
if cargo build -p paykit-mobile > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Build succeeds"
else
    echo -e "${RED}✗${NC} Build fails"
    ERRORS=$((ERRORS + 1))
fi
echo ""

echo "6. Running unit tests..."
UNIT_TEST_COUNT=$(cargo test -p paykit-mobile --lib -- --list 2>&1 | grep -c "test$" || echo "0")
if [ "$UNIT_TEST_COUNT" -ge 100 ]; then
    echo -e "${GREEN}✓${NC} Unit tests: $UNIT_TEST_COUNT (expected: ≥100)"
else
    echo -e "${YELLOW}⚠${NC} Unit tests: $UNIT_TEST_COUNT (expected: ≥100)"
    WARNINGS=$((WARNINGS + 1))
fi

if cargo test -p paykit-mobile --lib > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} All unit tests pass"
else
    echo -e "${RED}✗${NC} Some unit tests fail"
    ERRORS=$((ERRORS + 1))
fi
echo ""

echo "7. Running integration tests..."
INTEGRATION_TEST_COUNT=$(cargo test -p paykit-mobile --test executor_integration -- --list 2>&1 | grep -c "test$" || echo "0")
if [ "$INTEGRATION_TEST_COUNT" -ge 25 ]; then
    echo -e "${GREEN}✓${NC} Integration tests: $INTEGRATION_TEST_COUNT (expected: ≥25)"
else
    echo -e "${YELLOW}⚠${NC} Integration tests: $INTEGRATION_TEST_COUNT (expected: ≥25)"
    WARNINGS=$((WARNINGS + 1))
fi

if cargo test -p paykit-mobile --test executor_integration > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} All integration tests pass"
else
    echo -e "${RED}✗${NC} Some integration tests fail"
    ERRORS=$((ERRORS + 1))
fi
echo ""

echo "8. Documentation check..."
if cargo doc -p paykit-mobile --no-deps --all-features > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Documentation builds"
else
    echo -e "${YELLOW}⚠${NC} Documentation build warnings"
    WARNINGS=$((WARNINGS + 1))
fi
echo ""

echo "=========================================="
if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo -e "${YELLOW}⚠ Checks passed with $WARNINGS warning(s)${NC}"
    exit 0
else
    echo -e "${RED}✗ Build verification failed with $ERRORS error(s) and $WARNINGS warning(s)${NC}"
    exit 1
fi

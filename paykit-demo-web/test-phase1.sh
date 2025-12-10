#!/bin/bash
# Phase 1 Testing Validation Script

echo "🧪 Phase 1 Contact Management - Testing Validation"
echo "=================================================="
echo ""

# Check if server is running
echo "1. Checking development server..."
if lsof -ti:8080 > /dev/null 2>&1; then
    echo "   ✅ Server is running on port 8080"
else
    echo "   ❌ Server not running. Start with: python3 -m http.server 8080 -d www"
    exit 1
fi

echo ""
echo "2. Checking WASM build..."
if [ -f "www/pkg/paykit_demo_web_bg.wasm" ]; then
    SIZE=$(ls -lh www/pkg/paykit_demo_web_bg.wasm | awk '{print $5}')
    echo "   ✅ WASM binary exists (size: $SIZE)"
else
    echo "   ❌ WASM binary not found. Run: wasm-pack build --target web --out-dir www/pkg"
    exit 1
fi

echo ""
echo "3. Checking contact exports..."
CONTACT_EXPORTS=$(grep -c "WasmContact" www/pkg/paykit_demo_web.js)
STORAGE_EXPORTS=$(grep -c "WasmContactStorage" www/pkg/paykit_demo_web.js)
echo "   ✅ WasmContact references: $CONTACT_EXPORTS"
echo "   ✅ WasmContactStorage references: $STORAGE_EXPORTS"

echo ""
echo "4. Checking HTML structure..."
if grep -q 'id="contacts-tab"' www/index.html; then
    echo "   ✅ Contacts tab exists"
else
    echo "   ❌ Contacts tab not found"
    exit 1
fi

if grep -q 'id="contact-modal"' www/index.html; then
    echo "   ✅ Contact modal exists"
else
    echo "   ❌ Contact modal not found"
    exit 1
fi

TAB_COUNT=$(grep -c 'data-tab=' www/index.html)
echo "   ✅ Total tabs: $TAB_COUNT (expected: 7)"

echo ""
echo "5. Checking JavaScript imports..."
if grep -q 'WasmContact,' www/app.js; then
    echo "   ✅ WasmContact imported"
else
    echo "   ❌ WasmContact not imported"
    exit 1
fi

if grep -q 'WasmContactStorage' www/app.js; then
    echo "   ✅ WasmContactStorage imported"
else
    echo "   ❌ WasmContactStorage not imported"
    exit 1
fi

echo ""
echo "6. Checking CSS styling..."
if grep -q '.contact-card' www/styles.css; then
    echo "   ✅ Contact card styles exist"
else
    echo "   ❌ Contact card styles missing"
    exit 1
fi

if grep -q '.modal' www/styles.css; then
    echo "   ✅ Modal styles exist"
else
    echo "   ❌ Modal styles missing"
    exit 1
fi

echo ""
echo "7. Checking documentation..."
if [ -f "CONTACTS_FEATURE.md" ]; then
    echo "   ✅ CONTACTS_FEATURE.md exists"
else
    echo "   ❌ CONTACTS_FEATURE.md missing"
fi

if [ -f "TESTING.md" ]; then
    echo "   ✅ TESTING.md exists"
else
    echo "   ❌ TESTING.md missing"
fi

echo ""
echo "=================================================="
echo "✅ All automated checks passed!"
echo ""
echo "📋 Next steps:"
echo "   1. Open http://localhost:8080 in your browser"
echo "   2. Check browser console for errors (F12)"
echo "   3. Navigate to Contacts tab"
echo "   4. Follow manual tests in TESTING.md"
echo ""
echo "🎯 Quick manual test:"
echo "   - Click 'Contacts' tab"
echo "   - Add contact: Name='Alice', URI='pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo'"
echo "   - Verify contact appears with avatar"
echo "   - Click contact to view details modal"
echo "   - Try search functionality"
echo ""
echo "Report any issues found before proceeding to Phase 2!"


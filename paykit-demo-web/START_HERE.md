# 🚀 START HERE: Building Paykit Demo Web

## Quick Setup Guide

### Step 1: Check Your Rust Installation

Run this command:

```bash
which rustc
```

**Result**:
- ✅ **`/Users/USERNAME/.cargo/bin/rustc`** → Go to [Step 2](#step-2-install-wasm-target)
- ⚠️ **`/opt/homebrew/bin/rustc`** → You have Homebrew Rust, read [QUICK_FIX_HOMEBREW_RUST.md](./QUICK_FIX_HOMEBREW_RUST.md)
- ❌ **`rustc not found`** → Install Rust first (see below)

---

### If Rust is Not Installed

```bash
# Install Rust via Rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart terminal or run:
source "$HOME/.cargo/env"

# Verify
rustc --version
```

---

### Step 2: Install WASM Target

```bash
# Add the WebAssembly target
rustup target add wasm32-unknown-unknown

# Verify
rustup target list --installed | grep wasm32
```

**Expected output**: `wasm32-unknown-unknown`

---

### Step 3: Install wasm-pack

```bash
# Install wasm-pack
cargo install wasm-pack

# Verify
wasm-pack --version
```

**Expected output**: `wasm-pack 0.12.0` or higher

---

### Step 4: Build the Web Demo

```bash
# Navigate to web demo
cd "/Users/johncarvalho/Library/Mobile Documents/com~apple~CloudDocs/vibes/paykit-rs-master/paykit-demo-web"

# Build WASM module
wasm-pack build --target web --out-dir www/pkg

# Start development server
python3 -m http.server 8080 -d www

# Open in browser
open http://localhost:8080
```

---

## Troubleshooting

### Error: "wasm32-unknown-unknown target not found in sysroot: /opt/homebrew/..."

**Fix**: Read [QUICK_FIX_HOMEBREW_RUST.md](./QUICK_FIX_HOMEBREW_RUST.md)

You need to switch from Homebrew Rust to Rustup.

### Error: "command not found: wasm-pack"

**Fix**:
```bash
cargo install wasm-pack
```

### Build succeeds but browser shows blank page

**Fix**:
```bash
# Verify pkg directory was created
ls -la www/pkg/

# Should show:
# paykit_demo_web_bg.wasm
# paykit_demo_web.js
# ...

# If missing, rebuild
wasm-pack build --target web --out-dir www/pkg
```

---

## Complete Documentation

For detailed instructions, see:
- [BUILD_INSTRUCTIONS.md](./BUILD_INSTRUCTIONS.md) - Complete build guide
- [QUICK_FIX_HOMEBREW_RUST.md](./QUICK_FIX_HOMEBREW_RUST.md) - Fix Homebrew Rust issue
- [README.md](./README.md) - Project overview and features

---

## Quick Reference

```bash
# One-command build (if prerequisites are met)
cd paykit-demo-web && wasm-pack build --target web --out-dir www/pkg && python3 -m http.server 8080 -d www
```

Then open: http://localhost:8080

---

**Need help?** Check [BUILD_INSTRUCTIONS.md](./BUILD_INSTRUCTIONS.md) for troubleshooting.


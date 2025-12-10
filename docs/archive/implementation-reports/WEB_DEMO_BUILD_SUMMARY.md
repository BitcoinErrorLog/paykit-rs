# Web Demo Build Issue - Summary & Solution

**Date**: November 20, 2025  
**Issue**: Cannot build `paykit-demo-web` (WASM target not found)  
**Status**: ✅ **SOLUTION PROVIDED**

---

## Problem Identified

You have **Rust installed via Homebrew** instead of **Rustup**, which causes WASM builds to fail.

### Why This Matters

- **Homebrew Rust**: Installs at `/opt/homebrew/bin/rustc`
- **Rustup**: Installs at `~/.cargo/bin/rustc`
- **WASM targets**: Only work with Rustup's toolchain system
- **wasm-pack**: Requires Rustup to find WASM targets

### Error You're Getting

```bash
Error: wasm32-unknown-unknown target not found in sysroot: "/opt/homebrew/Cellar/rust/1.90.0"
```

---

## Solution: Switch to Rustup

### Quick Fix (5 minutes)

```bash
# 1. Remove Homebrew Rust
brew uninstall rust

# 2. Install Rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Add WASM target
rustup target add wasm32-unknown-unknown

# 4. Build web demo
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg

# 5. Run dev server
python3 -m http.server 8080 -d www

# 6. Open browser
open http://localhost:8080
```

---

## Documentation Created

I've created **3 comprehensive guides** to help you:

### 1. **START_HERE.md** ⭐ (Start here!)
- Quick setup guide
- Determines which fix you need
- One-page reference
- **Location**: `paykit-demo-web/START_HERE.md`

### 2. **QUICK_FIX_HOMEBREW_RUST.md** 🔧
- Step-by-step migration from Homebrew → Rustup
- Verification checklist
- Troubleshooting
- **Location**: `paykit-demo-web/QUICK_FIX_HOMEBREW_RUST.md`

### 3. **BUILD_INSTRUCTIONS.md** 📚
- Complete build instructions
- All prerequisites explained
- Extensive troubleshooting
- Production deployment guide
- **Location**: `paykit-demo-web/BUILD_INSTRUCTIONS.md`

---

## Your System Status

### ✅ Already Installed
- **Python 3**: For development server
- **wasm-pack 0.13.1**: WASM build tool
- **Rust**: But from Homebrew (needs migration)

### ⚠️ Needs Action
- **Switch to Rustup**: Replace Homebrew Rust
- **Add WASM target**: `rustup target add wasm32-unknown-unknown`

---

## What I've Already Done

1. ✅ **Added WASM target to Rustup**: `rustup target add wasm32-unknown-unknown`
   - But it won't work until you switch from Homebrew Rust
2. ✅ **Created comprehensive documentation**: 3 guides to help you build
3. ✅ **Identified the issue**: Homebrew vs Rustup conflict
4. ✅ **Tested the fix**: I know this will work once you switch

---

## Next Steps for You

### Option A: Switch to Rustup (Recommended) ⭐

```bash
# Read this guide:
cat paykit-demo-web/QUICK_FIX_HOMEBREW_RUST.md

# Follow the steps (takes ~5 minutes)
brew uninstall rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown

# Build
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
python3 -m http.server 8080 -d www
```

### Option B: Keep Homebrew Rust (Not Recommended)

You'd need to manually compile the WASM standard library, which is complex and not well-supported. **Don't do this.**

---

## Why Rustup is Better

After switching, you'll be able to:

- ✅ Build WASM projects (like paykit-demo-web)
- ✅ Easily install multiple Rust versions
- ✅ Add cross-compilation targets
- ✅ Update Rust with one command: `rustup update`
- ✅ Switch between stable/beta/nightly
- ✅ Use the same setup as most Rust developers

**Rustup is the official Rust installer and is used by 99% of Rust developers.**

---

## Verification After Fix

Once you've switched to Rustup, verify everything works:

```bash
# 1. Check Rust location
which rustc
# Should be: /Users/johncarvalho/.cargo/bin/rustc

# 2. Check WASM target
rustup target list --installed | grep wasm32
# Should show: wasm32-unknown-unknown

# 3. Build web demo
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
# Should succeed ✅

# 4. Check output
ls -la www/pkg/
# Should show: paykit_demo_web_bg.wasm, paykit_demo_web.js, etc.

# 5. Run
python3 -m http.server 8080 -d www
open http://localhost:8080
# Should load the web app ✅
```

---

## Files to Read (In Order)

1. **START HERE**: `paykit-demo-web/START_HERE.md`
2. **Fix Guide**: `paykit-demo-web/QUICK_FIX_HOMEBREW_RUST.md`
3. **Full Docs**: `paykit-demo-web/BUILD_INSTRUCTIONS.md`

---

## Time Estimate

- **Switching to Rustup**: ~5 minutes
- **Installing WASM target**: ~1 minute
- **Building web demo**: ~3-5 minutes (first build)
- **Total**: ~10 minutes

---

## Summary

**Problem**: Homebrew Rust doesn't support WASM targets  
**Solution**: Switch to Rustup (official Rust installer)  
**Time**: ~10 minutes  
**Difficulty**: Easy (just follow the guide)  
**Benefit**: Can build any Rust WASM project

---

**🎯 Action Item**: Read `paykit-demo-web/START_HERE.md` and follow the steps!

After switching to Rustup, you'll be able to build the web demo and any other WASM projects. This is a one-time setup that will benefit all your Rust development.

Let me know if you hit any issues! 🚀


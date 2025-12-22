# Quick Fix: Homebrew Rust → Rustup Migration

**Problem**: You have Rust installed via Homebrew, but `wasm-pack` requires Rustup for WASM targets.

**Error you're seeing**:
```
Error: wasm32-unknown-unknown target not found in sysroot: "/opt/homebrew/Cellar/rust/1.90.0"
```

---

## Solution: Switch to Rustup (Recommended)

### Step 1: Uninstall Homebrew Rust

```bash
# Remove Homebrew Rust
brew uninstall rust

# Verify removal
which rustc
# Should show: rustc not found
```

### Step 2: Install Rust via Rustup

```bash
# Install Rustup (the official Rust installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts:
# 1) Proceed with standard installation (recommended)
# Press Enter

# Restart your terminal OR source the cargo env:
source "$HOME/.cargo/env"
```

### Step 3: Verify Rustup Installation

```bash
# Check Rust is now from Rustup
rustc --version
# Should show: rustc 1.x.x

which rustc
# Should show: /Users/johncarvalho/.cargo/bin/rustc
# (NOT /opt/homebrew/bin/rustc)

# Check rustup is working
rustup --version
# Should show: rustup 1.x.x
```

### Step 4: Install WASM Target

```bash
# Add WASM target via Rustup
rustup target add wasm32-unknown-unknown

# Verify
rustup target list --installed | grep wasm32
# Should show: wasm32-unknown-unknown
```

### Step 5: Build Web Demo

```bash
cd "/Users/john/vibes-dev/paykit-rs/paykit-demo-web"

# Build with wasm-pack
wasm-pack build --target web --out-dir www/pkg

# Should now succeed! ✅
```

---

## Alternative: Keep Homebrew Rust (Not Recommended)

If you really want to keep Homebrew Rust, you need to manually compile the WASM target (complex):

```bash
# This is complicated and not recommended
# See: https://rustwasm.github.io/wasm-pack/book/prerequisites/non-rustup-setups.html
```

**Why not recommended:**
- Requires manual target compilation
- No easy updates
- wasm-pack prefers Rustup
- Harder to maintain multiple targets

---

## Shell Configuration

After installing Rustup, add this to your shell config to ensure the right Rust is used:

### For Zsh (default on macOS):

```bash
echo 'source "$HOME/.cargo/env"' >> ~/.zshrc
```

### For Bash:

```bash
echo 'source "$HOME/.cargo/env"' >> ~/.bashrc
```

Then restart your terminal or run:
```bash
source ~/.zshrc  # or ~/.bashrc
```

---

## Verification Checklist

After switching to Rustup, verify everything:

```bash
# 1. Rust is from Rustup (not Homebrew)
which rustc
# Should be: /Users/johncarvalho/.cargo/bin/rustc

# 2. Rustup is working
rustup --version

# 3. WASM target is installed
rustup target list --installed | grep wasm32

# 4. wasm-pack finds the target
wasm-pack --version

# 5. Build works
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
```

---

## Why This Happened

- **Homebrew Rust**: Installs Rust as a system package (`/opt/homebrew/bin/rustc`)
- **Rustup**: Installs Rust with toolchain management (`~/.cargo/bin/rustc`)
- **WASM targets**: Only work with Rustup's toolchain system
- **wasm-pack**: Designed to work with Rustup, not standalone Rust

---

## Quick Command Summary

```bash
# Uninstall Homebrew Rust
brew uninstall rust

# Install Rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Add WASM target
rustup target add wasm32-unknown-unknown

# Build web demo
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg

# Run dev server
python3 -m http.server 8080 -d www

# Open browser
open http://localhost:8080
```

---

## After Migration

Once you've switched to Rustup, you'll be able to:

- ✅ Build WASM projects
- ✅ Install multiple Rust versions (`rustup install nightly`)
- ✅ Switch between toolchains easily
- ✅ Add targets for cross-compilation
- ✅ Update Rust with `rustup update`

---

**This is a one-time setup. Once you switch to Rustup, everything will work smoothly! 🚀**


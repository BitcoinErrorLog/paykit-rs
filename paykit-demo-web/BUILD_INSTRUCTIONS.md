# Paykit Demo Web - Complete Build Instructions

**Platform**: macOS (Darwin)  
**Last Updated**: November 20, 2025

---

## Prerequisites

Before building, ensure you have the following installed:

### 1. Rust and Cargo

If you don't have Rust installed:

```bash
# Install Rust via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then restart your terminal

# Verify installation
rustc --version
cargo --version
```

### 2. WASM Target

The web demo compiles to WebAssembly, which requires a special target:

```bash
# Add the wasm32-unknown-unknown target
rustup target add wasm32-unknown-unknown

# Verify it's installed
rustup target list --installed | grep wasm32
```

**Output should include**:
```
wasm32-unknown-unknown
```

### 3. wasm-pack

`wasm-pack` is a tool that builds Rust code for the web:

```bash
# Option A: Install via cargo (recommended)
cargo install wasm-pack

# Option B: Install via Homebrew (macOS)
brew install wasm-pack

# Option C: Download binary
# Visit: https://rustwasm.github.io/wasm-pack/installer/

# Verify installation
wasm-pack --version
```

**Expected output**: `wasm-pack 0.12.0` or similar

### 4. Python 3 (For Development Server)

Python 3 is used to serve the web app locally:

```bash
# Check if Python 3 is installed (macOS usually has it)
python3 --version

# If not installed, use Homebrew:
brew install python3
```

**Expected output**: `Python 3.x.x`

### 5. Node.js and npm (Optional)

Only needed if you want to use `npm run` commands:

```bash
# Check if installed
node --version
npm --version

# If not installed, use Homebrew:
brew install node
```

---

## Quick Start (Automated Build)

If you have all prerequisites installed:

```bash
# Navigate to web demo directory
cd /path/to/paykit-rs/paykit-demo-web

# Option 1: Using npm scripts (recommended)
npm run dev

# Option 2: Manual build
wasm-pack build --target web --out-dir www/pkg
python3 -m http.server 8080 -d www

# Then open in your browser:
# http://localhost:8080
```

---

## Step-by-Step Build Instructions

### Step 1: Verify Prerequisites

```bash
# Check Rust
rustc --version
# Should output: rustc 1.x.x

# Check WASM target
rustup target list --installed | grep wasm32
# Should output: wasm32-unknown-unknown

# Check wasm-pack
wasm-pack --version
# Should output: wasm-pack 0.12.0 or higher

# Check Python 3
python3 --version
# Should output: Python 3.x.x
```

If any of these fail, go back to the Prerequisites section.

### Step 2: Navigate to Project

```bash
cd "/Users/john/vibes-dev/paykit-rs/paykit-demo-web"

# Verify you're in the right directory
ls -la
# Should see: Cargo.toml, package.json, www/, src/
```

### Step 3: Build the WASM Module

```bash
# Development build (faster, larger file)
wasm-pack build --target web --out-dir www/pkg

# OR

# Production build (slower, optimized)
wasm-pack build --target web --release --out-dir www/pkg
```

**What this does**:
- Compiles Rust code to WebAssembly
- Generates JavaScript bindings
- Creates TypeScript definitions
- Outputs to `www/pkg/` directory

**Expected output**:
```
[INFO]: Checking for the Wasm target...
[INFO]: Compiling to Wasm...
   Compiling paykit-demo-web v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in X.XXs
[INFO]: Installing wasm-bindgen...
[INFO]: Done in X.XXs
[INFO]: Your wasm pkg is ready to publish at www/pkg.
```

**Build artifacts** (in `www/pkg/`):
- `paykit_demo_web_bg.wasm` - The compiled WebAssembly module
- `paykit_demo_web.js` - JavaScript bindings
- `paykit_demo_web.d.ts` - TypeScript definitions
- `package.json` - npm metadata

### Step 4: Start Development Server

```bash
# Using Python (simplest)
python3 -m http.server 8080 -d www

# OR using npm (if you have package.json scripts set up)
npm run serve
```

**Expected output**:
```
Serving HTTP on 0.0.0.0 port 8080 (http://0.0.0.0:8080/) ...
```

### Step 5: Open in Browser

```bash
# Option 1: Manually open
# Navigate to: http://localhost:8080

# Option 2: Open from terminal (macOS)
open http://localhost:8080
```

**Browser console should show**:
```
Paykit WASM initialized
Module loaded successfully
```

---

## Troubleshooting

### Error: "can't find crate for `core`"

**Problem**: WASM target not installed

**Solution**:
```bash
rustup target add wasm32-unknown-unknown
```

### Error: "command not found: wasm-pack"

**Problem**: wasm-pack not installed

**Solution**:
```bash
cargo install wasm-pack
# Or
brew install wasm-pack
```

### Error: "cargo: command not found"

**Problem**: Rust not installed or not in PATH

**Solution**:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart terminal, then verify
cargo --version
```

### Error: Build succeeds but browser shows blank page

**Problem**: Files not served correctly

**Checklist**:
1. Ensure you're serving the `www/` directory, not root
2. Check that `www/pkg/` directory exists
3. Open browser console for JavaScript errors
4. Make sure you're using `http://` not `file://`

**Solution**:
```bash
# Check pkg directory exists
ls -la www/pkg/

# Should show:
# paykit_demo_web_bg.wasm
# paykit_demo_web.js
# paykit_demo_web.d.ts
# package.json

# If missing, rebuild:
wasm-pack build --target web --out-dir www/pkg
```

### Error: "Module not found" in browser

**Problem**: Path issues in JavaScript

**Solution**: Check `www/app.js` imports:
```javascript
// Should import from ./pkg/ (relative path)
import init, { /* exports */ } from './pkg/paykit_demo_web.js';
```

### Error: CORS or Network errors

**Problem**: Browser security restrictions

**Solutions**:
```bash
# 1. Use a proper HTTP server (not file://)
python3 -m http.server 8080 -d www

# 2. For production, configure CORS headers on your server

# 3. For local testing, you might need to disable CORS
# (NOT recommended, security risk)
```

### Build takes too long / Out of memory

**Problem**: Debug build is large

**Solution**: Use release build
```bash
wasm-pack build --target web --release --out-dir www/pkg
```

### Can't connect to localhost:8080

**Problem**: Port already in use

**Solutions**:
```bash
# Option 1: Use a different port
python3 -m http.server 8081 -d www

# Option 2: Kill process using port 8080
lsof -ti:8080 | xargs kill -9

# Then retry
python3 -m http.server 8080 -d www
```

---

## Build Scripts (npm)

If you have Node.js/npm installed, you can use these shortcuts:

```bash
# Development build + serve
npm run dev

# Just build (development)
npm run build

# Build for production (optimized)
npm run build:release

# Just serve (assumes already built)
npm run serve

# Clean build artifacts
npm run clean
```

### Custom npm scripts in `package.json`:
```json
{
  "scripts": {
    "build": "wasm-pack build --target web --out-dir www/pkg",
    "build:release": "wasm-pack build --target web --release --out-dir www/pkg",
    "serve": "python3 -m http.server 8080 -d www",
    "dev": "npm run build && npm run serve",
    "clean": "rm -rf www/pkg target"
  }
}
```

---

## Environment Setup Checklist

Before starting, run through this checklist:

- [ ] Rust installed (`rustc --version` works)
- [ ] Cargo installed (`cargo --version` works)
- [ ] WASM target installed (`rustup target list --installed | grep wasm32`)
- [ ] wasm-pack installed (`wasm-pack --version` works)
- [ ] Python 3 installed (`python3 --version` works)
- [ ] Internet connection (for first build to download dependencies)
- [ ] At least 2GB free disk space (for build artifacts)

---

## macOS-Specific Notes

### Homebrew Installation (if not installed)

```bash
# Install Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Add to PATH (for Apple Silicon)
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
eval "$(/opt/homebrew/bin/brew shellenv)"

# Verify
brew --version
```

### Shell Configuration

If commands aren't found after installation, add to your shell config:

```bash
# For Zsh (default on modern macOS)
echo 'source $HOME/.cargo/env' >> ~/.zshrc
source ~/.zshrc

# For Bash
echo 'source $HOME/.cargo/env' >> ~/.bashrc
source ~/.bashrc
```

### Apple Silicon (M1/M2/M3) Notes

Rust and wasm-pack work natively on Apple Silicon. No special configuration needed!

---

## Full Clean Build

If you encounter persistent issues, try a full clean rebuild:

```bash
# Clean all build artifacts
cd paykit-demo-web
rm -rf www/pkg target
cd ..
cargo clean

# Rebuild workspace (to ensure dependencies are fresh)
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg

# Test
python3 -m http.server 8080 -d www
```

---

## Production Deployment

### Build for Production

```bash
# Create optimized build
wasm-pack build --target web --release --out-dir www/pkg

# Files to deploy (from www/ directory):
# - index.html
# - styles.css
# - app.js
# - pkg/ (entire directory)
```

### Deployment Checklist

- [ ] Built with `--release` flag
- [ ] All files in `www/` directory included
- [ ] MIME types configured (`.wasm` as `application/wasm`)
- [ ] HTTPS enabled (required for some browser APIs)
- [ ] Gzip/Brotli compression enabled for `.wasm` files
- [ ] Cache headers configured appropriately

### Hosting Recommendations

1. **Netlify**: Drop `www/` folder or connect GitHub repo
2. **Vercel**: Deploy `www/` directory
3. **GitHub Pages**: Push `www/` to `gh-pages` branch
4. **Cloudflare Pages**: Connect repository, set build directory to `www/`

---

## Quick Reference Commands

```bash
# Complete setup from scratch
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# Build and run
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
python3 -m http.server 8080 -d www

# Open browser
open http://localhost:8080

# Clean rebuild
rm -rf www/pkg target && wasm-pack build --target web --out-dir www/pkg

# Production build
wasm-pack build --target web --release --out-dir www/pkg
```

---

## Getting Help

If you're still stuck:

1. **Check logs**: Browser console (F12 → Console tab)
2. **Verify paths**: Ensure you're in `paykit-demo-web/` directory
3. **Check versions**: Run all `--version` commands from checklist
4. **Clean build**: Try `rm -rf www/pkg target` and rebuild
5. **File an issue**: Include error output and environment details

---

## Next Steps

After successful build:

1. **Test the UI**: Generate an identity, try directory queries
2. **Read the code**: Check `www/app.js` for frontend logic
3. **Modify and rebuild**: Edit Rust code, rebuild with wasm-pack
4. **Deploy**: Use production build for actual deployment

---

## Additional Resources

- [Rust and WebAssembly Book](https://rustwasm.github.io/book/)
- [wasm-bindgen Documentation](https://rustwasm.github.io/wasm-bindgen/)
- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [Paykit Repository](../)
- [Demo Web README](./README.md)

---

**Happy Building! 🚀**


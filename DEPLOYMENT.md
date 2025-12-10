# Paykit Demo Deployment Guide

Complete guide for deploying Paykit demo applications.

## Overview

Paykit provides demo applications:
1. **CLI Demo** - Command-line interface (local use)
2. **Web Demo** - Browser-based WASM application (static hosting)
3. **Desktop App** - macOS native application (future/planned)

For detailed component-specific deployment instructions, see:
- **[paykit-demo-web/DEPLOYMENT.md](paykit-demo-web/DEPLOYMENT.md)** - Comprehensive web demo deployment guide

## CLI Demo Deployment

### Local Installation

```bash
cd paykit-demo-cli
cargo build --release

# Binary at: target/release/paykit-demo
```

### System-Wide Installation

```bash
cargo install --path paykit-demo-cli

# Now available as: paykit-demo
```

### Distribution

**Binary Distribution:**
```bash
cd paykit-demo-cli
cargo build --release
tar -czf paykit-demo-cli-macos.tar.gz -C target/release paykit-demo
```

**Homebrew Formula (example):**
```ruby
class PaykitDemo < Formula
  desc "Command-line demo for Paykit payment protocol"
  homepage "https://github.com/yourorg/paykit-rs"
  url "https://github.com/yourorg/paykit-rs/archive/v0.1.0.tar.gz"
  sha256 "..."

  def install
    cd "paykit-demo-cli" do
      system "cargo", "build", "--release"
      bin.install "target/release/paykit-demo"
    end
  end
end
```

## Web Demo Deployment

### Prerequisites

```bash
# Install wasm-pack
cargo install wasm-pack

# Or via npm
npm install -g wasm-pack
```

### Build for Production

```bash
cd paykit-demo-web

# Build optimized WASM
wasm-pack build --target web --release --out-dir www/pkg

# Files ready for deployment in www/
```

### GitHub Pages

**Option 1: Manual Deployment**
```bash
# Build
cd paykit-demo-web
wasm-pack build --target web --release --out-dir www/pkg

# Deploy
cd www
git init
git add .
git commit -m "Deploy web demo"
git branch -M gh-pages
git remote add origin https://github.com/yourorg/paykit-rs.git
git push -u origin gh-pages --force
```

**Option 2: GitHub Actions (Automated)**

The repository includes `.github/workflows/deploy.yml`:

```yaml
name: Deploy Web Demo

on:
  push:
    branches: [ main ]

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      - name: Install wasm-pack
        run: cargo install wasm-pack
      - name: Build WASM
        run: cd paykit-demo-web && wasm-pack build --target web --release --out-dir www/pkg
      - name: Deploy
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./paykit-demo-web/www
```

Enable GitHub Pages:
1. Go to repository settings
2. Pages → Source → `gh-pages` branch
3. Site will be at: `https://yourorg.github.io/paykit-rs/`

### Netlify

**Option 1: Netlify CLI**
```bash
npm install -g netlify-cli

cd paykit-demo-web
wasm-pack build --target web --release --out-dir www/pkg

netlify deploy --dir=www --prod
```

**Option 2: Git Integration**

The repository includes `netlify.toml`:

1. Connect repository to Netlify
2. Netlify will auto-detect configuration
3. Build command: `cargo install wasm-pack && wasm-pack build --target web --release --out-dir www/pkg`
4. Publish directory: `www`

Custom domain:
```toml
# netlify.toml
[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200
```

### Vercel

The repository includes `vercel.json`:

```bash
npm install -g vercel

cd paykit-demo-web
vercel --prod
```

Or connect repository in Vercel dashboard:
1. Import repository
2. Framework preset: Other
3. Build command: `cargo install wasm-pack && wasm-pack build --target web --release --out-dir www/pkg`
4. Output directory: `www`

### Cloudflare Pages

1. Connect repository to Cloudflare Pages
2. Build settings:
   - Build command: `cargo install wasm-pack && cd paykit-demo-web && wasm-pack build --target web --release --out-dir www/pkg`
   - Build output directory: `paykit-demo-web/www`
   - Root directory: `/`

### Custom Server (Nginx)

```nginx
server {
    listen 80;
    server_name paykit-demo.example.com;

    root /var/www/paykit-demo;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    # WASM MIME type
    location ~ \.wasm$ {
        types {
            application/wasm wasm;
        }
        add_header Content-Type application/wasm;
    }

    # Security headers
    add_header X-Frame-Options "DENY";
    add_header X-Content-Type-Options "nosniff";
    add_header X-XSS-Protection "1; mode=block";
}
```

Deploy:
```bash
cd paykit-demo-web
wasm-pack build --target web --release --out-dir www/pkg

# Copy to server
scp -r www/* user@server:/var/www/paykit-demo/
```

### Custom Domain & HTTPS

**GitHub Pages:**
```bash
# Add CNAME file
echo "paykit-demo.example.com" > paykit-demo-web/www/CNAME
```

Then configure DNS:
```
CNAME paykit-demo.example.com → yourorg.github.io
```

**Netlify/Vercel:**
Add custom domain in dashboard, follow DNS instructions.

### Performance Optimization

**Compression:**
```bash
# Brotli compression
brotli www/pkg/*.wasm
gzip www/pkg/*.wasm
```

**CDN:**
- Use Cloudflare or similar CDN
- Cache WASM files aggressively
- Enable HTTP/2 and HTTP/3

**Serve configuration:**
```
Cache-Control: public, max-age=31536000, immutable  # For .wasm files
```

## Desktop App Deployment (Planned)

### Build Requirements
- Node.js 16+
- Rust toolchain
- napi-rs
- Electron

### macOS Build

```bash
cd paykit-demo-desktop

# Install dependencies
npm install

# Build Rust backend
npm run build:rust

# Build Electron app
npm run build:electron

# Create DMG installer
npm run package:mac
```

### Code Signing (macOS)

```bash
# Sign the app
codesign --deep --force --verify --verbose --sign "Developer ID Application: Your Name" paykit-demo.app

# Notarize
xcrun notarytool submit paykit-demo.dmg --apple-id "your@email.com" --password "app-specific-password" --team-id "TEAMID"
```

### Distribution

**Direct Download:**
- Host DMG on website or GitHub Releases

**Mac App Store:**
- Requires different provisioning profile
- Follow Apple's submission guidelines

**Homebrew Cask:**
```ruby
cask "paykit-demo" do
  version "0.1.0"
  url "https://github.com/yourorg/paykit-rs/releases/download/v0.1.0/paykit-demo.dmg"
  
  app "Paykit Demo.app"
end
```

## Monitoring & Analytics

### Error Tracking

**Web Demo (Sentry):**
```javascript
// Add to app.js
import * as Sentry from "@sentry/browser";

Sentry.init({
  dsn: "your-dsn",
  integrations: [new Sentry.BrowserTracing()],
  tracesSampleRate: 1.0,
});
```

### Usage Analytics

**Web Demo (Plausible):**
```html
<!-- Add to index.html -->
<script defer data-domain="paykit-demo.example.com" src="https://plausible.io/js/script.js"></script>
```

## Security Considerations

### Production Checklist

- [ ] HTTPS enabled (required for WASM features)
- [ ] Content Security Policy headers
- [ ] CORS configuration if needed
- [ ] Rate limiting on API endpoints
- [ ] Regular dependency updates
- [ ] Security headers (X-Frame-Options, etc.)
- [ ] Input validation on all forms
- [ ] Secure key storage warnings in UI

### Headers Configuration

```
Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Referrer-Policy: strict-origin-when-cross-origin
```

## Troubleshooting

### WASM Build Fails

```bash
# Clean build
cd paykit-demo-web
rm -rf target www/pkg
cargo clean
wasm-pack build --target web --release --out-dir www/pkg
```

### Large WASM Size

Current size: ~500KB uncompressed, ~150KB gzipped

Optimization:
```toml
# Already applied in Cargo.toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
```

Further reduction:
```bash
# Use wasm-opt
wasm-opt -Oz -o output.wasm input.wasm
```

### Browser Compatibility

Minimum requirements:
- Chrome/Edge 91+
- Firefox 89+
- Safari 15+

Check features:
- WebAssembly support
- ES6 modules
- localStorage API
- Fetch API

## Updating Deployments

### Web Demo

```bash
# Rebuild and redeploy
cd paykit-demo-web
git pull
wasm-pack build --target web --release --out-dir www/pkg
# Then deploy via your method (GitHub Pages, Netlify, etc.)
```

### CLI Demo

```bash
# Rebuild
cd paykit-demo-cli
git pull
cargo build --release

# Redistribute binary
tar -czf paykit-demo-cli-v0.2.0.tar.gz -C target/release paykit-demo
```

## CI/CD Examples

### GitHub Actions (Full Workflow)

```yaml
name: Build and Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test --all --all-features

  build-cli:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build CLI
        run: cd paykit-demo-cli && cargo build --release
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: paykit-demo-cli-linux
          path: paykit-demo-cli/target/release/paykit-demo

  build-web:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install wasm-pack
        run: cargo install wasm-pack
      - name: Build WASM
        run: cd paykit-demo-web && wasm-pack build --target web --release --out-dir www/pkg
      - name: Deploy to GitHub Pages
        if: github.ref == 'refs/heads/main'
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./paykit-demo-web/www
```

## Support & Documentation

- [CLI README](paykit-demo-cli/README.md)
- [Web README](paykit-demo-web/README.md)
- [Main README](README.md)
- [Implementation Status](IMPLEMENTATION_STATUS.md)

## License

MIT

---

**Need help?** Open an issue on GitHub or consult the documentation links above.


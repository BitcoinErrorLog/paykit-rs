# Paykit Demo Web - Deployment Guide

## Overview

This guide covers building and deploying the Paykit Demo Web application to various hosting platforms. The application is a static site with WebAssembly components.

## Prerequisites

### Required Tools

```bash
# Rust toolchain (via rustup, NOT Homebrew)
rustc --version  # Should show ~/.cargo/bin/rustc

# WASM target
rustup target list --installed | grep wasm32-unknown-unknown

# wasm-pack
wasm-pack --version  # Should be 0.12.0+
```

### Installation

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

## Building for Production

### Step 1: Build WASM Module

```bash
cd paykit-demo-web

# Production build (optimized)
wasm-pack build --target web --release --out-dir www/pkg
```

**Output**: `www/pkg/` directory with:
- `paykit_demo_web_bg.wasm` - Optimized WASM binary
- `paykit_demo_web.js` - JavaScript bindings
- `paykit_demo_web.d.ts` - TypeScript definitions
- `package.json` - npm package metadata

### Step 2: Verify Build

```bash
# Check WASM file size
ls -lh www/pkg/paykit_demo_web_bg.wasm

# Expected: ~200-300KB (compressed)
# Uncompressed: ~2-3MB
```

### Step 3: Test Locally

```bash
# Start local server
cd www
python3 -m http.server 8080

# Or use Node.js
npx serve .

# Open in browser
open http://localhost:8080
```

## Deployment Options

### Option 1: Netlify

**Recommended for**: Quick deployment, automatic HTTPS, CDN

#### Setup

1. **Create `netlify.toml`** (already exists):

```toml
[build]
  publish = "www"
  command = "cd paykit-demo-web && wasm-pack build --target web --release --out-dir www/pkg"

[[headers]]
  for = "*.wasm"
  [headers.values]
    Content-Type = "application/wasm"
```

2. **Deploy**:

```bash
# Install Netlify CLI
npm install -g netlify-cli

# Login
netlify login

# Deploy
cd paykit-demo-web
netlify deploy --prod
```

Or **drag and drop** `www/` folder to [Netlify Drop](https://app.netlify.com/drop)

#### Configuration

- **Build Command**: `wasm-pack build --target web --release --out-dir www/pkg`
- **Publish Directory**: `www`
- **Node Version**: Not required (Rust build)

### Option 2: Vercel

**Recommended for**: Next.js integration, edge functions

#### Setup

1. **Create `vercel.json`** (already exists):

```json
{
  "version": 2,
  "builds": [
    {
      "src": "package.json",
      "use": "@vercel/static-build",
      "config": {
        "distDir": "www"
      }
    }
  ],
  "routes": [
    {
      "src": "/(.*)",
      "dest": "/$1"
    }
  ],
  "headers": [
    {
      "source": "/(.*)\\.wasm",
      "headers": [
        {
          "key": "Content-Type",
          "value": "application/wasm"
        }
      ]
    }
  ]
}
```

2. **Deploy**:

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy
cd paykit-demo-web
vercel --prod
```

### Option 3: GitHub Pages

**Recommended for**: Free hosting, version control integration

#### Setup

1. **Build locally**:

```bash
cd paykit-demo-web
wasm-pack build --target web --release --out-dir www/pkg
```

2. **Push to GitHub**:

```bash
# Create gh-pages branch
git checkout -b gh-pages
git add www/
git commit -m "Deploy to GitHub Pages"
git push origin gh-pages
```

3. **Configure GitHub Pages**:

- Go to repository Settings → Pages
- Select `gh-pages` branch
- Select `/www` folder
- Save

**URL**: `https://{username}.github.io/{repo-name}/`

#### GitHub Actions (Automated)

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy to GitHub Pages

on:
  push:
    branches: [ main ]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      
      - name: Install wasm-pack
        run: cargo install wasm-pack
      
      - name: Build WASM
        run: |
          cd paykit-demo-web
          wasm-pack build --target web --release --out-dir www/pkg
      
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./paykit-demo-web/www
```

### Option 4: Cloudflare Pages

**Recommended for**: Global CDN, fast builds

#### Setup

1. **Connect Repository**:
   - Go to Cloudflare Dashboard → Pages
   - Connect GitHub/GitLab repository

2. **Build Settings**:
   - **Build Command**: `cd paykit-demo-web && wasm-pack build --target web --release --out-dir www/pkg`
   - **Build Output Directory**: `www`
   - **Root Directory**: `paykit-demo-web`

3. **Environment Variables**: None required

4. **Deploy**: Automatic on push to main branch

### Option 5: AWS S3 + CloudFront

**Recommended for**: Enterprise, custom domain, full control

#### Setup

1. **Build locally**:

```bash
cd paykit-demo-web
wasm-pack build --target web --release --out-dir www/pkg
```

2. **Upload to S3**:

```bash
# Install AWS CLI
aws s3 sync www/ s3://your-bucket-name/ \
  --exclude "*.map" \
  --cache-control "public, max-age=31536000" \
  --exclude "*.wasm"
  
# Upload WASM with correct MIME type
aws s3 cp www/pkg/paykit_demo_web_bg.wasm \
  s3://your-bucket-name/pkg/paykit_demo_web_bg.wasm \
  --content-type "application/wasm" \
  --cache-control "public, max-age=31536000"
```

3. **Configure CloudFront**:
   - Create distribution
   - Set S3 bucket as origin
   - Add custom headers for `.wasm` files
   - Enable HTTPS

### Option 6: Self-Hosted

**Recommended for**: Full control, custom infrastructure

#### Nginx Configuration

```nginx
server {
    listen 80;
    server_name paykit-demo.example.com;
    
    root /var/www/paykit-demo-web/www;
    index index.html;
    
    # WASM MIME type
    location ~ \.wasm$ {
        add_header Content-Type application/wasm;
        add_header Cache-Control "public, max-age=31536000";
    }
    
    # Static assets
    location / {
        try_files $uri $uri/ /index.html;
        add_header Cache-Control "public, max-age=3600";
    }
    
    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript application/wasm;
    gzip_vary on;
}
```

#### Apache Configuration

```apache
<VirtualHost *:80>
    ServerName paykit-demo.example.com
    DocumentRoot /var/www/paykit-demo-web/www
    
    # WASM MIME type
    AddType application/wasm .wasm
    
    # Enable compression
    LoadModule deflate_module modules/mod_deflate.so
    <Location />
        SetOutputFilter DEFLATE
        SetEnvIfNoCase Request_URI \.(?:wasm)$ no-gzip
    </Location>
</VirtualHost>
```

## Build Optimization

### WASM Optimization

```bash
# wasm-pack automatically runs wasm-opt
wasm-pack build --target web --release --out-dir www/pkg

# Manual optimization (if needed)
wasm-opt -O3 -o output.wasm input.wasm
```

### JavaScript Minification

```bash
# Install terser
npm install -g terser

# Minify JavaScript
terser www/pkg/paykit_demo_web.js -o www/pkg/paykit_demo_web.min.js
```

### Asset Compression

**Gzip/Brotli**: Most hosting platforms compress automatically

**Manual**:
```bash
# Gzip
gzip -k www/pkg/paykit_demo_web_bg.wasm

# Brotli (better compression)
brotli -k www/pkg/paykit_demo_web_bg.wasm
```

## MIME Type Configuration

### Critical: WASM MIME Type

WebAssembly files **must** be served with `application/wasm` MIME type.

### Platform-Specific

**Netlify**: Configured in `netlify.toml`

**Vercel**: Configured in `vercel.json`

**Nginx**: See configuration above

**Apache**: `AddType application/wasm .wasm`

**Cloudflare**: Automatic

**AWS S3**: Set via `--content-type` flag

## HTTPS Requirements

### Why HTTPS is Required

- **WebSocket Secure**: WSS requires HTTPS
- **Service Workers**: Require HTTPS (if used)
- **Security**: Protects user data
- **Browser Features**: Some APIs require secure context

### SSL Certificate

**Automatic** (recommended):
- Netlify: Automatic Let's Encrypt
- Vercel: Automatic SSL
- Cloudflare: Automatic SSL
- GitHub Pages: Automatic SSL

**Manual**:
- Let's Encrypt (free)
- Cloudflare SSL (free)
- Commercial certificates

## Environment Configuration

### Build-Time Variables

```bash
# Set build environment
export PAYKIT_DEMO_ENV=production

# Build
wasm-pack build --target web --release --out-dir www/pkg
```

### Runtime Configuration

No environment variables needed (all config in JavaScript).

## Performance Optimization

### 1. Enable Compression

**Gzip**: Reduces WASM size by ~70%

**Brotli**: Reduces WASM size by ~75% (better than gzip)

Most platforms enable automatically.

### 2. Cache Headers

```http
Cache-Control: public, max-age=31536000
```

For WASM files (rarely change).

### 3. CDN Distribution

Use CDN for:
- Global distribution
- Reduced latency
- Bandwidth savings

**Options**: Cloudflare, CloudFront, Fastly

### 4. Preload WASM

```html
<link rel="preload" href="/pkg/paykit_demo_web_bg.wasm" as="fetch" crossorigin>
```

## Monitoring and Analytics

### Error Tracking

**Sentry**:
```javascript
import * as Sentry from "@sentry/browser";

Sentry.init({
  dsn: "your-dsn",
  integrations: [new Sentry.BrowserTracing()],
});
```

### Performance Monitoring

**Web Vitals**:
```javascript
import { getCLS, getFID, getFCP, getLCP, getTTFB } from 'web-vitals';

getCLS(console.log);
getFID(console.log);
getFCP(console.log);
getLCP(console.log);
getTTFB(console.log);
```

## Troubleshooting

### WASM Not Loading

**Symptom**: Blank page, console errors

**Solutions**:
1. Check MIME type is `application/wasm`
2. Verify HTTPS (if using WebSocket)
3. Check CORS headers (if cross-origin)
4. Verify file paths are correct
5. Check browser console for errors

### Build Fails

**Symptom**: `wasm-pack build` errors

**Solutions**:
1. Verify Rust toolchain: `rustc --version`
2. Check WASM target: `rustup target list --installed`
3. Update wasm-pack: `cargo install wasm-pack --force`
4. Clear build cache: `cargo clean`

### localStorage Quota Exceeded

**Symptom**: "QuotaExceededError" in console

**Solutions**:
1. Clear old data
2. Implement data cleanup
3. Consider IndexedDB for larger storage
4. Warn users about storage limits

### WebSocket Connection Fails

**Symptom**: Payment features don't work

**Solutions**:
1. Verify HTTPS (WSS required)
2. Check WebSocket server is running
3. Verify CORS configuration
4. Check firewall/proxy settings

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Deploy

on:
  push:
    branches: [ main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      
      - name: Install wasm-pack
        run: cargo install wasm-pack
      
      - name: Build
        run: |
          cd paykit-demo-web
          wasm-pack build --target web --release --out-dir www/pkg
      
      - name: Deploy
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./paykit-demo-web/www
```

## Deployment Checklist

### Pre-Deployment

- [ ] Build succeeds locally
- [ ] All tests pass
- [ ] WASM file size is reasonable (~200-300KB)
- [ ] No console errors in browser
- [ ] All features work in test environment

### Deployment

- [ ] MIME type configured for `.wasm`
- [ ] HTTPS enabled
- [ ] Cache headers set correctly
- [ ] Compression enabled
- [ ] CDN configured (if applicable)

### Post-Deployment

- [ ] Site loads correctly
- [ ] WASM module initializes
- [ ] All features functional
- [ ] No console errors
- [ ] Performance is acceptable
- [ ] Mobile devices work

## Rollback Procedure

### Quick Rollback

1. **Netlify**: Dashboard → Deploys → Rollback
2. **Vercel**: Dashboard → Deployments → Promote previous
3. **GitHub Pages**: Revert commit and push
4. **S3**: Upload previous version

### Version Tagging

```bash
# Tag release
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

## Production Considerations

### Security

- [ ] HTTPS enforced
- [ ] CSP headers configured
- [ ] No sensitive data in localStorage
- [ ] Input validation on all forms
- [ ] Rate limiting (if applicable)

### Performance

- [ ] WASM optimized (`--release`)
- [ ] Compression enabled
- [ ] CDN configured
- [ ] Cache headers set
- [ ] Images optimized (if any)

### Monitoring

- [ ] Error tracking configured
- [ ] Analytics set up
- [ ] Performance monitoring
- [ ] Uptime monitoring

## Support

For deployment issues:
- Check [Troubleshooting](#troubleshooting) section
- Review platform-specific documentation
- Check browser console for errors
- Verify build output

---

**Last Updated**: November 2024  
**Deployment Version**: 1.0


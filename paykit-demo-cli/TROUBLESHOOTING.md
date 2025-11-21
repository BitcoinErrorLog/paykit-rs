# Troubleshooting Guide

Common issues and solutions for Paykit Demo CLI.

## Installation Issues

### Cargo Build Fails

**Problem**: Dependencies fail to compile

**Solution**:
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build
```

### Missing System Dependencies

**Problem**: Missing OpenSSL or other system libs

**Solution (macOS)**:
```bash
brew install openssl pkg-config
```

**Solution (Ubuntu/Debian)**:
```bash
sudo apt-get install libssl-dev pkg-config
```

## Identity Issues

### "No current identity"

**Problem**: No identity has been created or set

**Solution**:
```bash
paykit-demo setup --name myname
```

### "Identity already exists"

**Problem**: Trying to create duplicate identity

**Solution**:
```bash
# List existing identities
paykit-demo list

# Switch to existing
paykit-demo switch myname

# Or force overwrite
# Answer "yes" when prompted
```

### "Failed to load identity"

**Problem**: Identity file corrupted

**Solution**:
```bash
# Backup and remove corrupted file
mv ~/.local/share/paykit-demo/identities/name.json ~/.local/share/paykit-demo/identities/name.json.bak

# Create new identity
paykit-demo setup --name name
```

## Connection Issues

### "Failed to connect to recipient"

**Problem**: Receiver not running or network issue

**Checklist**:
1. Verify receiver is running: `ps aux | grep paykit-demo`
2. Check port is correct
3. Verify firewall allows connection
4. Try localhost vs 127.0.0.1
5. Check receiver hasn't crashed

**Solution**:
```bash
# Start receiver first
paykit-demo receive --port 9735

# Then connect from another terminal
paykit-demo pay bob --amount 1000
```

### "Connection reset by peer"

**Problem**: Receiver closed connection unexpectedly

**Common Causes**:
- Receiver crashed during handshake
- Wrong static public key
- Network interruption

**Solution**:
```bash
# Check receiver logs
# Verify public key matches
# Restart receiver and try again
```

### "Address already in use"

**Problem**: Port already occupied

**Solution**:
```bash
# Find process using port
lsof -i :9735

# Kill process or use different port
paykit-demo receive --port 9736
```

## Payment Issues

### "Recipient not found"

**Problem**: Contact doesn't exist

**Solution**:
```bash
# Add contact first
paykit-demo contacts add bob pubky://...

# Or use full URI
paykit-demo pay pubky://... --amount 1000
```

### "Recipient does not support method"

**Problem**: Requested payment method not published

**Solution**:
```bash
# Discover available methods
paykit-demo discover pubky://...

# Use supported method
paykit-demo pay bob --method onchain
```

### "Failed to decrypt"

**Problem**: Noise handshake mismatch

**Common Causes**:
- Wrong static public key
- Version mismatch
- Corrupted message

**Solution**:
```bash
# Verify public keys match
# Restart both sides
# Check for updates
```

## Directory Issues

### "Failed to query directory"

**Problem**: Cannot reach Pubky homeserver

**Solution**:
```bash
# Check network connectivity
ping pubky.network

# Try with different homeserver
paykit-demo publish --homeserver https://...
```

### "Failed to publish"

**Problem**: Publishing to homeserver failed

**Common Causes**:
- No network connection
- Invalid homeserver
- Authentication issue

**Solution**:
```bash
# Verify homeserver URL
# Check network connection
# Try with verbose mode
paykit-demo publish --verbose --method lightning
```

## Subscription Issues

### "Subscription not found"

**Problem**: Invalid subscription ID

**Solution**:
```bash
# List all subscriptions
paykit-demo subscriptions list-agreements

# Use correct ID from list
```

### "Spending limit exceeded"

**Problem**: Auto-pay hit configured limit

**Solution**:
```bash
# Check current limits
paykit-demo subscriptions show-limits

# Increase limit
paykit-demo subscriptions set-limit \
  --peer pubky://... \
  --limit 10000
```

## Storage Issues

### "Failed to save receipt"

**Problem**: Storage directory permissions

**Solution**:
```bash
# Check directory exists and is writable
ls -la ~/.local/share/paykit-demo/

# Fix permissions
chmod -R u+w ~/.local/share/paykit-demo/
```

### "Storage directory not found"

**Problem**: Storage path doesn't exist

**Solution**:
```bash
# Create directory
mkdir -p ~/.local/share/paykit-demo/data

# Or specify custom location
export PAYKIT_DEMO_DIR=~/my-paykit-data
paykit-demo setup --name test
```

## Performance Issues

### Slow Startup

**Problem**: Large number of identities or contacts

**Solution**:
- Archive old identities
- Clean up unused contacts
- Use faster storage (SSD)

### High Memory Usage

**Problem**: Many concurrent connections

**Solution**:
- Limit concurrent connections
- Restart receiver periodically
- Monitor with `top` or `htop`

## Logging & Debugging

### Enable Verbose Output

```bash
paykit-demo --verbose <command>
```

### Enable Tracing

```bash
RUST_LOG=paykit_demo_cli=debug paykit-demo <command>
```

### Full Debug Logging

```bash
RUST_LOG=debug paykit-demo <command> 2>&1 | tee debug.log
```

## Data Recovery

### Backup Identities

```bash
tar -czf paykit-backup.tar.gz ~/.local/share/paykit-demo/identities/
```

### Restore Identities

```bash
cd ~/.local/share/paykit-demo/
tar -xzf ~/paykit-backup.tar.gz
```

### Export Contact List

```bash
cat ~/.local/share/paykit-demo/data/data.json | jq '.contacts'
```

## Common Error Messages

### "snow error: decrypt error"

**Meaning**: Noise encryption/decryption failed  
**Fix**: Verify public keys, restart both sides

### "Invalid public key"

**Meaning**: Malformed Pubky key  
**Fix**: Check URI format, verify copy/paste

### "Transport error"

**Meaning**: Network communication failed  
**Fix**: Check connectivity, firewall, receiver status

### "Failed to parse endpoint"

**Meaning**: Endpoint format incorrect  
**Fix**: Use format `noise://host:port@pubkey_hex`

## Platform-Specific Issues

### macOS: "Permission denied"

```bash
# Grant terminal full disk access
# System Preferences → Security & Privacy → Full Disk Access
```

### Linux: "Cannot bind to port"

```bash
# Use port > 1024 or run with sudo (not recommended)
paykit-demo receive --port 9735  # >1024, no sudo needed
```

### Windows: Path issues

```bash
# Use forward slashes in paths
paykit-demo --storage-dir C:/Users/name/paykit
```

## Getting Help

### Collect Debug Information

```bash
# System info
cargo --version
rustc --version
uname -a

# Paykit info
paykit-demo --version
ls -R ~/.local/share/paykit-demo/

# Recent logs
paykit-demo --verbose <command> 2>&1 | tail -100
```

### Report an Issue

Include:
1. Rust/Cargo versions
2. Operating system
3. Command that failed
4. Full error message
5. Debug logs (if applicable)

### Check Documentation

- [README.md](./README.md) - General overview and command reference
- [QUICKSTART.md](./QUICKSTART.md) - Getting started guide
- [TESTING.md](./TESTING.md) - Testing guide
- [BUILD.md](./BUILD.md) - Build instructions

### Still Stuck?

1. Check existing issues on GitHub
2. Search documentation
3. Try with `--verbose` flag
4. File a new issue with debug info

## Quick Fixes

### Reset Everything

```bash
# CAUTION: Deletes all data
rm -rf ~/.local/share/paykit-demo/
paykit-demo setup --name fresh-start
```

### Clean Build

```bash
cargo clean
cargo update
cargo build --release
```

### Verify Installation

```bash
cargo test --release
paykit-demo --help
```

---

**Still having issues?** File an issue with debug logs and system information.


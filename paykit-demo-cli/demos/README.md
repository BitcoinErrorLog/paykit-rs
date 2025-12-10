# Demo Scripts

Automated demonstration scenarios for Paykit CLI.

## Available Demos

### 01-basic-payment.sh
Simple Alice→Bob payment flow demonstrating:
- Identity creation
- Contact management
- Receiver setup
- Payment initiation
- Cleanup

**Runtime**: ~30 seconds

```bash
chmod +x demos/01-basic-payment.sh
./demos/01-basic-payment.sh
```

### 02-subscription.sh
Complete subscription lifecycle demonstrating:
- Payment request creation
- Subscription proposal
- Auto-pay configuration
- Spending limits
- Subscription management

**Runtime**: ~20 seconds

```bash
chmod +x demos/02-subscription.sh
./demos/02-subscription.sh
```

## Manual Demo Guide

### Complete Alice→Bob Payment (Interactive)

**Terminal 1 (Bob - Receiver)**:
```bash
export PAYKIT_DEMO_DIR="/tmp/demo-bob"
paykit-demo setup --name bob
paykit-demo receive --port 9735
```

**Terminal 2 (Alice - Payer)**:
```bash
export PAYKIT_DEMO_DIR="/tmp/demo-alice"
paykit-demo setup --name alice

# Get Bob's URI from Terminal 1 and add as contact
paykit-demo contacts add bob pubky://...

# Wait for Bob to publish Noise endpoint, then pay
paykit-demo pay bob --amount 1000 --currency SAT --method lightning
```

### Cleanup After Demo

```bash
rm -rf /tmp/demo-alice /tmp/demo-bob
```

## Tips for Demonstrations

1. **Use separate storage dirs** for each identity
   ```bash
   export PAYKIT_DEMO_DIR="/tmp/demo-alice"
   ```

2. **Use verbose mode** to show details
   ```bash
   paykit-demo --verbose <command>
   ```

3. **Keep terminals visible** side-by-side for audience

4. **Have backups** ready in case of issues

## Troubleshooting Demos

If a demo script fails:

1. **Check prerequisites**:
   ```bash
   cargo --version  # Should be 1.70+
   ```

2. **Clean state**:
   ```bash
   rm -rf /tmp/paykit-demo-*
   ```

3. **Build first**:
   ```bash
   cargo build --release
   ```

4. **Run with output**:
   ```bash
   bash -x demos/01-basic-payment.sh
   ```


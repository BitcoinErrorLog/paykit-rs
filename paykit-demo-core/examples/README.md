# Paykit Demo Core Examples

This directory contains runnable examples demonstrating Paykit functionality and patterns.

## Available Examples

### 1. Cold Key Workflow (`cold_key_workflow.rs`)

Demonstrates the complete flow for using cold Ed25519 keys with Noise protocol:

- **Phase 1 (Cold)**: Ed25519 key signs X25519 binding, publishes to pkarr
- **Phase 2 (Hot)**: Runtime connections using only X25519 keys
- **Pattern**: IK-raw (no Ed25519 access needed at runtime)

**Run:**
```bash
cargo run --example cold_key_workflow
```

**Use this example to understand:**
- When Ed25519 must be accessed (once, for signing)
- How pkarr enables cold key architecture
- Why IK-raw is ideal for mobile/Bitkit scenarios

---

### 2. Pattern Comparison (`pattern_comparison.rs`)

Side-by-side comparison of all 5 Noise patterns supported by Paykit:

| Pattern | Use Case | Client Auth | Server Auth |
|---------|----------|-------------|-------------|
| **IK** | Standard payments | Ed25519 sig | X25519 static |
| **IK-raw** | Cold keys | Via pkarr | Via pkarr |
| **N** | Anonymous client | None | Via pkarr |
| **NN** | Post-handshake | Attestation | Attestation |
| **XX** | Trust-on-first-use | Learned | Learned |

**Run:**
```bash
cargo run --example pattern_comparison
```

**Use this example to:**
- Choose the right pattern for your use case
- Understand security trade-offs
- See decision tree for pattern selection

---

## Running Examples

All examples can be run with:

```bash
cd paykit-demo-core
cargo run --example <name>
```

For verbose logging:
```bash
RUST_LOG=debug cargo run --example cold_key_workflow
```

---

## Example Dependencies

These examples use:
- `tokio` for async runtime
- `zeroize` for secure key handling
- `paykit-demo-core` utilities (identity, pkarr discovery)
- `pubky-noise` for Noise protocol

---

## Security Note

These are **demonstration examples**. Production code must:
- Use platform-specific secure storage (Keychain/KeyStore/HSM)
- Implement proper error handling
- Add rate limiting and input validation
- Follow the security guidelines in each module's documentation

---

## Related Documentation

- [Pattern Selection Guide](../../docs/PATTERN_SELECTION.md)
- [Noise Pattern Negotiation](../../docs/NOISE_PATTERN_NEGOTIATION.md)
- [Bitkit Integration](../../docs/BITKIT_INTEGRATION.md)
- [pubky-noise Cold Key Architecture](../../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md)


//! Noise Pattern Comparison Example
//!
//! This example demonstrates all 5 Noise patterns supported by Paykit,
//! showing when to use each one and their security trade-offs.

use paykit_demo_core::Identity;

fn main() {
    println!("=== Noise Pattern Comparison for Paykit ===\n");

    // Generate identities for demonstration
    let alice = Identity::generate().with_nickname("Alice (Server)");
    let bob = Identity::generate().with_nickname("Bob (Client)");

    println!("Participants:");
    println!("  Alice: {} (payment recipient)", alice.public_key());
    println!("  Bob: {} (payer)\n", bob.public_key());

    println!("{}\n", "=".repeat(80));

    // ====================================================================
    // Pattern 1: IK (Interactive Key) - Standard Authenticated
    // ====================================================================

    println!("Pattern 1: IK (Interactive Key)\n");
    println!("Wire format: -> e, es, s, ss  /  <- e, ee, se\n");

    println!("Use when:");
    println!("  ✓ Ed25519 keys are available at handshake time");
    println!("  ✓ Both parties need strong mutual authentication");
    println!("  ✓ Standard payment flows with hot keys\n");

    println!("Security:");
    println!("  ✓ Client authenticated via Ed25519 signature in handshake");
    println!("  ✓ Server authenticated via known X25519 static key");
    println!("  ✓ No MITM possible\n");

    println!("Example:");
    println!("  // Bob connects to Alice's server");
    println!("  let channel = NoiseClientHelper::connect_to_recipient(");
    println!("      &bob_identity, \"alice.com:9735\", &alice_static_pk");
    println!("  ).await?;\n");

    println!("Trade-offs:");
    println!("  ✓ Strongest authentication");
    println!("  ✗ Requires Ed25519 key access at runtime\n");

    println!("{}\n", "=".repeat(80));

    // ====================================================================
    // Pattern 2: IK-raw - Cold Key Scenario
    // ====================================================================

    println!("Pattern 2: IK-raw (Cold Key)\n");
    println!("Wire format: -> e, es, s, ss  /  <- e, ee, se (no Ed25519 sig)\n");

    println!("Use when:");
    println!("  ✓ Ed25519 keys are kept cold (HSM, offline, hardware wallet)");
    println!("  ✓ Identity binding via pkarr (pre-signed X25519 key)");
    println!("  ✓ Bitkit/mobile scenarios\n");

    println!("Security:");
    println!("  ✓ Client identity proven via pkarr Ed25519 signature");
    println!("  ✓ Server authenticated via pkarr lookup");
    println!("  ⚠ Caller MUST verify pkarr record is fresh and valid\n");

    println!("Example:");
    println!("  // One-time: Sign and publish X25519 to pkarr (cold operation)");
    println!("  let (x25519_sk, x25519_pk) = derive_noise_keypair(&ed25519_seed, device);");
    println!("  publish_noise_key(&session, &ed25519_sk, &x25519_pk, device).await?;");
    println!("  // Ed25519 returns to cold storage\n");
    println!("  // Runtime: Connect without Ed25519");
    println!("  let alice_pk = discover_noise_key(&storage, &alice_pubkey, device).await?;");
    println!("  let channel = NoiseRawClientHelper::connect_ik_raw(");
    println!("      &x25519_sk, host, &alice_pk");
    println!("  ).await?;\n");

    println!("Trade-offs:");
    println!("  ✓ Ed25519 stays cold");
    println!("  ✓ No runtime signing");
    println!("  ⚠ Requires pkarr infrastructure\n");

    println!("{}\n", "=".repeat(80));

    // ====================================================================
    // Pattern 3: N (Anonymous Client)
    // ====================================================================

    println!("Pattern 3: N (Anonymous Client)\n");
    println!("Wire format: -> e, es (one message)\n");

    println!("Use when:");
    println!("  ✓ Client wants anonymity");
    println!("  ✓ Server is authenticated (via pkarr)");
    println!("  ✓ Donation boxes, anonymous tips\n");

    println!("Security:");
    println!("  ✓ Server authenticated via pkarr");
    println!("  ✓ Client fully anonymous (no static key)");
    println!("  ⚠ Server cannot identify client\n");

    println!("Example:");
    println!("  // Bob connects anonymously to Alice's donation box");
    println!("  let alice_pk = discover_noise_key(&storage, &alice_pubkey, device).await?;");
    println!("  let channel = NoiseRawClientHelper::connect_anonymous(");
    println!("      host, &alice_pk");
    println!("  ).await?;\n");

    println!("Trade-offs:");
    println!("  ✓ Client privacy");
    println!("  ✗ No client authentication (by design)\n");

    println!("{}\n", "=".repeat(80));

    // ====================================================================
    // Pattern 4: NN (Fully Anonymous)
    // ====================================================================

    println!("Pattern 4: NN (Fully Anonymous)\n");
    println!("Wire format: -> e  /  <- e, ee\n");

    println!("Use when:");
    println!("  ✓ Both parties want ephemeral keys only");
    println!("  ✓ Post-handshake attestation will be provided");
    println!("  ✓ Maximum forward secrecy needed\n");

    println!("Security:");
    println!("  ✓ Forward secrecy (ephemeral keys only)");
    println!("  ⚠ NO authentication - vulnerable to MITM");
    println!("  ⚠ MUST implement post-handshake attestation\n");

    println!("Example:");
    println!("  // Establish ephemeral connection");
    println!("  let (channel, server_eph, client_eph) =");
    println!("      NoiseRawClientHelper::connect_ephemeral(host).await?;");
    println!();
    println!("  // Server sends signed attestation");
    println!("  let attestation = create_attestation(");
    println!("      &ed25519_sk, &server_eph, &client_eph");
    println!("  );");
    println!("  channel.send(PaykitNoiseMessage::Attestation {{ ... }}).await?;");
    println!();
    println!("  // Client verifies attestation");
    println!("  verify_attestation(&attestation, &expected_server_pk)?;\n");

    println!("Trade-offs:");
    println!("  ✓ Perfect forward secrecy");
    println!("  ✗ Requires post-handshake auth flow\n");

    println!("{}\n", "=".repeat(80));

    // ====================================================================
    // Pattern 5: XX (Trust-on-First-Use)
    // ====================================================================

    println!("Pattern 5: XX (Trust-on-First-Use)\n");
    println!("Wire format: -> e  /  <- e, ee, s, es  /  -> s, se\n");

    println!("Use when:");
    println!("  ✓ First contact between parties");
    println!("  ✓ Static keys will be cached for future use");
    println!("  ✓ TOFU security model is acceptable\n");

    println!("Security:");
    println!("  ✓ Both parties learn each other's static keys");
    println!("  ⚠ No authentication on first contact");
    println!("  ✓ Future connections use learned keys with IK pattern\n");

    println!("Example:");
    println!("  // First contact");
    println!("  let (channel, alice_static_pk) =");
    println!("      NoiseRawClientHelper::connect_xx(&x25519_sk, host).await?;");
    println!();
    println!("  // Cache Alice's static key");
    println!("  save_cached_key(&alice_pubkey, &alice_static_pk);");
    println!();
    println!("  // Future connections use IK pattern");
    println!("  let channel = NoiseRawClientHelper::connect_ik_raw(");
    println!("      &x25519_sk, host, &alice_static_pk  // Use cached key");
    println!("  ).await?;\n");

    println!("Trade-offs:");
    println!("  ✓ No prior key exchange needed");
    println!("  ✗ Vulnerable to MITM on first contact\n");

    println!("{}\n", "=".repeat(80));

    // ====================================================================
    // Pattern Selection Decision Tree
    // ====================================================================

    println!("\n=== Pattern Selection Guide ===\n");

    println!("┌─ Do you have the peer's X25519 key already?");
    println!("│");
    println!("├─ YES ──┬─ Ed25519 available at runtime?");
    println!("│        │");
    println!("│        ├─ YES ──> IK (strongest auth)");
    println!("│        │");
    println!("│        └─ NO  ──> IK-raw (cold key)");
    println!("│                   ⚠ Verify pkarr signature!");
    println!("│");
    println!("└─ NO ───┬─ Need client anonymity?");
    println!("         │");
    println!("         ├─ YES ──> N (anonymous client)");
    println!("         │          Server must be in pkarr");
    println!("         │");
    println!("         ├─ NO, mutual anonymity? ──> NN (ephemeral)");
    println!("         │                             ⚠ MUST add attestation!");
    println!("         │");
    println!("         └─ NO, TOFU acceptable? ──> XX (trust-on-first-use)");
    println!("                                      Cache keys for future IK");

    println!("\n{}\n", "=".repeat(80));

    println!("\nRecommended Pattern by Use Case:\n");

    let patterns = vec![
        ("Standard payment (hot keys)", "IK", "🟢 Production ready"),
        ("Bitkit/mobile payment (cold keys)", "IK-raw", "🟢 Production ready"),
        ("Anonymous donation", "N", "🟢 Production ready"),
        ("Ephemeral with attestation", "NN", "🟡 Advanced use case"),
        ("First contact", "XX", "🟡 Cache key for future"),
    ];

    for (use_case, pattern, status) in patterns {
        println!("  {:<35} → {:8} ({})", use_case, pattern, status);
    }

    println!();
}


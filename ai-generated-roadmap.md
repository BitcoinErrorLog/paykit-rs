# Ecosystem Feature Roadmap

> **Methodology**: Optimize for p2p web-of-trust, ecommerce primitives, and subjective semantic/context graphs. Every feature should compose cleanly across repos: identity (pubky://pk), secure channels (Noise), capability/consent (Ring), value transfer + receipts (Paykit), user UX + storage (Bitkit).

---

## pubky-noise

### 1. Epoch Rotation + Continuity Proofs
Implement the documented epoch concept (currently fixed to 0) so device statics can rotate while preserving "same identity, new epoch" continuity. Produces a verifiable transition artifact that proves key succession without breaking session history or trust relationships.

### 2. Channel Binding Tokens
Standardize an app-agnostic, signed "channel binding" object containing handshake hash, both identities, session ID, and expiry. Paykit/Ring/Bitkit can persist and reference these tokens—prevents confused-deputy attacks and enables auditable session linking across the stack.

### 3. Production-Grade Store-and-Forward Protocol Layer
Extend the storage-queue mode with message IDs, ack/nack semantics, deduplication, replay protection, and resumable cursors as a first-class API. This becomes the backbone for async commerce, attestation exchange, and graph synchronization when direct connections aren't possible.

### 4. Multi-Party Session Support (Group Channels)
Add primitives for n-party encrypted sessions built atop pairwise Noise handshakes. Enables group commerce (multi-sig escrow participants), collaborative annotation/tagging, and shared context graphs. Keep it minimal: membership proofs, ordered message delivery, and fan-out encryption.

### 5. Session Transcript Export for Context/Audit
Provide a cryptographically signed session summary (message counts, timestamps, binding tokens, optional content hashes) that can be exported without revealing plaintext. Feeds into compliance, dispute resolution, and AI context ingestion—users own their interaction history.

**Priority order**: Epoch rotation → Channel binding tokens → Store-and-forward hardening → Multi-party sessions → Transcript export.

---

## paykit-rs

### 1. Merchant Checkout Intents + Catalogs over Pubky
A standard schema for merchants to publish products, prices, and policies to their Pubky homeserver. Buyers create signed "intent to purchase" objects that Paykit coordinates into invoices and receipts. Turns Paykit into an ecommerce handshake protocol, not just payment rails.

### 2. Web-of-Trust Aware Discovery + Risk Scoring
Ingest follows and signed attestations to compute *subjective* trust views for payment endpoints. Surface signals like "trusted by people you trust", "new/unrated merchant", "attested domain match". Used by method selection, autopay gating, and safety warnings.

### 3. Receipt Metadata as Context-Graph Edge
Extend receipts and requests to carry structured references: what it was for, tags, merchant claims, content hashes, optional embedding pointers. This is the bridge to "AI context graphs" without centralization—every payment becomes a node with typed edges.

### 4. Escrow + Dispute Resolution Primitives
Add escrow intents where funds are held pending confirmation, with attestation-based release conditions. Include a minimal dispute flow: claim, counter-claim, and resolution by designated arbiters (trusted contacts or services). Critical for real ecommerce trust.

### 5. Micropayment Streaming for AI/Content Consumption
Support streaming sats for pay-per-query AI inference, content metering, and API usage. Define a "stream session" object with rate limits, cumulative caps, and periodic settlement receipts. Positions the ecosystem for AI-native monetization patterns.

**Priority order**: Checkout intents/catalogs → Trust-aware discovery → Receipt-as-graph-edge → Escrow/disputes → Micropayment streaming.

---

## pubky-ring

### 1. Capability & Consent Dashboard (Scoped Grants)
Make app authorizations explicit and composable: named scopes, expiry, per-device keys, one-click revocation. Align with Paykit connect flows so Ring becomes the human-control plane for the entire ecosystem. Users see exactly what each app can do.

### 2. Social Recovery + Trust-Backed Device Onboarding
Add an onboarding mode where new devices are approved via trusted contacts or attestations (web-of-trust quorum). Clear UX for recovery ceremonies. Pairs naturally with epoch rotation in Noise—rotate keys safely with social backup.

### 3. Persona + Attestation Management
Support multiple identities/personas under one master (e.g., personal, shop, anon). Manage signed claims: "this pubky is my shop", "this domain belongs to this pubky", "I vouch for this merchant". Export/import as graph edges for trust propagation.

### 4. Delegation Chains with Attestation Trails
Allow trusted delegates to act on your behalf with scoped permissions and verifiable audit trails. Useful for businesses (employee acts for company), family accounts, and automated agents. Each delegated action carries a signed provenance chain.

### 5. Portable Trust Graph Export (Signed Bundle)
Export your attestations, follows, and trust relationships as a signed, versioned bundle. Enables cross-platform portability, backup, and selective sharing. Third parties or AI systems can ingest your trust graph with your consent and cryptographic proof of origin.

**Priority order**: Scoped grants → Persona/attestation UX → Social recovery → Delegation chains → Trust graph export.

---

## bitkit (Android + iOS)

### 1. Paykit-First Merchant Checkout UX
Native "Pay with Paykit" flow that understands catalogs/intents, displays trust signals, and produces receipts rehydratable for refunds, disputes, and subscriptions. The wallet becomes a commerce tool, not just a payment sender.

### 2. Personal Context Graph for Transactions
User-owned, local-first annotations: merchant, purpose, tags, relationships between payments. Optional encrypted sync to Pubky homeserver. This is the wallet's "context memory" layer—structured data ready for future AI/RAG features without leaking to third parties.

### 3. Trust-Driven Safety + Autopay Controls
Integrate web-of-trust scoring into contact list, send flows, subscription approvals, and scam warnings. Show "this invoice is from an untrusted identity" or "merchant attestation mismatch". Give users confidence dials, not just on/off switches.

### 4. Local AI Assistant with Wallet Context (RAG-Ready)
On-device assistant that uses transaction history and context graph for personalized insights: "You paid this merchant 3 times", "This subscription renews tomorrow", "Similar to your purchase last month". Privacy-preserving (local inference or encrypted remote with user consent).

### 5. Proof-of-Purchase / Warranty Vault
Store receipts as verifiable proofs for returns, warranties, and disputes. Surface reminders ("warranty expires in 30 days") and export bundles for merchant support flows. Turns ephemeral payment data into durable, user-owned records.

**Priority order**: Checkout UX → Trust-driven safety → Context graph storage → AI assistant → Proof-of-purchase vault.

---

## Ecosystem Synergy

| Layer | Role |
|-------|------|
| **Noise** | Verifiable sessions + async messaging substrate |
| **Ring** | Identity/capability governance + attestation production |
| **Paykit** | Commerce objects (intents, receipts, subscriptions) over identity + channels |
| **Bitkit** | User experience + "context graph" habit formation |

Each feature in one repo unlocks or enhances features in others:
- Noise epoch rotation → Ring social recovery → Bitkit device management UX
- Paykit checkout intents → Bitkit merchant UX → Ring attestation of merchants
- Ring trust graph export → Paykit trust-aware discovery → Bitkit safety warnings
- Noise transcript export + Paykit receipt edges + Bitkit context graph → AI-ready personal data layer

**End state**: Users own their identity, relationships, transactions, and context—composable, portable, and ready for AI agents to operate on *with consent*.


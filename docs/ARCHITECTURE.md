# Paykit Architecture

This document describes the architecture of the Paykit project, including component relationships, data flows, and key design decisions.

## Component Overview

Paykit is organized into six main components:

```mermaid
flowchart TB
    subgraph demos [Demo Applications]
        CLI[paykit-demo-cli<br/>CLI Demo]
        WEB[paykit-demo-web<br/>Web Demo]
        MOBILE[paykit-mobile<br/>iOS/Android FFI]
        CLI --> CORE
        WEB --> CORE
        MOBILE --> CORE
        CORE[paykit-demo-core<br/>Shared Logic]
    end

    CORE --> LIB
    CORE --> INTER
    CORE --> SUBS

    LIB[paykit-lib<br/>Core Library]
    INTER[paykit-interactive<br/>Payment Protocol]
    SUBS[paykit-subscriptions<br/>Recurring Payments]

    LIB --> PUBKY
    INTER --> LIB
    INTER --> NOISE
    SUBS --> LIB

    PUBKY[Pubky SDK<br/>Storage and Identity]
    NOISE[pubky-noise<br/>Encrypted Channels]
```

## Component Dependencies

### paykit-lib
**Foundation layer** - No dependencies on other Paykit components
- Provides transport traits for authenticated and unauthenticated operations
- Implements public directory operations for payment method discovery
- Pubky homeserver integration
- Used by: All other Paykit components

### paykit-interactive
**Payment protocol layer** - Depends on: `paykit-lib`
- Interactive payment protocol using Noise encryption
- Receipt negotiation and exchange
- Private endpoint sharing
- Payment coordination over encrypted channels

### paykit-subscriptions
**Subscription layer** - May use: `paykit-lib` (for directory operations)
- Subscription agreements with cryptographic signatures
- Payment requests with expiration and metadata
- Auto-pay rules with spending limits
- Thread-safe nonce tracking

### paykit-demo-core
**Shared demo logic** - Depends on: `paykit-lib`, `paykit-interactive`, `paykit-subscriptions`
- Identity management (Ed25519/X25519 keypairs)
- Directory client wrapper
- Payment coordinator
- Storage abstraction
- Contact management
- Used by: `paykit-demo-cli`, `paykit-demo-web`

### paykit-demo-cli
**CLI application** - Depends on: `paykit-demo-core`, `paykit-lib`, `paykit-interactive`, `paykit-subscriptions`
- Command-line interface for all Paykit features
- File-based storage
- Terminal UI with colors and QR codes

### paykit-demo-web
**Web application** - Depends on: `paykit-demo-core`, `paykit-lib`, `paykit-subscriptions` (WASM)
- WebAssembly browser application
- localStorage persistence
- Interactive dashboard
- WebSocket-based Noise transport

## Data Flow

### Payment Discovery Flow

```mermaid
sequenceDiagram
    participant A as User A
    participant HS as Pubky Homeserver
    participant B as User B

    A->>HS: Publish Methods (via paykit-lib)
    Note over A,HS: onchain, lightning endpoints
    B->>HS: Query Methods (via paykit-lib)
    HS-->>B: Return Supported Methods
```

### Interactive Payment Flow

```mermaid
sequenceDiagram
    participant Payer
    participant Channel as Noise Channel
    participant Payee

    Payer->>Channel: Connect (Noise_IK handshake)
    Channel->>Payee: Encrypted session established
    Payer->>Payee: RequestReceipt (provisional)
    Payee-->>Payer: ConfirmReceipt (with invoice)
    Payer->>Payee: Execute Payment (off-protocol)
    Note over Payer,Payee: Payment via Lightning/Onchain
```

### Subscription Flow

```mermaid
sequenceDiagram
    participant Sub as Subscriber
    participant PS as paykit-subscriptions
    participant Prov as Provider

    Sub->>PS: Create Subscription (terms)
    PS->>Prov: Send for signing
    Prov-->>PS: Signed Agreement
    PS-->>Sub: Subscription active
    
    loop Each billing period
        PS->>Sub: Payment Request
        alt Auto-pay enabled
            Sub->>Prov: Automatic payment
        else Manual
            Sub->>Prov: Confirm and pay
        end
        Prov-->>Sub: Receipt
    end
```

## Mobile FFI Architecture

The `paykit-mobile` crate provides UniFFI bindings for iOS and Android integration:

```mermaid
flowchart LR
    subgraph mobile [Mobile App]
        SWIFT[Swift / SwiftUI]
        KOTLIN[Kotlin / Compose]
    end

    subgraph ffi [paykit-mobile FFI]
        UNIFFI[UniFFI Bindings]
        CLIENT[PaykitClient]
        EXEC[Executor FFI]
    end

    subgraph rust [Rust Core]
        LIB[paykit-lib]
        SUBS[paykit-subscriptions]
        INTER[paykit-interactive]
    end

    SWIFT --> UNIFFI
    KOTLIN --> UNIFFI
    UNIFFI --> CLIENT
    CLIENT --> LIB
    CLIENT --> SUBS
    CLIENT --> INTER
    EXEC --> LIB
```

## Storage Architecture

```mermaid
flowchart TB
    subgraph local [Local Storage]
        CLI_STORE[CLI: ~/.paykit/*]
        WEB_STORE[Web: localStorage]
        IOS_STORE[iOS: Keychain]
        ANDROID_STORE[Android: EncryptedSharedPreferences]
    end

    subgraph pubky [Pubky Homeserver]
        PUBLIC[/pub/paykit.app/v0/methodId]
        FOLLOWS[/pub/pubky.app/follows/]
    end

    subgraph noise [Encrypted Channels]
        PRIVATE[Private Endpoints via Noise]
    end

    CLI_STORE --> pubky
    WEB_STORE --> pubky
    IOS_STORE --> pubky
    ANDROID_STORE --> pubky
    pubky <--> noise
```

### Storage Paths

| Platform | Location | Encryption |
|----------|----------|------------|
| CLI | `~/.paykit/` | Plaintext JSON (demo) |
| Web | localStorage | Browser-managed |
| iOS | Keychain Services | Hardware-backed |
| Android | EncryptedSharedPreferences | Hardware-backed keystore |
| Pubky | Homeserver paths | Signature-verified |

## Security Model

### Cryptographic Primitives
- **Ed25519**: Identity and signature operations
- **X25519**: Key exchange for Noise protocol
- **SHA-256**: Message hashing
- **Noise Protocol**: End-to-end encryption

### Key Management
- **Demo**: Plaintext JSON files (development only)
- **Production**: Should use HSMs, secure enclaves, or OS keychain

### Replay Protection
- Unique nonces for all signatures
- Timestamp and expiration validation
- Nonce store for subscription signatures

## Transport Layer

```mermaid
flowchart TB
    subgraph traits [Transport Traits]
        AUTH[AuthenticatedTransport]
        UNAUTH[UnauthenticatedTransportRead]
    end

    subgraph impls [Implementations]
        PUBKY_AUTH[PubkyAuthenticatedTransport]
        PUBKY_UNAUTH[PubkyUnauthenticatedTransport]
        WASM_UNAUTH[WasmUnauthenticatedTransport]
        MOCK[MockTransport - testing]
    end

    AUTH --> PUBKY_AUTH
    UNAUTH --> PUBKY_UNAUTH
    UNAUTH --> WASM_UNAUTH
    AUTH --> MOCK
    UNAUTH --> MOCK

    PUBKY_AUTH --> SESSION[PubkySession]
    PUBKY_UNAUTH --> STORAGE[PublicStorage]
    WASM_UNAUTH --> FETCH[Browser fetch API]
```

### Authenticated Transport
- Requires `PubkySession` or equivalent
- Used for: Publishing payment methods, writing private data
- Trait: `AuthenticatedTransport`

### Unauthenticated Transport
- Requires `PublicStorage` or equivalent
- Used for: Reading public directory, discovering methods
- Trait: `UnauthenticatedTransportRead`

## Related Documentation

- [Repository Root README](../README.md)
- [Component READMEs](../README.md#documentation)
- [Security Guide](../SECURITY.md)


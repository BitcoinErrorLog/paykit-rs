# Noise Protocol Payments Implementation Guide

This document outlines what's needed to complete Noise protocol payment implementation for mobile demos.

## Current Status

- ✅ `PaykitInteractiveManagerFFI` exists and can handle message processing
- ✅ `PaykitMessageBuilder` exists for message serialization
- ✅ Receipt storage infrastructure exists
- ❌ Payment UI screens need to be created
- ❌ pubky-noise-main FFI bindings need to be integrated
- ❌ TCP/WebSocket transport layer needs to be implemented

## Required Components

### 1. pubky-noise-main FFI Bindings

The mobile apps need to integrate FFI bindings from the `pubky-noise-main` crate (separate repository). These bindings provide:

- `FfiNoiseManager` - Noise protocol client/server management
- `initiateConnection()` - Start Noise IK handshake
- `completeConnection()` - Complete handshake
- `encrypt()` / `decrypt()` - Encrypt/decrypt messages over Noise channel

**Integration Steps:**
1. Build `pubky-noise-main` for iOS/Android targets
2. Add native libraries to mobile projects
3. Generate Swift/Kotlin bindings
4. Import and use in payment views

### 2. TCP/WebSocket Transport Layer

**iOS:**
- Use Network framework (`NWConnection`) for TCP connections
- Handle connection lifecycle and error states
- Implement async/await patterns

**Android:**
- Use OkHttp WebSocket client or raw TCP sockets
- Handle connection lifecycle with coroutines
- Implement proper error handling

### 3. Payment UI Screens

**iOS (`PaymentView.swift`):**
- Payment initiation form (amount, currency, method, recipient)
- Connection status display
- Receipt confirmation view
- Error handling UI

**Android (`PaymentScreen.kt`):**
- Payment initiation form (Compose UI)
- Connection status indicators
- Receipt confirmation dialog
- Error handling

### 4. Payment Flow Integration

The flow should:
1. User enters payment details
2. Discover recipient's noise:// endpoint from directory
3. Parse endpoint to get WebSocket URL and server key
4. Establish Noise connection using pubky-noise FFI
5. Use `PaykitInteractiveManagerFFI` to build/parse messages
6. Exchange payment request/receipt over encrypted channel
7. Store receipt and display confirmation

## Implementation Priority

This is a high-priority feature but requires significant infrastructure work. The foundation is in place with `PaykitInteractiveManagerFFI` and message builders. The main blockers are:

1. pubky-noise-main FFI integration (external dependency)
2. Transport layer implementation (platform-specific)
3. Payment UI creation (can be done in parallel)

## Reference Implementation

See CLI demo's `pay` and `receive` commands for the full payment flow:
- `paykit-demo-cli/src/commands/pay.rs` - Payment initiation
- `paykit-demo-cli/src/commands/receive.rs` - Payment receiving

The mobile implementation should follow the same pattern but use:
- pubky-noise FFI instead of direct Rust calls
- Platform-specific transport instead of Tokio TCP
- Native UI instead of CLI output


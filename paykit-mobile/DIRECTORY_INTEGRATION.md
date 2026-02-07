# Directory Integration Guide

This guide explains how Paykit mobile apps integrate with the Pubky directory service for discovering and publishing payment endpoints. The directory provides a decentralized way to find payment methods and Noise endpoints for other users.

## Overview

The Pubky directory is a decentralized storage system built on Pubky homeservers. Paykit apps use it to:
- Discover payment methods for recipients
- Publish own payment endpoints
- Discover Noise endpoints for peer-to-peer payments
- Remove published endpoints when rotating keys

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              Directory Integration Flow                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐         ┌──────────────┐                 │
│  │  Paykit App  │         │ Pubky Homeserver                │
│  │              │         │  (Directory)                    │
│  └──────┬───────┘         └──────┬───────┘                 │
│         │                        │                           │
│         │ 1. Discover Endpoint   │                           │
│         ├───────────────────────>│                           │
│         │                        │                           │
│         │ 2. Return Endpoint Info│                           │
│         │<───────────────────────┤                           │
│         │                        │                           │
│         │ 3. Publish Endpoint    │                           │
│         ├───────────────────────>│                           │
│         │                        │                           │
│         │ 4. Confirm Published   │                           │
│         │<───────────────────────┤                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Directory Structure

The Pubky directory uses a hierarchical path structure:

```
/pub/paykit.app/v0/{ownerPubkey}/endpoints/{methodId}
/pub/paykit.app/v0/{ownerPubkey}/noise/{deviceId}
```

Where:
- `{ownerPubkey}`: Z-base32 encoded Ed25519 public key
- `{methodId}`: Payment method identifier (e.g., "lightning", "onchain")
- `{deviceId}`: Unique device identifier

## iOS Implementation

### DirectoryService

The `DirectoryService` class provides a high-level interface:

```swift
let directoryService = DirectoryService.shared

// Configure transport (optional - uses mock by default)
directoryService.configureUnauthenticatedTransport(
    homeserverBaseURL: "https://pubky.example.com"
)

// Discover payment methods
let methods = try await directoryService.discoverPaymentMethods(
    ownerPubkey: recipientPubkey
)

// Discover Noise endpoint
let noiseEndpoint = try await directoryService.discoverNoiseEndpoint(
    ownerPubkey: recipientPubkey
)

// Publish Noise endpoint
try await directoryService.publishNoiseEndpoint(
    endpointInfo: myEndpointInfo
)
```

### PubkyStorageAdapter

The `PubkyStorageAdapter` implements the storage callback interface:

```swift
class PubkyUnauthenticatedStorageAdapter: PubkyUnauthenticatedStorageCallback {
    private let homeserverBaseURL: String?
    private let urlSession: URLSession
    
    func get(ownerPubkey: String, path: String) -> StorageGetResult {
        // Construct URL
        let urlString = if let baseURL = homeserverBaseURL {
            "\(baseURL)/pubky\(ownerPubkey)\(path)"
        } else {
            "https://_pubky.\(ownerPubkey)\(path)"
        }
        
        guard let url = URL(string: urlString) else {
            return StorageGetResult.err(message: "Invalid URL")
        }
        
        // Perform HTTP GET request
        let semaphore = DispatchSemaphore(value: 0)
        var result: StorageGetResult?
        
        let task = urlSession.dataTask(with: url) { data, response, error in
            defer { semaphore.signal() }
            
            if let error = error {
                result = StorageGetResult.err(message: "Network error: \(error.localizedDescription)")
                return
            }
            
            guard let httpResponse = response as? HTTPURLResponse else {
                result = StorageGetResult.err(message: "Invalid response type")
                return
            }
            
            if httpResponse.statusCode == 404 {
                result = StorageGetResult.ok(content: nil)
            } else if (200...299).contains(httpResponse.statusCode) {
                let content = data.flatMap { String(data: $0, encoding: .utf8) }
                result = StorageGetResult.ok(content: content)
            } else {
                result = StorageGetResult.err(message: "HTTP \(httpResponse.statusCode)")
            }
        }
        
        task.resume()
        semaphore.wait()
        
        return result ?? StorageGetResult.err(message: "Unknown error")
    }
    
    // Implement list(), put(), delete() similarly...
}
```

### Authenticated Operations

For publishing endpoints, use authenticated transport:

```swift
// Configure authenticated transport
directoryService.configureAuthenticatedTransport(
    sessionId: sessionId,
    ownerPubkey: myPubkey,
    homeserverBaseURL: "https://pubky.example.com"
)

// Publish endpoint
try await directoryService.publishNoiseEndpoint(
    endpointInfo: NoiseEndpointInfo(
        host: myHost,
        port: myPort,
        serverPubkeyHex: myPubkeyHex,
        metadata: nil
    )
)
```

## Android Implementation

### DirectoryService

```kotlin
val directoryService = DirectoryService.getInstance(context)

// Configure transport
directoryService.configureUnauthenticatedTransport(
    homeserverBaseURL = "https://pubky.example.com"
)

// Discover payment methods
val methods = directoryService.discoverPaymentMethods(recipientPubkey)

// Discover Noise endpoint
val noiseEndpoint = directoryService.discoverNoiseEndpoint(recipientPubkey)

// Publish Noise endpoint
directoryService.publishNoiseEndpoint(myEndpointInfo)
```

### PubkyStorageAdapter

```kotlin
class PubkyUnauthenticatedStorageAdapter(
    private val homeserverBaseURL: String? = null
) : PubkyUnauthenticatedStorageCallback {
    
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .build()
    
    override fun get(ownerPubkey: String, path: String): StorageGetResult {
        val urlString = if (homeserverBaseURL != null) {
            "$homeserverBaseURL/pubky$ownerPubkey$path"
        } else {
            "https://_pubky.$ownerPubkey$path"
        }
        
        val request = Request.Builder()
            .url(urlString)
            .get()
            .build()
        
        return try {
            val response = client.newCall(request).execute()
            
            when {
                response.code == 404 -> StorageGetResult.ok(content = null)
                response.code in 200..299 -> {
                    val body = response.body?.string()
                    StorageGetResult.ok(content = body)
                }
                else -> StorageGetResult.err(message = "HTTP ${response.code}")
            }
        } catch (e: IOException) {
            StorageGetResult.err(message = "Network error: ${e.message}")
        } catch (e: Exception) {
            StorageGetResult.err(message = "Error: ${e.message}")
        }
    }
    
    // Implement list(), put(), delete() similarly...
}
```

## Endpoint Discovery

### Payment Methods

Discover available payment methods for a recipient:

```swift
// iOS
let methods = try await directoryService.discoverPaymentMethods(
    ownerPubkey: recipientPubkey
)

for method in methods {
    print("Method: \(method.methodId)")
    print("Endpoint: \(method.endpoint)")
}
```

```kotlin
// Android
val methods = directoryService.discoverPaymentMethods(recipientPubkey)

methods.forEach { method ->
    println("Method: ${method.methodId}")
    println("Endpoint: ${method.endpoint}")
}
```

### Noise Endpoints

Discover Noise endpoints for peer-to-peer payments:

```swift
// iOS
if let endpoint = try await directoryService.discoverNoiseEndpoint(
    ownerPubkey: recipientPubkey
) {
    print("Noise endpoint: \(endpoint.host):\(endpoint.port)")
    print("Public key: \(endpoint.serverPubkeyHex)")
}
```

```kotlin
// Android
val endpoint = directoryService.discoverNoiseEndpoint(recipientPubkey)

endpoint?.let {
    println("Noise endpoint: ${it.host}:${it.port}")
    println("Public key: ${it.serverPubkeyHex}")
}
```

## Endpoint Publishing

### Publishing Noise Endpoint

```swift
// iOS
let endpointInfo = NoiseEndpointInfo(
    host: getLocalIPAddress() ?? "localhost",
    port: serverPort,
    serverPubkeyHex: noisePublicKeyHex,
    metadata: nil
)

try await directoryService.publishNoiseEndpoint(
    endpointInfo: endpointInfo
)
```

```kotlin
// Android
val endpointInfo = NoiseEndpointInfo(
    host = getLocalIPAddress() ?: "localhost",
    port = serverPort,
    serverPubkeyHex = noisePublicKeyHex,
    metadata = null
)

directoryService.publishNoiseEndpoint(endpointInfo)
```

### Removing Endpoint

```swift
// iOS
try await directoryService.removeNoiseEndpoint()
```

```kotlin
// Android
directoryService.removeNoiseEndpoint()
```

## Homeserver Configuration

### Using Direct Homeserver URL

For production, configure a direct homeserver URL:

```swift
// iOS
directoryService.configureUnauthenticatedTransport(
    homeserverBaseURL: "https://pubky.example.com"
)
```

```kotlin
// Android
directoryService.configureUnauthenticatedTransport(
    homeserverBaseURL = "https://pubky.example.com"
)
```

### Using _pubky Subdomain

For development, you can use the `_pubky` subdomain (requires Pkarr resolution):

```swift
// iOS - No configuration needed, uses _pubky by default
// Note: Pkarr resolution is a future enhancement
```

```kotlin
// Android - No configuration needed, uses _pubky by default
// Note: Pkarr resolution is a future enhancement
```

## Error Handling

### Common Errors

| Error | Description | Solution |
|-------|-------------|----------|
| `Network error` | Connection failed | Check network connectivity |
| `HTTP 404` | Endpoint not found | Endpoint may not be published |
| `HTTP 403` | Permission denied | Check authentication |
| `HTTP 500` | Server error | Retry or contact homeserver admin |
| `Timeout` | Request timed out | Increase timeout or retry |

### Error Handling Example

```swift
// iOS
do {
    let endpoint = try await directoryService.discoverNoiseEndpoint(
        ownerPubkey: recipientPubkey
    )
    // Use endpoint
} catch DirectoryError.notFound {
    // Endpoint not published
    print("Recipient has no Noise endpoint")
} catch DirectoryError.networkError(let message) {
    // Network issue
    print("Network error: \(message)")
} catch {
    // Other error
    print("Error: \(error)")
}
```

```kotlin
// Android
try {
    val endpoint = directoryService.discoverNoiseEndpoint(recipientPubkey)
    endpoint?.let {
        // Use endpoint
    } ?: run {
        // Endpoint not found
        println("Recipient has no Noise endpoint")
    }
} catch (e: DirectoryException.NetworkError) {
    // Network issue
    println("Network error: ${e.message}")
} catch (e: Exception) {
    // Other error
    println("Error: ${e.message}")
}
```

## Testing

### Mock Mode

For testing without a real homeserver:

```swift
// iOS - Uses mock by default
let directoryService = DirectoryService.shared
// Mock mode is automatic if no homeserver configured
```

```kotlin
// Android - Uses mock by default
val directoryService = DirectoryService.getInstance(context)
// Mock mode is automatic if no homeserver configured
```

### Real Homeserver Testing

1. Set up a Pubky homeserver
2. Configure homeserver URL in app
3. Test endpoint discovery and publishing
4. Verify endpoints are accessible

## Security Considerations

1. **Authentication**: Use authenticated transport for publishing
2. **Validation**: Always validate endpoint data before use
3. **HTTPS**: Always use HTTPS for homeserver communication
4. **Timeouts**: Set appropriate timeouts for network requests
5. **Error Handling**: Don't expose sensitive information in errors

## Troubleshooting

### Endpoints Not Found

- Verify recipient has published endpoints
- Check homeserver URL is correct
- Verify network connectivity
- Check homeserver logs

### Publishing Fails

- Verify authentication is configured
- Check session is valid
- Verify homeserver permissions
- Review error messages

### Network Timeouts

- Increase timeout values
- Check network connectivity
- Verify homeserver is accessible
- Consider using direct IP if DNS fails

## References

- [Pubky Protocol Specification](https://github.com/pubky/pubky)
- [Payment Flow Guide](./PAYMENT_FLOW_GUIDE.md)
- [Key Architecture Guide](./KEY_ARCHITECTURE.md)


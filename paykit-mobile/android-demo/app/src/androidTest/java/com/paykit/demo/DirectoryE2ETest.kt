// DirectoryE2ETest.kt
// E2E Tests for Directory Operations
//
// Tests endpoint discovery, publishing, and removal via the Pubky directory.

package com.paykit.demo

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DirectoryE2ETest {
    
    // MARK: - Properties
    
    private lateinit var mockDirectoryService: MockDirectoryService
    private lateinit var mockKeyManager: MockKeyManager
    
    // MARK: - Setup/Teardown
    
    @Before
    fun setUp() {
        mockDirectoryService = MockDirectoryService()
        mockKeyManager = MockKeyManager()
    }
    
    @After
    fun tearDown() {
        mockDirectoryService.clear()
    }
    
    // MARK: - Endpoint Publishing Tests
    
    /** Test publishing a Noise endpoint */
    @Test
    fun testPublishEndpoint() {
        val identity = mockKeyManager.createIdentity(nickname = "publisher")
        val port = TestConfig.randomPort()
        val noisePubkey = TestDataGenerator.mockNoisePubkey()
        
        mockDirectoryService.publishEndpoint(
            pubkey = identity.publicKeyZ32,
            host = "192.168.1.100",
            port = port,
            noisePubkey = noisePubkey,
            metadata = "My payment server"
        )
        
        // Verify endpoint was published
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = identity.publicKeyZ32)
        assertNotNull(endpoint)
        assertEquals("192.168.1.100", endpoint?.host)
        assertEquals(port, endpoint?.port)
        assertEquals(noisePubkey, endpoint?.serverNoisePubkey)
        assertEquals("My payment server", endpoint?.metadata)
    }
    
    /** Test publishing endpoint without metadata */
    @Test
    fun testPublishEndpointWithoutMetadata() {
        val identity = mockKeyManager.createIdentity(nickname = "publisher")
        
        mockDirectoryService.publishEndpoint(
            pubkey = identity.publicKeyZ32,
            host = "10.0.0.1",
            port = 8080,
            noisePubkey = TestDataGenerator.mockNoisePubkey()
        )
        
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = identity.publicKeyZ32)
        assertNotNull(endpoint)
        assertNull(endpoint?.metadata)
    }
    
    /** Test updating an existing endpoint */
    @Test
    fun testUpdateEndpoint() {
        val identity = mockKeyManager.createIdentity(nickname = "publisher")
        
        // Publish initial endpoint
        mockDirectoryService.publishEndpoint(
            pubkey = identity.publicKeyZ32,
            host = "192.168.1.1",
            port = 8080,
            noisePubkey = "initial_key"
        )
        
        // Update with new endpoint
        val newPort = TestConfig.randomPort()
        mockDirectoryService.publishEndpoint(
            pubkey = identity.publicKeyZ32,
            host = "192.168.1.2",
            port = newPort,
            noisePubkey = "updated_key",
            metadata = "Updated server"
        )
        
        // Verify update
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = identity.publicKeyZ32)
        assertEquals("192.168.1.2", endpoint?.host)
        assertEquals(newPort, endpoint?.port)
        assertEquals("updated_key", endpoint?.serverNoisePubkey)
    }
    
    // MARK: - Endpoint Discovery Tests
    
    /** Test discovering an existing endpoint */
    @Test
    fun testDiscoverExistingEndpoint() {
        val pubkey = "existing_user_pubkey"
        
        mockDirectoryService.publishEndpoint(
            pubkey = pubkey,
            host = "example.com",
            port = 9999,
            noisePubkey = TestDataGenerator.mockNoisePubkey()
        )
        
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = pubkey)
        assertNotNull(endpoint)
        PaymentAssertions.assertEndpointValid(
            endpoint!!,
            expectedHost = "example.com",
            expectedPort = 9999
        )
    }
    
    /** Test discovering non-existent endpoint */
    @Test
    fun testDiscoverNonExistentEndpoint() {
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "unknown_user")
        assertNull(endpoint)
    }
    
    /** Test discovering multiple endpoints */
    @Test
    fun testDiscoverMultipleEndpoints() {
        // Publish multiple endpoints
        val users = listOf("user1", "user2", "user3")
        users.forEachIndexed { index, user ->
            mockDirectoryService.publishEndpoint(
                pubkey = user,
                host = "server$index.example.com",
                port = 8000 + index,
                noisePubkey = TestDataGenerator.mockNoisePubkey()
            )
        }
        
        // Verify each can be discovered
        users.forEachIndexed { index, user ->
            val endpoint = mockDirectoryService.discoverEndpoint(pubkey = user)
            assertNotNull("Should find endpoint for $user", endpoint)
            assertEquals("server$index.example.com", endpoint?.host)
            assertEquals(8000 + index, endpoint?.port)
        }
    }
    
    // MARK: - Endpoint Removal Tests
    
    /** Test removing an endpoint */
    @Test
    fun testRemoveEndpoint() {
        val pubkey = "removable_user"
        
        // Publish endpoint
        mockDirectoryService.publishEndpoint(
            pubkey = pubkey,
            host = "temp.example.com",
            port = 7777,
            noisePubkey = TestDataGenerator.mockNoisePubkey()
        )
        
        // Verify it exists
        assertNotNull(mockDirectoryService.discoverEndpoint(pubkey = pubkey))
        
        // Remove it
        mockDirectoryService.removeEndpoint(pubkey = pubkey)
        
        // Verify it's gone
        assertNull(mockDirectoryService.discoverEndpoint(pubkey = pubkey))
    }
    
    /** Test removing non-existent endpoint (should not error) */
    @Test
    fun testRemoveNonExistentEndpoint() {
        // Should not throw
        mockDirectoryService.removeEndpoint(pubkey = "never_existed")
        
        // Verify still nil
        assertNull(mockDirectoryService.discoverEndpoint(pubkey = "never_existed"))
    }
    
    // MARK: - Endpoint Validation Tests
    
    /** Test endpoint with localhost */
    @Test
    fun testEndpointWithLocalhost() {
        mockDirectoryService.publishEndpoint(
            pubkey = "local_user",
            host = "127.0.0.1",
            port = 12345,
            noisePubkey = TestDataGenerator.mockNoisePubkey()
        )
        
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "local_user")
        assertEquals("127.0.0.1", endpoint?.host)
    }
    
    /** Test endpoint with IPv6 */
    @Test
    fun testEndpointWithIPv6() {
        mockDirectoryService.publishEndpoint(
            pubkey = "ipv6_user",
            host = "::1",
            port = 8888,
            noisePubkey = TestDataGenerator.mockNoisePubkey()
        )
        
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "ipv6_user")
        assertEquals("::1", endpoint?.host)
    }
    
    /** Test endpoint with domain name */
    @Test
    fun testEndpointWithDomain() {
        mockDirectoryService.publishEndpoint(
            pubkey = "domain_user",
            host = "payments.example.org",
            port = 443,
            noisePubkey = TestDataGenerator.mockNoisePubkey()
        )
        
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "domain_user")
        assertEquals("payments.example.org", endpoint?.host)
        assertEquals(443, endpoint?.port)
    }
    
    // MARK: - Metadata Tests
    
    /** Test endpoint with long metadata */
    @Test
    fun testEndpointWithLongMetadata() {
        val longMetadata = "A".repeat(1000)
        
        mockDirectoryService.publishEndpoint(
            pubkey = "metadata_user",
            host = "meta.example.com",
            port = 5555,
            noisePubkey = TestDataGenerator.mockNoisePubkey(),
            metadata = longMetadata
        )
        
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "metadata_user")
        assertEquals(1000, endpoint?.metadata?.length)
    }
    
    /** Test endpoint with special characters in metadata */
    @Test
    fun testEndpointWithSpecialMetadata() {
        val specialMetadata = "Server: 🚀 Payment Hub™ | v2.0 <test>"
        
        mockDirectoryService.publishEndpoint(
            pubkey = "special_user",
            host = "special.example.com",
            port = 6666,
            noisePubkey = TestDataGenerator.mockNoisePubkey(),
            metadata = specialMetadata
        )
        
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "special_user")
        assertEquals(specialMetadata, endpoint?.metadata)
    }
    
    // MARK: - Stress Tests
    
    /** Test publishing many endpoints */
    @Test
    fun testPublishManyEndpoints() {
        for (i in 0 until 100) {
            mockDirectoryService.publishEndpoint(
                pubkey = "user_$i",
                host = "server$i.example.com",
                port = 10000 + i,
                noisePubkey = TestDataGenerator.mockNoisePubkey()
            )
        }
        
        // Verify all can be discovered
        for (i in 0 until 100) {
            val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "user_$i")
            assertNotNull("Should find endpoint for user_$i", endpoint)
        }
    }
    
    /** Test clearing all endpoints */
    @Test
    fun testClearAllEndpoints() {
        // Publish several endpoints
        for (i in 0 until 10) {
            mockDirectoryService.publishEndpoint(
                pubkey = "user_$i",
                host = "server.example.com",
                port = 8000 + i,
                noisePubkey = TestDataGenerator.mockNoisePubkey()
            )
        }
        
        // Clear all
        mockDirectoryService.clear()
        
        // Verify all are gone
        for (i in 0 until 10) {
            assertNull(mockDirectoryService.discoverEndpoint(pubkey = "user_$i"))
        }
    }
}

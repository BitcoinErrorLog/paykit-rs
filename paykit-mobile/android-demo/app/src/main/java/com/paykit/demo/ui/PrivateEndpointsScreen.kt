package com.paykit.demo.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.paykit.demo.storage.PrivateEndpointStorage
import com.paykit.mobile.KeyManager
import com.paykit.mobile.PrivateEndpointOffer

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PrivateEndpointsScreen(
    onNavigateBack: () -> Unit
) {
    val context = LocalContext.current
    val keyManager = remember { KeyManager(context) }
    val identityName by keyManager.currentIdentityName.collectAsState()
    
    var peers by remember { mutableStateOf<List<String>>(emptyList()) }
    var endpoints by remember { mutableStateOf<Map<String, List<PrivateEndpointOffer>>>(emptyMap()) }
    var isLoading by remember { mutableStateOf(true) }
    var showClearDialog by remember { mutableStateOf(false) }
    
    val storage = remember(identityName) {
        val name = identityName ?: "default"
        PrivateEndpointStorage(context, name)
    }
    
    fun refresh() {
        isLoading = true
        peers = storage.listPeers()
        endpoints = peers.associateWith { peer -> storage.listForPeer(peer) }
        isLoading = false
    }
    
    LaunchedEffect(identityName) {
        refresh()
    }
    
    // Clear All Dialog
    if (showClearDialog) {
        AlertDialog(
            onDismissRequest = { showClearDialog = false },
            title = { Text("Clear All Endpoints") },
            text = { Text("Are you sure you want to remove all private endpoints?") },
            confirmButton = {
                TextButton(
                    onClick = {
                        storage?.clearAll()
                        refresh()
                        showClearDialog = false
                    }
                ) {
                    Text("Clear", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Private Endpoints") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(onClick = { refresh() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                    IconButton(onClick = { showClearDialog = true }) {
                        Icon(Icons.Default.Delete, contentDescription = "Clear All")
                    }
                }
            )
        }
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            when {
                isLoading -> {
                    CircularProgressIndicator(
                        modifier = Modifier.align(Alignment.Center)
                    )
                }
                peers.isEmpty() -> {
                    EmptyEndpointsContent()
                }
                else -> {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(16.dp),
                        verticalArrangement = Arrangement.spacedBy(16.dp)
                    ) {
                        // Statistics Card
                        item {
                            StatisticsCard(
                                totalCount = endpoints.values.sumOf { it.size },
                                peerCount = peers.size,
                                expiredCount = 0 // TODO: Implement expiration
                            )
                        }
                        
                        // Peers
                        items(peers) { peer ->
                            PeerEndpointsCard(
                                peer = peer,
                                endpoints = endpoints[peer] ?: emptyList(),
                                onRemove = { methodId ->
                                    storage?.remove(peer, methodId)
                                    refresh()
                                },
                                onRemoveAll = {
                                    storage?.removeAllForPeer(peer)
                                    refresh()
                                }
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun EmptyEndpointsContent() {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            Icons.Default.Lock,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        
        Spacer(modifier = Modifier.height(16.dp))
        
        Text(
            "No Private Endpoints",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold
        )
        
        Spacer(modifier = Modifier.height(8.dp))
        
        Text(
            "Private endpoints are exchanged during secure payment sessions for enhanced privacy.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 16.dp)
        )
        
        Spacer(modifier = Modifier.height(24.dp))
        
        Column(
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            FeatureRow(Icons.Default.Person, "Per-peer dedicated addresses")
            FeatureRow(Icons.Default.Schedule, "Automatic expiration")
            FeatureRow(Icons.Default.Lock, "Encrypted storage")
        }
    }
}

@Composable
private fun FeatureRow(icon: androidx.compose.ui.graphics.vector.ImageVector, text: String) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Icon(
            icon,
            contentDescription = null,
            modifier = Modifier.size(16.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun StatisticsCard(
    totalCount: Int,
    peerCount: Int,
    expiredCount: Int
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceEvenly
        ) {
            StatItem(
                value = totalCount.toString(),
                label = "Total",
                icon = Icons.Default.Link
            )
            StatItem(
                value = peerCount.toString(),
                label = "Peers",
                icon = Icons.Default.Person
            )
            StatItem(
                value = expiredCount.toString(),
                label = "Expired",
                icon = Icons.Default.Warning,
                isWarning = expiredCount > 0
            )
        }
    }
}

@Composable
private fun StatItem(
    value: String,
    label: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    isWarning: Boolean = false
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Icon(
            icon,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = if (isWarning) MaterialTheme.colorScheme.error 
                   else MaterialTheme.colorScheme.primary
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
            value,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold
        )
        Text(
            label,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun PeerEndpointsCard(
    peer: String,
    endpoints: List<PrivateEndpointOffer>,
    onRemove: (String) -> Unit,
    onRemoveAll: () -> Unit
) {
    var expanded by remember { mutableStateOf(true) }
    
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            // Header
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Icon(
                        Icons.Default.Person,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary
                    )
                    Text(
                        truncatePeer(peer),
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Medium
                    )
                }
                
                Row {
                    IconButton(onClick = { expanded = !expanded }) {
                        Icon(
                            if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                            contentDescription = if (expanded) "Collapse" else "Expand"
                        )
                    }
                    IconButton(onClick = onRemoveAll) {
                        Icon(
                            Icons.Default.DeleteSweep,
                            contentDescription = "Remove All",
                            tint = MaterialTheme.colorScheme.error
                        )
                    }
                }
            }
            
            // Endpoints
            if (expanded) {
                Spacer(modifier = Modifier.height(8.dp))
                
                endpoints.forEach { endpoint ->
                    EndpointRow(
                        endpoint = endpoint,
                        onRemove = { onRemove(endpoint.methodId) }
                    )
                    if (endpoint != endpoints.last()) {
                        Divider(modifier = Modifier.padding(vertical = 8.dp))
                    }
                }
            }
        }
    }
}

@Composable
private fun EndpointRow(
    endpoint: PrivateEndpointOffer,
    onRemove: () -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.weight(1f)
        ) {
            MethodBadge(methodId = endpoint.methodId)
            
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    truncateEndpoint(endpoint.endpoint),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
        }
        
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            StatusBadge(isExpired = false)
            
            IconButton(onClick = onRemove) {
                Icon(
                    Icons.Default.Close,
                    contentDescription = "Remove",
                    tint = MaterialTheme.colorScheme.error,
                    modifier = Modifier.size(20.dp)
                )
            }
        }
    }
}

@Composable
private fun MethodBadge(methodId: String) {
    val (color, icon) = when (methodId.lowercase()) {
        "lightning" -> Color(0xFFFF9800) to Icons.Default.Bolt
        "onchain" -> Color(0xFFFFC107) to Icons.Default.CurrencyBitcoin
        else -> MaterialTheme.colorScheme.primary to Icons.Default.CreditCard
    }
    
    Surface(
        color = color.copy(alpha = 0.2f),
        shape = RoundedCornerShape(6.dp)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                icon,
                contentDescription = null,
                modifier = Modifier.size(12.dp),
                tint = color
            )
            Text(
                methodId,
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.Medium,
                color = color
            )
        }
    }
}

@Composable
private fun StatusBadge(isExpired: Boolean) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Box(
            modifier = Modifier
                .size(8.dp)
                .clip(CircleShape)
                .background(if (isExpired) Color.Red else Color.Green)
        )
        Text(
            if (isExpired) "Expired" else "Active",
            style = MaterialTheme.typography.labelSmall,
            color = if (isExpired) Color.Red else Color.Green
        )
    }
}

private fun truncatePeer(peer: String): String {
    return if (peer.length > 16) {
        "${peer.take(8)}...${peer.takeLast(8)}"
    } else {
        peer
    }
}

private fun truncateEndpoint(endpoint: String): String {
    return if (endpoint.length > 40) {
        "${endpoint.take(20)}...${endpoint.takeLast(15)}"
    } else {
        endpoint
    }
}


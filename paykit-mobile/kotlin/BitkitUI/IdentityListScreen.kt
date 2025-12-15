package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * Identity information model
 */
data class IdentityInfo(
    val id: String,
    val name: String,
    val publicKey: String,
    val createdAt: Long
)

/**
 * Identity Management view model for Bitkit integration
 */
class BitkitIdentityViewModel(
    private val identityManager: IdentityManagerProtocol
) {
    var identities by mutableStateOf<List<IdentityInfo>>(emptyList())
        private set
    var currentIdentityName by mutableStateOf<String?>(null)
        private set
    var showingCreateDialog by mutableStateOf(false)
    var identityToDelete by mutableStateOf<IdentityInfo?>(null)
    var showingDeleteDialog by mutableStateOf(false)
    var errorMessage by mutableStateOf<String?>(null)
    var showingError by mutableStateOf(false)
    
    fun loadIdentities() {
        identities = identityManager.listIdentities()
        currentIdentityName = identityManager.getCurrentIdentityName()
    }
    
    fun switchToIdentity(name: String) {
        try {
            identityManager.switchIdentity(name)
            currentIdentityName = name
            loadIdentities()
        } catch (e: Exception) {
            errorMessage = e.message
            showingError = true
        }
    }
    
    fun createIdentity(name: String) {
        try {
            identityManager.createIdentity(name)
            loadIdentities()
        } catch (e: Exception) {
            errorMessage = e.message
            showingError = true
        }
    }
    
    fun deleteIdentity(name: String) {
        try {
            identityManager.deleteIdentity(name)
            loadIdentities()
        } catch (e: Exception) {
            errorMessage = e.message
            showingError = true
        }
    }
}

/**
 * Identity Manager protocol that Bitkit must implement
 */
interface IdentityManagerProtocol {
    fun listIdentities(): List<IdentityInfo>
    fun getCurrentIdentityName(): String?
    fun switchIdentity(name: String)
    fun createIdentity(name: String)
    fun deleteIdentity(name: String)
}

/**
 * Identity Management screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitIdentityListScreen(
    viewModel: BitkitIdentityViewModel,
    onNavigateBack: () -> Unit
) {
    var newIdentityName by remember { mutableStateOf("") }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identities") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { viewModel.showingCreateDialog = true }
            ) {
                Icon(Icons.Default.Add, contentDescription = "Create Identity")
            }
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            items(viewModel.identities) { identity ->
                IdentityRow(
                    identity = identity,
                    isCurrent = viewModel.currentIdentityName == identity.name,
                    onSwitch = { viewModel.switchToIdentity(identity.name) },
                    onDelete = {
                        viewModel.identityToDelete = identity
                        viewModel.showingDeleteDialog = true
                    }
                )
            }
        }
    }
    
    // Create Dialog
    if (viewModel.showingCreateDialog) {
        AlertDialog(
            onDismissRequest = { viewModel.showingCreateDialog = false },
            title = { Text("Create Identity") },
            text = {
                OutlinedTextField(
                    value = newIdentityName,
                    onValueChange = { newIdentityName = it },
                    label = { Text("Identity Name") },
                    modifier = Modifier.fillMaxWidth()
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        viewModel.createIdentity(newIdentityName)
                        newIdentityName = ""
                        viewModel.showingCreateDialog = false
                    },
                    enabled = newIdentityName.isNotEmpty()
                ) {
                    Text("Create")
                }
            },
            dismissButton = {
                TextButton(onClick = { viewModel.showingCreateDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
    
    // Delete Dialog
    viewModel.identityToDelete?.let { identity ->
        if (viewModel.showingDeleteDialog) {
            AlertDialog(
                onDismissRequest = { viewModel.showingDeleteDialog = false },
                title = { Text("Delete Identity") },
                text = {
                    Text("Are you sure? This will delete all data for '${identity.name}'. This cannot be undone.")
                },
                confirmButton = {
                    TextButton(
                        onClick = {
                            viewModel.deleteIdentity(identity.name)
                            viewModel.identityToDelete = null
                            viewModel.showingDeleteDialog = false
                        }
                    ) {
                        Text("Delete", color = MaterialTheme.colorScheme.error)
                    }
                },
                dismissButton = {
                    TextButton(onClick = {
                        viewModel.identityToDelete = null
                        viewModel.showingDeleteDialog = false
                    }) {
                        Text("Cancel")
                    }
                }
            )
        }
    }
    
    // Error Dialog
    if (viewModel.showingError) {
        AlertDialog(
            onDismissRequest = { viewModel.showingError = false },
            title = { Text("Error") },
            text = { Text(viewModel.errorMessage ?: "Unknown error") },
            confirmButton = {
                TextButton(onClick = { viewModel.showingError = false }) {
                    Text("OK")
                }
            }
        )
    }
    
    LaunchedEffect(Unit) {
        viewModel.loadIdentities()
    }
}

@Composable
fun IdentityRow(
    identity: IdentityInfo,
    isCurrent: Boolean,
    onSwitch: () -> Unit,
    onDelete: () -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        identity.name,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.Medium
                    )
                    if (isCurrent) {
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(
                            "(Current)",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                }
                Text(
                    identity.publicKey,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            if (!isCurrent) {
                TextButton(onClick = onSwitch) {
                    Text("Switch")
                }
            }
            
            IconButton(onClick = onDelete) {
                Icon(Icons.Default.Delete, contentDescription = "Delete")
            }
        }
    }
}

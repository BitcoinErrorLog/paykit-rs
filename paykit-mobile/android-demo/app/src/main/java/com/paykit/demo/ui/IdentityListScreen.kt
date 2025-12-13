package com.paykit.demo.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.paykit.mobile.IdentityInfo
import com.paykit.mobile.KeyManager
import com.paykit.mobile.KeyManagerException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun IdentityListScreen(
    onNavigateBack: () -> Unit,
    keyManager: KeyManager = KeyManager(LocalContext.current)
) {
    var identities by remember { mutableStateOf<List<IdentityInfo>>(emptyList()) }
    var showingCreateDialog by remember { mutableStateOf(false) }
    var identityToDelete by remember { mutableStateOf<IdentityInfo?>(null) }
    var showingDeleteDialog by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var showingError by remember { mutableStateOf(false) }
    
    val currentIdentityName by keyManager.currentIdentityName.collectAsState()
    
    LaunchedEffect(Unit) {
        loadIdentities(keyManager) { identities = it }
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identities") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showingCreateDialog = true }
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
            items(identities) { identity ->
                IdentityRow(
                    identity = identity,
                    isCurrent = currentIdentityName == identity.name,
                    onSwitch = {
                        try {
                            keyManager.switchIdentity(identity.name)
                            loadIdentities(keyManager) { identities = it }
                        } catch (e: Exception) {
                            errorMessage = e.message
                            showingError = true
                        }
                    },
                    onDelete = {
                        identityToDelete = identity
                        showingDeleteDialog = true
                    }
                )
            }
        }
    }
    
    if (showingCreateDialog) {
        CreateIdentityDialog(
            onDismiss = { showingCreateDialog = false },
            onCreate = { name, nickname ->
                try {
                    keyManager.createIdentity(name, nickname)
                    loadIdentities(keyManager) { identities = it }
                    showingCreateDialog = false
                } catch (e: Exception) {
                    errorMessage = e.message
                    showingError = true
                }
            }
        )
    }
    
    identityToDelete?.let { identity ->
        if (showingDeleteDialog) {
            AlertDialog(
                onDismissRequest = {
                    showingDeleteDialog = false
                    identityToDelete = null
                },
                title = { Text("Delete Identity") },
                text = {
                    Text("Are you sure? This will delete all data for '${identity.name}'. This cannot be undone.")
                },
                confirmButton = {
                    TextButton(
                        onClick = {
                            try {
                                keyManager.deleteIdentity(identity.name)
                                loadIdentities(keyManager) { identities = it }
                                showingDeleteDialog = false
                                identityToDelete = null
                            } catch (e: Exception) {
                                errorMessage = e.message
                                showingError = true
                                showingDeleteDialog = false
                            }
                        }
                    ) {
                        Text("Delete", color = MaterialTheme.colorScheme.error)
                    }
                },
                dismissButton = {
                    TextButton(
                        onClick = {
                            showingDeleteDialog = false
                            identityToDelete = null
                        }
                    ) {
                        Text("Cancel")
                    }
                }
            )
        }
    }
    
    if (showingError) {
        AlertDialog(
            onDismissRequest = { showingError = false },
            title = { Text("Error") },
            text = { Text(errorMessage ?: "An unknown error occurred") },
            confirmButton = {
                TextButton(onClick = { showingError = false }) {
                    Text("OK")
                }
            }
        )
    }
}

@Composable
fun IdentityRow(
    identity: IdentityInfo,
    isCurrent: Boolean,
    onSwitch: () -> Unit,
    onDelete: () -> Unit
) {
    var showingMenu by remember { mutableStateOf(false) }
    
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
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
                        text = identity.nickname ?: identity.name,
                        style = MaterialTheme.typography.titleMedium
                    )
                    if (isCurrent) {
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(
                            text = "(Current)",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.secondary
                        )
                    }
                }
                
                Spacer(modifier = Modifier.height(4.dp))
                
                Text(
                    text = identity.publicKeyZ32,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.secondary,
                    maxLines = 1
                )
                
                if (identity.nickname != null && identity.nickname != identity.name) {
                    Spacer(modifier = Modifier.height(2.dp))
                    Text(
                        text = identity.name,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.secondary
                    )
                }
            }
            
            if (!isCurrent) {
                Spacer(modifier = Modifier.width(8.dp))
                Button(onClick = onSwitch) {
                    Text("Switch")
                }
            }
            
            Spacer(modifier = Modifier.width(8.dp))
            
            IconButton(onClick = { showingMenu = true }) {
                Icon(Icons.Default.MoreVert, contentDescription = "More")
            }
            
            DropdownMenu(
                expanded = showingMenu,
                onDismissRequest = { showingMenu = false }
            ) {
                if (!isCurrent) {
                    DropdownMenuItem(
                        text = { Text("Switch to This Identity") },
                        onClick = {
                            onSwitch()
                            showingMenu = false
                        }
                    )
                }
                DropdownMenuItem(
                    text = { Text("Delete", color = MaterialTheme.colorScheme.error) },
                    onClick = {
                        onDelete()
                        showingMenu = false
                    }
                )
            }
        }
    }
}

@Composable
fun CreateIdentityDialog(
    onDismiss: () -> Unit,
    onCreate: (String, String?) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var nickname by remember { mutableStateOf("") }
    
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Create Identity") },
        text = {
            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Identity Name") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )
                
                OutlinedTextField(
                    value = nickname,
                    onValueChange = { nickname = it },
                    label = { Text("Nickname (Optional)") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )
                
                Text(
                    text = "The identity name must be unique and cannot be changed later.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.secondary
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    onCreate(
                        name.trim(),
                        nickname.trim().takeIf { it.isNotEmpty() }
                    )
                },
                enabled = name.isNotBlank()
            ) {
                Text("Create")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

private fun loadIdentities(
    keyManager: KeyManager,
    onLoaded: (List<IdentityInfo>) -> Unit
) {
    try {
        val identities = keyManager.listIdentities()
        onLoaded(identities)
    } catch (e: Exception) {
        onLoaded(emptyList())
    }
}


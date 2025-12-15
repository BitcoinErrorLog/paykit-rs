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
import com.paykit.mobile.bitkit.Contact

/**
 * Contacts view model for Bitkit integration
 */
class BitkitContactsViewModel(private val contactStorage: ContactStorageProtocol) {
    var contacts by mutableStateOf<List<Contact>>(emptyList())
        private set
    var filteredContacts by mutableStateOf<List<Contact>>(emptyList())
        private set
    var isLoading by mutableStateOf(false)
        private set
    var searchText by mutableStateOf("")
    var showingAddSheet by mutableStateOf(false)
    
    fun loadContacts() {
        isLoading = true
        contacts = contactStorage.listContacts()
        filteredContacts = contacts
        isLoading = false
    }
    
    fun search(query: String) {
        searchText = query
        filteredContacts = if (query.isEmpty()) {
            contacts
        } else {
            contacts.filter { contact ->
                contact.name.contains(query, ignoreCase = true) ||
                contact.pubkey.contains(query, ignoreCase = true)
            }
        }
    }
    
    fun addContact(name: String, pubkey: String) {
        val contact = Contact(
            id = java.util.UUID.randomUUID().toString(),
            name = name,
            pubkey = pubkey
        )
        contacts = contacts + contact
        filteredContacts = contacts
        // Bitkit should persist this to their storage
    }
    
    fun deleteContact(contact: Contact) {
        contacts = contacts.filter { it.id != contact.id }
        filteredContacts = contacts
        // Bitkit should delete from their storage
    }
}

/**
 * Contacts screen component
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitContactsScreen(viewModel: BitkitContactsViewModel) {
    var newContactName by remember { mutableStateOf("") }
    var newContactPubkey by remember { mutableStateOf("") }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Contacts") },
                actions = {
                    IconButton(onClick = { viewModel.showingAddSheet = true }) {
                        Icon(Icons.Default.Add, contentDescription = "Add Contact")
                    }
                }
            )
        }
    ) { padding ->
        when {
            viewModel.isLoading -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            viewModel.filteredContacts.isEmpty() -> {
                EmptyContactsView(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    onAddClick = { viewModel.showingAddSheet = true }
                )
            }
            else -> {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                ) {
                    items(viewModel.filteredContacts) { contact ->
                        ContactRow(
                            contact = contact,
                            onDelete = { viewModel.deleteContact(contact) }
                        )
                    }
                }
            }
        }
    }
    
    // Add Contact Sheet
    if (viewModel.showingAddSheet) {
        AddContactSheet(
            name = newContactName,
            pubkey = newContactPubkey,
            onNameChange = { newContactName = it },
            onPubkeyChange = { newContactPubkey = it },
            onAdd = {
                viewModel.addContact(newContactName, newContactPubkey)
                newContactName = ""
                newContactPubkey = ""
                viewModel.showingAddSheet = false
            },
            onDismiss = { viewModel.showingAddSheet = false }
        )
    }
}

@Composable
fun EmptyContactsView(
    modifier: Modifier = Modifier,
    onAddClick: () -> Unit
) {
    Column(
        modifier = modifier.padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            Icons.Default.PersonAdd,
            contentDescription = null,
            modifier = Modifier.size(80.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(24.dp))
        Text(
            text = "No Contacts",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = MaterialTheme.typography.titleLarge.fontWeight
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Add contacts to easily send payments",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(24.dp))
        Button(onClick = onAddClick) {
            Icon(Icons.Default.Add, contentDescription = null)
            Spacer(modifier = Modifier.width(8.dp))
            Text("Add Contact")
        }
    }
}

@Composable
fun ContactRow(
    contact: Contact,
    onDelete: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 4.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = contact.name,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = contact.pubkey,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Default.Delete, contentDescription = "Delete")
            }
        }
    }
}

@Composable
fun AddContactSheet(
    name: String,
    pubkey: String,
    onNameChange: (String) -> Unit,
    onPubkeyChange: (String) -> Unit,
    onAdd: () -> Unit,
    onDismiss: () -> Unit
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add Contact") },
        text = {
            Column(verticalArrangement = Arrangement.spacing(16.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = onNameChange,
                    label = { Text("Name") },
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = pubkey,
                    onValueChange = onPubkeyChange,
                    label = { Text("Public Key") },
                    modifier = Modifier.fillMaxWidth()
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = onAdd,
                enabled = name.isNotEmpty() && pubkey.isNotEmpty()
            ) {
                Text("Add")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

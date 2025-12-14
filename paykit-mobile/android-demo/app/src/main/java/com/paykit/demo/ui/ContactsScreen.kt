package com.paykit.demo.ui

import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.paykit.demo.model.Contact
import com.paykit.demo.PaykitDemoApp
import com.paykit.demo.PaykitClientWrapper
import com.paykit.demo.DirectoryService
import com.paykit.mobile.KeyManager
import com.paykit.demo.storage.ContactStorage
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.*

/**
 * Contacts Screen
 *
 * Displays and manages payment contacts with persistent storage.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContactsScreen() {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current
    val keyManager = remember { KeyManager(context) }
    val currentIdentityName by keyManager.currentIdentityName.collectAsState()
    val storage = remember(currentIdentityName) {
        ContactStorage(context, currentIdentityName ?: "default")
    }
    
    var contacts by remember { mutableStateOf(storage.listContacts()) }
    var searchQuery by remember { mutableStateOf("") }
    var showAddDialog by remember { mutableStateOf(false) }
    var selectedContact by remember { mutableStateOf<Contact?>(null) }
    var showDiscoveryDialog by remember { mutableStateOf(false) }
    var discoveredContacts by remember { mutableStateOf<List<DiscoveredContact>>(emptyList()) }
    var isDiscovering by remember { mutableStateOf(false) }
    var discoveryError by remember { mutableStateOf<String?>(null) }
    
    val paykitClient = remember { PaykitDemoApp.paykitClient }
    val directoryService = remember { paykitClient.createDirectoryService(context) }
    val scope = rememberCoroutineScope()
    
    val filteredContacts = remember(contacts, searchQuery) {
        if (searchQuery.isEmpty()) contacts
        else storage.searchContacts(searchQuery)
    }
    
    fun refreshContacts() {
        contacts = storage.listContacts()
    }
    
    fun discoverContacts() {
        isDiscovering = true
        discoveryError = null
        
        scope.launch {
            try {
                // Get current identity's public key
                val currentIdentityName = currentIdentityName ?: "default"
                val publicKey = keyManager.getCurrentPublicKeyZ32() ?: run {
                    discoveryError = "No active identity found"
                    isDiscovering = false
                    return@launch
                }
                
                // Fetch known contacts from Pubky follows
                val contactPubkeys = directoryService.fetchKnownContacts(publicKey)
                
                // Fetch supported payments for each contact to get more info
                val discovered = mutableListOf<DiscoveredContact>()
                for (pubkey in contactPubkeys) {
                    // Check if contact already exists locally
                    val existingContact = contacts.firstOrNull { it.publicKeyZ32 == pubkey }
                    if (existingContact != null) {
                        continue // Skip contacts we already have
                    }
                    
                    // Try to fetch supported payments to see if they have payment methods
                    val supportedPayments = directoryService.fetchSupportedPayments(pubkey)
                    val hasPaymentMethods = supportedPayments.isNotEmpty()
                    
                    // Create a discovered contact
                    val discoveredContact = DiscoveredContact(
                        publicKeyZ32 = pubkey,
                        hasPaymentMethods = hasPaymentMethods,
                        supportedMethods = supportedPayments.map { it.methodId }
                    )
                    discovered.add(discoveredContact)
                }
                
                discoveredContacts = discovered
                showDiscoveryDialog = true
                isDiscovering = false
            } catch (e: Exception) {
                discoveryError = "Failed to discover contacts: ${e.message}"
                isDiscovering = false
            }
        }
    }
    
    fun importDiscovered(contactsToImport: List<DiscoveredContact>) {
        for (discovered in contactsToImport) {
            // Generate a default name from the public key
            val name = "Contact ${discovered.publicKeyZ32.take(8)}"
            val contact = Contact.create(
                discovered.publicKeyZ32,
                name,
                if (discovered.hasPaymentMethods) "Discovered from follows" else null
            )
            storage.saveContact(contact)
        }
        refreshContacts()
        showDiscoveryDialog = false
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Contacts") },
                actions = {
                    IconButton(onClick = { discoverContacts() }) {
                        Icon(Icons.Default.PersonAdd, contentDescription = "Discover Contacts")
                    }
                    IconButton(onClick = { showAddDialog = true }) {
                        Icon(Icons.Default.Add, contentDescription = "Add Contact")
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            // Search bar
            OutlinedTextField(
                value = searchQuery,
                onValueChange = { searchQuery = it },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                placeholder = { Text("Search contacts") },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                singleLine = true,
                trailingIcon = {
                    if (searchQuery.isNotEmpty()) {
                        IconButton(onClick = { searchQuery = "" }) {
                            Icon(Icons.Default.Clear, contentDescription = "Clear")
                        }
                    }
                }
            )
            
            if (filteredContacts.isEmpty()) {
                // Empty state
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(32.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center
                ) {
                    Icon(
                        Icons.Default.PersonAdd,
                        contentDescription = null,
                        modifier = Modifier.size(80.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        text = if (searchQuery.isEmpty()) "No Contacts Yet" else "No Results",
                        style = MaterialTheme.typography.titleLarge
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = if (searchQuery.isEmpty()) 
                            "Add contacts to easily send payments\nto your favorite recipients."
                        else 
                            "No contacts match your search.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center
                    )
                    if (searchQuery.isEmpty()) {
                        Spacer(modifier = Modifier.height(24.dp))
                        Button(onClick = { showAddDialog = true }) {
                            Icon(Icons.Default.Add, contentDescription = null)
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("Add Contact")
                        }
                    }
                }
            } else {
                // Contact list
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(vertical = 8.dp)
                ) {
                    items(
                        items = filteredContacts,
                        key = { it.id }
                    ) { contact ->
                        ContactListItem(
                            contact = contact,
                            onClick = { selectedContact = contact },
                            onCopyKey = {
                                clipboardManager.setText(AnnotatedString(contact.publicKeyZ32))
                                Toast.makeText(context, "Public key copied", Toast.LENGTH_SHORT).show()
                            },
                            onDelete = {
                                storage.deleteContact(contact.id)
                                refreshContacts()
                            }
                        )
                    }
                }
            }
        }
    }
    
    // Add contact dialog
    if (showAddDialog) {
        AddContactDialog(
            onDismiss = { showAddDialog = false },
            onSave = { name, publicKey, notes ->
                val contact = Contact.create(publicKey, name, notes)
                storage.saveContact(contact)
                refreshContacts()
                showAddDialog = false
            }
        )
    }
    
    // Contact detail sheet
    selectedContact?.let { contact ->
        ContactDetailSheet(
            contact = contact,
            onDismiss = { selectedContact = null },
            onCopyKey = {
                clipboardManager.setText(AnnotatedString(contact.publicKeyZ32))
                Toast.makeText(context, "Public key copied", Toast.LENGTH_SHORT).show()
            }
        )
    }
    
    // Discovery dialog
    if (showDiscoveryDialog) {
        DiscoverContactsDialog(
            contacts = discoveredContacts,
            isDiscovering = isDiscovering,
            error = discoveryError,
            onDismiss = { showDiscoveryDialog = false },
            onImport = { contactsToImport -> importDiscovered(contactsToImport) }
        )
    }
}

/**
 * A discovered contact from Pubky follows
 */
data class DiscoveredContact(
    val id: String,
    val publicKeyZ32: String,
    val hasPaymentMethods: Boolean,
    val supportedMethods: List<String>
) {
    constructor(
        publicKeyZ32: String,
        hasPaymentMethods: Boolean,
        supportedMethods: List<String>
    ) : this(
        id = publicKeyZ32,
        publicKeyZ32 = publicKeyZ32,
        hasPaymentMethods = hasPaymentMethods,
        supportedMethods = supportedMethods
    )
    
    val abbreviatedKey: String
        get() {
            return if (publicKeyZ32.length > 16) {
                val prefix = publicKeyZ32.take(8)
                val suffix = publicKeyZ32.takeLast(8)
                "$prefix...$suffix"
            } else {
                publicKeyZ32
            }
        }
}

@Composable
fun DiscoverContactsDialog(
    contacts: List<DiscoveredContact>,
    isDiscovering: Boolean,
    error: String?,
    onDismiss: () -> Unit,
    onImport: (List<DiscoveredContact>) -> Unit
) {
    var selectedContacts by remember { mutableStateOf<Set<String>>(emptySet()) }
    
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Discovered Contacts") },
        text = {
            Column {
                if (isDiscovering) {
                    CircularProgressIndicator()
                    Spacer(modifier = Modifier.height(16.dp))
                    Text("Discovering contacts...")
                } else if (error != null) {
                    Text(
                        text = error,
                        color = MaterialTheme.colorScheme.error
                    )
                } else if (contacts.isEmpty()) {
                    Text("No new contacts found in your follows list.\nAll contacts may already be imported.")
                } else {
                    Text("Select contacts to import:")
                    Spacer(modifier = Modifier.height(8.dp))
                    LazyColumn(
                        modifier = Modifier.heightIn(max = 400.dp)
                    ) {
                        items(contacts) { contact ->
                            DiscoveredContactItem(
                                contact = contact,
                                isSelected = selectedContacts.contains(contact.id),
                                onToggle = {
                                    selectedContacts = if (selectedContacts.contains(contact.id)) {
                                        selectedContacts - contact.id
                                    } else {
                                        selectedContacts + contact.id
                                    }
                                }
                            )
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val toImport = contacts.filter { selectedContacts.contains(it.id) }
                    onImport(toImport)
                },
                enabled = selectedContacts.isNotEmpty() && !isDiscovering && error == null
            ) {
                Text("Import (${selectedContacts.size})")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

@Composable
fun DiscoveredContactItem(
    contact: DiscoveredContact,
    isSelected: Boolean,
    onToggle: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onToggle() }
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Checkbox(
            checked = isSelected,
            onCheckedChange = { onToggle() }
        )
        
        Spacer(modifier = Modifier.width(8.dp))
        
        Box(
            modifier = Modifier
                .size(40.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.secondaryContainer),
            contentAlignment = Alignment.Center
        ) {
            Icon(
                Icons.Default.PersonAdd,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSecondaryContainer
            )
        }
        
        Spacer(modifier = Modifier.width(12.dp))
        
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = contact.abbreviatedKey,
                style = MaterialTheme.typography.bodyMedium,
                fontFamily = FontFamily.Monospace
            )
            
            if (contact.hasPaymentMethods) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Default.CheckCircle,
                        contentDescription = null,
                        modifier = Modifier.size(12.dp),
                        tint = Color(0xFF4CAF50)
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = "Has payment methods",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                
                if (contact.supportedMethods.isNotEmpty()) {
                    Text(
                        text = contact.supportedMethods.joinToString(", "),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            } else {
                Text(
                    text = "No payment methods",
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(0xFFFF9800)
                )
            }
        }
    }
}

@Composable
fun ContactListItem(
    contact: Contact,
    onClick: () -> Unit,
    onCopyKey: () -> Unit,
    onDelete: () -> Unit
) {
    var showDeleteConfirm by remember { mutableStateOf(false) }
    
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 4.dp)
            .clickable { onClick() }
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Avatar
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primaryContainer),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = contact.name.take(1).uppercase(),
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onPrimaryContainer
                )
            }
            
            Spacer(modifier = Modifier.width(16.dp))
            
            // Info
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = contact.name,
                    style = MaterialTheme.typography.titleSmall
                )
                Text(
                    text = contact.abbreviatedKey,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            // Actions
            Column(horizontalAlignment = Alignment.End) {
                if (contact.paymentCount > 0) {
                    Text(
                        text = "${contact.paymentCount} payments",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                Row {
                    IconButton(onClick = onCopyKey) {
                        Icon(
                            Icons.Default.ContentCopy,
                            contentDescription = "Copy",
                            modifier = Modifier.size(20.dp)
                        )
                    }
                    IconButton(onClick = { showDeleteConfirm = true }) {
                        Icon(
                            Icons.Default.Delete,
                            contentDescription = "Delete",
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.error
                        )
                    }
                }
            }
        }
    }
    
    if (showDeleteConfirm) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            title = { Text("Delete Contact?") },
            text = { Text("Are you sure you want to delete ${contact.name}?") },
            confirmButton = {
                TextButton(
                    onClick = {
                        onDelete()
                        showDeleteConfirm = false
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = Color.Red)
                ) {
                    Text("Delete")
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirm = false }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddContactDialog(
    onDismiss: () -> Unit,
    onSave: (name: String, publicKey: String, notes: String?) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var publicKey by remember { mutableStateOf("") }
    var notes by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add Contact") },
        text = {
            Column {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it; error = null },
                    label = { Text("Name") },
                    modifier = Modifier.fillMaxWidth(),
                    isError = error != null && name.isEmpty()
                )
                Spacer(modifier = Modifier.height(8.dp))
                OutlinedTextField(
                    value = publicKey,
                    onValueChange = { publicKey = it; error = null },
                    label = { Text("Public Key (z-base32)") },
                    modifier = Modifier.fillMaxWidth(),
                    isError = error != null && publicKey.isEmpty(),
                    textStyle = LocalTextStyle.current.copy(fontFamily = FontFamily.Monospace)
                )
                Spacer(modifier = Modifier.height(8.dp))
                OutlinedTextField(
                    value = notes,
                    onValueChange = { notes = it },
                    label = { Text("Notes (optional)") },
                    modifier = Modifier.fillMaxWidth(),
                    minLines = 2
                )
                error?.let {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = it,
                        color = Color.Red,
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    when {
                        name.isEmpty() -> error = "Name is required"
                        publicKey.isEmpty() -> error = "Public key is required"
                        else -> onSave(name, publicKey, notes.ifEmpty { null })
                    }
                }
            ) {
                Text("Save")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContactDetailSheet(
    contact: Contact,
    onDismiss: () -> Unit,
    onCopyKey: () -> Unit
) {
    val dateFormat = remember { SimpleDateFormat("MMM d, yyyy", Locale.getDefault()) }
    
    ModalBottomSheet(
        onDismissRequest = onDismiss
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(24.dp)
        ) {
            // Header
            Row(
                verticalAlignment = Alignment.CenterVertically
            ) {
                Box(
                    modifier = Modifier
                        .size(56.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.primaryContainer),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = contact.name.take(1).uppercase(),
                        style = MaterialTheme.typography.headlineMedium,
                        color = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                }
                Spacer(modifier = Modifier.width(16.dp))
                Text(
                    text = contact.name,
                    style = MaterialTheme.typography.headlineSmall
                )
            }
            
            Spacer(modifier = Modifier.height(24.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(16.dp))
            
            // Public Key
            Text(
                text = "Public Key",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = contact.publicKeyZ32,
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace
            )
            Spacer(modifier = Modifier.height(8.dp))
            OutlinedButton(onClick = onCopyKey) {
                Icon(Icons.Default.ContentCopy, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Copy Public Key")
            }
            
            // Notes
            contact.notes?.let { notes ->
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = "Notes",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = notes,
                    style = MaterialTheme.typography.bodyMedium
                )
            }
            
            Spacer(modifier = Modifier.height(16.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(16.dp))
            
            // Statistics
            Text(
                text = "Statistics",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(8.dp))
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Payments")
                Text(
                    text = "${contact.paymentCount}",
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            Spacer(modifier = Modifier.height(4.dp))
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Added")
                Text(
                    text = dateFormat.format(Date(contact.createdAt)),
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            contact.lastPaymentAt?.let { lastPayment ->
                Spacer(modifier = Modifier.height(4.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text("Last Payment")
                    Text(
                        text = dateFormat.format(Date(lastPayment)),
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            
            Spacer(modifier = Modifier.height(32.dp))
        }
    }
}


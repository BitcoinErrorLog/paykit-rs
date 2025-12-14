package com.paykit.demo.ui

import android.Manifest
import android.content.pm.PackageManager
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.paykit.demo.PaykitDemoApp
import com.paykit.mobile.UriType

/**
 * QR Scanner Screen
 *
 * Scans QR codes and parses Paykit URIs.
 * Note: This is a basic implementation. For production, integrate a QR scanning library
 * like ML Kit Barcode Scanning or ZXing.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QRScannerScreen(
    onDismiss: () -> Unit,
    onScanned: (ScannedUriResult) -> Unit
) {
    val context = LocalContext.current
    val paykitClient = remember { PaykitDemoApp.paykitClient }
    
    var scannedCode by remember { mutableStateOf<String?>(null) }
    var hasPermission by remember { mutableStateOf(false) }
    var showResultDialog by remember { mutableStateOf(false) }
    var scannedResult by remember { mutableStateOf<ScannedUriResult?>(null) }
    
    // Request camera permission
    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        hasPermission = isGranted
        if (!isGranted) {
            Toast.makeText(context, "Camera permission is required for QR scanning", Toast.LENGTH_LONG).show()
        }
    }
    
    LaunchedEffect(Unit) {
        val permission = Manifest.permission.CAMERA
        hasPermission = ContextCompat.checkSelfPermission(context, permission) == PackageManager.PERMISSION_GRANTED
        if (!hasPermission) {
            permissionLauncher.launch(permission)
        }
    }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Scan QR Code") },
                navigationIcon = {
                    IconButton(onClick = onDismiss) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            if (!hasPermission) {
                // Permission not granted
                Icon(
                    Icons.Default.CameraAlt,
                    contentDescription = null,
                    modifier = Modifier.size(80.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = "Camera Permission Required",
                    style = MaterialTheme.typography.titleLarge
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Please grant camera permission to scan QR codes",
                    style = MaterialTheme.typography.bodyMedium,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(24.dp))
                Button(onClick = { permissionLauncher.launch(Manifest.permission.CAMERA) }) {
                    Text("Grant Permission")
                }
            } else {
                // Camera preview placeholder
                // TODO: Integrate actual camera preview with QR scanning
                // For now, show a placeholder and manual input option
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .weight(1f)
                        .padding(16.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center
                    ) {
                        Icon(
                            Icons.Default.QrCodeScanner,
                            contentDescription = null,
                            modifier = Modifier.size(120.dp),
                            tint = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.height(24.dp))
                        Text(
                            text = "QR Scanner",
                            style = MaterialTheme.typography.headlineMedium
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "Position QR code within frame",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            textAlign = TextAlign.Center
                        )
                        Spacer(modifier = Modifier.height(32.dp))
                        Text(
                            text = "Note: Camera preview integration pending.\nUse manual input for testing.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            textAlign = TextAlign.Center
                        )
                    }
                }
                
                // Manual input option for testing
                OutlinedTextField(
                    value = scannedCode ?: "",
                    onValueChange = { scannedCode = it },
                    label = { Text("Or enter QR code data manually") },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp),
                    trailingIcon = {
                        if (scannedCode != null && scannedCode!!.isNotEmpty()) {
                            IconButton(onClick = {
                                handleScannedCode(scannedCode!!, paykitClient) { result ->
                                    scannedResult = result
                                    showResultDialog = true
                                }
                            }) {
                                Icon(Icons.Default.Check, contentDescription = "Process")
                            }
                        }
                    }
                )
                Spacer(modifier = Modifier.height(16.dp))
            }
        }
    }
    
    // Result dialog
    scannedResult?.let { result ->
        AlertDialog(
            onDismissRequest = {
                showResultDialog = false
                onScanned(result)
                onDismiss()
            },
            title = { Text("QR Code Scanned") },
            text = {
                Column {
                    Text(resultDescription(result))
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showResultDialog = false
                        onScanned(result)
                        onDismiss()
                    }
                ) {
                    Text("OK")
                }
            },
            dismissButton = {
                TextButton(onClick = { showResultDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
}

private fun handleScannedCode(
    code: String,
    paykitClient: com.paykit.demo.PaykitClientWrapper,
    onResult: (ScannedUriResult) -> Unit
) {
    // Check if it's a Paykit URI
    if (!paykitClient.isPaykitQR(code)) {
        return
    }
    
    // Parse it
    try {
        val result = paykitClient.parseScannedQR(code) ?: return
        onResult(ScannedUriResult(result))
    } catch (e: Exception) {
        // Handle error
        e.printStackTrace()
    }
}

private fun resultDescription(result: ScannedUriResult): String {
    return when (result.uriType) {
        UriType.PUBKY -> "Pubky URI detected. Public key: ${result.publicKey ?: "unknown"}"
        UriType.INVOICE -> "Invoice detected. Method: ${result.methodId ?: "unknown"}"
        UriType.PAYMENT_REQUEST -> "Payment Request detected. ID: ${result.requestId ?: "unknown"}"
        UriType.UNKNOWN -> "Unknown QR code format"
    }
}

/**
 * Result of scanning a QR code
 */
data class ScannedUriResult(
    val uriType: UriType,
    val publicKey: String?,
    val methodId: String?,
    val data: String?,
    val requestId: String?,
    val requester: String?
) {
    constructor(scannedUri: com.paykit.mobile.ScannedUri) : this(
        uriType = scannedUri.uriType,
        publicKey = scannedUri.publicKey,
        methodId = scannedUri.methodId,
        data = scannedUri.data,
        requestId = scannedUri.requestId,
        requester = scannedUri.requester
    )
}


package com.paykit.mobile.bitkit

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.paykit.mobile.paykit_mobile.PaykitClient
import com.paykit.mobile.paykit_mobile.ScannedUri

/**
 * QR Scanner screen component for Bitkit integration
 * Note: Bitkit must integrate a QR scanning library (ML Kit or ZXing) for production use
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitQRScannerScreen(
    paykitClient: PaykitClient,
    onDismiss: () -> Unit,
    onScannedPubky: ((String) -> Unit)? = null,
    onScannedInvoice: ((String, String) -> Unit)? = null,
    onScannedPaymentRequest: ((String) -> Unit)? = null
) {
    var scannedCode by remember { mutableStateOf<String?>(null) }
    var hasPermission by remember { mutableStateOf(false) }
    var showResultDialog by remember { mutableStateOf(false) }
    var scannedResult by remember { mutableStateOf<ScannedUri?>(null) }
    
    val context = androidx.compose.ui.platform.LocalContext.current
    
    // Request camera permission
    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        hasPermission = isGranted
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
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentAlignment = Alignment.Center
        ) {
            if (!hasPermission) {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    Icon(
                        Icons.Default.CameraAlt,
                        contentDescription = null,
                        modifier = Modifier.size(80.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Text(
                        "Camera Permission Required",
                        style = MaterialTheme.typography.titleLarge
                    )
                    Text(
                        "Please grant camera permission to scan QR codes",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            } else {
                // QR scanner integration point
                // Bitkit must integrate ML Kit or ZXing here
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    Text("QR Scanner")
                    Text(
                        "Integrate ML Kit or ZXing for QR code scanning",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
    
    // Handle scanned code
    LaunchedEffect(scannedCode) {
        scannedCode?.let { code ->
            if (paykitClient.isPaykitQR(data = code)) {
                val result = paykitClient.parseScannedQR(data = code)
                if (result != null) {
                    scannedResult = result
                    showResultDialog = true
                }
            }
        }
    }
    
    // Result Dialog
    if (showResultDialog && scannedResult != null) {
        val result = scannedResult!!
        AlertDialog(
            onDismissRequest = { showResultDialog = false },
            title = { Text("Scanned QR Code") },
            text = {
                Text(
                    when (result.uriType) {
                        com.paykit.mobile.paykit_mobile.UriType.PUBKY -> "Pubky URI: ${result.publicKey ?: "unknown"}"
                        com.paykit.mobile.paykit_mobile.UriType.INVOICE -> "Invoice: ${result.methodId ?: "unknown"}"
                        com.paykit.mobile.paykit_mobile.UriType.PAYMENT_REQUEST -> "Payment Request: ${result.requestId ?: "unknown"}"
                        com.paykit.mobile.paykit_mobile.UriType.UNKNOWN -> "Unknown QR code format"
                    }
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    handleResult(result, onScannedPubky, onScannedInvoice, onScannedPaymentRequest)
                    showResultDialog = false
                    onDismiss()
                }) {
                    Text("OK")
                }
            },
            dismissButton = {
                TextButton(onClick = {
                    showResultDialog = false
                    onDismiss()
                }) {
                    Text("Cancel")
                }
            }
        )
    }
}

private fun handleResult(
    result: ScannedUri,
    onScannedPubky: ((String) -> Unit)?,
    onScannedInvoice: ((String, String) -> Unit)?,
    onScannedPaymentRequest: ((String) -> Unit)?
) {
    when (result.uriType) {
        com.paykit.mobile.paykit_mobile.UriType.PUBKY -> {
            result.publicKey?.let { onScannedPubky?.invoke(it) }
        }
        com.paykit.mobile.paykit_mobile.UriType.INVOICE -> {
            val methodId = result.methodId ?: return
            val data = result.data ?: return
            onScannedInvoice?.invoke(methodId, data)
        }
        com.paykit.mobile.paykit_mobile.UriType.PAYMENT_REQUEST -> {
            result.requestId?.let { onScannedPaymentRequest?.invoke(it) }
        }
        com.paykit.mobile.paykit_mobile.UriType.UNKNOWN -> {
            // Do nothing
        }
    }
}

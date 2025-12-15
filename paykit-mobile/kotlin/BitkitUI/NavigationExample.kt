package com.paykit.mobile.bitkit

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.paykit.mobile.paykit_mobile.PaykitClient

/**
 * Navigation destinations for Bitkit
 */
sealed class BitkitScreen(val route: String, val title: String, val icon: @Composable () -> Unit) {
    object Dashboard : BitkitScreen(
        "dashboard",
        "Dashboard",
        { Icon(Icons.Default.Home, contentDescription = "Dashboard") }
    )
    object Send : BitkitScreen(
        "send",
        "Send",
        { Icon(Icons.Default.Send, contentDescription = "Send") }
    )
    object Receive : BitkitScreen(
        "receive",
        "Receive",
        { Icon(Icons.Default.CallReceived, contentDescription = "Receive") }
    )
    object Contacts : BitkitScreen(
        "contacts",
        "Contacts",
        { Icon(Icons.Default.Person, contentDescription = "Contacts") }
    )
    object Receipts : BitkitScreen(
        "receipts",
        "Receipts",
        { Icon(Icons.Default.ReceiptLong, contentDescription = "Receipts") }
    )
}

/**
 * Main navigation example for Bitkit
 * Bitkit should adapt this to their navigation structure
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BitkitMainNavigation(
    paykitClient: PaykitClient,
    contactStorage: ContactStorageProtocol,
    receiptStorage: ReceiptStorageProtocol
) {
    val navController = rememberNavController()
    val selectedIndex = remember { mutableIntStateOf(0) }
    
    // Initialize ViewModels
    val dashboardViewModel = remember {
        BitkitDashboardViewModel(paykitClient)
    }
    val paymentViewModel = remember {
        BitkitPaymentViewModel(paykitClient)
    }
    val contactsViewModel = remember {
        BitkitContactsViewModel(contactStorage)
    }
    val receiptsViewModel = remember {
        BitkitReceiptsViewModel(receiptStorage)
    }
    
    Scaffold(
        bottomBar = {
            NavigationBar {
                BitkitScreen.values().forEachIndexed { index, screen ->
                    NavigationBarItem(
                        icon = screen.icon(),
                        label = { Text(screen.title) },
                        selected = selectedIndex.intValue == index,
                        onClick = {
                            selectedIndex.intValue = index
                            navController.navigate(screen.route) {
                                popUpTo(navController.graph.startDestinationId)
                                launchSingleTop = true
                            }
                        }
                    )
                }
            }
        }
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = BitkitScreen.Dashboard.route,
            modifier = Modifier.padding(padding)
        ) {
            composable(BitkitScreen.Dashboard.route) {
                BitkitDashboardScreen(
                    viewModel = dashboardViewModel,
                    onSendPayment = { navController.navigate(BitkitScreen.Send.route) },
                    onReceivePayment = { navController.navigate(BitkitScreen.Receive.route) },
                    onViewReceipts = { navController.navigate(BitkitScreen.Receipts.route) },
                    onViewContacts = { navController.navigate(BitkitScreen.Contacts.route) }
                )
            }
            composable(BitkitScreen.Send.route) {
                BitkitPaymentScreen(
                    viewModel = paymentViewModel,
                    onPaymentComplete = { navController.navigate(BitkitScreen.Receipts.route) }
                )
            }
            composable(BitkitScreen.Receive.route) {
                // Receive screen - Bitkit should implement
                Text("Receive Payment")
            }
            composable(BitkitScreen.Contacts.route) {
                BitkitContactsScreen(viewModel = contactsViewModel)
            }
            composable(BitkitScreen.Receipts.route) {
                BitkitReceiptsScreen(viewModel = receiptsViewModel)
            }
        }
    }
    
    // Load initial data
    LaunchedEffect(Unit) {
        dashboardViewModel.loadDashboard(
            receiptStorage = receiptStorage,
            contactStorage = contactStorage
        )
    }
}

// Extension to get all screens
private fun BitkitScreen.values(): List<BitkitScreen> {
    return listOf(
        BitkitScreen.Dashboard,
        BitkitScreen.Send,
        BitkitScreen.Receive,
        BitkitScreen.Contacts,
        BitkitScreen.Receipts
    )
}

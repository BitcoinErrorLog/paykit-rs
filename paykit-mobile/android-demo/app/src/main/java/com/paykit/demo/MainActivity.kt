package com.paykit.demo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.paykit.demo.ui.*
import com.paykit.demo.ui.theme.PaykitDemoTheme

/**
 * Main Activity for Paykit Demo
 *
 * This activity hosts the main navigation structure with bottom tabs
 * for accessing different features of the Paykit SDK.
 */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            PaykitDemoTheme {
                PaykitDemoContent()
            }
        }
    }
}

/**
 * Navigation destinations
 */
sealed class Screen(val route: String, val title: String, val icon: @Composable () -> Unit) {
    object Methods : Screen(
        "methods",
        "Methods",
        { Icon(Icons.Default.CreditCard, contentDescription = "Methods") }
    )
    object Subscriptions : Screen(
        "subscriptions",
        "Subscriptions",
        { Icon(Icons.Default.Repeat, contentDescription = "Subscriptions") }
    )
    object AutoPay : Screen(
        "autopay",
        "Auto-Pay",
        { Icon(Icons.Default.FlashOn, contentDescription = "Auto-Pay") }
    )
    object Requests : Screen(
        "requests",
        "Requests",
        { Icon(Icons.Default.SwapHoriz, contentDescription = "Requests") }
    )
    object Settings : Screen(
        "settings",
        "Settings",
        { Icon(Icons.Default.Settings, contentDescription = "Settings") }
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaykitDemoContent() {
    val navController = rememberNavController()
    val screens = listOf(
        Screen.Methods,
        Screen.Subscriptions,
        Screen.AutoPay,
        Screen.Requests,
        Screen.Settings
    )

    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = navBackStackEntry?.destination?.route

    Scaffold(
        bottomBar = {
            NavigationBar {
                screens.forEach { screen ->
                    NavigationBarItem(
                        icon = screen.icon,
                        label = { Text(screen.title) },
                        selected = currentRoute == screen.route,
                        onClick = {
                            navController.navigate(screen.route) {
                                // Pop up to the start destination of the graph to
                                // avoid building up a large stack of destinations
                                popUpTo(navController.graph.startDestinationId) {
                                    saveState = true
                                }
                                // Avoid multiple copies of the same destination
                                launchSingleTop = true
                                // Restore state when reselecting a previously selected item
                                restoreState = true
                            }
                        }
                    )
                }
            }
        }
    ) { innerPadding ->
        NavHost(
            navController = navController,
            startDestination = Screen.Methods.route,
            modifier = Modifier.padding(innerPadding)
        ) {
            composable(Screen.Methods.route) { PaymentMethodsScreen() }
            composable(Screen.Subscriptions.route) { SubscriptionsScreen() }
            composable(Screen.AutoPay.route) { AutoPayScreen() }
            composable(Screen.Requests.route) { PaymentRequestsScreen() }
            composable(Screen.Settings.route) { SettingsScreen() }
        }
    }
}

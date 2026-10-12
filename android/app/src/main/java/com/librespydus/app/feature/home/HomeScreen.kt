package com.librespydus.app.feature.home

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Formatting
import com.librespydus.app.ui.components.PlaceholderScreen
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsIcons

/** Home tab (§6, owned by A3). Wave-1 stub. */
@Composable
fun HomeScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val accountName by appState.accountName.collectAsStateWithLifecycle()
    val library by appState.library.collectAsStateWithLifecycle()
    val greeting = Formatting.firstName(accountName)?.let { "Hello $it!" } ?: "Hello!"
    PlaceholderScreen(
        title = greeting,
        message = "${library?.name.orEmpty()}\nNew arrivals, popular genres and your borrowed books will appear here.",
        icon = LsIcons.Home,
        modifier = modifier,
    )
}

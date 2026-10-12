package com.librespydus.app.feature.borrowed

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.librespydus.app.core.AppState
import com.librespydus.app.ui.components.PlaceholderScreen
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsIcons

/** Borrowed tab — "Your Books": loans and reservations (§6, owned by A2). Wave-1 stub. */
@Composable
fun BorrowedScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    PlaceholderScreen(
        title = "Your Books",
        message = "Your loans and reservations will appear here.",
        icon = LsIcons.Book,
        modifier = modifier,
    )
}

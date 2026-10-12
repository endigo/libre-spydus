package com.librespydus.app.feature.favourites

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.ui.components.PlaceholderScreen
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsIcons

/** Favourites tab: locally saved books (§6, owned by A2). Wave-1 stub. */
@Composable
fun FavouritesScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val favourites by appState.currentFavourites.collectAsStateWithLifecycle()
    PlaceholderScreen(
        title = "Favourites",
        message = if (favourites.isEmpty()) "Books you bookmark will appear here." else "${favourites.size} saved books will appear here.",
        icon = LsIcons.Bookmark,
        modifier = modifier,
    )
}

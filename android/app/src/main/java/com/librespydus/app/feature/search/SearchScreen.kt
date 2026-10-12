package com.librespydus.app.feature.search

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.ui.components.PlaceholderScreen
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsIcons

/** Search tab (§6, owned by A3). Wave-1 stub: shows the query handed over from Home. */
@Composable
fun SearchScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    var query by rememberSaveable { mutableStateOf("") }
    val pending by appState.pendingSearch.collectAsStateWithLifecycle()
    LaunchedEffect(pending) {
        appState.consumePendingSearch()?.let { query = it }
    }
    PlaceholderScreen(
        title = "Search",
        message = if (query.isBlank()) "Search the catalogue by title, author or keyword." else "Results for “$query” will appear here.",
        icon = LsIcons.Search,
        modifier = modifier,
    )
}

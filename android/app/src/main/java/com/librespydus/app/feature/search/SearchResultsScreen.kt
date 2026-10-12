package com.librespydus.app.feature.search

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Route
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.LsTopBar
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsIcons

/** Results list for a fixed query (`results(query, sort, title)`, §6, owned by A3). Wave-1 stub. */
@Composable
fun SearchResultsScreen(route: Route.Results, appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    Column(modifier.fillMaxSize().statusBarsPadding()) {
        LsTopBar(title = route.title ?: route.query, onBack = navigator::back)
        EmptyStateView(
            title = route.title ?: route.query,
            message = "Results for “${route.query}” (${route.sort.label}) will appear here.",
            icon = LsIcons.Search,
        )
    }
}

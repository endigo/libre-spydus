package com.librespydus.app.feature.search

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.Route
import com.librespydus.app.ui.components.CircleIconButton
import com.librespydus.app.ui.components.LsTopBar
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.launch

/**
 * A results list for a fixed query (`results(query, sort, title)`, §5.5): Home "View All", genre
 * tiles, subject chips, `ls_route results:`. The Search tab's list without the field, titled in
 * the top bar, with a sort menu.
 */
@Composable
fun SearchResultsScreen(route: Route.Results, appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val backend by appState.backend.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val report = remember(appState) { appState.browseReporter() }
    var activeSort by rememberSaveable { mutableStateOf(route.sort) }
    var attempt by rememberSaveable { mutableIntStateOf(0) }
    val model: SearchResultsModel = viewModel()
    val state by model.state.collectAsStateWithLifecycle()

    val currentBackend = backend
    val request = currentBackend?.let { SearchResultsModel.Request(route.query, activeSort, it.origin, verification, attempt) }
    LaunchedEffect(request) {
        if (request == null || currentBackend == null) return@LaunchedEffect
        model.viewModelScope.launch { model.load(request, currentBackend, report) }
    }

    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding(),
    ) {
        LsTopBar(
            title = route.title ?: route.query,
            onBack = navigator::back,
            trailing = { SortMenu(activeSort, onSelect = { activeSort = it }) },
        )
        SearchResultsList(
            state = state,
            query = route.query,
            onRetry = { attempt += 1 },
            onLoadMore = { model.viewModelScope.launch { model.loadMore() } },
            onRetryLoadMore = { model.viewModelScope.launch { model.retryLoadMore() } },
            onOpen = navigator::openBook,
            modifier = Modifier.weight(1f),
        )
    }
}

/**
 * A results list that starts from a URL (`ListResults(url, title)`, §5.10): a collection
 * section's "View All", a Featured saved search. The same list as [SearchResultsScreen], first
 * page from `page(url)`, without a sort menu.
 */
@Composable
fun ListResultsScreen(route: Route.ListResults, appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val backend by appState.backend.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val report = remember(appState) { appState.browseReporter() }
    var attempt by rememberSaveable { mutableIntStateOf(0) }
    val model: SearchResultsModel = viewModel()
    val state by model.state.collectAsStateWithLifecycle()

    val currentBackend = backend
    val request = currentBackend?.let {
        SearchResultsModel.Request(route.title, ResultsSort.RELEVANCE, it.origin, verification, attempt, url = route.url)
    }
    LaunchedEffect(request) {
        if (request == null || currentBackend == null) return@LaunchedEffect
        model.viewModelScope.launch { model.load(request, currentBackend, report) }
    }

    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding(),
    ) {
        LsTopBar(title = route.title, onBack = navigator::back)
        SearchResultsList(
            state = state,
            query = route.title,
            onRetry = { attempt += 1 },
            onLoadMore = { model.viewModelScope.launch { model.loadMore() } },
            onRetryLoadMore = { model.viewModelScope.launch { model.retryLoadMore() } },
            onOpen = navigator::openBook,
            modifier = Modifier.weight(1f),
            noResultsTitle = Copy.Search.listEmptyTitle(route.title),
            noResultsMessage = Copy.Search.listEmptyMessage,
        )
    }
}

@Composable
private fun SortMenu(selected: ResultsSort, onSelect: (ResultsSort) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        CircleIconButton(
            icon = BrowseIcons.SwapVert,
            contentDescription = Copy.Search.sortResults,
            onClick = { open = true },
            size = 44.dp,
            iconSize = 24.dp,
            background = Color.Transparent,
            contentColor = LsColors.textPrimary,
            shadow = false,
            modifier = Modifier.semantics { stateDescription = selected.label },
        )
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            ResultsSort.entries.forEach { sort ->
                DropdownMenuItem(
                    text = { Text(sort.label, style = LsType.body, color = LsColors.textPrimary) },
                    trailingIcon = {
                        if (sort == selected) Icon(LsIcons.Check, contentDescription = Copy.Common.selected, tint = LsColors.primary)
                        else Spacer(Modifier.size(24.dp))
                    },
                    onClick = {
                        open = false
                        onSelect(sort)
                    },
                )
            }
        }
    }
}

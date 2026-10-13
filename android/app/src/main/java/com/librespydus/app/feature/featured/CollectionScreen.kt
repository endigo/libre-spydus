package com.librespydus.app.feature.featured

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.Route
import com.librespydus.app.feature.home.HomeCoverShelf
import com.librespydus.app.feature.home.HomeSectionErrorCard
import com.librespydus.app.feature.home.HomeShelfEntry
import com.librespydus.app.feature.home.HomeShelfSkeleton
import com.librespydus.app.feature.search.BrowseBottomClearance
import com.librespydus.app.feature.search.BrowseIcons
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.browseReporter
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.LsTopBar
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsSpacing
import kotlinx.coroutines.launch

/**
 * A library's featured collection (`collection(FeaturedLink)`, §5.10): titled with the link, one
 * shelf per section of the page (title, "View All" → its results list, covers from its first
 * page). Shelves load as they scroll into view, three at a time. A page without sections (eLibrary),
 * or whose sections all came back empty, offers the library's website instead. Navy, under a fixed top bar that keeps content out of the
 * status bar; the last shelf scrolls clear of the tab bar.
 */
@Composable
fun CollectionScreen(route: Route.Collection, appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val model: CollectionModel = viewModel { CollectionModel(route.url) }
    val state by model.state.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val report = remember(appState) { appState.browseReporter() }
    var attempt by rememberSaveable { mutableIntStateOf(0) }
    val context = LocalContext.current

    val currentBackend = backend
    val request = currentBackend?.let { CollectionModel.Request(it.origin, verification, attempt) }
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
        when (val page = state.page) {
            BrowseLoadable.Idle, BrowseLoadable.Loading -> LoadingView(Modifier.weight(1f).padding(bottom = BrowseBottomClearance))
            is BrowseLoadable.Failed -> ErrorStateView(
                error = page.error,
                onRetry = { attempt += 1 }.takeIf { page.error.isRetryable },
                modifier = Modifier.weight(1f).padding(bottom = BrowseBottomClearance),
            )
            is BrowseLoadable.Loaded -> if (state.hasNoSections) {
                EmptyStateView(
                    title = page.value.title?.takeIf { it.isNotBlank() } ?: route.title,
                    message = WEBSITE_ONLY,
                    icon = LsIcons.Globe,
                    actionLabel = "Open website",
                    onAction = { InAppBrowser.open(context, route.url) },
                    modifier = Modifier.weight(1f).padding(bottom = BrowseBottomClearance),
                )
            } else if (state.allSectionsEmpty) {
                EmptyStateView(
                    title = "Nothing here right now",
                    message = "Check back soon, or open the collection on the library's website.",
                    icon = BrowseIcons.AutoAwesome,
                    actionLabel = "Open website",
                    onAction = { InAppBrowser.open(context, route.url) },
                    modifier = Modifier.weight(1f).padding(bottom = BrowseBottomClearance),
                )
            } else {
                CollectionShelves(state, currentBackend, model, report, navigator, Modifier.weight(1f))
            }
        }
    }
}

/** The copy under a collection page without sections. */
const val WEBSITE_ONLY = "This collection is on the library's website."

@Composable
private fun CollectionShelves(
    state: CollectionModel.State,
    backend: LibraryBackend?,
    model: CollectionModel,
    report: BrowseReporter,
    navigator: AppNavigator,
    modifier: Modifier = Modifier,
) {
    fun load(shelf: CollectionModel.Shelf, retry: Boolean = false) {
        val b = backend ?: return
        model.viewModelScope.launch { model.loadShelf(shelf.id, b, report, retry) }
    }
    LazyColumn(
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(top = LsSpacing.s, bottom = BrowseBottomClearance),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
    ) {
        items(state.visibleShelves, key = { "${state.generation}:${it.id}" }) { shelf ->
            // A lazy list composes a row as it scrolls into view: that's when its shelf loads.
            LaunchedEffect(state.generation, shelf.id) { load(shelf) }
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                SectionHeader(
                    title = shelf.section.title,
                    onAction = { navigator.openList(shelf.section.url, shelf.section.title) },
                    modifier = Modifier.padding(horizontal = LsSpacing.l),
                )
                when (val shelfState = shelf.state) {
                    CollectionModel.ShelfState.Idle, CollectionModel.ShelfState.Loading, CollectionModel.ShelfState.Empty -> HomeShelfSkeleton()
                    is CollectionModel.ShelfState.Failed -> HomeSectionErrorCard(shelfState.error) { load(shelf, retry = true) }
                    is CollectionModel.ShelfState.Loaded -> HomeCoverShelf(
                        shelfState.books.map { HomeShelfEntry(it, caption = it.title) },
                        navigator::openBook,
                    )
                }
            }
        }
    }
}

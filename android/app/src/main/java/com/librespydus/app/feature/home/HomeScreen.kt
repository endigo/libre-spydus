package com.librespydus.app.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Tab
import com.librespydus.app.feature.search.BrowseBottomClearance
import com.librespydus.app.feature.search.BrowseIcons
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.browseReporter
import com.librespydus.app.ui.components.GenreTile
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.SearchField
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.launch

/**
 * Home (§6): library logo (→ Profile), greeting, search field (→ Search tab), "New arrivals",
 * "Popular genres" and "Your borrowed books". Shelves load concurrently, each with its own
 * skeleton and error; pull to refresh; reloads on library, verification and sign-in changes.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val model: HomeModel = viewModel()
    val library by appState.library.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val session by appState.session.collectAsStateWithLifecycle()
    val accountName by appState.accountName.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val account by appState.accountGeneration.collectAsStateWithLifecycle()
    val arrivals by model.arrivals.collectAsStateWithLifecycle()
    val loans by model.loans.collectAsStateWithLifecycle()
    val report = remember(appState) { appState.browseReporter() }
    var refreshing by remember { mutableStateOf(false) }

    val currentBackend = backend
    if (currentBackend == null) {
        LoadingView(modifier.background(LsColors.background))
        return
    }
    val libraryName = library?.name ?: "your library"
    val signedIn = session != null
    val key = HomeReloadKey(currentBackend.origin, verification, account)
    val context = HomeLoadContext(currentBackend, session, key, report)

    // Loads run in the model's scope: pushing a detail screen mid-load doesn't cancel them.
    LaunchedEffect(key) { model.viewModelScope.launch { model.loadIfNeeded(context) } }

    PullToRefreshBox(
        isRefreshing = refreshing,
        onRefresh = {
            refreshing = true
            model.viewModelScope.launch {
                try {
                    model.refresh(context)
                } finally {
                    refreshing = false
                }
            }
        },
        modifier = modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding(),
    ) {
        Column(
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(top = LsSpacing.s, bottom = BrowseBottomClearance),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
        ) {
            Column(Modifier.padding(horizontal = LsSpacing.l), verticalArrangement = Arrangement.spacedBy(LsSpacing.l)) {
                HomeLibraryLogo(
                    name = libraryName,
                    logoUrl = library?.logoUrl,
                    onClick = { navigator.switchTab(Tab.PROFILE) },
                    modifier = Modifier.padding(top = LsSpacing.l),
                )
                Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                    Text(
                        HomeGreeting.title(accountName),
                        style = LsType.greeting,
                        color = LsColors.textPrimary,
                        modifier = Modifier.semantics { heading() },
                    )
                    Text(libraryName, style = LsType.body, color = LsColors.textSecondary)
                }
            }
            SearchField(
                value = "",
                onValueChange = {},
                onClick = { navigator.openSearch() },
                modifier = Modifier.padding(horizontal = LsSpacing.l),
            )

            // New arrivals
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                val route = model.arrivalsRoute
                SectionHeader(
                    title = model.arrivalsShelf?.title ?: "New arrivals",
                    onAction = route?.let { r -> { navigator.openResults(r.query, r.sort, r.title) } },
                    modifier = Modifier.padding(horizontal = LsSpacing.l),
                )
                when (val state = arrivals) {
                    BrowseLoadable.Idle, BrowseLoadable.Loading -> HomeShelfSkeleton()
                    is BrowseLoadable.Failed -> HomeSectionErrorCard(state.error) {
                        model.viewModelScope.launch { model.retryArrivals(context) }
                    }
                    is BrowseLoadable.Loaded -> if (state.value.isEmpty()) {
                        HomeSectionMessageCard(BrowseIcons.AutoAwesome, "Nothing new right now", "Check back soon for the latest additions.")
                    } else {
                        HomeCoverShelf(state.value.map { HomeShelfEntry(it, caption = it.title) }, navigator::openBook)
                    }
                }
            }

            // Popular genres
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                SectionHeader("Popular genres", modifier = Modifier.padding(horizontal = LsSpacing.l))
                LazyRow(
                    contentPadding = PaddingValues(horizontal = LsSpacing.l),
                    horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
                ) {
                    itemsIndexed(model.genres, key = { _, genre -> genre.key }) { index, genre ->
                        GenreTile(
                            label = genre.label,
                            icon = HomeGenreIcon.forKey(genre.key).vector(),
                            color = LsColors.genre[index % LsColors.genre.size],
                            onClick = {
                                val route = HomeModel.route(genre)
                                navigator.openResults(route.query, route.sort, route.title)
                            },
                        )
                    }
                }
            }

            // Your borrowed books
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                SectionHeader(
                    "Your borrowed books",
                    onAction = if (signedIn) ({ navigator.switchTab(Tab.BORROWED) }) else null,
                    modifier = Modifier.padding(horizontal = LsSpacing.l),
                )
                if (!signedIn) {
                    HomeSignInCard(libraryName, onSignIn = appState::requestLogin)
                } else {
                    when (val state = loans) {
                        BrowseLoadable.Idle, BrowseLoadable.Loading -> HomeShelfSkeleton()
                        is BrowseLoadable.Failed -> HomeSectionErrorCard(state.error) {
                            model.viewModelScope.launch { model.retryLoans(context) }
                        }
                        is BrowseLoadable.Loaded -> if (state.value.isEmpty()) {
                            HomeSectionMessageCard(BrowseIcons.MenuBook, "No loans right now", "Books you borrow will show up here with their due dates.")
                        } else {
                            HomeCoverShelf(
                                state.value.map { loan ->
                                    HomeShelfEntry(loan.ref, loan.caption, loan.tone.color(), accessibilityCaption = loan.caption)
                                },
                                navigator::openBook,
                            )
                        }
                    }
                }
            }
        }
    }
}

private fun LoanShelfItem.Tone.color() = when (this) {
    LoanShelfItem.Tone.NORMAL -> LsColors.textSecondary
    LoanShelfItem.Tone.SOON -> LsColors.warning
    LoanShelfItem.Tone.OVERDUE -> LsColors.danger
}

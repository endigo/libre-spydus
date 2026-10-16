package com.librespydus.app.feature.home

import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Tab
import com.librespydus.app.feature.featured.FeaturedSection
import com.librespydus.app.feature.featured.HomeFeaturedModel
import com.librespydus.app.feature.mycard.MyCardButton
import com.librespydus.app.feature.mycard.MyCardDebugHooks
import com.librespydus.app.feature.mycard.MyCardSheet
import com.librespydus.app.feature.search.BrowseBottomClearance
import com.librespydus.app.feature.search.BrowseIcons
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.browseReporter
import com.librespydus.app.ui.components.GenreTile
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.SearchField
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LocalAppLayout
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import com.librespydus.app.ui.theme.pageColumn
import kotlinx.coroutines.launch

/**
 * Home (§6): library logo (→ Profile), greeting with "My card" (§5.9, signed in), search field
 * (→ Search tab), "New arrivals", "Popular genres", "Featured" (§5.10, only for libraries with a
 * Featured list) and "Your loans". New arrivals and Featured load on their own (reload on
 * library and verification changes); the loans shelf and greeting come
 * from the account store's snapshot (§5.2a), shown at once and refreshed in the background when it
 * is over two minutes old. Each shelf has its own skeleton and error; pull to refresh reloads both.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val model: HomeModel = viewModel()
    val featuredModel: HomeFeaturedModel = viewModel()
    val library by appState.library.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val session by appState.session.collectAsStateWithLifecycle()
    val accountName by appState.accountName.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val snapshot by appState.account.snapshot.collectAsStateWithLifecycle()
    val accountRefreshing by appState.account.isRefreshing.collectAsStateWithLifecycle()
    val accountError by appState.account.lastRefreshError.collectAsStateWithLifecycle()
    val arrivals by model.arrivals.collectAsStateWithLifecycle()
    val featured by featuredModel.links.collectAsStateWithLifecycle()
    val loans = remember(snapshot, accountRefreshing, accountError) { HomeLoans.state(snapshot, accountRefreshing, accountError) }
    val report = remember(appState) { appState.browseReporter() }
    var refreshing by remember { mutableStateOf(false) }
    var showingCard by rememberSaveable { mutableStateOf(false) }
    val activity = LocalActivity.current

    val currentBackend = backend
    if (currentBackend == null) {
        LoadingView(modifier.background(LsColors.background))
        return
    }
    val libraryName = library?.name ?: Copy.Common.yourLibrary
    val signedIn = session != null
    val key = HomeReloadKey(currentBackend.origin, verification)
    val context = HomeLoadContext(currentBackend, key, report)

    // Loads run in the model's scope: pushing a detail screen mid-load doesn't cancel them.
    LaunchedEffect(key) {
        model.viewModelScope.launch { model.loadIfNeeded(context) }
        featuredModel.viewModelScope.launch { featuredModel.loadIfNeeded(currentBackend, key) }
    }
    LaunchedEffect(currentBackend, session) { appState.account.refresh() }
    LaunchedEffect(currentBackend) { if (MyCardDebugHooks.run(activity, appState)) showingCard = true }

    PullToRefreshBox(
        isRefreshing = refreshing,
        onRefresh = {
            refreshing = true
            model.viewModelScope.launch {
                try {
                    val account = appState.account.refresh(force = true)
                    val featuredRefresh = featuredModel.viewModelScope.launch { featuredModel.refresh(currentBackend, key) }
                    model.refresh(context)
                    account?.join()
                    featuredRefresh.join()
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
                .pageColumn()
                .verticalScroll(rememberScrollState())
                .padding(top = LsSpacing.s, bottom = BrowseBottomClearance),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
        ) {
            val greeting: @Composable (Modifier) -> Unit = { greetingModifier ->
                Column(greetingModifier, verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                        Text(
                            HomeGreeting.title(accountName),
                            style = LsType.greeting,
                            color = LsColors.textPrimary,
                            modifier = Modifier
                                .weight(1f)
                                .semantics { heading() },
                        )
                        if (signedIn) MyCardButton(onClick = { showingCard = true })
                    }
                    Text(libraryName, style = LsType.body, color = LsColors.textSecondary)
                }
            }
            if (LocalAppLayout.current.isWide) {
                // The logo beside the greeting (§6.1), as on the desktop.
                Row(
                    Modifier.padding(horizontal = LsSpacing.l).padding(top = LsSpacing.l),
                    horizontalArrangement = Arrangement.spacedBy(LsSpacing.l),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    HomeLibraryLogo(
                        name = libraryName,
                        logoUrl = library?.logoUrl,
                        onClick = { navigator.switchTab(Tab.PROFILE) },
                        size = WIDE_LOGO_SIZE,
                    )
                    greeting(Modifier.weight(1f))
                }
            } else {
                Column(Modifier.padding(horizontal = LsSpacing.l), verticalArrangement = Arrangement.spacedBy(LsSpacing.l)) {
                    HomeLibraryLogo(
                        name = libraryName,
                        logoUrl = library?.logoUrl,
                        onClick = { navigator.switchTab(Tab.PROFILE) },
                        modifier = Modifier.padding(top = LsSpacing.l),
                    )
                    greeting(Modifier)
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
                    title = model.arrivalsShelf?.title ?: Copy.Home.newArrivals,
                    onAction = route?.let { r -> { navigator.openResults(r.query, r.sort, r.title) } },
                    modifier = Modifier.padding(horizontal = LsSpacing.l),
                )
                when (val state = arrivals) {
                    BrowseLoadable.Idle, BrowseLoadable.Loading -> HomeShelfSkeleton()
                    is BrowseLoadable.Failed -> HomeSectionErrorCard(state.error) {
                        model.viewModelScope.launch { model.retryArrivals(context) }
                    }
                    is BrowseLoadable.Loaded -> if (state.value.isEmpty()) {
                        HomeSectionMessageCard(BrowseIcons.AutoAwesome, Copy.Home.nothingNewTitle, Copy.Home.nothingNewMessage)
                    } else {
                        HomeCoverShelf(state.value.map { HomeShelfEntry(it, caption = it.title) }, navigator::openBook)
                    }
                }
            }

            // Popular genres
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                SectionHeader(Copy.Home.popularGenres, modifier = Modifier.padding(horizontal = LsSpacing.l))
                LazyRow(
                    contentPadding = PaddingValues(horizontal = LsSpacing.l),
                    horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
                ) {
                    itemsIndexed(model.genres, key = { _, genre -> genre.key }) { index, genre ->
                        GenreTile(
                            label = genre.label,
                            icon = HomeGenreIcon.forKey(genre.key).vector(),
                            color = LsColors.genre[index % LsColors.genre.size],
                            accessibilityHint = Copy.Home.genreHint(genre.label),
                            onClick = {
                                val route = HomeModel.route(genre)
                                navigator.openResults(route.query, route.sort, route.title)
                            },
                        )
                    }
                }
            }

            // Featured: the library's own collections (§5.10); nothing at all when it has none.
            FeaturedSection(featured, navigator)

            // Your loans
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                SectionHeader(
                    Copy.Home.yourLoans,
                    onAction = if (signedIn) ({ navigator.switchTab(Tab.BORROWED) }) else null,
                    modifier = Modifier.padding(horizontal = LsSpacing.l),
                )
                if (!signedIn) {
                    HomeSignInCard(libraryName, onSignIn = appState::requestLogin)
                } else {
                    when (val state = loans) {
                        BrowseLoadable.Idle, BrowseLoadable.Loading -> HomeShelfSkeleton()
                        is BrowseLoadable.Failed -> HomeSectionErrorCard(state.error) { appState.account.refresh(force = true) }
                        is BrowseLoadable.Loaded -> if (state.value.isEmpty()) {
                            HomeSectionMessageCard(BrowseIcons.MenuBook, Copy.Home.noLoansTitle, Copy.Home.noLoansMessage)
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

    if (showingCard && signedIn) MyCardSheet(appState, onDismiss = { showingCard = false })
}

private fun LoanShelfItem.Tone.color() = when (this) {
    LoanShelfItem.Tone.NORMAL -> LsColors.textSecondary
    LoanShelfItem.Tone.SOON -> LsColors.warning
    LoanShelfItem.Tone.OVERDUE -> LsColors.danger
}

/** The library logo beside the greeting in the wide layout (§6.1). */
private val WIDE_LOGO_SIZE = 52.dp

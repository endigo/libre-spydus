package com.librespydus.app.feature.search

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.LazyListState
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.SearchField
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.components.SegmentedToggle
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LocalAppLayout
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import com.librespydus.app.ui.theme.PageLayout
import com.librespydus.app.ui.theme.pageColumn
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * The Search tab (§6): search field (focused when Home hands over), Relevance / Newest, results
 * with infinite scroll, and recent searches while the field is empty. Searches run on the IME's
 * Search action only.
 */
@Composable
fun SearchScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val recents = remember { RecentSearchStore.shared(context) }
    val recentItems by recents.items.collectAsStateWithLifecycle()
    val library by appState.library.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val pending by appState.pendingSearch.collectAsStateWithLifecycle()
    val report = remember(appState) { appState.browseReporter() }

    var text by rememberSaveable { mutableStateOf("") }
    /** The query whose results are shown (set on the IME action). */
    var submitted by rememberSaveable { mutableStateOf<String?>(null) }
    var sort by rememberSaveable { mutableStateOf(ResultsSort.RELEVANCE) }
    var attempt by rememberSaveable { mutableIntStateOf(0) }

    val model: SearchResultsModel = viewModel()
    val state by model.state.collectAsStateWithLifecycle()
    val focusRequester = remember { FocusRequester() }
    val focusManager = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    // A new search starts at the top; coming back to the tab keeps the position.
    val listState = rememberSaveable(submitted, sort, attempt, saver = LazyListState.Saver) { LazyListState() }
    var fieldFocused by remember { mutableStateOf(false) }

    fun submit() {
        val query = RecentSearchStore.collapseWhitespace(text)
        if (query.isEmpty()) return
        text = query
        recents.record(query)
        submitted = query
        attempt += 1
        keyboard?.hide()
        focusManager.clearFocus()
    }

    val currentBackend = backend
    val request = submitted?.let { query ->
        currentBackend?.let { SearchResultsModel.Request(query, sort, it.origin, verification, attempt) }
    }
    LaunchedEffect(request) {
        if (request == null || currentBackend == null) return@LaunchedEffect
        model.viewModelScope.launch { model.load(request, currentBackend, report) }
    }

    // Home's search field: focus the field, or run the query it handed over. Taking the pending
    // search changes `pending`, which restarts this effect, so focusing runs in its own effect.
    var focusRequests by remember { mutableIntStateOf(0) }
    LaunchedEffect(pending) {
        if (pending == null) return@LaunchedEffect
        val query = appState.consumePendingSearch()?.trim().orEmpty()
        if (query.isEmpty()) {
            focusRequests += 1
        } else {
            text = query
            submit()
        }
    }
    LaunchedEffect(focusRequests) {
        if (focusRequests == 0) return@LaunchedEffect
        // Focus only sticks once the tab switch has been laid out: try for a moment.
        delay(250)
        repeat(10) {
            if (fieldFocused) return@repeat
            runCatching { focusRequester.requestFocus() }
            delay(100)
        }
        keyboard?.show()
    }

    LaunchedEffect(listState.isScrollInProgress) {
        if (listState.isScrollInProgress) keyboard?.hide()
    }

    val wide = LocalAppLayout.current.isWide
    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding()
            .pageColumn(),
    ) {
        // The title, the field and (with results) Relevance / Newest; in the wide layout (§6.1) the
        // title in the other sections' size and the sort beside the field, as on the desktop.
        Column(
            Modifier.padding(start = LsSpacing.l, end = LsSpacing.l, top = LsSpacing.s, bottom = LsSpacing.m),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
        ) {
            Text(
                Copy.Search.title,
                style = if (wide) LsType.greeting else LsType.title,
                color = LsColors.textPrimary,
                modifier = Modifier
                    .padding(top = LsSpacing.l)
                    .semantics { heading() },
            )
            val field: @Composable (Modifier) -> Unit = { fieldModifier ->
                SearchField(
                    value = text,
                    onValueChange = {
                        text = it
                        // Clearing the field ends the search: recent searches show until the next search.
                        if (it.isBlank()) submitted = null
                    },
                    onSearch = { submit() },
                    focusRequester = focusRequester,
                    modifier = fieldModifier.onFocusChanged { fieldFocused = it.isFocused },
                )
            }
            val sortToggle: @Composable (Modifier) -> Unit = { toggleModifier ->
                SegmentedToggle(
                    options = ResultsSort.entries.map { it.label },
                    selectedIndex = sort.ordinal,
                    onSelect = { sort = ResultsSort.entries[it] },
                    modifier = toggleModifier.semantics { contentDescription = Copy.Search.sortResults },
                )
            }
            if (wide) {
                Row(horizontalArrangement = Arrangement.spacedBy(LsSpacing.m), verticalAlignment = Alignment.CenterVertically) {
                    field(Modifier.weight(1f))
                    if (submitted != null) sortToggle(Modifier.width(PageLayout.searchSortWidth))
                }
            } else {
                field(Modifier)
                if (submitted != null) sortToggle(Modifier.fillMaxWidth())
            }
        }
        val query = submitted
        if (query != null) {
            SearchResultsList(
                state = state,
                query = query,
                onRetry = { attempt += 1 },
                onLoadMore = { model.viewModelScope.launch { model.loadMore() } },
                onRetryLoadMore = { model.viewModelScope.launch { model.retryLoadMore() } },
                onOpen = navigator::openBook,
                listState = listState,
                modifier = Modifier.weight(1f),
            )
        } else {
            RecentSearches(
                items = recentItems,
                libraryName = library?.name ?: Copy.Common.yourLibrary,
                onRun = { recent ->
                    text = recent
                    submit()
                },
                onRemove = recents::remove,
                onClear = recents::clear,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

/** Recent searches (tap to run again, long-press to remove, Clear), or a prompt when there are none. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun RecentSearches(
    items: List<String>,
    libraryName: String,
    onRun: (String) -> Unit,
    onRemove: (String) -> Unit,
    onClear: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (items.isEmpty()) {
        EmptyStateView(
            title = Copy.Search.introTitle(libraryName),
            message = Copy.Search.introMessage,
            icon = BrowseIcons.MenuBook,
            modifier = modifier.padding(bottom = BrowseBottomClearance),
        )
        return
    }
    LazyColumn(
        modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = LsSpacing.l, end = LsSpacing.l, top = LsSpacing.s, bottom = BrowseBottomClearance),
    ) {
        item(key = "header") {
            SectionHeader(Copy.Search.recentSearches, actionLabel = Copy.Common.clear, onAction = onClear, modifier = Modifier.padding(bottom = LsSpacing.s))
        }
        items(items, key = { it }) { query ->
            var menuOpen by remember { mutableStateOf(false) }
            Box {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .combinedClickable(
                            onClickLabel = Copy.Search.searchAgain,
                            onLongClickLabel = Copy.Common.remove,
                            onLongClick = { menuOpen = true },
                            onClick = { onRun(query) },
                        )
                        // The label is the query as shown; the action says what a tap does.
                        .semantics(mergeDescendants = true) {}
                        .padding(vertical = LsSpacing.m),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(BrowseIcons.History, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(22.dp))
                    Spacer(Modifier.width(LsSpacing.m))
                    Text(query, style = LsType.body, color = LsColors.textPrimary, maxLines = 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                    Spacer(Modifier.width(LsSpacing.s))
                    Icon(BrowseIcons.NorthWest, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(18.dp))
                }
                DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                    DropdownMenuItem(
                        text = { Text(Copy.Common.remove, color = LsColors.danger) },
                        leadingIcon = { Icon(LsIcons.Close, contentDescription = null, tint = LsColors.danger) },
                        onClick = {
                            menuOpen = false
                            onRemove(query)
                        },
                    )
                }
            }
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(1.dp)
                    .background(LsColors.divider),
            )
        }
    }
}

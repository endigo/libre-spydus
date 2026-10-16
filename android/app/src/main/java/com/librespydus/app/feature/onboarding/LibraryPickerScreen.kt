package com.librespydus.app.feature.onboarding

import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Tab
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.SearchField
import com.librespydus.app.ui.theme.LocalAppLayout
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import com.librespydus.app.ui.theme.PageLayout
import com.librespydus.app.ui.theme.centredColumn
import kotlinx.coroutines.launch
import uniffi.librespydus_core.LibraryInfo
import java.io.File

/**
 * Library picker (docs/native-apps.md §6): Civica's directory grouped by region, with search.
 * Full screen on first launch ("Choose your library", [onDismiss] null) and over the tabs from
 * Profile ("Change library"). The directory is cached on disk for a week ([LibraryPickerModel]).
 */
@Composable
fun LibraryPickerScreen(appState: AppState, onDismiss: (() -> Unit)?, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val activity = LocalActivity.current
    val picker = remember {
        LibraryPickerModel(
            fetch = { appState.listLibraries() },
            cache = LibraryDirectoryCache(File(context.cacheDir, LibraryDirectoryCache.FILE_NAME)),
        )
    }
    val libraries by picker.libraries.collectAsStateWithLifecycle()
    val fetching by picker.isFetching.collectAsStateWithLifecycle()
    val loadError by picker.loadError.collectAsStateWithLifecycle()
    val refreshError by picker.refreshError.collectAsStateWithLifecycle()
    val query by picker.query.collectAsStateWithLifecycle()
    val loaded by picker.loaded.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    val focus = LocalFocusManager.current
    val onboarding = onDismiss == null

    LaunchedEffect(picker) {
        OnboardingDebugHooks.string(activity, OnboardingDebugHooks.PICKER_QUERY)
            ?.takeIf { query.isEmpty() && OnboardingDebugHooks.consumeOnce(OnboardingDebugHooks.PICKER_QUERY) }
            ?.let(picker::setQuery)
        picker.load()
    }

    fun choose(library: LibraryInfo) {
        focus.clearFocus()
        val changed = appState.library.value?.origin != library.origin
        if (!changed && onDismiss != null) return onDismiss() // "Change library" to the same one
        scope.launch {
            appState.selectLibrary(library)
            if (changed) appState.selectTab(Tab.HOME)
        }
    }

    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding()
            .imePadding()
            // The wide layout (§6.1): the picker in a centred column, first launch or Change library.
            .then(if (LocalAppLayout.current.isWide) Modifier.centredColumn(PageLayout.pickerColumnWidth) else Modifier),
    ) {
        PickerHeader(onboarding = onboarding, onClose = onDismiss)
        SearchField(
            value = query,
            onValueChange = picker::setQuery,
            placeholder = Copy.Onboarding.searchPlaceholder,
            onSearch = { focus.clearFocus() },
            modifier = Modifier
                .padding(horizontal = LsSpacing.l)
                .padding(bottom = LsSpacing.m),
        )
        Box(Modifier.fillMaxWidth().weight(1f)) {
            when {
                libraries.isEmpty() && (fetching || !loaded) -> LoadingView(message = Copy.Onboarding.loading)
                libraries.isEmpty() && loadError != null ->
                    ErrorStateView(loadError!!, onRetry = { scope.launch { picker.retry() } })
                libraries.isEmpty() -> EmptyStateView(
                    title = Copy.Onboarding.emptyTitle,
                    message = Copy.Onboarding.emptyMessage,
                    icon = AccountIcons.Place,
                    actionLabel = Copy.Common.retry,
                    onAction = { scope.launch { picker.retry() } },
                )
                else -> PickerList(
                    libraries = libraries,
                    query = query,
                    refreshing = fetching,
                    refreshMessage = refreshError?.let { Copy.Onboarding.savedList(it.message) },
                    currentOrigin = appState.library.value?.origin,
                    scrollToCurrent = !onboarding,
                    onRefresh = { scope.launch { picker.refresh() } },
                    onChoose = ::choose,
                    onScrollStarted = { focus.clearFocus() },
                )
            }
        }
    }
}

@Composable
private fun PickerHeader(onboarding: Boolean, onClose: (() -> Unit)?) {
    if (onboarding) {
        Column(
            Modifier
                .fillMaxWidth()
                .padding(start = LsSpacing.l, end = LsSpacing.l, top = LsSpacing.xl, bottom = LsSpacing.l),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
        ) {
            Icon(LsIcons.Library, contentDescription = null, tint = LsColors.primary, modifier = Modifier.size(36.dp))
            Text(Copy.Onboarding.title, style = LsType.greeting, color = LsColors.textPrimary, modifier = Modifier.semantics { heading() })
            Text(
                Copy.Onboarding.intro,
                style = LsType.body,
                color = LsColors.textSecondary,
            )
        }
    } else {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(start = LsSpacing.l, end = LsSpacing.s, top = LsSpacing.l, bottom = LsSpacing.m),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                Copy.Onboarding.changeTitle,
                style = LsType.title,
                color = LsColors.textPrimary,
                modifier = Modifier.weight(1f).semantics { heading() },
            )
            if (onClose != null) {
                Text(
                    Copy.Common.close,
                    style = LsType.bodyStrong,
                    color = LsColors.primary,
                    modifier = Modifier
                        .clip(RoundedCornerShape(LsRadius.pill))
                        .clickable(role = Role.Button, onClick = onClose)
                        .padding(horizontal = LsSpacing.m, vertical = LsSpacing.s),
                )
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class, ExperimentalMaterial3Api::class)
@Composable
private fun PickerList(
    libraries: List<LibraryInfo>,
    query: String,
    refreshing: Boolean,
    refreshMessage: String?,
    currentOrigin: String?,
    scrollToCurrent: Boolean,
    onRefresh: () -> Unit,
    onChoose: (LibraryInfo) -> Unit,
    onScrollStarted: () -> Unit,
) {
    val sections = remember(libraries, query) { LibraryPickerLogic.sections(libraries, query) }
    val listState = rememberLazyListState()
    var scrolledToCurrent by rememberSaveable { mutableStateOf(false) }

    // "Change library": start at the current library so its check mark shows.
    LaunchedEffect(sections) {
        if (!scrollToCurrent || scrolledToCurrent || query.isNotBlank() || currentOrigin == null) return@LaunchedEffect
        var index = if (refreshMessage != null) 1 else 0
        for (section in sections) {
            index++ // header
            val position = section.libraries.indexOfFirst { it.origin == currentOrigin }
            if (position >= 0) {
                listState.scrollToItem(index + position, scrollOffset = -320)
                break
            }
            index += section.libraries.size
        }
        scrolledToCurrent = true
    }
    LaunchedEffect(listState) {
        var wasScrolling = false
        androidx.compose.runtime.snapshotFlow { listState.isScrollInProgress }.collect { scrolling ->
            if (scrolling && !wasScrolling) onScrollStarted()
            wasScrolling = scrolling
        }
    }

    val bottomInset = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    PullToRefreshBox(isRefreshing = refreshing, onRefresh = onRefresh, modifier = Modifier.fillMaxSize()) {
        LazyColumn(
            state = listState,
            contentPadding = PaddingValues(start = LsSpacing.l, end = LsSpacing.l, bottom = LsSpacing.xxl + bottomInset),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
            modifier = Modifier.fillMaxSize(),
        ) {
            if (refreshMessage != null) {
                item(key = "refresh-error") {
                    AccountBanner(refreshMessage, tone = AccountBannerTone.ERROR, modifier = Modifier.padding(bottom = LsSpacing.s))
                }
            }
            if (sections.isEmpty()) {
                item(key = "no-match") {
                    EmptyStateView(
                        title = Copy.Onboarding.noMatchesTitle(query.trim()),
                        message = Copy.Onboarding.noMatchesMessage,
                        icon = LsIcons.Search,
                        modifier = Modifier.padding(top = LsSpacing.xl),
                    )
                }
            }
            sections.forEach { section ->
                stickyHeader(key = "header-${section.region.code}") { SectionTitle(section) }
                // The core drops entries whose origin and name both repeat, so this key is unique.
                items(section.libraries, key = { "${section.region.code}:${it.origin}:${it.name}" }) { library ->
                    LibraryRow(
                        library = library,
                        region = section.region,
                        isCurrent = library.origin == currentOrigin,
                        onClick = { onChoose(library) },
                    )
                }
            }
        }
    }
}

@Composable
private fun SectionTitle(section: PickerSection) {
    Row(
        Modifier
            .fillMaxWidth()
            .background(LsColors.background)
            .padding(top = LsSpacing.m, bottom = LsSpacing.xs),
        verticalAlignment = Alignment.Bottom,
    ) {
        Text(
            section.region.title,
            style = LsType.section,
            color = LsColors.textPrimary,
            modifier = Modifier.weight(1f).semantics { heading() },
        )
        val count = section.libraries.size
        Text(
            "$count",
            style = LsType.caption,
            color = LsColors.textMuted,
            modifier = Modifier.semantics { contentDescription = Copy.Onboarding.libraries(count) },
        )
    }
}

/** One directory entry: logo, name, country, the region's caveat, a check on the current one. */
@Composable
private fun LibraryRow(library: LibraryInfo, region: PickerRegion, isCurrent: Boolean, onClick: () -> Unit) {
    val shape = RoundedCornerShape(LsRadius.card)
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 64.dp)
            .clip(shape)
            .background(if (isCurrent) LsColors.surfaceElevated else LsColors.surface)
            .then(if (isCurrent) Modifier.border(1.5.dp, LsColors.primary.copy(alpha = 0.7f), shape) else Modifier)
            .clickable(role = Role.Button, onClick = onClick)
            .clearAndSetSemantics {
                // The name, country and caveat; the current one says so, the others what a tap does.
                val label = listOfNotNull(library.name, library.country?.takeIf { it.isNotBlank() }, region.caveat).joinToString(", ")
                contentDescription = if (isCurrent) label else Copy.A11y.labelAndHint(label, Copy.Onboarding.selectHint)
                if (isCurrent) stateDescription = Copy.Onboarding.currentLibrary
                selected = isCurrent
                role = Role.Button
                onClick { onClick(); true }
            }
            .padding(LsSpacing.m),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        LibraryLogo(library)
        Spacer(Modifier.width(LsSpacing.m))
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(library.name, style = LsType.bodyStrong, color = LsColors.textPrimary)
            library.country?.takeIf { it.isNotBlank() }?.let {
                Text(it, style = LsType.caption, color = LsColors.textSecondary)
            }
            region.caveat?.let { caveat ->
                Row(Modifier.padding(top = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        if (region == PickerRegion.UK) AccountIcons.VerifiedUser else AccountIcons.WifiOff,
                        contentDescription = null,
                        tint = LsColors.warning,
                        modifier = Modifier.size(14.dp),
                    )
                    Spacer(Modifier.width(LsSpacing.xs))
                    Text(caveat, style = LsType.caption, color = LsColors.warning)
                }
            }
        }
        if (isCurrent) {
            Spacer(Modifier.width(LsSpacing.s))
            Icon(
                AccountIcons.CheckCircle,
                contentDescription = Copy.Onboarding.currentLibrary,
                tint = LsColors.primary,
                modifier = Modifier.size(24.dp),
            )
        }
    }
}

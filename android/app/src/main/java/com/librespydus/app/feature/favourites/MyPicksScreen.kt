package com.librespydus.app.feature.favourites

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.feature.search.SearchResultRow
import com.librespydus.app.feature.search.SearchSkeletonList
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LocalAppLayout
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import com.librespydus.app.ui.theme.PageLayout
import com.librespydus.app.ui.theme.cardRows
import com.librespydus.app.ui.theme.pageColumn

/**
 * Favourites while signed in (§5.11): the reader's Pick list on the library account, "My Picks".
 * Rows are the search results row (tap → book page); a row's filled bookmark, its long-press menu
 * or the screen reader's action removes it at once (back, with "Couldn’t update My Picks", on
 * failure). Pull to refresh; pages past the first ten load as the list scrolls.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun MyPicksScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val picks = appState.picks
    val list by picks.list.collectAsStateWithLifecycle()
    val pending by picks.pending.collectAsStateWithLifecycle()
    val session by appState.session.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val content = remember(list) { MyPicksContent.of(list) }

    // Each time the tab shows (and after a verification): reload a list that is missing, failed,
    // old, or lacks a book saved since.
    LaunchedEffect(session, verification) { picks.refreshIfStale() }

    PullToRefreshBox(
        isRefreshing = list.refreshing,
        onRefresh = { picks.reload() },
        modifier = modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding(),
    ) {
        BoxWithConstraints(Modifier.fillMaxSize().pageColumn()) {
        // The wide layout (§6.1): the rows become a grid of cards.
        val columns = if (LocalAppLayout.current.isWide) PageLayout.cardColumns(maxWidth - LsSpacing.l * 2) else 1
        LazyColumn(
            contentPadding = PaddingValues(bottom = LsSpacing.xxl),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
            modifier = Modifier.fillMaxSize(),
        ) {
            item(key = "header") { PicksHeader() }
            when (content) {
                MyPicksContent.Loading -> item(key = "loading") {
                    SearchSkeletonList(rows = 4, modifier = Modifier.heightIn(min = 200.dp), columns = columns)
                }
                is MyPicksContent.Failed -> item(key = "error") {
                    ErrorStateView(
                        error = content.error,
                        onRetry = if (content.error.isRetryable) ({ picks.reload() }) else null,
                        modifier = Modifier.fillMaxWidth().heightIn(min = 420.dp),
                    )
                }
                MyPicksContent.Empty -> item(key = "empty") {
                    EmptyStateView(
                        title = Copy.Picks.emptyTitle,
                        message = FavouritesMode.MY_PICKS.emptyMessage,
                        icon = LsIcons.Bookmark,
                        modifier = Modifier.fillMaxWidth().heightIn(min = 420.dp),
                    )
                }
                is MyPicksContent.Rows -> {
                    if (columns == 1) {
                        items(content.rows, key = { it.item.id }) { row ->
                            PickRowView(
                                row = row,
                                busy = row.irn != null && row.irn in pending,
                                onOpen = { navigator.openBook(row.item.ref) },
                                onRemove = { row.irn?.let(picks::remove) },
                                modifier = Modifier
                                    .animateItem()
                                    .padding(horizontal = LsSpacing.l),
                            )
                        }
                    } else {
                        cardRows(content.rows, columns, key = { it.item.id }, modifier = Modifier.padding(horizontal = LsSpacing.l)) { _, row, cell ->
                            PickRowView(
                                row = row,
                                busy = row.irn != null && row.irn in pending,
                                onOpen = { navigator.openBook(row.item.ref) },
                                onRemove = { row.irn?.let(picks::remove) },
                                modifier = cell,
                            )
                        }
                    }
                    item(key = "footer") {
                        PicksFooter(content.footer, onLoadMore = { picks.loadMore() }, onRetry = { picks.retryLoadMore() })
                    }
                }
            }
        }
        }
    }
}

@Composable
private fun PicksHeader() {
    Column(
        Modifier
            .fillMaxWidth()
            .padding(start = LsSpacing.l, end = LsSpacing.l, top = LsSpacing.l, bottom = LsSpacing.xs),
    ) {
        Text(FavouritesMode.MY_PICKS.title, style = LsType.greeting, color = LsColors.textPrimary, modifier = Modifier.semantics { heading() })
        Text(FavouritesMode.MY_PICKS.subtitle, style = LsType.caption, color = LsColors.textSecondary)
    }
}

/** A search results row with a filled bookmark (tap: remove) and a long-press menu. */
@Composable
private fun PickRowView(row: PickRow, busy: Boolean, onOpen: () -> Unit, onRemove: () -> Unit, modifier: Modifier = Modifier) {
    var menu by remember { mutableStateOf(false) }
    val haptics = LocalHapticFeedback.current
    val canRemove = row.irn != null
    Box(modifier) {
        SearchResultRow(
            item = row.item,
            onClick = onOpen,
            // Fills a grid row's height (§6.1); in a list, its own.
            modifier = Modifier.fillMaxHeight(),
            onLongClick = {
                haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                menu = true
            },
            actions = if (canRemove) listOf(CustomAccessibilityAction(Copy.Picks.remove) { onRemove(); true }) else emptyList(),
            trailing = if (canRemove) {
                {
                    Box(
                        Modifier
                            .size(40.dp)
                            .clip(CircleShape)
                            .clickable(enabled = !busy, role = Role.Button, onClick = onRemove)
                            .semantics { contentDescription = Copy.Picks.remove },
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(LsIcons.BookmarkFilled, contentDescription = null, tint = LsColors.accent, modifier = Modifier.size(24.dp))
                    }
                }
            } else {
                null
            },
        )
        DropdownMenu(
            expanded = menu,
            onDismissRequest = { menu = false },
            containerColor = LsColors.surfaceElevated,
        ) {
            DropdownMenuItem(
                text = { Text(Copy.Common.open, color = LsColors.textPrimary) },
                leadingIcon = { Icon(LsIcons.Book, contentDescription = null, tint = LsColors.textSecondary) },
                onClick = {
                    menu = false
                    onOpen()
                },
            )
            if (canRemove) {
                DropdownMenuItem(
                    text = { Text(Copy.Picks.remove, color = LsColors.danger) },
                    leadingIcon = { Icon(AccountIcons.BookmarkRemove, contentDescription = null, tint = LsColors.danger) },
                    enabled = !busy,
                    onClick = {
                        menu = false
                        onRemove()
                    },
                )
            }
        }
    }
}

@Composable
private fun PicksFooter(footer: MyPicksContent.Footer, onLoadMore: () -> Unit, onRetry: () -> Unit) {
    Box(
        Modifier
            .fillMaxWidth()
            .padding(LsSpacing.l),
        contentAlignment = Alignment.Center,
    ) {
        when (footer) {
            MyPicksContent.Footer.None -> Unit
            is MyPicksContent.Footer.More -> {
                LaunchedEffect(footer.nextPageUrl) { onLoadMore() }
                CircularProgressIndicator(
                    color = LsColors.textSecondary,
                    strokeWidth = 2.dp,
                    modifier = Modifier
                        .size(24.dp)
                        .semantics { contentDescription = Copy.Picks.loadingMore },
                )
            }
            is MyPicksContent.Footer.Failed -> FooterRetry(footer.error, onRetry)
        }
    }
}

@Composable
private fun FooterRetry(error: UserFacingError, onRetry: () -> Unit) {
    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
        Text(error.message, style = LsType.caption, color = LsColors.textSecondary, textAlign = TextAlign.Center)
        if (error.isRetryable) {
            Row(
                Modifier
                    .clip(RoundedCornerShape(LsRadius.pill))
                    .clickable(role = Role.Button, onClick = onRetry)
                    .semantics(mergeDescendants = true) { contentDescription = Copy.A11y.labelAndHint(Copy.Common.retry, Copy.Picks.retryLoadingMoreHint) }
                    .padding(horizontal = LsSpacing.m, vertical = LsSpacing.xs),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(LsIcons.Refresh, contentDescription = null, tint = LsColors.primary, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(LsSpacing.xs))
                Text(Copy.Common.retry, style = LsType.bodyStrong, color = LsColors.primary)
            }
        }
    }
}

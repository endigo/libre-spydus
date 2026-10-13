package com.librespydus.app.feature.search

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.onLongClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Formatting
import com.librespydus.app.ui.components.Chip
import com.librespydus.app.ui.components.CoverImage
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsTabBar
import com.librespydus.app.ui.theme.LsType
import java.text.NumberFormat

/** Bottom padding under scrolling content on a tab, above the tab bar. */
val BrowseBottomClearance: Dp = LsSpacing.xxl

/**
 * The results list both the Search tab and [SearchResultsScreen] show: count line, result rows
 * (tap → detail), infinite scroll with a footer spinner, and the loading / no-results / error
 * states.
 */
@Composable
fun SearchResultsList(
    state: SearchResultsModel.State,
    query: String,
    onRetry: () -> Unit,
    onLoadMore: () -> Unit,
    onRetryLoadMore: () -> Unit,
    onOpen: (BookRef) -> Unit,
    modifier: Modifier = Modifier,
    listState: LazyListState = rememberLazyListState(),
    noResultsTitle: String = "No results for “$query”",
    noResultsMessage: String? = "Check the spelling, or try fewer or different words.",
) {
    when (val phase = state.phase) {
        SearchResultsModel.Phase.Idle, SearchResultsModel.Phase.Loading -> SearchSkeletonList(modifier)
        SearchResultsModel.Phase.NoResults -> EmptyStateView(
            title = noResultsTitle,
            message = noResultsMessage,
            icon = BrowseIcons.SearchOff,
            modifier = modifier.padding(bottom = BrowseBottomClearance),
        )
        is SearchResultsModel.Phase.Failed -> ErrorStateView(
            error = phase.error,
            onRetry = onRetry.takeIf { phase.error.isRetryable },
            modifier = modifier.padding(bottom = BrowseBottomClearance),
        )
        SearchResultsModel.Phase.Loaded -> LazyColumn(
            modifier = modifier.fillMaxSize(),
            state = listState,
            contentPadding = PaddingValues(top = LsSpacing.xs, bottom = BrowseBottomClearance),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
        ) {
            resultCount(state.total)?.let { count ->
                item(key = "count") {
                    Text(
                        count,
                        style = LsType.caption,
                        color = LsColors.textSecondary,
                        modifier = Modifier
                            .padding(horizontal = LsSpacing.l)
                            .semantics { heading() },
                    )
                }
            }
            itemsIndexed(state.items, key = { _, item -> item.id }) { index, item ->
                // Prefetch a few rows before the end.
                if (index >= state.items.size - 4) LaunchedEffect(state.items.size) { onLoadMore() }
                SearchResultRow(item, onClick = { onOpen(item.ref) }, modifier = Modifier.padding(horizontal = LsSpacing.l))
            }
            item(key = "footer") {
                ResultsFooter(state, onLoadMore, onRetryLoadMore)
            }
        }
    }
}

@Composable
private fun ResultsFooter(state: SearchResultsModel.State, onLoadMore: () -> Unit, onRetryLoadMore: () -> Unit) {
    Box(
        Modifier
            .fillMaxWidth()
            .padding(LsSpacing.l),
        contentAlignment = Alignment.Center,
    ) {
        val error = state.loadMoreError
        when {
            error != null -> Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                Text(error.message, style = LsType.caption, color = LsColors.textSecondary, textAlign = TextAlign.Center)
                if (error.isRetryable) {
                    Row(
                        Modifier
                            .clip(RoundedCornerShape(LsRadius.pill))
                            .clickable(role = Role.Button, onClick = onRetryLoadMore)
                            .semantics(mergeDescendants = true) { contentDescription = "Retry loading more results" }
                            .padding(horizontal = LsSpacing.m, vertical = LsSpacing.xs),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(LsIcons.Refresh, contentDescription = null, tint = LsColors.primary, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(LsSpacing.xs))
                        Text("Retry", style = LsType.bodyStrong, color = LsColors.primary)
                    }
                }
            }
            state.hasMore -> {
                LaunchedEffect(state.nextPageUrl) { onLoadMore() }
                CircularProgressIndicator(
                    color = LsColors.textSecondary,
                    strokeWidth = 2.dp,
                    modifier = Modifier
                        .size(24.dp)
                        .semantics { contentDescription = "Loading more results" },
                )
            }
            state.items.isNotEmpty() -> Text("End of results", style = LsType.caption, color = LsColors.textMuted)
        }
    }
}

/** "1 result" / "1,234 results"; null without a total. */
fun resultCount(total: Long?): String? {
    total ?: return null
    return if (total == 1L) "1 result" else "${NumberFormat.getIntegerInstance().format(total)} results"
}

/**
 * A result: cover, title, author, format chips (digital ones tinted) and year. My Picks (§5.11)
 * adds a long-press menu ([onLongClick]), screen-reader [actions] and a [trailing] button.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun SearchResultRow(
    item: SearchResultItem,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
    actions: List<CustomAccessibilityAction> = emptyList(),
    trailing: (@Composable () -> Unit)? = null,
) {
    val shape = RoundedCornerShape(LsRadius.card)
    val rise = 14.dp
    val byLine = Formatting.byLine(item.ref.author)
    val chips = item.formats.take(MAX_CHIPS)
    val description = buildList {
        add(item.ref.title)
        byLine?.let(::add)
        item.ref.year?.let(::add)
        if (item.formats.isNotEmpty()) add(item.formats.joinToString(", ") { it.label })
    }.joinToString(". ")
    Box(
        modifier
            .fillMaxWidth()
            .clearAndSetSemantics {
                contentDescription = description
                role = Role.Button
                onClick { onClick(); true }
                if (onLongClick != null) onLongClick(label = "More options") { onLongClick(); true }
                if (actions.isNotEmpty()) customActions = actions
            },
    ) {
        Box(
            Modifier
                .matchParentSize()
                .padding(top = rise)
                .clip(shape)
                .background(LsColors.surface)
                .combinedClickable(role = Role.Button, onLongClick = onLongClick, onClick = onClick),
        )
        Row(Modifier.padding(start = LsSpacing.l, end = LsSpacing.l, bottom = LsSpacing.l), verticalAlignment = Alignment.Top) {
            CoverImage(url = item.ref.coverUrl, title = item.ref.title, width = LsCover.smallWidth)
            Spacer(Modifier.width(LsSpacing.l))
            Column(
                Modifier
                    .weight(1f)
                    .padding(top = rise + LsSpacing.m),
                verticalArrangement = Arrangement.spacedBy(LsSpacing.xs),
            ) {
                Text(item.ref.title, style = LsType.bodyStrong, color = LsColors.textPrimary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                if (byLine != null) Text(byLine, style = LsType.caption, color = LsColors.textSecondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (chips.isNotEmpty()) {
                    FlowRow(
                        Modifier.padding(top = LsSpacing.xs),
                        horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
                        verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
                    ) {
                        chips.forEach { Chip(it.label, tint = if (it.isDigital) LsColors.digital else null) }
                        if (item.formats.size > MAX_CHIPS) Chip("+${item.formats.size - MAX_CHIPS}")
                    }
                }
                item.ref.year?.let { Text(it, style = LsType.caption, color = LsColors.textMuted, modifier = Modifier.padding(top = LsSpacing.xs)) }
            }
            if (trailing != null) {
                Box(Modifier.padding(top = rise + LsSpacing.xs, start = LsSpacing.s)) { trailing() }
            }
        }
    }
}

private const val MAX_CHIPS = 4

/** A pulsing placeholder block (Home shelves, search rows) while content loads. */
@Composable
fun SkeletonBlock(modifier: Modifier = Modifier, radius: Dp = 6.dp, onSheet: Boolean = false) {
    val transition = rememberInfiniteTransition(label = "skeleton")
    val alpha by transition.animateFloat(
        initialValue = 1f,
        targetValue = 0.45f,
        animationSpec = infiniteRepeatable(tween(900), RepeatMode.Reverse),
        label = "skeletonAlpha",
    )
    Box(
        modifier
            .alpha(alpha)
            .clip(RoundedCornerShape(radius))
            .background(if (onSheet) LsColors.chipOnSheet else LsColors.surfaceElevated),
    )
}

/** Placeholder rows while the first page loads. */
@Composable
fun SearchSkeletonList(modifier: Modifier = Modifier, rows: Int = 5) {
    Column(
        modifier
            .fillMaxSize()
            .padding(horizontal = LsSpacing.l)
            .padding(top = LsSpacing.s)
            .clearAndSetSemantics { contentDescription = "Loading results" },
        verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        repeat(rows) {
            Row(
                Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(LsRadius.card))
                    .background(LsColors.surface)
                    .padding(LsSpacing.l),
            ) {
                SkeletonBlock(Modifier.width(LsCover.smallWidth).height(LsCover.height(LsCover.smallWidth)), radius = LsRadius.cover)
                Spacer(Modifier.width(LsSpacing.l))
                Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                    SkeletonBlock(Modifier.fillMaxWidth().height(14.dp))
                    SkeletonBlock(Modifier.width(140.dp).height(12.dp))
                    SkeletonBlock(Modifier.padding(top = LsSpacing.xs).width(90.dp).height(18.dp), radius = 9.dp)
                }
            }
        }
    }
}

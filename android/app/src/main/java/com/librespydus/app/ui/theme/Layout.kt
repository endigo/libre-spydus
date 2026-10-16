package com.librespydus.app.ui.theme

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.isSecondaryPressed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.layout
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.floor
import kotlin.math.min

/*
 * The wide layout (docs/native-apps.md §6.1), shared by every screen: when a window is wide, the
 * compact (phone) layout is kept but given a sidebar in place of the tab bar, a centred content
 * column, card grids and a two-column book page. The same numbers as the iOS app's Layout.swift.
 */

/**
 * Which layout a window gets: [WIDE] when it is at least 600 dp wide and 480 dp tall, measured at
 * the root (a tablet either way round, a large freeform window); [COMPACT] otherwise (a phone either
 * way round, a narrow split screen, a small window). Screens read [LocalAppLayout].
 */
enum class AppLayout {
    COMPACT,
    WIDE;

    val isWide: Boolean get() = this == WIDE

    companion object {
        val minWideWidth: Dp = 600.dp
        val minWideHeight: Dp = 480.dp

        /** The layout for a window [width] × [height]. */
        fun of(width: Dp, height: Dp): AppLayout =
            if (width >= minWideWidth && height >= minWideHeight) WIDE else COMPACT
    }
}

/** The window's layout, provided at the root from its measured size. */
val LocalAppLayout = compositionLocalOf { AppLayout.COMPACT }

/** The wide layout's measurements (§6.1). */
object PageLayout {
    /** A screen's content stays in a centred column at most this wide. */
    val contentMaxWidth: Dp = 1040.dp
    val sidebarWidth: Dp = 260.dp
    val sidebarLogoSize: Dp = 40.dp

    /** Card grids (Your Books, results, My Picks): cards at least this wide, at most [cardMaxColumns] across. */
    val cardMinWidth: Dp = 300.dp
    const val cardMaxColumns = 3
    val cardSpacing: Dp = LsSpacing.m

    /** Cover grids (Favourites, the book page's related column): 2–[coverMaxColumns] covers at least this wide. */
    val coverMinWidth: Dp = 120.dp
    const val coverMaxColumns = 6
    val coverSpacing: Dp = LsSpacing.l

    /** The book page's left column while its related column shows, and the page width two columns need. */
    val detailLeftWidth: Dp = 440.dp
    val detailTwoColumnMinWidth: Dp = 820.dp

    /** Your Books' segments beside its title; Search's sort beside its field. */
    val borrowedSegmentsWidth: Dp = 420.dp
    val searchSortWidth: Dp = 240.dp

    /** The first-launch library picker's column. */
    val pickerColumnWidth: Dp = 760.dp

    /** How many cards fit across [width]: as many as stay at least [cardMinWidth] wide, 1 to [cardMaxColumns]. */
    fun cardColumns(width: Dp): Int =
        floor((width + cardSpacing) / (cardMinWidth + cardSpacing)).toInt().coerceIn(1, cardMaxColumns)

    /**
     * A cover grid across [width]: as many columns as keep covers at least [coverMinWidth] wide
     * (2 to [coverMaxColumns]), the covers then widened to fill the row, to whole dp.
     */
    fun coverGrid(width: Dp): CoverGrid {
        val columns = floor((width + coverSpacing) / (coverMinWidth + coverSpacing)).toInt().coerceIn(2, coverMaxColumns)
        val cover = (width - coverSpacing * (columns - 1)) / columns
        return CoverGrid(columns, floor(cover.value).dp)
    }
}

/** A cover grid's shape: [columns] covers each [coverWidth] wide, [PageLayout.coverSpacing] apart. */
data class CoverGrid(val columns: Int, val coverWidth: Dp)

/**
 * Keeps a screen's content in a centred column at most [PageLayout.contentMaxWidth] wide when the
 * layout is wide; nothing changes in the compact layout. The element itself still spans the width
 * it is given, so its background covers the margins.
 */
@Composable
fun Modifier.pageColumn(): Modifier =
    if (LocalAppLayout.current.isWide) centredColumn(PageLayout.contentMaxWidth) else this

/** Measures the content at most [maxWidth] wide and centres it in the width available. */
fun Modifier.centredColumn(maxWidth: Dp): Modifier = layout { measurable, constraints ->
    val maxPx = maxWidth.roundToPx()
    val inner = if (constraints.maxWidth > maxPx) {
        constraints.copy(minWidth = min(constraints.minWidth, maxPx), maxWidth = maxPx)
    } else {
        constraints
    }
    val placeable = measurable.measure(inner)
    val width = if (constraints.hasBoundedWidth) constraints.maxWidth else placeable.width
    layout(width, placeable.height) {
        placeable.placeRelative((width - placeable.width) / 2, 0)
    }
}

/**
 * Cards in rows of [columns] (§6.1): a row is as tall as its tallest card and every card in it
 * fills that height; a short last row keeps its cards' width with empty cells. [card] draws one
 * item with the modifier that sizes it, [index] counting across the whole list. Each row is one
 * list item, with [modifier], keyed by its first card's [key]; the list's spacing separates rows.
 */
fun <T> LazyListScope.cardRows(
    items: List<T>,
    columns: Int,
    key: (T) -> Any,
    modifier: Modifier = Modifier,
    card: @Composable (index: Int, item: T, modifier: Modifier) -> Unit,
) {
    items.chunked(columns).forEachIndexed { rowIndex, row ->
        item(key = key(row.first()), contentType = "card-row") {
            Row(
                modifier.fillMaxWidth().height(IntrinsicSize.Max),
                horizontalArrangement = Arrangement.spacedBy(PageLayout.cardSpacing),
            ) {
                row.forEachIndexed { column, item ->
                    Box(Modifier.weight(1f).fillMaxHeight()) {
                        card(rowIndex * columns + column, item, Modifier.fillMaxHeight())
                    }
                }
                repeat(columns - row.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

/**
 * Runs [onClick] when a mouse's secondary button presses here (a right-click): the pointer's way
 * to a card's long-press menu (§6.1). Touches and the primary button pass through untouched.
 */
fun Modifier.onRightClick(onClick: () -> Unit): Modifier = pointerInput(onClick) {
    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        if (down.type == PointerType.Mouse && currentEvent.buttons.isSecondaryPressed) {
            down.consume()
            onClick()
        }
    }
}

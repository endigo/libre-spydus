package com.librespydus.app.feature.detail

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Formatting
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.SkeletonBlock
import com.librespydus.app.feature.search.valueOrNull
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.components.ShelfCover
import com.librespydus.app.ui.theme.CoverGrid
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.PageLayout

/**
 * The book page's right column in the wide layout (docs/native-apps.md §6.1): the other books
 * (similar, by the author, in the series, other editions) as cover grids on navy, in place of the
 * sheet's carousels. While the book loads, two untitled grids of blank covers hold the space, so
 * the page opens in the two columns it most likely ends up with. The grids fill [width].
 * [onStartRelated] starts the related lists' loads once the record is in.
 */
@Composable
fun DetailRelatedColumn(
    state: DetailState,
    width: Dp,
    onStartRelated: () -> Unit,
    onOpenBook: (BookRef) -> Unit,
    modifier: Modifier = Modifier,
) {
    val grid = PageLayout.coverGrid(width)
    val record = state.record.valueOrNull
    // Again for an upgraded record (its links may differ).
    LaunchedEffect(record?.related) { if (state.hasRelatedPanels) onStartRelated() }
    Column(modifier.width(width), verticalArrangement = Arrangement.spacedBy(LsSpacing.xl)) {
        if (record == null) {
            repeat(2) {
                Column(Modifier.clearAndSetSemantics {}, verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                    SkeletonBlock(Modifier.width(PLACEHOLDER_TITLE_WIDTH).height(PLACEHOLDER_TITLE_HEIGHT))
                    PlaceholderRow(grid)
                }
            }
        } else {
            state.visiblePanels.forEach { panel ->
                RelatedSection(panel, state.related[panel] ?: BrowseLoadable.Loading, grid, onOpenBook)
            }
        }
    }
}

/** Whether the book page shows [DetailRelatedColumn]. */
object DetailRelatedColumn {
    /**
     * True while the column has something to show: the book is loading, or it has other books
     * that are loading or loaded. A failed record, or one without related books, hides it and the
     * page's left column fills the width.
     */
    fun isShown(state: DetailState): Boolean = when (state.record) {
        BrowseLoadable.Idle, BrowseLoadable.Loading -> true
        is BrowseLoadable.Failed -> false
        is BrowseLoadable.Loaded -> state.visiblePanels.isNotEmpty()
    }
}

/** One panel: its title over a grid of covers, blank ones while it loads. */
@Composable
private fun RelatedSection(
    panel: RelatedPanel,
    state: BrowseLoadable<List<BookRef>>,
    grid: CoverGrid,
    onOpenBook: (BookRef) -> Unit,
) {
    val refs = state.valueOrNull
    Column(
        Modifier
            .fillMaxWidth()
            .semantics { contentDescription = if (refs == null) Copy.Detail.loadingPanel(panel.title.lowercase()) else panel.title },
        verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        SectionHeader(panel.title)
        if (refs == null) {
            PlaceholderRow(grid)
        } else {
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.xl)) {
                refs.chunked(grid.columns).forEach { row ->
                    Row(horizontalArrangement = Arrangement.spacedBy(PageLayout.coverSpacing), verticalAlignment = Alignment.Top) {
                        row.forEach { ref ->
                            ShelfCover(
                                title = DetailText.displayTitle(ref.title),
                                coverUrl = ref.coverUrl,
                                width = grid.coverWidth,
                                caption = Formatting.displayAuthor(ref.author) ?: ref.year,
                                showTitle = true,
                                onClick = { onOpenBook(ref) },
                            )
                        }
                    }
                }
            }
        }
    }
}

/** A row of blank covers in the grid's shape. */
@Composable
private fun PlaceholderRow(grid: CoverGrid) {
    Row(Modifier.clearAndSetSemantics {}, horizontalArrangement = Arrangement.spacedBy(PageLayout.coverSpacing)) {
        repeat(grid.columns) {
            Box(
                Modifier
                    .width(grid.coverWidth)
                    .aspectRatio(LsCover.ASPECT)
                    .clip(RoundedCornerShape(LsRadius.cover))
                    .background(LsColors.surface),
            )
        }
    }
}

private val PLACEHOLDER_TITLE_WIDTH = 160.dp
private val PLACEHOLDER_TITLE_HEIGHT = 18.dp

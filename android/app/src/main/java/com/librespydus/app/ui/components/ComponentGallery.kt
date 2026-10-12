package com.librespydus.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.Tab
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/** DEBUG-only catalogue of every shared component (`ls_route gallery`), for visual checks. */
@Composable
fun ComponentGallery(onClose: () -> Unit, modifier: Modifier = Modifier) {
    var query by remember { mutableStateOf("") }
    var saved by remember { mutableStateOf(true) }
    var segment by remember { mutableIntStateOf(0) }
    var format by remember { mutableIntStateOf(0) }
    var tab by remember { mutableStateOf(Tab.HOME) }
    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding(),
    ) {
        LsTopBar(title = "Components", trailing = {
            CircleIconButton(LsIcons.Close, "Close gallery", onClose, size = 44.dp, background = LsColors.surface, shadow = false)
        })
        Column(
            Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .navigationBarsPadding(),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
        ) {
            Group("SearchField") {
                SearchField(query, { query = it })
                SearchField("", {}, onClick = {})
            }
            Group("SectionHeader · ShelfCover · CoverImage") {
                SectionHeader("New arrivals", onAction = {})
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(LsSpacing.l)) {
                    SAMPLE_COVERS.forEach { (title, url) -> ShelfCover(title, url, onClick = {}) }
                }
                SectionHeader("Your borrowed books", onAction = {})
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                    ShelfCover(SAMPLE_COVERS[0].first, SAMPLE_COVERS[0].second, width = 88.dp, caption = "2 days left", captionColor = LsColors.danger)
                    ShelfCover(SAMPLE_COVERS[1].first, SAMPLE_COVERS[1].second, width = 88.dp, caption = "9 days left")
                    ShelfCover("A title without any cover image", null, width = 88.dp, caption = "Due 28 Oct")
                    ShelfCover("Overdue book", null, width = 88.dp, caption = "Overdue by 1 day", captionColor = LsColors.danger)
                }
            }
            Group("GenreTile") {
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(LsSpacing.l)) {
                    CoreHelpers.homeGenres().forEachIndexed { i, genre ->
                        GenreTile(genre.label, LsIcons.genre(genre.key), LsColors.genre[i % LsColors.genre.size], onClick = {})
                    }
                }
            }
            Group("PrimaryPillButton · CircleIconButton · SecondaryPillButton") {
                Row(horizontalArrangement = Arrangement.spacedBy(LsSpacing.l)) {
                    PrimaryPillButton("Borrow Now", onClick = {}, modifier = Modifier.weight(1f))
                    CircleIconButton(if (saved) LsIcons.BookmarkFilled else LsIcons.Bookmark, "Favourite", { saved = !saved })
                }
                Row(horizontalArrangement = Arrangement.spacedBy(LsSpacing.l)) {
                    PrimaryPillButton("Loading", onClick = {}, loading = true)
                    PrimaryPillButton("Disabled", onClick = {}, enabled = false)
                    SecondaryPillButton("Retry", onClick = {})
                }
            }
            Group("Chip · SegmentedToggle") {
                Row(horizontalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                    listOf("Book", "Large print", "eBook").forEachIndexed { i, label ->
                        Chip(label, selected = format == i, tint = if (label == "eBook") LsColors.digital else null, onClick = { format = i })
                    }
                }
                SegmentedToggle(listOf("Loans", "Reservations"), segment, { segment = it }, Modifier.fillMaxWidth())
                SegmentedToggle(listOf("Relevance", "Newest"), 1 - segment, { segment = 1 - it }, Modifier.fillMaxWidth(0.7f))
            }
            Group("BookRowCard") {
                BookRowCard(
                    title = SAMPLE_COVERS[0].first, coverUrl = SAMPLE_COVERS[0].second, byLine = "By Andy Weir",
                    meta = "Due 14 Oct · 2 days left", metaColor = LsColors.danger, onClick = {},
                )
                BookRowCard(
                    title = "The midnight library", coverUrl = null, byLine = "By Matt Haig",
                    meta = "Pickup: RIVERTON LIBRARY", badge = "Ready for pickup", highlighted = true,
                    actions = listOf(CardAction("Cancel", {}), CardAction("Edit", {})), onClick = {},
                )
                BookRowCard(
                    title = SAMPLE_COVERS[2].first, coverUrl = SAMPLE_COVERS[2].second, byLine = "By Bonnie Garmus",
                    meta = "Waiting · Rank 3 · CANNINGTON LIBRARY", actions = listOf(CardAction("Cancel", {})),
                )
            }
            Group("Sheet (detail screen bottom)") {
                Column(
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(topStart = LsRadius.sheet, topEnd = LsRadius.sheet))
                        .background(LsColors.sheet)
                        .padding(LsSpacing.xl),
                    verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
                ) {
                    SectionHeader("Synopsis", color = LsColors.onSheet)
                    Row(horizontalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                        Chip("Fiction", onSheet = true)
                        Chip("Science fiction", onSheet = true)
                        Chip("Adult", onSheet = true, selected = true)
                    }
                    Text(
                        "A lone astronaut must save the earth from disaster in this propulsive interstellar adventure.",
                        style = LsType.body, color = LsColors.onSheetSecondary,
                    )
                    SectionHeader("Similar books", color = LsColors.onSheet, actionColor = LsColors.onSheetSecondary, onAction = {})
                    Row(horizontalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                        SAMPLE_COVERS.take(3).forEach { (t, u) -> CoverImage(u, t, width = LsCover.smallWidth + 16.dp) }
                    }
                }
            }
            Group("LoadingView · EmptyStateView · ErrorStateView") {
                StateBox { LoadingView(message = "Loading your books…") }
                StateBox { EmptyStateView("No results for “zzqx”", message = "Try a different title or author.", icon = LsIcons.Search) }
                StateBox {
                    ErrorStateView(
                        UserFacingError(UserFacingError.Kind.UNEXPECTED, UserFacingError.UNEXPECTED, "upstream error: HTTP 503 (https://canning.spydus.com/…)"),
                        onRetry = {},
                    )
                }
            }
            Group("Tab bar") {
                LsTabBar(selected = tab, onSelect = { tab = it })
            }
            Spacer(Modifier.height(LsSpacing.xxl))
        }
    }
}

@Composable
private fun Group(title: String, content: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.padding(horizontal = LsSpacing.xl), verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        Text(title.uppercase(), style = LsType.caption, color = LsColors.textMuted)
        content()
    }
}

@Composable
private fun StateBox(content: @Composable () -> Unit) {
    Box(
        Modifier
            .fillMaxWidth()
            .height(300.dp)
            .clip(RoundedCornerShape(LsRadius.card))
            .background(LsColors.surface.copy(alpha = 0.4f)),
    ) { content() }
}

private val SAMPLE_COVERS = listOf(
    "Project Hail Mary" to "https://secure.syndetics.com/index.aspx?isbn=9781804953693/lc.gif&client=citycanning&upc=&oclc=&issn=",
    "The Thursday Murder Club" to "https://secure.syndetics.com/index.aspx?isbn=9780241425459/lc.gif&client=citycanning&upc=&oclc=&issn=",
    "Lessons in chemistry" to "https://secure.syndetics.com/index.aspx?isbn=9780857528131/lc.gif&client=citycanning&upc=&oclc=&issn=",
    "Where the crawdads sing" to "https://secure.syndetics.com/index.aspx?isbn=9781472157362/lc.gif&client=citycanning&upc=&oclc=&issn=",
    "A placeholder cover for a long title" to null,
)

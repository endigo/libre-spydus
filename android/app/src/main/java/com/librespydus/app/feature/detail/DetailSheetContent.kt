package com.librespydus.app.feature.detail

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Formatting
import com.librespydus.app.feature.search.BrowseBottomClearance
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.trimmedOrNull
import com.librespydus.app.ui.components.Chip
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.components.ShelfCover
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import uniffi.librespydus_core.RecordDetail

/**
 * The white sheet under the detail header: synopsis, subject chips, details and the related
 * carousels, with loading / empty / error states.
 */
@Composable
fun DetailSheetContent(
    state: DetailState,
    onRetry: () -> Unit,
    /** Called when the carousels come into view (they load lazily). */
    onStartRelated: () -> Unit,
    onOpenBook: (BookRef) -> Unit,
    onOpenSubject: (SubjectChip) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier.padding(bottom = BrowseBottomClearance),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
    ) {
        when (val record = state.record) {
            BrowseLoadable.Idle, BrowseLoadable.Loading -> SheetLoading("Loading details…")
            is BrowseLoadable.Failed -> ErrorStateView(
                error = record.error,
                onRetry = onRetry.takeIf { record.error.isRetryable },
                onSheet = true,
                modifier = Modifier.padding(horizontal = LsSpacing.xl),
            )
            is BrowseLoadable.Loaded -> LoadedSheet(record.value, state, onStartRelated, onOpenBook, onOpenSubject)
        }
    }
}

@Composable
private fun LoadedSheet(
    record: RecordDetail,
    state: DetailState,
    onStartRelated: () -> Unit,
    onOpenBook: (BookRef) -> Unit,
    onOpenSubject: (SubjectChip) -> Unit,
) {
    val summary = record.summary.trimmedOrNull()
    val subjects = state.subjects
    val facts = state.facts
    val hasRelated = state.hasRelatedPanels
    val gutter = Modifier.padding(horizontal = LsSpacing.xl)
    if (summary != null) {
        Column(gutter, verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
            SectionHeader("Synopsis", color = LsColors.onSheet)
            ExpandableText(summary)
        }
    }
    if (subjects.isNotEmpty()) {
        FlowRow(
            gutter.semantics { contentDescription = "Subjects" },
            horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
        ) {
            subjects.forEach { subject ->
                Chip(
                    text = subject.label,
                    onSheet = true,
                    onClick = { onOpenSubject(subject) },
                    modifier = Modifier.widthIn(max = 280.dp),
                )
            }
        }
    }
    if (facts.isNotEmpty()) DetailFactsList(facts, gutter)
    if (hasRelated) {
        // Starts the related loads when the carousels come into view (again for an upgraded record).
        val windowHeight = LocalWindowInfo.current.containerSize.height
        val preload = with(LocalDensity.current) { 200.dp.toPx() }
        Box(
            Modifier
                .fillMaxWidth()
                .height(1.dp)
                .onGloballyPositioned { if (it.positionInWindow().y < windowHeight + preload) onStartRelated() },
        )
        state.visiblePanels.forEach { panel ->
            RelatedCarousel(panel, state.related[panel] ?: BrowseLoadable.Loading, onOpenBook)
        }
    }
    if (summary == null && subjects.isEmpty() && facts.isEmpty() && !hasRelated) {
        Text(
            "The library has no more details for this title.",
            style = LsType.body,
            color = LsColors.onSheetSecondary,
            modifier = gutter,
        )
    }
}

/** Text clamped to [lineLimit] lines with a "More" / "Less" toggle when it doesn't fit. */
@Composable
private fun ExpandableText(text: String, lineLimit: Int = 5) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    var overflows by remember(text) { mutableStateOf(false) }
    Column(Modifier.animateContentSize(), verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
        Text(
            text,
            style = LsType.body,
            color = LsColors.onSheetSecondary,
            maxLines = if (expanded) Int.MAX_VALUE else lineLimit,
            overflow = TextOverflow.Ellipsis,
            onTextLayout = { if (!expanded) overflows = it.hasVisualOverflow },
        )
        if (expanded || overflows) {
            Text(
                if (expanded) "Less" else "More",
                style = LsType.bodyStrong,
                color = LsColors.primary,
                modifier = Modifier
                    .clip(RoundedCornerShape(LsRadius.pill))
                    .clickable(role = Role.Button) { expanded = !expanded }
                    .semantics { contentDescription = if (expanded) "Show less of the synopsis" else "Show more of the synopsis" }
                    .padding(vertical = LsSpacing.xs),
            )
        }
    }
}

@Composable
private fun DetailFactsList(facts: List<DetailFact>, modifier: Modifier) {
    val stacked = LocalDensity.current.fontScale >= 1.3f
    Column(modifier, verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
        SectionHeader("Details", color = LsColors.onSheet)
        Column {
            facts.forEachIndexed { index, fact ->
                val rowModifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = LsSpacing.s)
                    .semantics(mergeDescendants = true) {}
                val label: @Composable (Modifier) -> Unit = { m ->
                    Text(fact.label, style = LsType.caption, color = LsColors.onSheetSecondary, modifier = m.padding(top = 2.dp))
                }
                val value: @Composable (Modifier) -> Unit = { m ->
                    Text(fact.value, style = LsType.body, color = LsColors.onSheet, maxLines = 5, overflow = TextOverflow.Ellipsis, modifier = m)
                }
                if (stacked) {
                    Column(rowModifier, verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                        label(Modifier)
                        value(Modifier)
                    }
                } else {
                    Row(rowModifier, horizontalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
                        label(Modifier.width(104.dp))
                        value(Modifier.weight(1f))
                    }
                }
                if (index < facts.lastIndex) {
                    Box(
                        Modifier
                            .fillMaxWidth()
                            .height(1.dp)
                            .background(LsColors.chipOnSheet),
                    )
                }
            }
        }
    }
}

/** One related panel: "Similar books", "More by this author", "In this series", "Other editions". */
@Composable
private fun RelatedCarousel(panel: RelatedPanel, state: BrowseLoadable<List<BookRef>>, onOpenBook: (BookRef) -> Unit) {
    val coverWidth = LsCover.shelfWidth
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        SectionHeader(panel.title, color = LsColors.onSheet, modifier = Modifier.padding(horizontal = LsSpacing.xl))
        val refs = (state as? BrowseLoadable.Loaded)?.value
        LazyRow(
            contentPadding = PaddingValues(horizontal = LsSpacing.xl, vertical = LsSpacing.s),
            horizontalArrangement = Arrangement.spacedBy(LsSpacing.l),
            modifier = Modifier.semantics { contentDescription = if (refs == null) "Loading ${panel.title.lowercase()}" else panel.title },
        ) {
            if (refs != null) {
                items(refs, key = { it.id }) { ref ->
                    ShelfCover(
                        title = DetailText.displayTitle(ref.title),
                        coverUrl = ref.coverUrl,
                        width = coverWidth,
                        caption = Formatting.displayAuthor(ref.author) ?: ref.year,
                        captionColor = LsColors.onSheetSecondary,
                        showTitle = true,
                        titleColor = LsColors.onSheet,
                        onClick = { onOpenBook(ref) },
                    )
                }
            } else {
                items(4) {
                    Box(
                        Modifier
                            .width(coverWidth)
                            .aspectRatio(LsCover.ASPECT)
                            .clip(RoundedCornerShape(LsRadius.cover))
                            .background(LsColors.chipOnSheet)
                            .clearAndSetSemantics {},
                    )
                }
            }
        }
    }
}

/** A spinner with a message, in sheet colours. */
@Composable
private fun SheetLoading(message: String) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(vertical = LsSpacing.xxl)
            .semantics(mergeDescendants = true) {},
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CircularProgressIndicator(color = LsColors.onSheetSecondary, strokeWidth = 2.dp, modifier = Modifier.size(20.dp))
        Text(message, style = LsType.body, color = LsColors.onSheetSecondary, modifier = Modifier.padding(start = LsSpacing.m))
    }
}

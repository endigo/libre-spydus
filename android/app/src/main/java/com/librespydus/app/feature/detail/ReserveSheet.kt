package com.librespydus.app.feature.detail

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.BottomSheetDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.Copy
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.Formatting
import com.librespydus.app.feature.search.BrowseIcons
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.featured.EditionLevel
import com.librespydus.app.feature.search.displayLabel
import com.librespydus.app.feature.search.trimmedOrNull
import com.librespydus.app.ui.components.Chip
import com.librespydus.app.ui.components.CircleIconButton
import com.librespydus.app.ui.components.CoverImage
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.launch
import uniffi.librespydus_core.ReserveOptions

/**
 * The reserve flow for one format (§6): pickup location, availability, expiry choice → Confirm →
 * result banner. [model] does the work; the detail screen runs its calls.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReserveSheet(
    request: ReserveRequest,
    model: ReserveModel,
    onConfirm: () -> Unit,
    onRetryOptions: () -> Unit,
    onDismiss: () -> Unit,
) {
    val state by model.state.collectAsStateWithLifecycle()
    val sheetState = rememberModalBottomSheetState(
        skipPartiallyExpanded = true,
        confirmValueChange = { it != SheetValue.Hidden || !model.state.value.isSubmitting },
    )
    val scope = rememberCoroutineScope()
    val close: () -> Unit = {
        scope.launch { sheetState.hide() }.invokeOnCompletion { onDismiss() }
    }
    ModalBottomSheet(
        onDismissRequest = { if (!model.state.value.isSubmitting) onDismiss() },
        sheetState = sheetState,
        containerColor = LsColors.background,
        contentColor = LsColors.textPrimary,
        dragHandle = { BottomSheetDefaults.DragHandle(color = LsColors.textMuted) },
    ) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding(),
        ) {
            Box(
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = LsSpacing.l),
            ) {
                CircleIconButton(
                    LsIcons.Close, Copy.Common.close, { if (!state.isSubmitting) close() },
                    size = 44.dp, iconSize = 22.dp, background = LsColors.surface, contentColor = LsColors.textPrimary, shadow = false,
                    modifier = Modifier
                        .align(Alignment.CenterStart)
                        .alpha(if (state.isSubmitting) 0.5f else 1f),
                )
                Text(
                    Copy.Reserve.title,
                    style = LsType.section,
                    color = LsColors.textPrimary,
                    modifier = Modifier
                        .align(Alignment.Center)
                        .semantics { heading() },
                )
            }
            Column(
                Modifier
                    .weight(1f, fill = false)
                    .verticalScroll(rememberScrollState())
                    .padding(LsSpacing.xl),
                verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
            ) {
                BookSummary(request)
                when (val options = state.options) {
                    BrowseLoadable.Idle, BrowseLoadable.Loading -> Box(Modifier.fillMaxWidth().heightIn(min = 160.dp)) {
                        LoadingView(message = Copy.Reserve.checkingOptions)
                    }
                    is BrowseLoadable.Failed -> ErrorStateView(
                        error = options.error,
                        onRetry = onRetryOptions.takeIf { options.error.isRetryable },
                        modifier = Modifier.heightIn(min = 200.dp),
                    )
                    is BrowseLoadable.Loaded -> if (options.value.canReserve) {
                        ReserveForm(options.value, state, model::selectDays)
                    } else {
                        Banner(options.value.message.trimmedOrNull() ?: Copy.Reserve.cantReserve, BannerTone.INFO)
                    }
                }
            }
            val options = (state.options as? BrowseLoadable.Loaded)?.value
            if (options != null) {
                Column(
                    Modifier
                        .fillMaxWidth()
                        .animateContentSize()
                        .padding(start = LsSpacing.xl, end = LsSpacing.xl, top = LsSpacing.m, bottom = LsSpacing.l),
                    verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
                ) {
                    state.result?.let { Banner(ReserveModel.message(it), if (it.ok) BannerTone.SUCCESS else BannerTone.ERROR) }
                    state.submitError?.let { Banner(it.message, BannerTone.ERROR) }
                    if (!options.canReserve || state.isComplete || state.result?.ok == false) {
                        PrimaryPillButton(if (state.isComplete) Copy.Common.done else Copy.Common.close, onClick = close, modifier = Modifier.fillMaxWidth())
                    } else {
                        PrimaryPillButton(Copy.Reserve.confirm, onClick = onConfirm, loading = state.isSubmitting, modifier = Modifier.fillMaxWidth())
                    }
                }
            }
        }
    }
}

@Composable
private fun BookSummary(request: ReserveRequest) {
    Row(
        Modifier.semantics(mergeDescendants = true) {},
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.l),
    ) {
        CoverImage(url = request.book.coverUrl, title = request.book.title, width = LsCover.smallWidth)
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
            Text(DetailText.displayTitle(request.book.title), style = LsType.bodyStrong, color = LsColors.textPrimary, maxLines = 3, overflow = TextOverflow.Ellipsis)
            Formatting.byLine(request.book.author)?.let {
                Text(it, style = LsType.caption, color = LsColors.textSecondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            // An edition-level item's one format has no label: no chip (§5.10).
            if (!EditionLevel.isEditionLevel(request.format)) {
                Chip(
                    request.format.displayLabel,
                    tint = if (CoreHelpers.isDigitalFormat(request.format)) LsColors.digital else null,
                    modifier = Modifier.padding(top = LsSpacing.xs),
                )
            }
        }
    }
}

@Composable
private fun ReserveForm(options: ReserveOptions, state: ReserveModel.State, onSelectDays: (String) -> Unit) {
    Column(
        Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(LsRadius.card))
            .background(LsColors.surface)
            .padding(horizontal = LsSpacing.l),
    ) {
        InfoRow(Copy.Reserve.collectFrom, options.pickupLocation.trimmedOrNull()?.let(ReserveModel::titleCased) ?: Copy.Reserve.usualBranch, BrowseIcons.LocationOn)
        options.availableSummary.trimmedOrNull()?.let {
            FormDivider()
            InfoRow(Copy.Reserve.availability, it, BrowseIcons.LibraryBooks)
        }
        if (options.expiryChoices.isNotEmpty()) {
            FormDivider()
            var open by remember { mutableStateOf(false) }
            val enabled = !state.isSubmitting && !state.isComplete
            val selected = options.expiryChoices.firstOrNull { it.days == state.selectedDays } ?: options.expiryChoices.first()
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(vertical = LsSpacing.m),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
            ) {
                Icon(BrowseIcons.CalendarMonth, contentDescription = null, tint = LsColors.textSecondary, modifier = Modifier.size(22.dp))
                Text(Copy.Reserve.notNeededAfter, style = LsType.body, color = LsColors.textSecondary, modifier = Modifier.weight(1f))
                Box {
                    Row(
                        Modifier
                            .clip(RoundedCornerShape(LsRadius.pill))
                            .clickable(enabled = enabled, role = Role.DropdownList, onClickLabel = Copy.Reserve.chooseNotNeededAfter) { open = true }
                            .alpha(if (enabled) 1f else 0.5f)
                            .padding(horizontal = LsSpacing.s, vertical = LsSpacing.xs),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(selected.label, style = LsType.body, color = LsColors.textPrimary)
                        Icon(LsIcons.ExpandMore, contentDescription = null, tint = LsColors.textPrimary, modifier = Modifier.size(20.dp))
                    }
                    DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                        options.expiryChoices.forEach { choice ->
                            DropdownMenuItem(
                                text = { Text(choice.label, style = LsType.body, color = LsColors.textPrimary) },
                                trailingIcon = if (choice.days == selected.days) {
                                    { Icon(LsIcons.Check, contentDescription = Copy.Common.selected, tint = LsColors.primary) }
                                } else {
                                    null
                                },
                                onClick = {
                                    open = false
                                    onSelectDays(choice.days)
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun InfoRow(label: String, value: String, icon: ImageVector) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(vertical = LsSpacing.m)
            .semantics(mergeDescendants = true) {},
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        Icon(icon, contentDescription = null, tint = LsColors.textSecondary, modifier = Modifier.padding(top = 2.dp).size(22.dp))
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(label, style = LsType.caption, color = LsColors.textSecondary)
            Text(value, style = LsType.body, color = LsColors.textPrimary)
        }
    }
}

@Composable
private fun FormDivider() {
    Box(
        Modifier
            .fillMaxWidth()
            .padding(start = 22.dp + LsSpacing.m)
            .heightIn(min = 1.dp, max = 1.dp)
            .background(LsColors.divider),
    )
}

private enum class BannerTone(val color: Color, val icon: ImageVector) {
    SUCCESS(LsColors.success, BrowseIcons.CheckCircle),
    ERROR(LsColors.danger, LsIcons.Error),
    INFO(LsColors.textSecondary, BrowseIcons.HelpOutline),
}

/** The reservation's outcome (or why it can't be made). */
@Composable
private fun Banner(message: String, tone: BannerTone) {
    val shape = RoundedCornerShape(LsRadius.card)
    Row(
        Modifier
            .fillMaxWidth()
            .clip(shape)
            .background(tone.color.copy(alpha = 0.14f))
            .border(1.dp, tone.color.copy(alpha = 0.45f), shape)
            .padding(LsSpacing.l)
            .semantics(mergeDescendants = true) {},
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        Icon(tone.icon, contentDescription = null, tint = tone.color, modifier = Modifier.size(22.dp).width(22.dp))
        Text(message, style = LsType.body, color = LsColors.textPrimary)
    }
}

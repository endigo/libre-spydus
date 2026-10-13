package com.librespydus.app.feature.borrowed

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDefaults
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SelectableDates
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Formatting
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.onboarding.AccountBanner
import com.librespydus.app.feature.onboarding.AccountBannerTone
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.ui.components.CoverImage
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.librespydus_core.EditOptions
import uniffi.librespydus_core.PickupChoice
import uniffi.librespydus_core.SpydusException
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.util.Locale

private sealed interface OptionsLoad {
    data object Loading : OptionsLoad
    data class Loaded(val options: EditOptions) : OptionsLoad
    data class Failed(val error: UserFacingError) : OptionsLoad
}

/**
 * "Edit reservation": loads the form (`editOptions`), lets the user change the pickup branch and
 * the two dates, then sends `editReservation`. A refusal (`ok = false`) stays in the sheet;
 * success closes it and the Borrowed list shows the message. [onFinish] runs once.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EditReservationSheet(
    row: ReservationRow,
    appState: AppState,
    /** Runs the save, so a rotation or a closing sheet doesn't cut it short (the ViewModel's scope). */
    workScope: CoroutineScope,
    onFinish: (EditOutcome) -> Unit,
) {
    var options by remember(row.id) { mutableStateOf<OptionsLoad>(OptionsLoad.Loading) }
    var draft by remember(row.id) { mutableStateOf<EditDraft?>(null) }
    var saving by remember { mutableStateOf(false) }
    var problem by remember { mutableStateOf<String?>(null) }
    var finished by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val currentSaving by rememberUpdatedState(saving)
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true, confirmValueChange = { !currentSaving })

    fun finish(outcome: EditOutcome) {
        if (finished) return
        finished = true
        scope.launch {
            // Delivered even if the hide animation is interrupted (a drag) or the sheet goes away.
            try {
                sheetState.hide()
            } finally {
                onFinish(outcome)
            }
        }
    }

    suspend fun save() {
        val href = row.editHref ?: return
        val current = draft ?: return
        if (saving) return
        val session = appState.session.value ?: return finish(EditOutcome.Failed(SpydusException.NotLoggedIn("not logged in")))
        val backend = appState.backend.value ?: return
        saving = true
        problem = null
        try {
            val result = backend.editReservation(href, session, current.overrides())
            saving = false // lets the sheet hide (confirmValueChange)
            if (result.ok) finish(EditOutcome.Saved(result))
            else problem = result.message?.trim()?.takeIf { it.isNotEmpty() } ?: Copy.EditReservation.changeFailed
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            saving = false
            if (BorrowedModel.needsRootSheet(e)) finish(EditOutcome.Failed(e)) else problem = appState.report(e).message
        } finally {
            saving = false
        }
    }

    suspend fun loadOptions() {
        val href = row.editHref ?: return finish(EditOutcome.Closed)
        val session = appState.session.value ?: return finish(EditOutcome.Failed(SpydusException.NotLoggedIn("not logged in")))
        val backend = appState.backend.value ?: return finish(EditOutcome.Closed)
        options = OptionsLoad.Loading
        problem = null
        try {
            val loaded = backend.editOptions(href, session)
            draft = EditDraft.from(loaded, currentPickup = row.item.fields["Pickup location"])
            options = OptionsLoad.Loaded(loaded)
            if (BorrowedDebugHooks.autoSavesEdit && backend.isDemo && BorrowedDebugHooks.consumeOnce("edit-saved")) {
                draft = draft?.let { d -> d.copy(pickup = loaded.pickupChoices.firstOrNull { it.code != d.pickup }?.code ?: d.pickup) }
                delay(1000)
                save()
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (BorrowedModel.needsRootSheet(e)) finish(EditOutcome.Failed(e)) else options = OptionsLoad.Failed(appState.report(e))
        }
    }

    LaunchedEffect(row.id) { loadOptions() }

    ModalBottomSheet(
        onDismissRequest = { if (!finished) onFinish(EditOutcome.Closed).also { finished = true } },
        sheetState = sheetState,
        containerColor = LsColors.background,
        contentColor = LsColors.textPrimary,
    ) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding(),
        ) {
            Box(Modifier.fillMaxWidth().padding(horizontal = LsSpacing.l, vertical = LsSpacing.xs)) {
                Text(
                    Copy.Common.close,
                    style = LsType.bodyStrong,
                    color = if (saving) LsColors.textMuted else LsColors.primary,
                    modifier = Modifier
                        .align(Alignment.CenterStart)
                        .clip(RoundedCornerShape(LsRadius.pill))
                        .clickable(enabled = !saving, role = Role.Button) { finish(EditOutcome.Closed) }
                        .padding(horizontal = LsSpacing.s, vertical = LsSpacing.s),
                )
                Text(
                    Copy.EditReservation.title,
                    style = LsType.section,
                    color = LsColors.textPrimary,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.align(Alignment.Center).semantics { heading() },
                )
            }
            when (val state = options) {
                OptionsLoad.Loading -> LoadingView(message = Copy.EditReservation.loading, modifier = Modifier.heightIn(min = 320.dp))
                is OptionsLoad.Failed -> ErrorStateView(
                    state.error,
                    onRetry = { scope.launch { loadOptions() } },
                    modifier = Modifier.heightIn(min = 320.dp),
                )
                is OptionsLoad.Loaded -> {
                    val current = draft
                    if (state.options.canEdit && current != null) {
                        EditForm(
                            row = row,
                            options = state.options,
                            draft = current,
                            onDraft = { draft = it },
                            problem = problem,
                            saving = saving,
                            onSave = { workScope.launch { save() } },
                        )
                    } else {
                        EmptyStateView(
                            title = Copy.EditReservation.cantEditTitle,
                            message = state.options.message?.trim()?.takeIf { it.isNotEmpty() }
                                ?: Copy.EditReservation.cantEditMessage,
                            icon = AccountIcons.Event,
                            modifier = Modifier.heightIn(min = 320.dp),
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun EditForm(
    row: ReservationRow,
    options: EditOptions,
    draft: EditDraft,
    onDraft: (EditDraft) -> Unit,
    problem: String?,
    saving: Boolean,
    onSave: () -> Unit,
) {
    val today = remember { LocalDate.now() }
    Column(
        Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = LsSpacing.l)
            .padding(top = LsSpacing.m, bottom = LsSpacing.xl),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
    ) {
        BookHeader(row)
        if (problem != null) AccountBanner(problem, tone = AccountBannerTone.ERROR)
        if (options.pickupChoices.isNotEmpty()) {
            PickupDropdown(
                choices = options.pickupChoices,
                selected = draft.pickup,
                enabled = !saving,
                onSelect = { onDraft(draft.copy(pickup = it)) },
            )
        }
        DateRow(
            title = Copy.EditReservation.delayUntil,
            field = draft.delay,
            earliest = today,
            footer = Copy.EditReservation.delayFooter,
            enabled = !saving,
            onChange = { onDraft(draft.copy(delay = it)) },
        )
        DateRow(
            title = Copy.EditReservation.notNeededAfter,
            field = draft.expiry,
            // "Not needed after" can't be before "Delay until".
            earliest = if (draft.delay.isOn) maxOf(today, draft.delay.date) else today,
            footer = Copy.EditReservation.notNeededFooter,
            enabled = !saving,
            onChange = { onDraft(draft.copy(expiry = it)) },
        )
        PrimaryPillButton(
            Copy.EditReservation.saveChanges,
            onClick = onSave,
            enabled = draft.hasChanges && !saving,
            loading = saving,
            modifier = Modifier.fillMaxWidth().padding(top = LsSpacing.s),
        )
    }
}

@Composable
private fun BookHeader(row: ReservationRow) {
    Row(
        Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(LsRadius.card))
            .background(LsColors.surface)
            .padding(LsSpacing.m)
            .semantics(mergeDescendants = true) {},
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CoverImage(url = row.ref.coverUrl, title = row.ref.title, width = 44.dp, shadow = false)
        Spacer(Modifier.width(LsSpacing.m))
        Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
            Text(row.ref.title, style = LsType.bodyStrong, color = LsColors.textPrimary, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Formatting.byLine(row.ref.author)?.let {
                Text(it, style = LsType.caption, color = LsColors.textSecondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Text(row.statusLabel, style = LsType.caption, color = if (row.isReady) LsColors.success else LsColors.textMuted)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PickupDropdown(choices: List<PickupChoice>, selected: String?, enabled: Boolean, onSelect: (String) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    val selectedName = choices.firstOrNull { it.code == selected }?.let(::branchName) ?: Copy.EditReservation.chooseBranch
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
        Text(Copy.EditReservation.collectFrom, style = LsType.caption, color = LsColors.textSecondary)
        ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { if (enabled) expanded = it }) {
            Row(
                Modifier
                    .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable, enabled)
                    .fillMaxWidth()
                    .heightIn(min = 52.dp)
                    .clip(RoundedCornerShape(LsRadius.field))
                    .background(LsColors.surface)
                    .semantics {
                        contentDescription = Copy.EditReservation.collectFrom
                        stateDescription = selectedName
                    }
                    .padding(horizontal = LsSpacing.l),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(AccountIcons.Place, contentDescription = null, tint = LsColors.textSecondary, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(LsSpacing.m))
                Text(selectedName, style = LsType.body, color = LsColors.textPrimary, modifier = Modifier.weight(1f))
                ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded)
            }
            ExposedDropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false },
                containerColor = LsColors.surfaceElevated,
            ) {
                choices.forEach { choice ->
                    DropdownMenuItem(
                        text = { Text(branchName(choice), style = LsType.body, color = LsColors.textPrimary) },
                        trailingIcon = if (choice.code == selected) {
                            { Icon(LsIcons.Check, contentDescription = Copy.Common.selected, tint = LsColors.primary) }
                        } else null,
                        onClick = {
                            onSelect(choice.code)
                            expanded = false
                        },
                    )
                }
            }
        }
    }
}

private fun branchName(choice: PickupChoice): String = ReservationRow.displayName(choice.name) ?: choice.name

/**
 * A date the form can set or clear: the date (opens a date picker) and a clear button, or "Set
 * date" when it's off. A date the form already had stays pickable even when it's before [earliest].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DateRow(
    title: String,
    field: EditDateField,
    earliest: LocalDate,
    footer: String,
    enabled: Boolean,
    onChange: (EditDateField) -> Unit,
) {
    var picking by remember { mutableStateOf(false) }
    // Mid-sentence in the screen-reader labels: "Change delay until date", "Clear not needed after".
    val fieldName = title.lowercase()
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
        Row(
            Modifier
                .fillMaxWidth()
                .heightIn(min = 56.dp)
                .clip(RoundedCornerShape(LsRadius.card))
                .background(LsColors.surface)
                .padding(start = LsSpacing.l, end = LsSpacing.s),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(title, style = LsType.body, color = LsColors.textPrimary, modifier = Modifier.weight(1f))
            if (field.isOn) {
                val date = editDate(field.date)
                Text(
                    date,
                    style = LsType.bodyStrong,
                    color = LsColors.textPrimary,
                    modifier = Modifier
                        .clip(RoundedCornerShape(LsRadius.pill))
                        .background(LsColors.surfaceElevated)
                        .clickable(enabled = enabled, role = Role.Button, onClickLabel = Copy.EditReservation.changeDate(fieldName)) { picking = true }
                        .semantics {
                            contentDescription = Copy.EditReservation.dateLabel(title)
                            stateDescription = date
                        }
                        .padding(horizontal = LsSpacing.m, vertical = LsSpacing.s),
                )
                Box(
                    Modifier
                        .size(44.dp)
                        .clip(RoundedCornerShape(LsRadius.pill))
                        .clickable(enabled = enabled, role = Role.Button) { onChange(field.copy(isOn = false)) }
                        .semantics { contentDescription = Copy.EditReservation.clearField(fieldName) },
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(LsIcons.Close, contentDescription = null, tint = LsColors.textSecondary, modifier = Modifier.size(20.dp))
                }
            } else {
                Text(
                    Copy.EditReservation.setDate,
                    style = LsType.bodyStrong,
                    color = LsColors.primary,
                    modifier = Modifier
                        .clip(RoundedCornerShape(LsRadius.pill))
                        .clickable(enabled = enabled, role = Role.Button, onClickLabel = Copy.EditReservation.setFieldDate(fieldName)) { picking = true }
                        .padding(horizontal = LsSpacing.m, vertical = LsSpacing.s),
                )
            }
        }
        Text(
            if (field.isCleared) Copy.EditReservation.clearedFooter(footer) else footer,
            style = LsType.caption,
            color = LsColors.textMuted,
            modifier = Modifier.padding(horizontal = LsSpacing.s),
        )
    }
    if (picking) {
        val minimum = minOf(earliest, field.date)
        val state = rememberDatePickerState(
            initialSelectedDateMillis = utcMillis(maxOf(field.date, minimum)),
            selectableDates = object : SelectableDates {
                override fun isSelectableDate(utcTimeMillis: Long) = localDate(utcTimeMillis) >= minimum
                override fun isSelectableYear(year: Int) = year >= minimum.year
            },
        )
        val colors = DatePickerDefaults.colors(
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textSecondary,
            headlineContentColor = LsColors.textPrimary,
            weekdayContentColor = LsColors.textSecondary,
            subheadContentColor = LsColors.textSecondary,
            navigationContentColor = LsColors.textPrimary,
            yearContentColor = LsColors.textPrimary,
            currentYearContentColor = LsColors.primary,
            dayContentColor = LsColors.textPrimary,
            disabledDayContentColor = LsColors.textMuted,
            selectedDayContainerColor = LsColors.primary,
            selectedDayContentColor = LsColors.onPrimary,
            todayContentColor = LsColors.primary,
            todayDateBorderColor = LsColors.primary,
            selectedYearContainerColor = LsColors.primary,
            selectedYearContentColor = LsColors.onPrimary,
            dividerColor = LsColors.divider,
        )
        DatePickerDialog(
            onDismissRequest = { picking = false },
            confirmButton = {
                TextButton(
                    onClick = {
                        state.selectedDateMillis?.let { onChange(field.copy(isOn = true, date = localDate(it))) }
                        picking = false
                    },
                    enabled = state.selectedDateMillis != null,
                ) { Text(Copy.Common.done, color = LsColors.primary) }
            },
            dismissButton = { TextButton(onClick = { picking = false }) { Text(Copy.Common.cancel, color = LsColors.textPrimary) } },
            colors = colors,
        ) {
            DatePicker(state = state, colors = colors, title = {
                Text(title, style = LsType.bodyStrong, modifier = Modifier.padding(start = 24.dp, top = 16.dp))
            })
        }
    }
}

/**
 * "1 Nov", with the year only when it isn't this year ("3 Jan 2027"; §7.1). The library's dates are
 * Gregorian whatever the device's calendar.
 */
private fun editDate(date: LocalDate, today: LocalDate = LocalDate.now()): String =
    (if (date.year == today.year) EDIT_DATE else EDIT_DATE_YEAR).format(date)

private val EDIT_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM", Locale.ENGLISH)
private val EDIT_DATE_YEAR: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM uuuu", Locale.ENGLISH)

/** The Material date picker works in UTC midnight millis. */
private fun utcMillis(date: LocalDate): Long = date.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli()

private fun localDate(utcMillis: Long): LocalDate = Instant.ofEpochMilli(utcMillis).atZone(ZoneOffset.UTC).toLocalDate()

package com.librespydus.app.feature.detail

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.Copy
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.Formatting
import com.librespydus.app.core.PickIrn
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.feature.search.BrowseIcons
import com.librespydus.app.feature.search.browseReporter
import com.librespydus.app.feature.search.displayLabel
import com.librespydus.app.feature.search.valueOrNull
import com.librespydus.app.ui.components.Chip
import com.librespydus.app.ui.components.CircleIconButton
import com.librespydus.app.ui.components.CoverImage
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.librespydus_core.SpydusException

/** Height of the Reserve row; half of it reaches into the white sheet. */
private val ActionsHeight = 52.dp

/**
 * Book detail for the `detail(BookRef)` route (§6): navy header with cover, title, availability,
 * format chips, "Reserve" and the favourite bookmark; a white sheet with the synopsis, subject
 * chips, details and related carousels; the reserve sheet (demo mode only for workers).
 */
@Composable
fun BookDetailScreen(book: BookRef, appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val model: BookDetailModel = viewModel { BookDetailModel(book) }
    val state by model.state.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val library by appState.library.collectAsStateWithLifecycle()
    val session by appState.session.collectAsStateWithLifecycle()
    val loginNeeded by appState.loginNeeded.collectAsStateWithLifecycle()
    val verification by appState.verificationGeneration.collectAsStateWithLifecycle()
    val favourites by appState.currentFavourites.collectAsStateWithLifecycle()
    val pickIrns by appState.picks.irns.collectAsStateWithLifecycle()
    val account by appState.account.snapshot.collectAsStateWithLifecycle()
    val report = remember(appState) { appState.browseReporter() }
    val scope = rememberCoroutineScope()

    var reserveRequest by remember { mutableStateOf<ReserveRequest?>(null) }
    /** "Reserve" tapped while signed out: reserve this format once signed in. */
    var pendingBorrow by remember { mutableStateOf<FormatRef?>(null) }
    /** A reserve failure that needs a root sheet (sign in again, verification), reported once the reserve sheet is gone. */
    var deferredError by remember { mutableStateOf<Throwable?>(null) }

    val currentBackend = backend
    LaunchedEffect(verification, currentBackend) {
        val b = currentBackend ?: return@LaunchedEffect
        model.viewModelScope.launch {
            model.load(b, report)
            model.retryFailedRelated(b)
        }
    }
    val availabilityUrl = state.selectedFormat?.availabilityUrl
    LaunchedEffect(availabilityUrl, verification, state.formatLookup) {
        val b = currentBackend ?: return@LaunchedEffect
        model.viewModelScope.launch { model.loadAvailability(b, report) }
    }
    // An edition-level item's "Copies" list, once its availability names the holdings (§5.10).
    val holdingsUrl = state.holdingsUrl
    LaunchedEffect(holdingsUrl, verification) {
        val b = currentBackend ?: return@LaunchedEffect
        if (holdingsUrl != null) model.viewModelScope.launch { model.loadCopies(b, report) }
    }
    LaunchedEffect(session) {
        val format = pendingBorrow ?: return@LaunchedEffect
        if (session == null) return@LaunchedEffect
        pendingBorrow = null
        // Let the login sheet finish closing before this screen opens its own.
        delay(650)
        if (appState.session.value != null && !appState.loginNeeded.value) reserveRequest = ReserveRequest(format, model.state.value.ref)
    }
    LaunchedEffect(loginNeeded) {
        // The login sheet was closed without signing in: forget the borrow.
        if (!loginNeeded && appState.session.value == null) pendingBorrow = null
    }
    LaunchedEffect(reserveRequest, deferredError) {
        val error = deferredError ?: return@LaunchedEffect
        if (reserveRequest != null) return@LaunchedEffect
        delay(300)
        // Report before clearing: clearing changes this effect's key and cancels it.
        appState.report(error)
        deferredError = null
    }

    fun borrow(format: FormatRef) {
        if (appState.session.value != null) {
            reserveRequest = ReserveRequest(format, state.ref)
        } else {
            pendingBorrow = format
            appState.requestLogin()
        }
    }

    // Signed in, the bookmark is My Picks on the library account (§5.11); signed out, My Picks on this phone.
    val pickIrn = PickIrn.of(state.ref, state.record.valueOrNull)
    val bookmark = if (session != null) DetailBookmark.myPicks(pickIrn, pickIrns) else DetailBookmark.local(favourites.any { it.id == state.ref.id })
    fun toggleBookmark() {
        if (appState.session.value != null) pickIrn?.let(appState.picks::toggle)
        else scope.launch { appState.toggleFavourite(model.state.value.favouriteRef) }
    }
    // The format on screen is already on loan or reserved: a status pill instead of Reserve.
    val heldIrns = state.heldIrns
    val held = remember(account, session, heldIrns) {
        HeldStatus.holding(account.takeIf { session != null }, heldIrns)?.let { HeldStatus.of(it) }
    }
    // Signed in with loans or reservations, and the edition on screen isn't identified yet.
    val awaitingHolding = session != null && state.isIdentifyingEdition && account?.let { snapshot ->
        snapshot.loans.items.isNotEmpty() || snapshot.reservations.ready.items.isNotEmpty() ||
            snapshot.reservations.pending.items.isNotEmpty()
    } == true

    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background),
    ) {
        // Fixed navy bar: scrolled content passes under it, never under the status bar.
        Row(
            Modifier
                .fillMaxWidth()
                .background(LsColors.background)
                .statusBarsPadding()
                .padding(horizontal = LsSpacing.l, vertical = LsSpacing.s),
        ) {
            CircleIconButton(
                LsIcons.ArrowBack, Copy.Common.back, navigator::back,
                size = 44.dp, iconSize = 24.dp, background = LsColors.surface, contentColor = LsColors.textPrimary, shadow = false,
            )
        }
        Column(
            Modifier
                .weight(1f)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState()),
        ) {
            DetailHeader(
                state = state,
                onSelectFormat = model::select,
                onRetryAvailability = { currentBackend?.let { b -> model.viewModelScope.launch { model.loadAvailability(b, report) } } },
                onRetryLookup = { currentBackend?.let { b -> model.viewModelScope.launch { model.refindFormats(b, report) } } },
            )
            Box(Modifier.fillMaxWidth()) {
                DetailSheetContent(
                    state = state,
                    onRetry = { currentBackend?.let { b -> model.viewModelScope.launch { model.load(b, report) } } },
                    onStartRelated = { currentBackend?.let(model::startRelated) },
                    onOpenBook = navigator::openBook,
                    onOpenSubject = { navigator.openResults(it.query, ResultsSort.RELEVANCE, it.label) },
                    onRetryCopies = { currentBackend?.let { b -> model.viewModelScope.launch { model.loadCopies(b, report) } } },
                    modifier = Modifier
                        .padding(top = ActionsHeight / 2)
                        .fillMaxWidth()
                        .sheetBackground()
                        .padding(top = ActionsHeight / 2 + LsSpacing.l),
                )
                DetailActions(
                    reservable = state.reservableFormat,
                    held = held,
                    awaitingHolding = awaitingHolding,
                    onOpenHeld = appState::openBorrowed,
                    signedIn = session != null,
                    bookmark = bookmark,
                    onBorrow = ::borrow,
                    onToggleBookmark = ::toggleBookmark,
                    modifier = Modifier.align(Alignment.TopCenter),
                )
            }
        }
    }

    reserveRequest?.let { request ->
        val reserveModel = remember(request) { ReserveModel() }
        val libraryName = library?.name
        val needsRootSheet: (Throwable) -> Unit = { error ->
            deferredError = error
            if (error is SpydusException.NotLoggedIn) pendingBorrow = request.format
            reserveRequest = null
        }
        fun loadOptions() {
            val b = currentBackend ?: return
            model.viewModelScope.launch { reserveModel.loadOptions(request.format.reserveUrl, b, appState.session.value, libraryName, needsRootSheet) }
        }
        LaunchedEffect(request) { loadOptions() }
        ReserveSheet(
            request = request,
            model = reserveModel,
            onConfirm = {
                val b = currentBackend
                if (b != null) {
                    model.viewModelScope.launch {
                        // The new hold shows up in Borrowed and Home without a manual refresh.
                        reserveModel.confirm(request.format.reserveUrl, b, appState.session.value, libraryName, needsRootSheet, onPlaced = appState::accountDidChange)
                    }
                }
            },
            onRetryOptions = ::loadOptions,
            onDismiss = { reserveRequest = null },
        )
    }
}

/** White sheet with rounded top corners, drawn past the bottom so overscroll shows white, not navy. */
private fun Modifier.sheetBackground(): Modifier = drawBehind {
    val radius = LsRadius.sheet.toPx()
    drawRoundRect(
        color = LsColors.sheet,
        size = Size(size.width, size.height + 4000f),
        cornerRadius = CornerRadius(radius, radius),
    )
}

/** The navy top: cover, title, byline, year, availability line and format chips. */
@Composable
private fun DetailHeader(
    state: DetailState,
    onSelectFormat: (Int) -> Unit,
    onRetryAvailability: () -> Unit,
    onRetryLookup: () -> Unit,
) {
    val ref = state.ref
    Column(
        Modifier
            .fillMaxWidth()
            .padding(start = LsSpacing.xl, end = LsSpacing.xl, top = LsSpacing.s, bottom = LsSpacing.xl),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(LsSpacing.l), verticalAlignment = Alignment.Top) {
            CoverImage(url = ref.coverUrl, title = ref.title, width = LsCover.detailWidth)
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                if (ref.title.isNotBlank()) {
                    Text(
                        DetailText.displayTitle(ref.title),
                        style = LsType.title,
                        color = LsColors.textPrimary,
                        maxLines = 6,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.semantics { heading() },
                    )
                }
                Formatting.byLine(ref.author)?.let {
                    Text(it, style = LsType.body, color = LsColors.textSecondary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
                ref.year?.let { Text(it, style = LsType.caption, color = LsColors.textMuted) }
                Box(Modifier.padding(top = LsSpacing.xs)) {
                    AvailabilityRow(state.headerAvailability, onRetryAvailability, onRetryLookup)
                }
            }
        }
        if (state.showsFormatChips) {
            FlowRow(
                Modifier.semantics { contentDescription = Copy.Detail.formats },
                horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
                verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
            ) {
                state.formats.forEachIndexed { index, format ->
                    val selected = index == state.selectedFormatIndex
                    Chip(
                        text = format.displayLabel,
                        selected = selected,
                        tint = if (CoreHelpers.isDigitalFormat(format)) LsColors.digital else null,
                        onClick = { onSelectFormat(index) },
                        // The format, then what a tap does: "Large print. Shows availability for this format".
                        modifier = Modifier.semantics {
                            this.selected = selected
                            contentDescription = Copy.A11y.labelAndHint(format.displayLabel, Copy.Detail.formatHint)
                        },
                    )
                }
            }
        }
    }
}

@Composable
private fun AvailabilityRow(availability: HeaderAvailability, onRetryAvailability: () -> Unit, onRetryLookup: () -> Unit) {
    when (availability) {
        is HeaderAvailability.Loading -> Row(
            Modifier.semantics(mergeDescendants = true) {},
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
        ) {
            CircularProgressIndicator(color = LsColors.textSecondary, strokeWidth = 2.dp, modifier = Modifier.size(14.dp))
            Text(availability.text, style = LsType.caption, color = LsColors.textSecondary)
        }
        is HeaderAvailability.Line -> {
            val line = availability.line
            val (icon, color) = when (line.tone) {
                AvailabilityLine.Tone.AVAILABLE -> BrowseIcons.CheckCircle to LsColors.success
                AvailabilityLine.Tone.NORMAL -> BrowseIcons.LibraryBooks to LsColors.textPrimary
                AvailabilityLine.Tone.DIGITAL -> BrowseIcons.Cloud to LsColors.digital
                AvailabilityLine.Tone.MUTED -> BrowseIcons.HelpOutline to LsColors.textSecondary
            }
            Row(
                Modifier.semantics(mergeDescendants = true) { contentDescription = Copy.Detail.availability(line.text) },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
            ) {
                Icon(icon, contentDescription = null, tint = color, modifier = Modifier.size(20.dp))
                Text(line.text, style = LsType.bodyStrong, color = color)
            }
        }
        HeaderAvailability.AvailabilityFailed -> RetryLine(Copy.Detail.availabilityFailed, Copy.Detail.retryAvailabilityHint, onRetryAvailability)
        HeaderAvailability.LookupFailed -> RetryLine(Copy.Detail.lookupFailed, Copy.Detail.retryLookupHint, onRetryLookup)
        HeaderAvailability.CheckWebsite -> Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
        ) {
            Icon(LsIcons.Globe, contentDescription = null, tint = LsColors.textSecondary, modifier = Modifier.size(16.dp))
            Text(Copy.Detail.checkWebsite, style = LsType.caption, color = LsColors.textSecondary)
        }
        HeaderAvailability.None -> Unit
    }
}

@Composable
private fun RetryLine(text: String, hint: String, onRetry: () -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(text, style = LsType.caption, color = LsColors.textSecondary)
        Spacer(Modifier.width(LsSpacing.xs))
        Text(
            Copy.Common.retry,
            style = LsType.caption.copy(fontWeight = LsType.bodyStrong.fontWeight),
            color = LsColors.primary,
            modifier = Modifier
                .clip(RoundedCornerShape(LsRadius.pill))
                .clickable(role = Role.Button, onClick = onRetry)
                .semantics { contentDescription = Copy.A11y.labelAndHint(Copy.Common.retry, hint) }
                .padding(horizontal = LsSpacing.s, vertical = LsSpacing.xs),
        )
    }
}

/**
 * "Reserve" (physical/reservable formats), or the [HeldStatusPill] in its place when the patron
 * already has the format, and the blue bookmark (My Picks on the library account signed in, on this
 * phone signed out).
 */
@Composable
private fun DetailActions(
    reservable: FormatRef?,
    held: HeldStatus?,
    /** Held or not isn't known yet: Reserve keeps its slot but shows nothing. */
    awaitingHolding: Boolean,
    onOpenHeld: (BorrowedSegment) -> Unit,
    signedIn: Boolean,
    bookmark: DetailBookmark,
    onBorrow: (FormatRef) -> Unit,
    onToggleBookmark: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier
            .fillMaxWidth()
            .padding(horizontal = LsSpacing.xl),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        if (held != null) {
            HeldStatusPill(held, onClick = { onOpenHeld(held.segment) }, modifier = Modifier.weight(1f))
        } else if (reservable != null) {
            // An edition-level item's format has no label (§5.10).
            val format = reservable.displayLabel.trim().lowercase().ifEmpty { null }
            val hint = when {
                signedIn -> format?.let(Copy.Detail::reserveFormatHint) ?: Copy.Detail.reserveItemHint
                else -> format?.let(Copy.Detail::signInToReserveFormatHint) ?: Copy.Detail.signInToReserveItemHint
            }
            val alpha by animateFloatAsState(if (awaitingHolding) 0f else 1f, label = "reserve")
            PrimaryPillButton(
                text = Copy.Detail.reserve,
                onClick = { if (!awaitingHolding) onBorrow(reservable) },
                modifier = Modifier
                    .weight(1f)
                    .alpha(alpha)
                    .then(
                        if (awaitingHolding) {
                            Modifier.clearAndSetSemantics { }
                        } else {
                            Modifier.semantics { contentDescription = Copy.A11y.labelAndHint(Copy.Detail.reserve, hint) }
                        },
                    ),
            )
        } else {
            Spacer(Modifier.weight(1f))
        }
        CircleIconButton(
            icon = if (bookmark.saved) LsIcons.BookmarkFilled else LsIcons.Bookmark,
            contentDescription = bookmark.label,
            onClick = { if (bookmark.enabled) onToggleBookmark() },
            size = ActionsHeight,
            modifier = Modifier
                .alpha(if (bookmark.enabled) 1f else 0.5f)
                .semantics {
                    selected = bookmark.saved
                    if (!bookmark.enabled) disabled()
                },
        )
    }
}

/**
 * "On loan · Due 14 Oct", "Ready to collect · Riverton Library", "Reserved · …": the size of the
 * Reserve pill it replaces, but a quiet surface with a tinted icon and text, so it reads as a
 * status rather than an action. Tapping it opens Your Books on that list.
 */
@Composable
private fun HeldStatusPill(status: HeldStatus, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val (icon, tint) = when (status.tone) {
        HeldStatus.Tone.LOAN -> LsIcons.BookFilled to LsColors.textPrimary
        HeldStatus.Tone.OVERDUE -> LsIcons.Error to LsColors.danger
        HeldStatus.Tone.READY -> BrowseIcons.CheckCircle to LsColors.success
        HeldStatus.Tone.RESERVED -> AccountIcons.Schedule to LsColors.warning
    }
    val shape = RoundedCornerShape(LsRadius.pill)
    Row(
        modifier
            .defaultMinSize(minWidth = 120.dp)
            .height(ActionsHeight)
            .clip(shape)
            .background(LsColors.surfaceElevated)
            .border(1.dp, tint.copy(alpha = 0.35f), shape)
            .clickable(onClick = onClick)
            // One element: "On loan, due 14 October. Opens Your Books", a button.
            .clearAndSetSemantics {
                contentDescription = Copy.A11y.labelAndHint(status.accessibilityLabel, HeldStatus.HINT)
                role = Role.Button
                onClick { onClick(); true }
            }
            .padding(horizontal = LsSpacing.l),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.s, Alignment.CenterHorizontally),
    ) {
        Icon(icon, contentDescription = null, tint = tint, modifier = Modifier.size(20.dp))
        Text(status.text, style = LsType.bodyStrong, color = tint, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}


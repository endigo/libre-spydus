package com.librespydus.app.feature.borrowed

import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.indication
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Formatting
import com.librespydus.app.feature.onboarding.AccountBanner
import com.librespydus.app.feature.onboarding.AccountBannerTone
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.ui.components.BookRowCard
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.components.SegmentedToggle
import com.librespydus.app.ui.components.ToastPill
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsTabBar
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.delay
import java.time.LocalDate
import java.util.Date

/**
 * "Your Books" (docs/native-apps.md §6 Borrowed): Loans | Reservations. Loans are sorted by due
 * date with the danger colour when due within 3 days or overdue; reservations show "Ready to
 * collect" first, then pending. A reservation's Edit (sheet) and Cancel (confirmed) sit behind a
 * swipe toward the leading edge, in its long-press menu and in its TalkBack actions; so does a
 * loan's Renew (confirmed, §5.12), whose progress and result show as a toast above the tab bar.
 *
 * Both lists come from the account store's snapshot (§5.2a): the saved one shows at once with
 * "Updated …" under the segments while a fresh one loads in the background (when it is over two
 * minutes old as the screen opens, and on pull to refresh). A failed refresh keeps the lists and
 * says so; only without any snapshot is there a loading or error state.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BorrowedScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val activity = LocalActivity.current
    // Activity-scoped: the tab's composition goes away when another tab is shown.
    val owner = (activity as? ComponentActivity) ?: checkNotNull(LocalViewModelStoreOwner.current)
    val vm: BorrowedViewModel = viewModel(viewModelStoreOwner = owner, key = "borrowed") { BorrowedViewModel(appState) }
    val model = vm.model

    val library by appState.library.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val session by appState.session.collectAsStateWithLifecycle()
    val demo by appState.isDemo.collectAsStateWithLifecycle()
    val snapshot by appState.account.snapshot.collectAsStateWithLifecycle()
    val accountRefreshing by appState.account.isRefreshing.collectAsStateWithLifecycle()
    val refreshError by appState.account.lastRefreshError.collectAsStateWithLifecycle()
    val segment by model.segment.collectAsStateWithLifecycle()
    val pendingSegment by appState.pendingBorrowedSegment.collectAsStateWithLifecycle()
    val busy by model.busyRows.collectAsStateWithLifecycle()
    val notice by model.notice.collectAsStateWithLifecycle()
    val confirming by model.confirmingCancel.collectAsStateWithLifecycle()
    val editing by model.editing.collectAsStateWithLifecycle()
    val renewing by model.renewing.collectAsStateWithLifecycle()
    val confirmingRenew by model.confirmingRenew.collectAsStateWithLifecycle()
    val toast by model.toast.collectAsStateWithLifecycle()
    val refreshing by vm.refreshing.collectAsStateWithLifecycle()
    val libraryName = library?.name ?: Copy.Common.yourLibrary
    val listState = rememberLazyListState()
    // The swiped-open card (one at a time): closed by a scroll, a tap or a segment switch.
    val swipe = remember { ReservationSwipeController() }
    val content = remember(snapshot, accountRefreshing, refreshError) {
        BorrowedContent.of(snapshot, accountRefreshing, refreshError, LocalDate.now())
    }
    val loans = content.loans
    val reservations = content.reservations

    LaunchedEffect(backend, session) { vm.appear() }
    // A book page's "On loan" / "Reserved" pill opens Your Books on that list.
    LaunchedEffect(pendingSegment) { model.showPendingSegment() }
    LaunchedEffect(listState) {
        snapshotFlow { listState.isScrollInProgress }.collect { if (it) swipe.close() }
    }
    LaunchedEffect(segment) { swipe.close() }

    // DEBUG screenshot hooks (demo mode only), once the lists have loaded.
    val loaded = loans is BorrowedLoad.Loaded && reservations is BorrowedLoad.Loaded
    LaunchedEffect(loaded) {
        if (!loaded) return@LaunchedEffect
        BorrowedDebugHooks.run(activity, model, demo && backend?.isDemo == true)
    }
    LaunchedEffect(loaded, segment) {
        if (loaded && BorrowedDebugHooks.flag(activity, BorrowedDebugHooks.SCROLL_BOTTOM)) {
            delay(600)
            val last = listState.layoutInfo.totalItemsCount - 1
            if (last >= 0) {
                listState.scrollToItem(last)
                listState.scrollBy(100_000f)
            }
        }
    }
    LaunchedEffect(notice?.id) {
        val id = notice?.id ?: return@LaunchedEffect
        delay(6_000)
        model.dismissNotice(id)
    }
    // "Renewing…" stays until the result replaces it; the result goes after a few seconds.
    LaunchedEffect(toast?.id) {
        val shown = toast ?: return@LaunchedEffect
        delay(shown.tone.millis ?: return@LaunchedEffect)
        model.dismissToast(shown.id)
    }

    PullToRefreshBox(
        isRefreshing = refreshing,
        onRefresh = { if (session != null) vm.pullToRefresh() },
        modifier = modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding(),
    ) {
        LazyColumn(
            state = listState,
            contentPadding = PaddingValues(
                start = LsSpacing.l,
                end = LsSpacing.l,
                // Clear the raised Search button at max scroll.
                bottom = LsSpacing.xxl,
            ),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
            modifier = Modifier
                .fillMaxSize()
                .closesReservationSwipeOnTap(swipe),
        ) {
            item(key = "title") {
                Text(
                    Copy.YourBooks.title,
                    style = LsType.greeting,
                    color = LsColors.textPrimary,
                    modifier = Modifier
                        .padding(top = LsSpacing.l)
                        .semantics { heading() },
                )
            }
            if (session == null) {
                item(key = "signed-out") {
                    EmptyStateView(
                        title = Copy.YourBooks.signedOutTitle,
                        message = Copy.YourBooks.signedOutMessage(libraryName),
                        icon = AccountIcons.AccountCircle,
                        actionLabel = Copy.Account.signIn,
                        onAction = appState::requestLogin,
                        modifier = Modifier.fillMaxWidth().heightIn(min = STATE_HEIGHT + 80.dp),
                    )
                }
                return@LazyColumn
            }
            item(key = "segments") {
                val loanCount = loans.valueOrNull?.size
                val reservationCount = reservations.valueOrNull?.count
                Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                    SegmentedToggle(
                        options = listOf(
                            loanCount?.let(Copy.YourBooks::loansCount) ?: Copy.YourBooks.loans,
                            reservationCount?.let(Copy.YourBooks::reservationsCount) ?: Copy.YourBooks.reservations,
                        ),
                        selectedIndex = segment.ordinal,
                        onSelect = { model.select(BorrowedModel.Segment.entries[it]) },
                        modifier = Modifier.fillMaxWidth(),
                    )
                    content.status?.let { AccountStatusLine(it, onRetry = vm::retry) }
                }
            }
            notice?.let { shown ->
                item(key = "notice-${shown.id}") {
                    AccountBanner(
                        shown.message,
                        tone = if (shown.isError) AccountBannerTone.ERROR else AccountBannerTone.SUCCESS,
                        onClick = { model.dismissNotice(shown.id) },
                        modifier = Modifier
                            .animateItem()
                            .semantics {
                                liveRegion = LiveRegionMode.Polite
                                contentDescription = Copy.A11y.labelAndHint(shown.message, Copy.A11y.dismissHint)
                            },
                    )
                }
            }
            when (segment) {
                BorrowedModel.Segment.LOANS -> loansContent(
                    loans,
                    libraryName,
                    renewing = renewing,
                    swipe = swipe,
                    onRetry = vm::retry,
                    onOpen = { navigator.openBook(it.ref) },
                    onRenew = model::askToRenew,
                )
                BorrowedModel.Segment.RESERVATIONS -> reservationsContent(
                    reservations,
                    busy = busy,
                    swipe = swipe,
                    onRetry = vm::retry,
                    onOpen = { navigator.openBook(it.ref) },
                    onCancel = model::askToCancel,
                    onEdit = model::edit,
                )
            }
        }
        RenewToast(
            toast,
            onDismiss = model::dismissToast,
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(start = LsSpacing.l, end = LsSpacing.l, bottom = LsSpacing.m),
        )
    }

    confirming?.let { row ->
        AlertDialog(
            onDismissRequest = { model.askToCancel(null) },
            title = { Text(Copy.YourBooks.cancelTitle) },
            text = { Text(CancelConfirm.message(row)) },
            confirmButton = {
                TextButton(onClick = { vm.launch { cancel(row) } }) { Text(Copy.YourBooks.cancelReservation, color = LsColors.danger) }
            },
            dismissButton = {
                TextButton(onClick = { model.askToCancel(null) }) { Text(Copy.YourBooks.keepReservation, color = LsColors.textPrimary) }
            },
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textPrimary,
            textContentColor = LsColors.textSecondary,
        )
    }

    confirmingRenew?.let { row ->
        AlertDialog(
            onDismissRequest = { model.askToRenew(null) },
            title = { Text(Copy.YourBooks.renewTitle) },
            text = { Text(RenewText.confirm(row)) },
            confirmButton = {
                TextButton(onClick = { vm.launch { renew(row) } }) { Text(Copy.YourBooks.renew, color = LsColors.primary) }
            },
            dismissButton = {
                TextButton(onClick = { model.askToRenew(null) }) { Text(Copy.Common.notNow, color = LsColors.textPrimary) }
            },
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textPrimary,
            textContentColor = LsColors.textSecondary,
        )
    }

    editing?.let { row ->
        EditReservationSheet(row = row, appState = appState, workScope = vm.viewModelScope) { outcome ->
            vm.launch {
                finishEdit(outcome, row)
                reportDeferredError()
            }
        }
    }
}

private val STATE_HEIGHT = 380.dp

/**
 * The muted line under the segments: "Updated 12 min ago" (ticking, in the device's time format)
 * with a small spinner while a refresh runs, or "Couldn’t refresh · Showing the saved list" + Retry.
 */
@Composable
private fun AccountStatusLine(status: AccountStatus, onRetry: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 24.dp)
            .semantics(mergeDescendants = true) { liveRegion = LiveRegionMode.Polite },
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        when (status) {
            is AccountStatus.Updated -> {
                val context = LocalContext.current
                val timeFormat = remember(context) { android.text.format.DateFormat.getTimeFormat(context) }
                val now by produceState(System.currentTimeMillis(), status.fetchedAtMs) {
                    while (true) {
                        value = System.currentTimeMillis()
                        delay(STATUS_TICK_MILLIS)
                    }
                }
                if (status.refreshing) {
                    CircularProgressIndicator(color = LsColors.textMuted, strokeWidth = 1.5.dp, modifier = Modifier.size(12.dp))
                    Spacer(Modifier.width(LsSpacing.s))
                }
                val updated = Formatting.updatedText(status.fetchedAtMs, now, time = { timeFormat.format(Date.from(it.toInstant())) })
                Text(
                    updated,
                    style = LsType.caption,
                    color = LsColors.textMuted,
                    modifier = Modifier.clearAndSetSemantics {
                        contentDescription = if (status.refreshing) Copy.A11y.refreshing(updated) else updated
                    },
                )
            }
            is AccountStatus.RefreshFailed -> {
                Icon(AccountIcons.Warning, contentDescription = null, tint = LsColors.warning, modifier = Modifier.size(14.dp))
                Spacer(Modifier.width(LsSpacing.xs))
                Text(
                    AccountStatus.REFRESH_FAILED,
                    style = LsType.caption,
                    color = LsColors.textSecondary,
                    modifier = Modifier.weight(1f, fill = false),
                )
                if (status.error.isRetryable) {
                    Text(
                        Copy.Common.retry,
                        style = LsType.caption.copy(fontWeight = FontWeight.SemiBold),
                        color = LsColors.primary,
                        modifier = Modifier
                            .clip(RoundedCornerShape(LsRadius.pill))
                            .clickable(role = Role.Button, onClick = onRetry)
                            .semantics { contentDescription = Copy.A11y.labelAndHint(Copy.Common.retry, Copy.YourBooks.refreshHint) }
                            .padding(horizontal = LsSpacing.s, vertical = LsSpacing.xs),
                    )
                }
            }
        }
    }
}

/** How often "Updated N min ago" is re-worded. */
private const val STATUS_TICK_MILLIS = 30_000L

private fun LazyListScope.stateItem(key: String, content: @Composable () -> Unit) {
    item(key = key) {
        Box(Modifier.fillMaxWidth().heightIn(min = STATE_HEIGHT), contentAlignment = Alignment.Center) { content() }
    }
}

private fun LazyListScope.loansContent(
    loans: BorrowedLoad<List<LoanRow>>,
    libraryName: String,
    renewing: Set<String>,
    swipe: ReservationSwipeController,
    onRetry: () -> Unit,
    onOpen: (LoanRow) -> Unit,
    onRenew: (LoanRow) -> Unit,
) {
    when (loans) {
        BorrowedLoad.Idle, BorrowedLoad.Loading -> stateItem("loans-loading") { LoadingView(message = Copy.YourBooks.loadingLoans) }
        is BorrowedLoad.Failed -> stateItem("loans-error") { ErrorStateView(loans.error, onRetry = onRetry) }
        is BorrowedLoad.Loaded -> if (loans.value.isEmpty()) {
            stateItem("loans-empty") {
                EmptyStateView(
                    title = Copy.YourBooks.noLoansTitle,
                    message = Copy.YourBooks.noLoansMessage(libraryName),
                    icon = com.librespydus.app.ui.theme.LsIcons.Book,
                )
            }
        } else {
            items(loans.value, key = { "loan-${it.id}" }) { row -> LoanCard(row, row.id in renewing, swipe, onOpen, onRenew) }
            if (loans.value.canRenew) {
                item(key = "loan-swipe-hint") { Footnote(LsIcons.ArrowBack, Copy.YourBooks.loanSwipeHint) }
            }
        }
    }
}

/** A muted line under a list (the swipe hints). */
@Composable
private fun Footnote(icon: ImageVector, text: String) {
    Row(
        Modifier.fillMaxWidth().padding(top = LsSpacing.s),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(14.dp))
        Spacer(Modifier.width(LsSpacing.xs))
        Text(text, style = LsType.caption, color = LsColors.textMuted)
    }
}

/**
 * A loan: tap opens the book; Renew (when the library links it) is behind a swipe
 * ([ReservationSwipeBox]), in a long-press menu and in the card's TalkBack custom actions. While it
 * renews ([renewing]) the card is dimmed with a spinner and takes no actions.
 */
@Composable
private fun LoanCard(
    row: LoanRow,
    renewing: Boolean,
    swipe: ReservationSwipeController,
    onOpen: (LoanRow) -> Unit,
    onRenew: (LoanRow) -> Unit,
) {
    val actions = LoanAction.of(row)
    val run: (LoanAction) -> Unit = { onRenew(row) }
    val overdue = row.due?.isOverdue == true
    ActionCard(
        // Apart from the reservations' ids: one swipe controller serves both lists.
        swipeId = "loan-${row.id}",
        label = listOfNotNull(
            Copy.Loans.overdue.takeIf { overdue },
            row.ref.title,
            Formatting.byLine(row.ref.author),
            row.meta,
            Copy.YourBooks.renewing.takeIf { renewing },
        ).joinToString(", "),
        actions = actions,
        busy = renewing,
        swipe = swipe,
        longClickLabel = Copy.Common.moreOptions,
        onOpen = { onOpen(row) },
        onAction = run,
    ) { modifier ->
        BookRowCard(
            title = row.ref.title,
            coverUrl = row.ref.coverUrl,
            byLine = Formatting.byLine(row.ref.author),
            meta = row.meta,
            metaColor = if (row.isUrgent) LsColors.danger else LsColors.textSecondary,
            badge = if (overdue) Copy.Loans.overdue else null,
            badgeColor = LsColors.danger,
            modifier = modifier,
        )
    }
}

private fun LazyListScope.reservationsContent(
    reservations: BorrowedLoad<ReservationSections>,
    busy: Set<String>,
    swipe: ReservationSwipeController,
    onRetry: () -> Unit,
    onOpen: (ReservationRow) -> Unit,
    onCancel: (ReservationRow) -> Unit,
    onEdit: (ReservationRow) -> Unit,
) {
    when (reservations) {
        BorrowedLoad.Idle, BorrowedLoad.Loading -> stateItem("reservations-loading") { LoadingView(message = Copy.YourBooks.loadingReservations) }
        is BorrowedLoad.Failed -> stateItem("reservations-error") { ErrorStateView(reservations.error, onRetry = onRetry) }
        is BorrowedLoad.Loaded -> {
            val sections = reservations.value
            if (sections.isEmpty) {
                stateItem("reservations-empty") {
                    EmptyStateView(
                        title = Copy.YourBooks.noReservationsTitle,
                        message = Copy.YourBooks.noReservationsMessage,
                        icon = com.librespydus.app.ui.theme.LsIcons.Bookmark,
                    )
                }
                return
            }
            if (sections.ready.isNotEmpty()) {
                item(key = "ready-header") { SectionHeader(Copy.YourBooks.readyToCollect, Modifier.padding(top = LsSpacing.xs)) }
                items(sections.ready, key = { "reservation-${it.id}" }) { row ->
                    ReservationCard(row, row.id in busy, swipe, onOpen, onCancel, onEdit)
                }
            }
            if (sections.pending.isNotEmpty()) {
                item(key = "pending-header") { SectionHeader(Copy.YourBooks.pending, Modifier.padding(top = LsSpacing.xs)) }
                items(sections.pending, key = { "reservation-${it.id}" }) { row ->
                    ReservationCard(row, row.id in busy, swipe, onOpen, onCancel, onEdit)
                }
            }
            if (sections.hasActions) {
                item(key = "swipe-hint") { Footnote(LsIcons.ArrowBack, SWIPE_HINT) }
            }
        }
    }
}

internal const val SWIPE_HINT = Copy.YourBooks.swipeHint

/** Where `BookRowCard`'s surface starts below the cover rising above it (its `rise`). */
private val CARD_RISE = 14.dp

/**
 * A reservation: tap opens the book; Edit / Cancel are behind a swipe ([ReservationSwipeBox]), in
 * a long-press menu and in the card's TalkBack custom actions. While a cancel runs ([busy]) the
 * card is dimmed with a spinner and takes no actions.
 */
@Composable
private fun ReservationCard(
    row: ReservationRow,
    busy: Boolean,
    swipe: ReservationSwipeController,
    onOpen: (ReservationRow) -> Unit,
    onCancel: (ReservationRow) -> Unit,
    onEdit: (ReservationRow) -> Unit,
) {
    val actions = ReservationAction.of(row)
    val run: (ReservationAction) -> Unit = { action ->
        when (action) {
            ReservationAction.EDIT -> onEdit(row)
            ReservationAction.CANCEL -> onCancel(row)
        }
    }
    val statusColor: Color = when {
        row.isReady -> LsColors.success
        row.isInTransit -> LsColors.warning
        else -> LsColors.textSecondary
    }
    ActionCard(
        swipeId = row.id,
        // Read in its own words: the queue place as "number 4 in the queue", "Updating" while a
        // cancel or edit runs.
        label = listOfNotNull(
            row.statusLabel,
            row.ref.title,
            Formatting.byLine(row.ref.author),
            row.spokenMeta,
            Copy.A11y.updating.takeIf { busy },
        ).joinToString(", "),
        actions = actions,
        busy = busy,
        swipe = swipe,
        longClickLabel = Copy.YourBooks.showActions,
        onOpen = { onOpen(row) },
        onAction = run,
    ) { modifier ->
        BookRowCard(
            title = row.ref.title,
            coverUrl = row.ref.coverUrl,
            byLine = Formatting.byLine(row.ref.author),
            meta = row.meta,
            metaColor = if (row.isReady) LsColors.textPrimary else LsColors.textSecondary,
            badge = row.statusLabel,
            badgeColor = statusColor,
            highlighted = row.isReady,
            modifier = modifier,
        )
    }
}

/**
 * A Your Books card with [actions]: tap runs [onOpen]; the actions are behind a swipe
 * ([ReservationSwipeBox]), in a long-press menu ([longClickLabel]) and in its TalkBack custom
 * actions. TalkBack reads the whole card as one node, [label]. While [busy] the card is dimmed
 * with a spinner and takes no actions. [card] draws the card with the modifier it's given.
 */
@Composable
private fun <A : SwipeAction> ActionCard(
    swipeId: String,
    label: String,
    actions: List<A>,
    busy: Boolean,
    swipe: ReservationSwipeController,
    longClickLabel: String,
    onOpen: () -> Unit,
    onAction: (A) -> Unit,
    card: @Composable (Modifier) -> Unit,
) {
    var menuShown by remember { mutableStateOf(false) }
    val presses = remember { MutableInteractionSource() }
    ReservationSwipeBox(
        id = swipeId,
        controller = swipe,
        actions = actions,
        onAction = onAction,
        enabled = !busy,
        surfaceTop = CARD_RISE,
    ) {
        Box(
            Modifier
                .combinedClickable(
                    interactionSource = presses,
                    indication = null,
                    enabled = !busy,
                    onLongClickLabel = if (actions.isEmpty()) null else longClickLabel,
                    onLongClick = if (actions.isEmpty()) null else ({
                        swipe.close()
                        menuShown = true
                    }),
                    // A tap while a card is swiped open only closes it.
                    onClick = { if (!swipe.dismiss()) onOpen() },
                )
                // Clears the card's own text (and the cover's label) under it; the click actions stay.
                .clearAndSetSemantics {
                    contentDescription = label
                    if (!busy) {
                        customActions = actions.map { action -> CustomAccessibilityAction(action.fullTitle) { onAction(action); true } }
                    }
                },
        ) {
            card(Modifier.alpha(if (busy) 0.7f else 1f))
            // The press ripple, on the card's surface only (not the strip the cover rises into).
            Box(
                Modifier
                    .matchParentSize()
                    .padding(top = CARD_RISE)
                    .clip(RoundedCornerShape(LsRadius.card))
                    .indication(presses, ripple()),
            )
            if (busy) {
                CircularProgressIndicator(
                    color = LsColors.textSecondary,
                    strokeWidth = 2.dp,
                    modifier = Modifier
                        .align(Alignment.CenterEnd)
                        .padding(end = LsSpacing.xl)
                        .size(22.dp),
                )
            }
            if (actions.isNotEmpty()) {
                // Anchors the long-press menu under the card's trailing edge.
                Box(Modifier.align(Alignment.BottomEnd)) {
                    ActionMenu(
                        expanded = menuShown && !busy,
                        actions = actions,
                        onDismiss = { menuShown = false },
                        onAction = { action ->
                            menuShown = false
                            onAction(action)
                        },
                    )
                }
            }
        }
    }
}

/** The long-press menu: "Edit reservation", "Cancel reservation" (destructive, in coral); "Renew loan". */
@Composable
private fun <A : SwipeAction> ActionMenu(
    expanded: Boolean,
    actions: List<A>,
    onDismiss: () -> Unit,
    onAction: (A) -> Unit,
) {
    DropdownMenu(
        expanded = expanded,
        onDismissRequest = onDismiss,
        shape = RoundedCornerShape(LsRadius.field),
        containerColor = LsColors.surfaceElevated,
    ) {
        actions.forEach { action ->
            val color = if (action.isDestructive) LsColors.danger else LsColors.textPrimary
            DropdownMenuItem(
                text = { Text(action.fullTitle, style = LsType.body, color = color) },
                leadingIcon = { Icon(action.icon, contentDescription = null, tint = color, modifier = Modifier.size(20.dp)) },
                onClick = { onAction(action) },
            )
        }
    }
}

/** A renewal's "Renewing…" and its result, in a pill that slides up above the tab bar (§5.12). */
@Composable
private fun RenewToast(toast: BorrowedModel.Toast?, onDismiss: (Long) -> Unit, modifier: Modifier = Modifier) {
    // Kept while the pill slides out.
    var shown by remember { mutableStateOf(toast) }
    LaunchedEffect(toast) { toast?.let { shown = it } }
    AnimatedVisibility(
        visible = toast != null,
        enter = slideInVertically { it / 2 } + fadeIn(),
        exit = slideOutVertically { it / 2 } + fadeOut(),
        modifier = modifier,
    ) {
        shown?.let { ToastPill(it.message, it.tone, successIcon = AccountIcons.CheckCircle, onDismiss = { onDismiss(it.id) }) }
    }
}

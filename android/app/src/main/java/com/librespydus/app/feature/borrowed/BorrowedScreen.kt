package com.librespydus.app.feature.borrowed

import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Formatting
import com.librespydus.app.feature.onboarding.AccountBanner
import com.librespydus.app.feature.onboarding.AccountBannerTone
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.ui.components.BookRowCard
import com.librespydus.app.ui.components.CardAction
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.components.SegmentedToggle
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsTabBar
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.delay

/**
 * "Your Books" (docs/native-apps.md §6 Borrowed): Loans | Reservations. Loans are sorted by due
 * date with the danger colour when due within 3 days or overdue; reservations show "Ready for
 * pickup" first, then pending, each with Cancel (confirmed) and Edit (sheet). Pull to refresh;
 * reloads on sign-in/out, library change and verification.
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
    val accountGeneration by appState.accountGeneration.collectAsStateWithLifecycle()
    val verificationGeneration by appState.verificationGeneration.collectAsStateWithLifecycle()
    val demo by appState.isDemo.collectAsStateWithLifecycle()
    val segment by model.segment.collectAsStateWithLifecycle()
    val loans by model.loans.collectAsStateWithLifecycle()
    val reservations by model.reservations.collectAsStateWithLifecycle()
    val busy by model.busyRows.collectAsStateWithLifecycle()
    val notice by model.notice.collectAsStateWithLifecycle()
    val confirming by model.confirmingCancel.collectAsStateWithLifecycle()
    val editing by model.editing.collectAsStateWithLifecycle()
    val refreshing by vm.refreshing.collectAsStateWithLifecycle()
    val libraryName = library?.name ?: "your library"
    val listState = rememberLazyListState()

    LaunchedEffect(backend, session, accountGeneration, verificationGeneration) { vm.appear() }

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
                bottom = LsTabBar.centerButtonLift + LsSpacing.xxl,
            ),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
            modifier = Modifier.fillMaxSize(),
        ) {
            item(key = "title") {
                Text(
                    "Your Books",
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
                        title = "Sign in to see your books",
                        message = "Your loans and reservations at $libraryName appear here.",
                        icon = AccountIcons.AccountCircle,
                        actionLabel = "Sign in",
                        onAction = appState::requestLogin,
                        modifier = Modifier.fillMaxWidth().heightIn(min = STATE_HEIGHT + 80.dp),
                    )
                }
                return@LazyColumn
            }
            item(key = "segments") {
                val loanCount = loans.valueOrNull?.size
                val reservationCount = reservations.valueOrNull?.count
                SegmentedToggle(
                    options = listOf(
                        loanCount?.let { "Loans ($it)" } ?: "Loans",
                        reservationCount?.let { "Reservations ($it)" } ?: "Reservations",
                    ),
                    selectedIndex = segment.ordinal,
                    onSelect = { model.select(BorrowedModel.Segment.entries[it]) },
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            notice?.let { shown ->
                item(key = "notice-${shown.id}") {
                    AccountBanner(
                        shown.message,
                        tone = if (shown.isError) AccountBannerTone.ERROR else AccountBannerTone.SUCCESS,
                        onClick = { model.dismissNotice(shown.id) },
                        modifier = Modifier
                            .animateItem()
                            .semantics { liveRegion = LiveRegionMode.Polite },
                    )
                }
            }
            when (segment) {
                BorrowedModel.Segment.LOANS -> loansContent(loans, libraryName, onRetry = vm::reload, onOpen = { navigator.openBook(it.ref) })
                BorrowedModel.Segment.RESERVATIONS -> reservationsContent(
                    reservations,
                    busy = busy,
                    onRetry = vm::reload,
                    onOpen = { navigator.openBook(it.ref) },
                    onCancel = model::askToCancel,
                    onEdit = model::edit,
                )
            }
        }
    }

    confirming?.let { row ->
        AlertDialog(
            onDismissRequest = { model.askToCancel(null) },
            title = { Text("Cancel this reservation?") },
            text = { Text(CancelConfirm.message(row)) },
            confirmButton = {
                TextButton(onClick = { vm.launch { cancel(row) } }) { Text("Cancel reservation", color = LsColors.danger) }
            },
            dismissButton = {
                TextButton(onClick = { model.askToCancel(null) }) { Text("Keep reservation", color = LsColors.textPrimary) }
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

private fun LazyListScope.stateItem(key: String, content: @Composable () -> Unit) {
    item(key = key) {
        Box(Modifier.fillMaxWidth().heightIn(min = STATE_HEIGHT), contentAlignment = Alignment.Center) { content() }
    }
}

private fun LazyListScope.loansContent(
    loans: BorrowedLoad<List<LoanRow>>,
    libraryName: String,
    onRetry: () -> Unit,
    onOpen: (LoanRow) -> Unit,
) {
    when (loans) {
        BorrowedLoad.Idle, BorrowedLoad.Loading -> stateItem("loans-loading") { LoadingView(message = "Loading your loans…") }
        is BorrowedLoad.Failed -> stateItem("loans-error") { ErrorStateView(loans.error, onRetry = onRetry) }
        is BorrowedLoad.Loaded -> if (loans.value.isEmpty()) {
            stateItem("loans-empty") {
                EmptyStateView(
                    title = "No loans right now",
                    message = "Books you borrow from $libraryName show up here with their due dates.",
                    icon = com.librespydus.app.ui.theme.LsIcons.Book,
                )
            }
        } else {
            items(loans.value, key = { "loan-${it.id}" }) { row -> LoanCard(row, onClick = { onOpen(row) }) }
            item(key = "renewals") {
                Row(
                    Modifier.fillMaxWidth().padding(top = LsSpacing.s),
                    horizontalArrangement = Arrangement.Center,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(AccountIcons.Sync, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(14.dp))
                    Spacer(Modifier.width(LsSpacing.xs))
                    Text("Renewals coming soon", style = LsType.caption, color = LsColors.textMuted)
                }
            }
        }
    }
}

@Composable
private fun LoanCard(row: LoanRow, onClick: () -> Unit) {
    val overdue = row.due?.isOverdue == true
    BookRowCard(
        title = row.ref.title,
        coverUrl = row.ref.coverUrl,
        byLine = Formatting.byLine(row.ref.author),
        meta = row.meta,
        metaColor = if (row.isUrgent) LsColors.danger else LsColors.textSecondary,
        badge = if (overdue) "Overdue" else null,
        badgeColor = LsColors.danger,
        onClick = onClick,
    )
}

private fun LazyListScope.reservationsContent(
    reservations: BorrowedLoad<ReservationSections>,
    busy: Set<String>,
    onRetry: () -> Unit,
    onOpen: (ReservationRow) -> Unit,
    onCancel: (ReservationRow) -> Unit,
    onEdit: (ReservationRow) -> Unit,
) {
    when (reservations) {
        BorrowedLoad.Idle, BorrowedLoad.Loading -> stateItem("reservations-loading") { LoadingView(message = "Loading your reservations…") }
        is BorrowedLoad.Failed -> stateItem("reservations-error") { ErrorStateView(reservations.error, onRetry = onRetry) }
        is BorrowedLoad.Loaded -> {
            val sections = reservations.value
            if (sections.isEmpty) {
                stateItem("reservations-empty") {
                    EmptyStateView(
                        title = "No reservations",
                        message = "Reserve a book with Borrow Now and follow it here until it's ready to collect.",
                        icon = com.librespydus.app.ui.theme.LsIcons.Bookmark,
                    )
                }
                return
            }
            if (sections.ready.isNotEmpty()) {
                item(key = "ready-header") { SectionHeader("Ready for pickup", Modifier.padding(top = LsSpacing.xs)) }
                items(sections.ready, key = { "reservation-${it.id}" }) { row ->
                    ReservationCard(row, row.id in busy, onOpen, onCancel, onEdit)
                }
            }
            if (sections.pending.isNotEmpty()) {
                item(key = "pending-header") { SectionHeader("Pending", Modifier.padding(top = LsSpacing.xs)) }
                items(sections.pending, key = { "reservation-${it.id}" }) { row ->
                    ReservationCard(row, row.id in busy, onOpen, onCancel, onEdit)
                }
            }
        }
    }
}

@Composable
private fun ReservationCard(
    row: ReservationRow,
    busy: Boolean,
    onOpen: (ReservationRow) -> Unit,
    onCancel: (ReservationRow) -> Unit,
    onEdit: (ReservationRow) -> Unit,
) {
    val actions = buildList {
        if (row.cancelHref != null) add(CardAction("Cancel", { onCancel(row) }))
        if (row.editHref != null) add(CardAction("Edit", { onEdit(row) }))
    }
    val statusColor: Color = when {
        row.isReady -> LsColors.success
        row.isInTransit -> LsColors.warning
        else -> LsColors.textSecondary
    }
    Box(
        Modifier.semantics {
            if (!busy) {
                customActions = buildList {
                    if (row.cancelHref != null) add(CustomAccessibilityAction("Cancel reservation") { onCancel(row); true })
                    if (row.editHref != null) add(CustomAccessibilityAction("Edit reservation") { onEdit(row); true })
                }
            }
        },
    ) {
        BookRowCard(
            title = row.ref.title,
            coverUrl = row.ref.coverUrl,
            byLine = Formatting.byLine(row.ref.author),
            meta = row.meta,
            metaColor = if (row.isReady) LsColors.textPrimary else LsColors.textSecondary,
            badge = row.statusLabel,
            badgeColor = statusColor,
            highlighted = row.isReady,
            actions = if (busy) emptyList() else actions,
            onClick = if (busy) null else ({ onOpen(row) }),
            modifier = Modifier.alpha(if (busy) 0.7f else 1f),
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
    }
}

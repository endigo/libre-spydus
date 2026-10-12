import SwiftUI

/// "Your Books" (docs/native-apps.md §6 Borrowed): Loans | Reservations. Loans are sorted by due
/// date with the danger colour when due within 3 days or overdue; reservations show "Ready for
/// pickup" first, then pending. A reservation's Edit (sheet) and Cancel (confirmed) sit behind a
/// swipe to the left, in its long-press menu and in its VoiceOver actions.
///
/// Both lists come from the saved account (`AppModel.account`, §5.2a): shown at once, with
/// "Updated 12 min ago" under the segments, and fetched again in the background when it's out of
/// date on opening the tab, on pull to refresh and after a cancel or edit. A failed refresh keeps
/// the saved lists ("Couldn't refresh — showing saved data").
struct BorrowedView: View {
    @Environment(AppModel.self) private var model
    @State private var viewModel = BorrowedViewModel()
    @State private var scrollMark = ScrollMark()

    var body: some View {
        @Bindable var viewModel = viewModel
        ScrollView {
            VStack(alignment: .leading, spacing: Theme.Spacing.l) {
                Text("Your Books")
                    .themeFont(.greeting)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .accessibilityAddTraits(.isHeader)
                    .padding(.top, Theme.Spacing.l)
                if model.isSignedIn {
                    VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                        segmentPicker
                        statusLine
                    }
                    if let notice = viewModel.notice {
                        noticeBanner(notice)
                    }
                    switch viewModel.segment {
                    case .loans: loansContent
                    case .reservations: reservationsContent
                    }
                } else {
                    signedOut
                }
            }
            .padding(.horizontal, Theme.Spacing.l)
            .padding(.bottom, Theme.Spacing.xxl)
            .animation(.default, value: viewModel.notice)
            // A tap anywhere but a card or control closes an open card's swipe actions.
            .contentShape(Rectangle())
            .gesture(TapGesture().onEnded { closeSwipe() }, including: viewModel.swipedRowID == nil ? .subviews : .all)
            .onGeometryChange(for: CGFloat.self) { $0.frame(in: .scrollView).minY } action: { y in
                closeSwipeIfScrolled(to: y)
            }
        }
        .defaultScrollAnchor(BorrowedDebugHooks.initialScrollAnchor)
        .screenBackground()
        .refreshable { await model.refreshAccount(force: true) }
        .task(id: model.reloadKey) { await appear() }
        .onChange(of: model.selectedTab) { _, tab in
            viewModel.swipedRowID = nil
            // Back on the tab: refresh in the background if the account is out of date.
            guard tab == .borrowed, model.isSignedIn else { return }
            Task { await model.refreshAccount(force: false) }
        }
        .onChange(of: viewModel.segment) { viewModel.swipedRowID = nil }
        .onChange(of: model.pendingBorrowedSegment, initial: true) { _, segment in
            // A book page's "On loan" / "Reserved" pill asked for this segment.
            guard segment != nil, let segment = model.takePendingBorrowedSegment() else { return }
            viewModel.segment = segment
        }
        .onChange(of: viewModel.confirmingCancel == nil) { _, answered in
            // Confirmed or dismissed: either way the card closes.
            if answered { closeSwipe() }
        }
        .onChange(of: viewModel.notice) { _, notice in
            if let notice { AccessibilityNotification.Announcement(notice.message).post() }
        }
        .toolbar(.hidden, for: .navigationBar)
        .onChange(of: viewModel.editing != nil) { _, editing in
            if editing { model.isPresentingFeatureSheet = true }
        }
        .sheet(item: $viewModel.editing, onDismiss: {
            model.isPresentingFeatureSheet = false
            viewModel.reportDeferredError(model)
        }) { row in
            EditReservationSheet(row: row) { outcome in
                Task { await viewModel.finishEdit(outcome, row: row, model: model) }
            }
            .environment(model)
            .presentationBackground(Theme.Colors.background)
        }
    }

    private func appear() async {
        await viewModel.appear(model)
        #if DEBUG
        await BorrowedDebugHooks.run(viewModel, model: model)
        #endif
    }

    /// Retry after a failed refresh.
    private func retry() {
        Task { await model.refreshAccount(force: true) }
    }

    private var lists: BorrowedViewModel.Lists? {
        viewModel.lists(model)
    }

    // MARK: Header

    private var segmentPicker: some View {
        @Bindable var viewModel = viewModel
        return Picker("Show", selection: $viewModel.segment) {
            ForEach(BorrowedViewModel.Segment.allCases) { segment in
                Text(segmentTitle(segment)).tag(segment)
            }
        }
        .pickerStyle(.segmented)
    }

    private func segmentTitle(_ segment: BorrowedViewModel.Segment) -> String {
        let count: Int? = switch segment {
        case .loans: lists?.loans.count
        case .reservations: lists?.reservations.count
        }
        return count.map { "\(segment.title) (\($0))" } ?? segment.title
    }

    /// "Updated 12 min ago" (+ a spinner while refreshing), or the failed-refresh note; only once
    /// there's something on screen.
    @ViewBuilder
    private var statusLine: some View {
        if let updatedAt = model.account.updatedAt {
            AccountStatusLine(updatedAt: updatedAt, isRefreshing: model.account.isRefreshing,
                              failed: model.account.lastRefreshError != nil, retry: retry)
        }
    }

    /// No snapshot yet: loading, or why the first fetch failed.
    @ViewBuilder
    private func placeholder(loading message: String) -> some View {
        if let error = model.account.lastRefreshError, !model.account.isRefreshing {
            ErrorStateView(error: error, retry: retry)
                .frame(minHeight: Self.stateHeight)
        } else {
            LoadingView(message: message)
                .frame(minHeight: Self.stateHeight)
        }
    }

    private func noticeBanner(_ notice: BorrowedViewModel.Notice) -> some View {
        Banner(message: notice.message, tone: notice.isError ? .error : .success)
            .onTapGesture { viewModel.notice = nil }
            .accessibilityHint("Double-tap to dismiss")
            .accessibilityAction { viewModel.notice = nil }
            .transition(.move(edge: .top).combined(with: .opacity))
            .task(id: notice.id) {
                try? await Task.sleep(for: .seconds(6))
                if viewModel.notice?.id == notice.id { viewModel.notice = nil }
            }
    }

    // MARK: Loans

    @ViewBuilder
    private var loansContent: some View {
        if let rows = lists?.loans {
            loansList(rows)
        } else {
            placeholder(loading: "Loading your loans…")
        }
    }

    @ViewBuilder
    private func loansList(_ rows: [LoanRow]) -> some View {
        if rows.isEmpty {
            EmptyStateView(systemImage: "books.vertical", title: "No loans right now",
                           message: "Books you borrow from \(model.libraryName) show up here with their due dates.")
                .frame(minHeight: Self.stateHeight)
        } else {
            ForEach(rows) { row in
                loanCard(row)
            }
            Label("Renewals coming soon", systemImage: "arrow.triangle.2.circlepath")
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textMuted)
                .frame(maxWidth: .infinity)
                .padding(.top, Theme.Spacing.s)
        }
    }

    private func loanCard(_ row: LoanRow) -> some View {
        let overdue = row.due?.isOverdue == true
        return BookRowCard(row.ref, meta: row.meta,
                           metaColor: row.isUrgent ? Theme.Colors.danger : Theme.Colors.textSecondary,
                           chips: overdue ? [Chip(text: "Overdue", tone: .danger, systemImage: "exclamationmark.circle")] : [])
            .contentShape(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous))
            .onTapGesture { open(row.ref) }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(accessibilityText([row.ref.title, Formatting.byline(row.ref.author),
                                                   overdue ? "Overdue" : nil, row.meta]))
            .accessibilityAddTraits(.isButton)
            .accessibilityAction { open(row.ref) }
    }

    // MARK: Reservations

    @ViewBuilder
    private var reservationsContent: some View {
        if let sections = lists?.reservations {
            reservationsList(sections)
        } else {
            placeholder(loading: "Loading your reservations…")
        }
    }

    @ViewBuilder
    private func reservationsList(_ sections: ReservationSections) -> some View {
        if sections.isEmpty {
            EmptyStateView(systemImage: "bookmark.circle", title: "No reservations",
                           message: "Tap Reserve on any book and follow it here until it's ready to collect.")
                .frame(minHeight: Self.stateHeight)
        } else {
            if !sections.ready.isEmpty {
                SectionHeader(title: "Ready for pickup")
                    .padding(.top, Theme.Spacing.xs)
                ForEach(sections.ready) { row in
                    reservationCard(row)
                }
            }
            if !sections.pending.isEmpty {
                SectionHeader(title: "Pending")
                    .padding(.top, Theme.Spacing.xs)
                ForEach(sections.pending) { row in
                    reservationCard(row)
                }
            }
            if sections.hasActions {
                let hint = SwipeReveal.hint()
                Label(hint.text, systemImage: hint.systemImage)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textMuted)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: .infinity)
                    .padding(.top, Theme.Spacing.s)
                    // VoiceOver lists Edit and Cancel as actions on each card instead.
                    .accessibilityHidden(true)
            }
        }
    }

    private func reservationCard(_ row: ReservationRow) -> some View {
        @Bindable var viewModel = viewModel
        let busy = viewModel.isBusy(row)
        let actions = ReservationAction.available(for: row, busy: busy)
        let cardShape = RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
        return ReservationSwipeCard(id: row.id, actions: actions, openID: $viewModel.swipedRowID) { action in
            perform(action, on: row)
        } content: {
            BookRowCard(row.ref, meta: row.meta,
                        metaColor: row.isReady ? Theme.Colors.textPrimary : Theme.Colors.textSecondary,
                        chips: [statusChip(row)], highlighted: row.isReady)
                .overlay(alignment: .trailing) {
                    if busy {
                        ProgressView()
                            .tint(Theme.Colors.textSecondary)
                            .padding(.trailing, Theme.Spacing.xl)
                    }
                }
                .contentShape(cardShape)
                .contentShape(.contextMenuPreview, cardShape)
                .onTapGesture { tapReservation(row) }
                .contextMenu {
                    ForEach(actions) { action in
                        Button(action.menuTitle, systemImage: action.systemImage, role: action.role) {
                            perform(action, on: row)
                        }
                    }
                }
        }
        .opacity(busy ? 0.7 : 1)
        .allowsHitTesting(!busy)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityText([row.ref.title, Formatting.byline(row.ref.author), row.statusLabel,
                                               row.meta?.replacingOccurrences(of: "\n", with: ", "),
                                               busy ? "Updating" : nil]))
        .accessibilityAddTraits(.isButton)
        .accessibilityAction { open(row.ref) }
        .accessibilityActions {
            ForEach(actions) { action in
                Button(action.menuTitle) { perform(action, on: row) }
            }
        }
        .cancelReservationConfirmation(for: row, confirming: $viewModel.confirmingCancel) { row in
            Task { await viewModel.cancel(row, model: model) }
        }
    }

    private func perform(_ action: ReservationAction, on row: ReservationRow) {
        switch action {
        case .edit: viewModel.edit(row)
        case .cancel: viewModel.requestCancel(row)
        }
    }

    /// Opens the book, or only closes the swipe actions while a card shows them.
    private func tapReservation(_ row: ReservationRow) {
        if viewModel.swipedRowID != nil {
            closeSwipe()
        } else {
            open(row.ref)
        }
    }

    private func statusChip(_ row: ReservationRow) -> Chip {
        if row.isReady {
            return Chip(text: row.statusLabel, tone: .success, systemImage: "checkmark.circle.fill")
        }
        let inTransit = row.statusLabel.range(of: "transit", options: .caseInsensitive) != nil
        return Chip(text: row.statusLabel, tone: inTransit ? .warning : .neutral,
                    systemImage: inTransit ? "shippingbox" : "clock")
    }

    // MARK: Signed out

    private var signedOut: some View {
        EmptyStateView(systemImage: "person.crop.circle.badge.questionmark", title: "Sign in to see your books",
                       message: "Your loans and reservations at \(model.libraryName) appear here.",
                       actionTitle: "Sign in") { model.loginNeeded = true }
            .frame(minHeight: Self.stateHeight + 80)
    }

    // MARK: Helpers

    private static let stateHeight: CGFloat = 380

    private func open(_ ref: BookRef) {
        model.push(.detail(ref), on: .borrowed)
    }

    private func closeSwipe() {
        guard viewModel.swipedRowID != nil else { return }
        withAnimation(SwipeReveal.animation) { viewModel.swipedRowID = nil }
    }

    /// Closes an open card once the list has scrolled a few points from where it was when the
    /// card opened (a pull to refresh counts).
    private func closeSwipeIfScrolled(to y: CGFloat) {
        defer { scrollMark.y = y }
        guard viewModel.swipedRowID != nil else {
            scrollMark.openedAt = nil
            return
        }
        let openedAt = scrollMark.openedAt ?? scrollMark.y
        scrollMark.openedAt = openedAt
        if abs(y - openedAt) > 6 { closeSwipe() }
    }

    private func accessibilityText(_ parts: [String?]) -> String {
        parts.compactMap(\.self).filter { !$0.isEmpty }.joined(separator: ", ")
    }
}

/// The list's scroll position, updated every frame: a plain class, so writing it doesn't
/// re-render the screen.
private final class ScrollMark {
    var y: CGFloat = 0
    /// Where the list was when the open card's swipe actions appeared.
    var openedAt: CGFloat?
}

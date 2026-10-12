import SwiftUI

/// "Your Books" (docs/native-apps.md §6 Borrowed): Loans | Reservations. Loans are sorted by due
/// date with the danger colour when due within 3 days or overdue; reservations show "Ready for
/// pickup" first, then pending, each with Cancel (confirmed) and Edit (sheet). Pull to refresh;
/// reloads on sign-in/out and library change.
struct BorrowedView: View {
    @Environment(AppModel.self) private var model
    @State private var viewModel = BorrowedViewModel()
    @State private var lastLoad: Date?

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
                    segmentPicker
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
        }
        .defaultScrollAnchor(BorrowedDebugHooks.initialScrollAnchor)
        .screenBackground()
        .refreshable { await load(keepContent: true) }
        .task(id: model.reloadKey) { await load(keepContent: false) }
        .onChange(of: model.selectedTab) { _, tab in
            // Back on the tab after a while (e.g. a reservation placed from a book): refresh quietly.
            guard tab == .borrowed, model.isSignedIn, let lastLoad, Date.now.timeIntervalSince(lastLoad) > 30 else { return }
            Task { await load(keepContent: true) }
        }
        .onChange(of: viewModel.notice) { _, notice in
            if let notice { AccessibilityNotification.Announcement(notice.message).post() }
        }
        .toolbar(.hidden, for: .navigationBar)
        .sheet(item: $viewModel.editing, onDismiss: { viewModel.reportDeferredError(model) }) { row in
            EditReservationSheet(row: row) { outcome in
                Task { await viewModel.finishEdit(outcome, row: row, model: model) }
            }
            .environment(model)
            .presentationBackground(Theme.Colors.background)
        }
    }

    private func load(keepContent: Bool) async {
        await viewModel.load(model, keepContent: keepContent)
        lastLoad = .now
        #if DEBUG
        await BorrowedDebugHooks.run(viewModel, model: model)
        #endif
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
        case .loans: viewModel.loans.value?.count
        case .reservations: viewModel.reservations.value?.count
        }
        return count.map { "\(segment.title) (\($0))" } ?? segment.title
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
        switch viewModel.loans {
        case .idle, .loading:
            LoadingView(message: "Loading your loans…")
                .frame(minHeight: Self.stateHeight)
        case .failed(let error):
            ErrorStateView(error: error) { Task { await load(keepContent: false) } }
                .frame(minHeight: Self.stateHeight)
        case .loaded(let rows) where rows.isEmpty:
            EmptyStateView(systemImage: "books.vertical", title: "No loans right now",
                           message: "Books you borrow from \(model.libraryName) show up here with their due dates.")
                .frame(minHeight: Self.stateHeight)
        case .loaded(let rows):
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
        switch viewModel.reservations {
        case .idle, .loading:
            LoadingView(message: "Loading your reservations…")
                .frame(minHeight: Self.stateHeight)
        case .failed(let error):
            ErrorStateView(error: error) { Task { await load(keepContent: false) } }
                .frame(minHeight: Self.stateHeight)
        case .loaded(let sections) where sections.isEmpty:
            EmptyStateView(systemImage: "bookmark.circle", title: "No reservations",
                           message: "Tap Reserve on any book and follow it here until it's ready to collect.")
                .frame(minHeight: Self.stateHeight)
        case .loaded(let sections):
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
        }
    }

    private func reservationCard(_ row: ReservationRow) -> some View {
        @Bindable var viewModel = viewModel
        let busy = viewModel.isBusy(row)
        var actions: [BookRowCard.Action] = []
        if row.cancelHref != nil {
            actions.append(.init(title: "Cancel") { viewModel.confirmingCancel = row })
        }
        if row.editHref != nil {
            actions.append(.init(title: "Edit") { viewModel.edit(row) })
        }
        return BookRowCard(row.ref, meta: row.meta,
                           metaColor: row.isReady ? Theme.Colors.textPrimary : Theme.Colors.textSecondary,
                           chips: [statusChip(row)], actions: busy ? [] : actions, highlighted: row.isReady)
            .overlay(alignment: .trailing) {
                if busy {
                    ProgressView()
                        .tint(Theme.Colors.textSecondary)
                        .padding(.trailing, Theme.Spacing.xl)
                }
            }
            .opacity(busy ? 0.7 : 1)
            .allowsHitTesting(!busy)
            .contentShape(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous))
            .onTapGesture { open(row.ref) }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(accessibilityText([row.ref.title, Formatting.byline(row.ref.author), row.statusLabel,
                                                   row.meta?.replacingOccurrences(of: "\n", with: ", "),
                                                   busy ? "Updating" : nil]))
            .accessibilityAddTraits(.isButton)
            .accessibilityAction { open(row.ref) }
            .accessibilityActions {
                if !busy, row.cancelHref != nil {
                    Button("Cancel reservation") { viewModel.confirmingCancel = row }
                }
                if !busy, row.editHref != nil {
                    Button("Edit reservation") { viewModel.edit(row) }
                }
            }
            .cancelReservationConfirmation(for: row, confirming: $viewModel.confirmingCancel) { row in
                Task { await viewModel.cancel(row, model: model) }
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

    private func accessibilityText(_ parts: [String?]) -> String {
        parts.compactMap(\.self).filter { !$0.isEmpty }.joined(separator: ", ")
    }
}

import LibreSpydusCore
import SwiftUI

/// Book detail for `Route.detail` (docs/native-apps.md §6): navy header with cover, title,
/// availability, format chips, "Reserve" and the favourite bookmark; a white sheet with the
/// synopsis, subject chips, details and related carousels. When the saved account has the edition
/// on screen on loan or reserved, a status pill ("On loan · Due 14 Oct") takes Reserve's place and
/// opens Your Books.
struct BookDetailView: View {
    @Environment(AppModel.self) private var app
    @State private var model: BookDetailModel
    @State private var reserveRequest: ReserveRequest?
    /// "Reserve" tapped while signed out: reserve this format once signed in.
    @State private var pendingBorrow: FormatRef?
    /// A reserve-sheet failure that needs a root sheet (sign in again, verification). It's reported
    /// after the reserve sheet is gone: two sheets can't be up at once.
    @State private var deferredError: (any Error)?

    private static let bottomID = "detail-bottom"

    /// How far the Reserve row reaches into the white sheet (half the button height).
    private let actionsOverlap: CGFloat = 25

    init(ref: BookRef) {
        _model = State(initialValue: BookDetailModel(ref: ref))
    }

    var body: some View {
        GeometryReader { proxy in
            ScrollViewReader { scroller in
                ScrollView {
                    // At least a screen tall, so the white sheet reaches the tab bar (the proxy's
                    // height already excludes the tab bar: TabStack applies `tabBarClearance()`).
                    content
                        .frame(minHeight: max(proxy.size.height, 0), alignment: .top)
                }
                #if DEBUG
                .task(id: model.record.value != nil) {
                    guard model.record.value != nil, let actions = DetailDebugActions.take() else { return }
                    await apply(actions, scroller: scroller)
                }
                #endif
            }
        }
        .screenBackground()
        .navigationTitle("")
        .navigationBarTitleDisplayMode(.inline)
        .navyNavigationBar()
        .task(id: app.verificationGeneration) {
            await model.load(backend: app.backend, report: app.report)
            model.retryFailedRelated(backend: app.backend)
        }
        .task(id: model.selectedFormat?.availabilityUrl) {
            await model.loadAvailability(backend: app.backend, report: app.report)
        }
        .task(id: model.copiesSource) {
            await model.loadCopies(backend: app.backend, report: app.report)
        }
        .onChange(of: reserveRequest != nil) { _, reserving in
            // A login/verification sheet from a background refresh waits until this one closes.
            if reserving { app.isPresentingFeatureSheet = true }
        }
        .sheet(item: $reserveRequest, onDismiss: {
            app.isPresentingFeatureSheet = false
            reportDeferredError()
        }) { request in
            ReserveSheet(request: request) { error in
                deferredError = error
                if case .NotLoggedIn = error as? SpydusError { pendingBorrow = request.format }
                reserveRequest = nil
            }
            .environment(app)
        }
        .onChange(of: app.isSignedIn) { _, signedIn in
            if signedIn { continuePendingBorrow() }
        }
        .onChange(of: app.loginNeeded) { _, needed in
            // The login sheet was closed without signing in: forget the borrow.
            if !needed, !app.isSignedIn { pendingBorrow = nil }
        }
    }

    private var content: some View {
        VStack(spacing: 0) {
            DetailHeader(model: model, backend: app.backend, report: app.report)
            actions
                .zIndex(1)
                .padding(.bottom, -actionsOverlap)
            DetailSheetContent(model: model, retry: reload, startRelated: startRelated, retryCopies: retryCopies)
                .padding(.top, actionsOverlap + Theme.Spacing.l)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .background(alignment: .top) {
                    // Drawn past the bottom so pushing up at the end shows white, not navy.
                    UnevenRoundedRectangle(topLeadingRadius: Theme.Radius.sheet,
                                           topTrailingRadius: Theme.Radius.sheet, style: .continuous)
                        .fill(Theme.Colors.sheet)
                        .padding(.bottom, -1000)
                }
            Color.clear
                .frame(height: 1)
                .id(Self.bottomID)
        }
    }

    // MARK: Reserve + bookmark

    /// The patron's loan or reservation of the edition on screen, recomputed as the saved account
    /// or the selected format changes.
    private var held: HeldStatus? {
        model.holding(in: app.isSignedIn ? app.account.snapshot : nil).map { HeldStatus($0) }
    }

    /// Signed in with loans or reservations, and the edition on screen isn't identified yet.
    private var awaitingHolding: Bool {
        guard app.isSignedIn, let snapshot = app.account.snapshot, model.isIdentifyingEdition else { return false }
        let reservations = snapshot.reservations
        return !(snapshot.loans.items.isEmpty && reservations.ready.items.isEmpty && reservations.pending.items.isEmpty)
    }

    private var actions: some View {
        HStack(spacing: Theme.Spacing.m) {
            if let held {
                HeldStatusPill(status: held) { app.openYourBooks(on: held.segment) }
            } else if let format = model.reservableFormat {
                PrimaryPillButton(title: "Reserve", fillsWidth: true) { borrow(format) }
                    .accessibilityHint(reserveHint(format))
                    // Held or not isn't known yet: keep the slot, show nothing.
                    .opacity(awaitingHolding ? 0 : 1)
                    .disabled(awaitingHolding)
                    .accessibilityHidden(awaitingHolding)
                    .animation(.easeOut(duration: 0.15), value: awaitingHolding)
            } else {
                Spacer(minLength: 0)
            }
            if app.isSignedIn {
                pickButton
            } else {
                favouriteButton
            }
        }
        .padding(.horizontal, Theme.Spacing.xl)
    }

    /// Signed out: the local favourites (§5.3).
    private var favouriteButton: some View {
        let isFavourite = app.favourites.contains(id: model.ref.id)
        return CircleIconButton(systemImage: isFavourite ? "bookmark.fill" : "bookmark",
                                accessibilityLabel: isFavourite ? "Remove from favourites" : "Add to favourites") {
            app.favourites.toggle(model.favouriteRef)
        }
        .accessibilityAddTraits(isFavourite ? .isSelected : [])
        .sensoryFeedback(.selection, trigger: isFavourite)
    }

    /// Signed in: My Picks on the library account (§5.11). Flips at once; the store flips it back
    /// if the library refuses. Waits (disabled) until the book's IRN is known.
    private var pickButton: some View {
        let irn = model.pickIrn
        let isPicked = irn.map(app.picks.contains) ?? false
        return CircleIconButton(systemImage: isPicked ? "bookmark.fill" : "bookmark",
                                accessibilityLabel: isPicked ? PicksText.remove : PicksText.save) {
            guard let irn, !app.picks.isChanging(irn) else { return }
            Task { await app.picks.setPicked(!isPicked, irn: irn) }
        }
        .disabled(irn == nil)
        .opacity(irn == nil ? 0.5 : 1)
        .accessibilityAddTraits(isPicked ? .isSelected : [])
        .sensoryFeedback(.selection, trigger: isPicked)
    }

    /// "Reserve the book" / "Sign in to reserve the book"; an edition-level item's format has no
    /// name, so "this item".
    private func reserveHint(_ format: FormatRef) -> String {
        let what = format.isEditionLevel ? "this item" : "the \(format.displayLabel.lowercased())"
        return app.isSignedIn ? "Reserve \(what)" : "Sign in to reserve \(what)"
    }

    private func borrow(_ format: FormatRef) {
        if app.isSignedIn {
            reserveRequest = ReserveRequest(format: format, book: model.ref)
        } else {
            pendingBorrow = format
            app.loginNeeded = true
        }
    }

    private func continuePendingBorrow() {
        guard let format = pendingBorrow else { return }
        pendingBorrow = nil
        Task {
            // Let the login sheet finish dismissing before this screen presents its own.
            try? await Task.sleep(for: .milliseconds(650))
            // Signed in to an account that has this edition already: the pill shows instead.
            guard app.isSignedIn, app.rootSheet == nil, held == nil else { return }
            reserveRequest = ReserveRequest(format: format, book: model.ref)
        }
    }

    private func reportDeferredError() {
        guard let error = deferredError else { return }
        deferredError = nil
        app.report(error)
    }

    // MARK: Loading

    private func reload() {
        Task { await model.load(backend: app.backend, report: app.report) }
    }

    private func startRelated() {
        model.startRelated(backend: app.backend)
    }

    private func retryCopies() {
        Task { await model.loadCopies(backend: app.backend, report: app.report) }
    }

    #if DEBUG
    /// `-LSDetailAction` screenshot hooks (DetailDebugActions).
    private func apply(_ actions: DetailDebugActions, scroller: ScrollViewProxy) async {
        func wait(while condition: () -> Bool) async {
            for _ in 0..<120 where condition() {
                try? await Task.sleep(for: .milliseconds(250))
            }
        }
        await wait { model.formatLookup == .pending || model.formatLookup == .searching }
        if let code = actions.formatCode,
           let index = model.formats.firstIndex(where: { $0.code.caseInsensitiveCompare(code) == .orderedSame }) {
            model.select(formatAt: index)
        }
        if actions.favourite {
            if !app.isSignedIn {
                if !app.favourites.contains(id: model.ref.id) { app.favourites.toggle(model.favouriteRef) }
            } else if app.isDemo, let irn = model.pickIrn, !app.picks.contains(irn) {
                // Demo only: never saves to a real account's My Picks.
                await app.picks.setPicked(true, irn: irn)
            }
        }
        if actions.scrollToRelated || actions.scrollToBottom || actions.openRelated {
            model.startRelated(backend: app.backend)
            await wait { model.hasRelatedPanels && (model.related.isEmpty || model.related.values.contains { $0.isLoading }) }
            try? await Task.sleep(for: .milliseconds(800)) // covers
        }
        if actions.scrollToRelated {
            withAnimation { scroller.scrollTo(DetailSheetContent.relatedID, anchor: .top) }
        }
        if actions.scrollToBottom {
            withAnimation { scroller.scrollTo(Self.bottomID, anchor: .bottom) }
        }
        if actions.scrollToCopies, model.isEditionLevel {
            await wait { model.copiesSource == nil || model.copies.value == nil }
            try? await Task.sleep(for: .milliseconds(300))
            withAnimation { scroller.scrollTo(DetailSheetContent.copiesID, anchor: .top) }
        }
        if actions.openRelated,
           let first = RelatedPanel.allCases.lazy.compactMap({ model.related[$0]?.value?.first }).first {
            app.push(.detail(first))
        }
        // Demo mode, or signed out (which only raises the login sheet): never a real reservation.
        if actions.reserve, app.isDemo || !app.isSignedIn, let format = model.reservableFormat {
            if app.isSignedIn {
                reserveRequest = ReserveRequest(format: format, book: model.ref, autoConfirm: actions.confirm)
            } else {
                borrow(format)
            }
        }
    }
    #endif
}

/// The navy top: cover, title, byline, availability line and format chips.
private struct DetailHeader: View {
    let model: BookDetailModel
    let backend: any LibraryBackend
    let report: BookDetailModel.Reporter

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.l) {
            HStack(alignment: .top, spacing: Theme.Spacing.l) {
                CoverImage(url: model.ref.coverUrl, title: model.ref.title, author: model.ref.author,
                           width: Theme.Cover.detailWidth)
                VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                    if !model.ref.title.isEmpty {
                        Text(DetailText.displayTitle(model.ref.title))
                            .themeFont(.title)
                            .foregroundStyle(Theme.Colors.textPrimary)
                            .lineLimit(6)
                            .fixedSize(horizontal: false, vertical: true)
                            .accessibilityAddTraits(.isHeader)
                    }
                    if let byline = Formatting.byline(model.ref.author) {
                        Text(byline)
                            .themeFont(.body)
                            .foregroundStyle(Theme.Colors.textSecondary)
                            .lineLimit(2)
                    }
                    if let year = model.ref.year {
                        Text(year)
                            .themeFont(.caption)
                            .foregroundStyle(Theme.Colors.textMuted)
                    }
                    availability
                        .padding(.top, Theme.Spacing.xs)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            if model.formats.count > 1 {
                formatChips
            }
        }
        .padding(.horizontal, Theme.Spacing.xl)
        .padding(.top, Theme.Spacing.s)
        .padding(.bottom, Theme.Spacing.xl)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private var availability: some View {
        switch model.headerAvailability {
        case .loading(let text):
            HStack(spacing: Theme.Spacing.s) {
                ProgressView()
                    .controlSize(.small)
                    .tint(Theme.Colors.textSecondary)
                Text(text)
            }
            .themeFont(.caption)
            .foregroundStyle(Theme.Colors.textSecondary)
            .accessibilityElement(children: .combine)
        case .line(let line):
            Label {
                Text(line.text)
            } icon: {
                Image(systemName: icon(for: line.tone))
            }
            .themeFont(.bodyStrong)
            .foregroundStyle(color(for: line.tone))
            .accessibilityLabel("Availability: \(line.text)")
        case .availabilityFailed:
            retryLine("Couldn't check availability") {
                await model.loadAvailability(backend: backend, report: report)
            }
        case .lookupFailed:
            retryLine("Couldn't look up availability") {
                await model.refindFormats(backend: backend, report: report)
            }
        case .checkWebsite:
            Label("Check availability on the library website", systemImage: "globe")
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textSecondary)
        case .none:
            EmptyView()
        }
    }

    private func retryLine(_ text: String, action: @escaping @MainActor () async -> Void) -> some View {
        HStack(spacing: Theme.Spacing.s) {
            Text(text)
                .foregroundStyle(Theme.Colors.textSecondary)
            Button("Retry") { Task { await action() } }
                .foregroundStyle(Theme.Colors.primary)
                .buttonStyle(.borderless)
                .accessibilityLabel("Retry: \(text)")
        }
        .themeFont(.caption)
    }

    private var formatChips: some View {
        FlowRow(spacing: Theme.Spacing.s) {
            ForEach(Array(model.formats.enumerated()), id: \.offset) { index, format in
                let selected = index == model.selectedFormatIndex
                Button {
                    model.select(formatAt: index)
                } label: {
                    Chip(format: format, selected: selected)
                        .padding(.vertical, Theme.Spacing.xs)
                        .contentShape(Rectangle())
                }
                .buttonStyle(PressScaleStyle())
                .accessibilityLabel(format.displayLabel)
                .accessibilityHint("Shows availability for this format")
                .accessibilityAddTraits(selected ? .isSelected : [])
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Formats")
    }

    private func icon(for tone: AvailabilityLine.Tone) -> String {
        switch tone {
        case .available: "checkmark.circle.fill"
        case .normal: "books.vertical.fill"
        case .digital: "cloud.fill"
        case .muted: "questionmark.circle"
        }
    }

    private func color(for tone: AvailabilityLine.Tone) -> Color {
        switch tone {
        case .available: Theme.Colors.success
        case .normal: Theme.Colors.textPrimary
        case .digital: Theme.Colors.digital
        case .muted: Theme.Colors.textSecondary
        }
    }
}

/// Reserve's slot when the patron already has the edition: a status, not a primary action
/// (elevated surface, tinted icon and text), that opens Your Books on the loan or reservation.
private struct HeldStatusPill: View {
    let status: HeldStatus
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: Theme.Spacing.s) {
                Image(systemName: status.systemImage)
                Text(status.text)
                    .themeFont(.bodyStrong)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
            }
            .foregroundStyle(tint)
            .padding(.horizontal, Theme.Spacing.l)
            .frame(minHeight: 50)
            .frame(maxWidth: .infinity)
            .background(Capsule().fill(Theme.Colors.surfaceElevated))
            .overlay(Capsule().strokeBorder(tint.opacity(0.35), lineWidth: 1))
            .contentShape(Capsule())
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(status.accessibilityLabel)
        .accessibilityHint("Opens Your Books")
        .accessibilityAddTraits(.isButton)
    }

    private var tint: Color {
        switch status.tone {
        case .loan: Theme.Colors.textPrimary
        case .overdue: Theme.Colors.danger
        case .ready: Theme.Colors.success
        case .reserved: Theme.Colors.warning
        }
    }
}

/// What the reserve sheet reserves.
struct ReserveRequest: Identifiable, Hashable {
    var format: FormatRef
    var book: BookRef
    /// DEBUG screenshot hook (`-LSDetailAction confirm`): confirm once the options load (demo only).
    var autoConfirm = false

    var id: String { format.reserveUrl ?? format.code }
}

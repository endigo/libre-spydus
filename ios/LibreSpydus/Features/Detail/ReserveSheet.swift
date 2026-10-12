import LibreSpydusCore
import Observation
import SwiftUI

/// The reserve flow for one format (docs/native-apps.md §6): `reserveOptions` → pickup location,
/// availability, expiry choice → Confirm → `placeReservation` → result banner.
///
/// Workers exercise it in demo mode only (§2): never against a real library.
struct ReserveSheet: View {
    let request: ReserveRequest
    /// A failure that needs a root sheet (sign in again, verification). The parent dismisses this
    /// sheet and reports it, since two sheets can't be up at once.
    let onNeedsRootSheet: (any Error) -> Void

    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @State private var model = ReserveModel()

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: Theme.Spacing.xl) {
                    bookSummary
                    content
                }
                .padding(Theme.Spacing.xl)
            }
            .safeAreaInset(edge: .bottom, spacing: 0) {
                footer
            }
            .onChange(of: model.result?.ok) { _, ok in
                // The new hold shows up in Borrowed without a manual refresh.
                if ok == true { app.accountDidChange() }
            }
            .screenBackground()
            .navigationTitle("Reserve")
            .navigationBarTitleDisplayMode(.inline)
            .navyNavigationBar()
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button {
                        dismiss()
                    } label: {
                        Image(systemName: "xmark")
                    }
                    .tint(Theme.Colors.textPrimary)
                    .disabled(model.isSubmitting)
                    .accessibilityLabel("Close")
                }
            }
        }
        .presentationDetents([.large])
        .presentationDragIndicator(.visible)
        .presentationBackground(Theme.Colors.background)
        .interactiveDismissDisabled(model.isSubmitting)
        .task {
            await model.loadOptions(url: request.format.reserveUrl, backend: app.backend, session: app.session,
                                    libraryName: app.libraryName, needsRootSheet: onNeedsRootSheet)
            #if DEBUG
            if request.autoConfirm, app.isDemo, model.options.value?.canReserve == true {
                try? await Task.sleep(for: .seconds(1))
                await model.confirm(url: request.format.reserveUrl, backend: app.backend, session: app.session,
                                    libraryName: app.libraryName, needsRootSheet: onNeedsRootSheet)
            }
            #endif
        }
    }

    private var bookSummary: some View {
        HStack(alignment: .top, spacing: Theme.Spacing.l) {
            CoverImage(url: request.book.coverUrl, title: request.book.title, author: request.book.author,
                       width: Theme.Cover.smallWidth)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(DetailText.displayTitle(request.book.title))
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .lineLimit(3)
                if let byline = Formatting.byline(request.book.author) {
                    Text(byline)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .lineLimit(1)
                }
                Chip(format: request.format)
                    .padding(.top, Theme.Spacing.xs)
            }
        }
        .accessibilityElement(children: .combine)
    }

    @ViewBuilder
    private var content: some View {
        switch model.options {
        case .idle, .loading:
            LoadingView(message: "Checking reservation options…")
                .frame(minHeight: 160)
        case .failed(let error):
            ErrorStateView(error: error) { reload() }
        case .loaded(let options):
            if options.canReserve {
                form(options)
            } else {
                Banner(message: options.message.nonEmpty ?? "This title can't be reserved right now.", tone: .info)
            }
        }
    }

    /// The result banner and Confirm / Done, pinned to the bottom so they're always in view.
    @ViewBuilder
    private var footer: some View {
        if let options = model.options.value {
            VStack(spacing: Theme.Spacing.m) {
                if let result = model.result {
                    Banner(message: Self.message(for: result), tone: result.ok ? .success : .error)
                }
                if let error = model.submitError {
                    Banner(message: error.message, tone: .error)
                }
                if !options.canReserve || model.isComplete || model.result?.ok == false {
                    PrimaryPillButton(title: model.isComplete ? "Done" : "Close", fillsWidth: true) { dismiss() }
                } else {
                    PrimaryPillButton(title: "Confirm reservation", isLoading: model.isSubmitting, fillsWidth: true) {
                        Task {
                            await model.confirm(url: request.format.reserveUrl, backend: app.backend, session: app.session,
                                                libraryName: app.libraryName, needsRootSheet: onNeedsRootSheet)
                        }
                    }
                }
            }
            .padding(.horizontal, Theme.Spacing.xl)
            .padding(.top, Theme.Spacing.m)
            .padding(.bottom, Theme.Spacing.l)
            .background(Theme.Colors.background)
            .animation(.easeOut(duration: 0.2), value: model.result)
        }
    }

    @ViewBuilder
    private func form(_ options: ReserveOptions) -> some View {
        @Bindable var model = model
        VStack(spacing: 0) {
            infoRow("Pick up at", value: options.pickupLocation.nonEmpty.map(Self.titleCased) ?? "Your usual library",
                    systemImage: "mappin.and.ellipse")
            if let available = options.availableSummary.nonEmpty {
                divider
                infoRow("Availability", value: available, systemImage: "books.vertical")
            }
            if !options.expiryChoices.isEmpty {
                divider
                HStack(spacing: Theme.Spacing.m) {
                    Image(systemName: "calendar")
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .frame(width: 24)
                        .accessibilityHidden(true)
                    Text("Not needed after")
                        .themeFont(.body)
                        .foregroundStyle(Theme.Colors.textSecondary)
                    Spacer(minLength: Theme.Spacing.s)
                    Picker("Not needed after", selection: $model.selectedDays) {
                        ForEach(options.expiryChoices, id: \.days) { choice in
                            Text(choice.label).tag(Optional(choice.days))
                        }
                    }
                    .labelsHidden()
                    .pickerStyle(.menu)
                    .tint(Theme.Colors.textPrimary)
                    .disabled(model.isSubmitting || model.isComplete)
                }
                .padding(.vertical, Theme.Spacing.s)
            }
        }
        .padding(.horizontal, Theme.Spacing.l)
        .background(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous).fill(Theme.Colors.surface))
    }

    private var divider: some View {
        Rectangle().fill(Theme.Colors.divider).frame(height: 1)
    }

    private func infoRow(_ label: String, value: String, systemImage: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: Theme.Spacing.m) {
            Image(systemName: systemImage)
                .foregroundStyle(Theme.Colors.textSecondary)
                .frame(width: 24)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 2) {
                Text(label)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                Text(value)
                    .themeFont(.body)
                    .foregroundStyle(Theme.Colors.textPrimary)
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, Theme.Spacing.m)
        .accessibilityElement(children: .combine)
    }

    private func reload() {
        Task {
            await model.loadOptions(url: request.format.reserveUrl, backend: app.backend, session: app.session,
                                    libraryName: app.libraryName, needsRootSheet: onNeedsRootSheet)
        }
    }

    static func message(for result: ReserveResult) -> String {
        result.message.nonEmpty ?? (result.ok ? "Reservation placed." : "The library couldn't place this reservation.")
    }

    /// "RIVERTON LIBRARY" → "Riverton Library"; mixed-case names are left alone.
    static func titleCased(_ text: String) -> String {
        text == text.uppercased() ? text.capitalized : text
    }
}

/// The reserve sheet's state: options, the chosen expiry, and one submission at a time.
@MainActor
@Observable
final class ReserveModel {
    private(set) var options: Loadable<ReserveOptions> = .idle
    /// `ExpiryChoice.days` sent as `xdays`; the first choice by default.
    var selectedDays: String?
    private(set) var isSubmitting = false
    /// The library's answer (`ok: false` is data to show, not an error).
    private(set) var result: ReserveResult?
    /// A thrown failure on Confirm (network, unexpected page); Confirm can be tried again.
    private(set) var submitError: UserFacingError?

    /// Placed: Confirm is replaced by Done, so a reservation can't be sent twice.
    var isComplete: Bool { result?.ok == true }

    func loadOptions(url: String?, backend: any LibraryBackend, session: String?, libraryName: String,
                     needsRootSheet: (any Error) -> Void) async {
        guard let url = url.nonEmpty else {
            options = .failed(UserFacingError(kind: .unexpected, message: "This format can't be reserved from the app."))
            return
        }
        guard let session else {
            needsRootSheet(SpydusError.NotLoggedIn(message: "not logged in"))
            return
        }
        options = .loading
        do {
            let loaded = try await backend.reserveOptions(url: url, session: session)
            selectedDays = loaded.expiryChoices.first?.days
            options = .loaded(loaded)
        } catch {
            let shown = UserFacingError(error, libraryName: libraryName)
            if Self.needsRootSheet(shown) {
                needsRootSheet(error)
            } else if !(error is CancellationError) {
                options = .failed(shown)
            }
        }
    }

    func confirm(url: String?, backend: any LibraryBackend, session: String?, libraryName: String,
                 needsRootSheet: (any Error) -> Void) async {
        guard !isSubmitting, !isComplete, let url = url.nonEmpty else { return }
        guard let session else {
            needsRootSheet(SpydusError.NotLoggedIn(message: "not logged in"))
            return
        }
        isSubmitting = true
        submitError = nil
        result = nil
        defer { isSubmitting = false }
        do {
            result = try await backend.placeReservation(url: url, session: session, xdays: selectedDays, pickup: nil)
        } catch {
            let shown = UserFacingError(error, libraryName: libraryName)
            if Self.needsRootSheet(shown) {
                needsRootSheet(error)
            } else {
                submitError = shown
            }
        }
    }

    private static func needsRootSheet(_ error: UserFacingError) -> Bool {
        error.kind == .notLoggedIn || error.kind == .verificationRequired
    }
}

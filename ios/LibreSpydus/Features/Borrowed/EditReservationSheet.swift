import LibreSpydusCore
import SwiftUI

/// The edit sheet's form state and the `EditOverrides` it sends. Mirrors the reservation form:
/// pickup branch, "Delay until" (SDTE) and "Not required after" (XDTE). A date is sent only when
/// it changed; switching off a date the form had sets the matching clear flag.
struct EditDraft: Equatable {
    struct DateField: Equatable {
        /// The form's value as the library wrote it ("2026-11-01"); nil when unset.
        var original: String?
        var isOn: Bool
        var date: Date

        init(original: String?, defaultDate: Date, calendar: Calendar) {
            self.original = original.nonEmpty
            let parsed = self.original.flatMap { DueDate.parse($0, calendar: calendar) }
            isOn = parsed != nil
            date = parsed ?? defaultDate
        }

        /// The new value to send, or nil when it's off or unchanged.
        func changedValue(calendar: Calendar) -> String? {
            guard isOn else { return nil }
            let value = EditDraft.formValue(date, like: original, calendar: calendar)
            return value == original ? nil : value
        }

        /// The form had a date and it was switched off.
        var isCleared: Bool { !isOn && original != nil }

        func hasChanges(calendar: Calendar) -> Bool {
            isCleared || changedValue(calendar: calendar) != nil
        }
    }

    /// Selected `PickupChoice.code`; nil when the form offers no branches (digital holds).
    var pickup: String?
    let initialPickup: String?
    var delay: DateField
    var expiry: DateField

    /// From the form. The pickup starts on the form's selection, else the branch the reservation
    /// lists (`currentPickup`), else the first choice.
    init(options: EditOptions, currentPickup: String? = nil, today: Date = .now, calendar: Calendar = .libraryDates) {
        let choices = options.pickupChoices
        let selected = options.pickupSelected.flatMap { code in choices.first { $0.code == code }?.code }
        let listed = currentPickup.flatMap { name in
            choices.first { $0.name.caseInsensitiveCompare(name) == .orderedSame }?.code
        }
        pickup = selected ?? listed ?? choices.first?.code
        initialPickup = pickup
        let start = calendar.startOfDay(for: today)
        delay = DateField(original: options.delayUntil,
                          defaultDate: calendar.date(byAdding: .day, value: 7, to: start) ?? start, calendar: calendar)
        expiry = DateField(original: options.notRequiredAfter,
                           defaultDate: calendar.date(byAdding: .month, value: 3, to: start) ?? start, calendar: calendar)
    }

    func hasChanges(calendar: Calendar = .libraryDates) -> Bool {
        pickup != initialPickup || delay.hasChanges(calendar: calendar) || expiry.hasChanges(calendar: calendar)
    }

    /// What `editReservation` sends. The pickup is always the one shown (the form's own value
    /// when unchanged), like the web app.
    func overrides(calendar: Calendar = .libraryDates) -> EditOverrides {
        EditOverrides(pickup: pickup,
                      delayUntil: delay.changedValue(calendar: calendar),
                      notRequiredAfter: expiry.changedValue(calendar: calendar),
                      clearDelay: delay.isCleared,
                      clearExpiry: expiry.isCleared)
    }

    /// `date` in the form's own format: "dd/MM/yyyy" when the form wrote dates that way, else
    /// the HTML date input's "yyyy-MM-dd" (what Spydus's edit form uses).
    static func formValue(_ date: Date, like sample: String?, calendar: Calendar = .libraryDates) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.timeZone = calendar.timeZone
        formatter.dateFormat = sample?.contains("/") == true ? "dd/MM/yyyy" : "yyyy-MM-dd"
        return formatter.string(from: date)
    }
}

/// "Edit reservation": loads the form (`editOptions`), lets the user change the pickup branch
/// and the two dates, then sends `editReservation`. A refusal (`ok: false`) stays in the sheet;
/// success closes it and the Borrowed list shows the message.
struct EditReservationSheet: View {
    enum Outcome {
        /// Closed without saving.
        case closed
        /// The library accepted the change.
        case saved(ReserveResult)
        /// Signed out or verification needed: reported once the sheet is gone.
        case failed(any Error)
    }

    let row: ReservationRow
    let finish: (Outcome) -> Void

    @Environment(AppModel.self) private var model
    /// The library's dates are Gregorian whatever the device's calendar.
    private let calendar = Calendar.libraryDates
    @State private var options: Loadable<EditOptions> = .idle
    @State private var draft: EditDraft?
    @State private var isSaving = false
    /// The library's refusal or a failed save, shown above the form.
    @State private var problem: String?

    var body: some View {
        NavigationStack {
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Theme.Colors.background.ignoresSafeArea())
                .navigationTitle("Edit reservation")
                .navigationBarTitleDisplayMode(.inline)
                .navyNavigationBar()
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Close") { finish(.closed) }
                            .disabled(isSaving)
                    }
                }
        }
        .interactiveDismissDisabled(isSaving)
        .task(id: row.id) { await loadOptions() }
    }

    @ViewBuilder
    private var content: some View {
        switch options {
        case .idle, .loading:
            LoadingView(message: "Loading reservation…")
        case .failed(let error):
            ErrorStateView(error: error) { Task { await loadOptions() } }
        case .loaded(let options):
            if options.canEdit, draft != nil {
                form(options)
            } else {
                EmptyStateView(systemImage: "calendar.badge.exclamationmark", title: "Can't edit this reservation",
                               message: options.message.nonEmpty ?? "This reservation can't be edited right now.")
            }
        }
    }

    // MARK: Form

    private func form(_ options: EditOptions) -> some View {
        Form {
            Section {
                bookHeader
            }
            if let problem {
                Section {
                    Banner(message: problem, tone: .error)
                        .listRowInsets(EdgeInsets())
                        .listRowBackground(Color.clear)
                }
            }
            if !options.pickupChoices.isEmpty {
                Section {
                    pickupPicker(options.pickupChoices)
                } header: {
                    sectionTitle("Pickup location")
                }
            }
            dateSection(title: "Delay until", field: draftBinding(\.delay), earliest: today,
                        footer: "The library won't fill the reservation before this date.")
            dateSection(title: "Not required after", field: draftBinding(\.expiry), earliest: earliestExpiry,
                        footer: "The reservation lapses if it isn't ready by this date.")
            Section {
                PrimaryPillButton(title: "Save changes", isLoading: isSaving, fillsWidth: true) {
                    Task { await save() }
                }
                .disabled(!(draft?.hasChanges(calendar: calendar) ?? false))
                .listRowInsets(EdgeInsets())
                .listRowBackground(Color.clear)
            }
        }
        .scrollContentBackground(.hidden)
        .disabled(isSaving)
        .tint(Theme.Colors.primary)
    }

    private var bookHeader: some View {
        HStack(spacing: Theme.Spacing.m) {
            CoverImage(url: row.ref.coverUrl, title: row.ref.title, author: row.ref.author, width: 44, shadow: false)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(row.ref.title)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .lineLimit(2)
                if let byline = Formatting.byline(row.ref.author) {
                    Text(byline)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .lineLimit(1)
                }
                Text(row.statusLabel)
                    .themeFont(.caption)
                    .foregroundStyle(row.isReady ? Theme.Colors.success : Theme.Colors.textMuted)
            }
        }
        .listRowBackground(Theme.Colors.surface)
        .accessibilityElement(children: .combine)
    }

    @ViewBuilder
    private func pickupPicker(_ choices: [PickupChoice]) -> some View {
        let selection = Binding<String?>(get: { draft?.pickup }, set: { draft?.pickup = $0 })
        let picker = Picker("Pickup location", selection: selection) {
            ForEach(choices, id: \.code) { choice in
                Text(ReservationRow.displayName(choice.name) ?? choice.name)
                    .tag(Optional(choice.code))
            }
        }
        .foregroundStyle(Theme.Colors.textPrimary)
        .listRowBackground(Theme.Colors.surface)
        // A few branches read best as a checklist; a big library system gets its own list.
        if choices.count <= 6 {
            picker.pickerStyle(.inline).labelsHidden()
        } else {
            picker.pickerStyle(.navigationLink)
        }
    }

    /// A date the form already had stays pickable even when it's before `earliest`.
    private func dateSection(title: String, field: Binding<EditDraft.DateField>, earliest: Date,
                             footer: String) -> some View {
        Section {
            Toggle(title, isOn: field.isOn)
                .foregroundStyle(Theme.Colors.textPrimary)
                .listRowBackground(Theme.Colors.surface)
            if field.wrappedValue.isOn {
                DatePicker("Date", selection: field.date, in: min(earliest, field.wrappedValue.date)...,
                           displayedComponents: .date)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .listRowBackground(Theme.Colors.surface)
                    .accessibilityLabel("\(title) date")
            }
        } footer: {
            Text(field.wrappedValue.isCleared ? "Cleared when you save. \(footer)" : footer)
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textMuted)
        }
    }

    private func sectionTitle(_ title: String) -> some View {
        Text(title)
            .themeFont(.caption)
            .foregroundStyle(Theme.Colors.textSecondary)
            .textCase(nil)
    }

    private func draftBinding(_ keyPath: WritableKeyPath<EditDraft, EditDraft.DateField>) -> Binding<EditDraft.DateField> {
        Binding(
            get: { draft?[keyPath: keyPath] ?? EditDraft.DateField(original: nil, defaultDate: today, calendar: calendar) },
            set: { draft?[keyPath: keyPath] = $0 }
        )
    }

    private var today: Date { calendar.startOfDay(for: .now) }

    /// "Not required after" can't be before "Delay until".
    private var earliestExpiry: Date {
        guard let delay = draft?.delay, delay.isOn else { return today }
        return max(today, delay.date)
    }

    // MARK: Actions

    private func loadOptions() async {
        guard let href = row.editHref else { return finish(.closed) }
        guard let session = model.session else { return finish(.failed(Self.signedOut)) }
        options = .loading
        problem = nil
        do {
            let loaded = try await model.backend.editOptions(url: href, session: session)
            // Closed while loading (the core's calls run to completion regardless).
            guard !Task.isCancelled else { return }
            draft = EditDraft(options: loaded, currentPickup: row.item.fields["Pickup location"], calendar: calendar)
            options = .loaded(loaded)
            #if DEBUG
            if BorrowedDebugHooks.autoSavesEdit, model.backend.isDemo {
                draft?.pickup = loaded.pickupChoices.first { $0.code != draft?.pickup }?.code
                try? await Task.sleep(for: .seconds(1))
                await save()
            }
            #endif
        } catch {
            guard !Task.isCancelled else { return }
            if BorrowedViewModel.needsRootSheet(error) {
                finish(.failed(error))
            } else if let shown = model.report(error) {
                options = .failed(shown)
            }
        }
    }

    private func save() async {
        guard let href = row.editHref, let draft, !isSaving else { return }
        guard let session = model.session else { return finish(.failed(Self.signedOut)) }
        isSaving = true
        defer { isSaving = false }
        problem = nil
        do {
            let result = try await model.backend.editReservation(url: href, session: session,
                                                                 overrides: draft.overrides(calendar: calendar))
            if result.ok {
                finish(.saved(result))
            } else {
                problem = result.message.nonEmpty ?? "The library didn't accept the change."
                AccessibilityNotification.Announcement(problem ?? "").post()
            }
        } catch {
            if BorrowedViewModel.needsRootSheet(error) {
                finish(.failed(error))
            } else if let shown = model.report(error) {
                problem = shown.message
            }
        }
    }


    /// The session went away while the sheet was up: sign in again once it closes.
    private static let signedOut = SpydusError.NotLoggedIn(message: "not logged in")
}

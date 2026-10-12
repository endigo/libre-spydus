import LibreSpydusCore
import SwiftUI

/// Library picker (docs/native-apps.md §6): the Civica directory grouped by region, with search.
/// Full screen on first launch ("Choose your library"); a sheet from Profile ("Change library").
struct LibraryPickerView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var picker = LibraryPickerModel()
    /// Whether this is first-launch onboarding, fixed when the picker appears so the header
    /// doesn't change while the cover animates away after a pick.
    @State private var isOnboarding: Bool?
    @FocusState private var searchFocused: Bool

    var body: some View {
        @Bindable var picker = picker
        let onboarding = isOnboarding ?? model.needsOnboarding
        VStack(spacing: 0) {
            header(onboarding: onboarding)
            SearchField(text: $picker.query, placeholder: "Search by library or country")
                .focused($searchFocused)
                .padding(.horizontal, Theme.Spacing.l)
                .padding(.bottom, Theme.Spacing.m)
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .screenBackground()
        .presentationBackground(Theme.Colors.background)
        .onAppear {
            if isOnboarding == nil { isOnboarding = model.needsOnboarding }
            #if DEBUG
            picker.applyDebugQuery()
            #endif
        }
        .task { await picker.load() }
    }

    // MARK: Header

    @ViewBuilder
    private func header(onboarding: Bool) -> some View {
        if onboarding {
            VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                Image(systemName: "books.vertical.fill")
                    .font(.system(size: 30, weight: .semibold))
                    .foregroundStyle(Theme.Colors.primary)
                    .accessibilityHidden(true)
                Text("Choose your library")
                    .themeFont(.greeting)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .accessibilityAddTraits(.isHeader)
                Text("LibreSpydus works with libraries that use Spydus. You can change this later in Profile.")
                    .themeFont(.body)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, Theme.Spacing.l)
            .padding(.top, Theme.Spacing.xl)
            .padding(.bottom, Theme.Spacing.l)
        } else {
            HStack {
                Text("Change library")
                    .themeFont(.title)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .accessibilityAddTraits(.isHeader)
                Spacer()
                Button("Close") { dismiss() }
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.primary)
            }
            .padding(.horizontal, Theme.Spacing.l)
            .padding(.top, Theme.Spacing.xl)
            .padding(.bottom, Theme.Spacing.m)
        }
    }

    // MARK: Content

    @ViewBuilder
    private var content: some View {
        if picker.isLoading {
            LoadingView(message: "Loading libraries…")
        } else if let error = picker.loadError, picker.libraries.isEmpty {
            ErrorStateView(error: error) {
                Task { await picker.retry() }
            }
        } else if picker.libraries.isEmpty {
            EmptyStateView(systemImage: "building.columns", title: "No libraries listed",
                           message: "Civica's directory came back empty. Try again later.",
                           actionTitle: "Retry") {
                Task { await picker.retry() }
            }
        } else {
            list
        }
    }

    private var list: some View {
        let sections = picker.sections
        return ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: Theme.Spacing.s, pinnedViews: .sectionHeaders) {
                    if let refreshError = picker.refreshError {
                        Banner(message: refreshError.message + " Showing the saved list.", tone: .error)
                            .padding(.bottom, Theme.Spacing.s)
                    }
                    if sections.isEmpty {
                        EmptyStateView(systemImage: "magnifyingglass", title: "No libraries match “\(picker.query)”",
                                       message: "Try a library name or a country.")
                            .padding(.top, Theme.Spacing.xxl)
                    }
                    ForEach(sections) { section in
                        Section {
                            ForEach(section.libraries, id: \.origin) { library in
                                LibraryRow(library: library, region: section.region,
                                           isCurrent: library.origin == model.library?.origin) {
                                    choose(library)
                                }
                                .id(library.origin)
                            }
                        } header: {
                            sectionHeader(section)
                        }
                    }
                }
                .padding(.horizontal, Theme.Spacing.l)
                .padding(.bottom, Theme.Spacing.xxl)
            }
            .scrollDismissesKeyboard(.immediately)
            .refreshable { await picker.refresh() }
            .onAppear {
                // "Change library": start at the current library so its check mark shows.
                if isOnboarding == false, !picker.isSearching, let origin = model.library?.origin,
                   picker.libraries.contains(where: { $0.origin == origin }) {
                    proxy.scrollTo(origin, anchor: .center)
                }
            }
        }
    }

    private func sectionHeader(_ section: LibraryDirectory.Section) -> some View {
        HStack(alignment: .firstTextBaseline) {
            Text(section.region.title)
                .themeFont(.section)
                .foregroundStyle(Theme.Colors.textPrimary)
                .accessibilityAddTraits(.isHeader)
            Spacer()
            Text("\(section.libraries.count)")
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textMuted)
                .accessibilityLabel("\(section.libraries.count) libraries")
        }
        .padding(.top, Theme.Spacing.m)
        .padding(.bottom, Theme.Spacing.xs)
        .frame(maxWidth: .infinity)
        .background(Theme.Colors.background)
    }

    private func choose(_ library: LibraryInfo) {
        searchFocused = false
        let changed = model.library?.origin != library.origin
        model.selectLibrary(library)
        if changed { model.selectedTab = .home }
        // First launch: the cover closes because a library is now selected.
        if isOnboarding == false { dismiss() }
    }
}

/// One directory entry: logo, name, country, the region's caveat, a check on the current one.
private struct LibraryRow: View {
    let library: LibraryInfo
    let region: LibraryDirectory.Region
    let isCurrent: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: Theme.Spacing.m) {
                LibraryLogo(library)
                VStack(alignment: .leading, spacing: 2) {
                    Text(library.name)
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.textPrimary)
                        .multilineTextAlignment(.leading)
                    if let country = library.country, !country.isEmpty {
                        Text(country)
                            .themeFont(.caption)
                            .foregroundStyle(Theme.Colors.textSecondary)
                    }
                    if let caveat = region.caveat {
                        Label(caveat, systemImage: region == .uk ? "checkmark.shield" : "wifi.exclamationmark")
                            .themeFont(.caption)
                            .foregroundStyle(Theme.Colors.warning)
                            .padding(.top, 2)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if isCurrent {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 22, weight: .semibold))
                        .foregroundStyle(Theme.Colors.primary)
                        .accessibilityHidden(true)
                }
            }
            .padding(.horizontal, Theme.Spacing.m)
            .padding(.vertical, Theme.Spacing.m)
            .background(
                RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
                    .fill(isCurrent ? Theme.Colors.surfaceElevated : Theme.Colors.surface)
            )
            .overlay {
                if isCurrent {
                    RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
                        .strokeBorder(Theme.Colors.primary.opacity(0.7), lineWidth: 1.5)
                }
            }
            .contentShape(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous))
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(isCurrent ? [.isButton, .isSelected] : .isButton)
        .accessibilityHint(isCurrent ? "Current library" : "Select this library")
    }
}

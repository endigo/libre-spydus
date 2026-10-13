import LibreSpydusCore
import SwiftUI

/// The white sheet under the detail header: synopsis, subject chips, an edition-level item's
/// copies, details and the related carousels, with loading / empty / error states.
struct DetailSheetContent: View {
    let model: BookDetailModel
    let retry: () -> Void
    /// Called when the carousels first scroll into view (they load lazily).
    let startRelated: () -> Void
    /// Loads an edition-level item's copies again after a failure.
    let retryCopies: () -> Void

    /// Scroll target just above the carousels.
    static let relatedID = "detail-related"
    /// Scroll target at the "Copies" list.
    static let copiesID = "detail-copies"

    var body: some View {
        LazyVStack(alignment: .leading, spacing: Theme.Spacing.xl) {
            switch model.record {
            case .idle, .loading:
                SheetLoadingView(message: Copy.Detail.loadingDetails)
            case .failed(let error):
                SheetErrorView(error: error, retry: retry)
            case .loaded(let record):
                loaded(record)
            }
        }
        .padding(.horizontal, Theme.Spacing.xl)
        .padding(.bottom, Theme.Spacing.xxl)
    }

    @ViewBuilder
    private func loaded(_ record: RecordDetail) -> some View {
        let summary = record.summary.nonEmpty
        let subjects = model.subjects
        let facts = model.facts
        let showsCopies = model.copiesSource != nil && model.copies.value?.isEmpty != true
        if let summary {
            VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                SectionHeader(title: Copy.Detail.synopsis, onSheet: true)
                ExpandableText(text: summary)
            }
        }
        if !subjects.isEmpty {
            SubjectChips(subjects: subjects)
                .padding(.top, summary == nil ? 0 : -Theme.Spacing.s)
        }
        if showsCopies {
            CopiesList(state: model.copies, retry: retryCopies)
                .id(Self.copiesID)
        }
        if !facts.isEmpty {
            DetailFactsList(facts: facts)
        }
        if model.hasRelatedPanels {
            // Starts the related loads when the carousels come into view (again for an upgraded record).
            Color.clear
                .frame(height: 0)
                .id(Self.relatedID)
                .task(id: record.related) { startRelated() }
            ForEach(model.visiblePanels) { panel in
                RelatedCarousel(panel: panel, state: model.related[panel] ?? .loading)
            }
        }
        if summary == nil, subjects.isEmpty, !showsCopies, facts.isEmpty, !model.hasRelatedPanels {
            Text(Copy.Detail.noMoreDetails)
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

// MARK: - Synopsis

/// Text clamped to `lineLimit` lines with a "More" / "Less" toggle when it doesn't fit.
struct ExpandableText: View {
    let text: String
    var lineLimit = 5

    @State private var expanded = false
    @State private var clampedHeight: CGFloat = 0
    @State private var fullHeight: CGFloat = 0

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
            Text(text)
                .lineLimit(expanded ? nil : lineLimit)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { clampedHeight = $0 }
                .background(alignment: .top) {
                    Text(text)
                        .fixedSize(horizontal: false, vertical: true)
                        .hidden()
                        .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { fullHeight = $0 }
                        .accessibilityHidden(true)
                }
            if expanded || fullHeight > clampedHeight + 1 {
                Button(expanded ? Copy.Detail.less : Copy.Detail.more) {
                    withAnimation(.easeInOut(duration: 0.2)) { expanded.toggle() }
                }
                .themeFont(.bodyStrong)
                .foregroundStyle(Theme.Colors.primary)
                .buttonStyle(.borderless)
                .accessibilityHint(expanded ? Copy.Detail.lessHint : Copy.Detail.moreHint)
            }
        }
        .themeFont(.body)
    }
}

// MARK: - Subjects

private struct SubjectChips: View {
    let subjects: [SubjectChip]

    var body: some View {
        FlowRow(spacing: Theme.Spacing.s) {
            ForEach(subjects, id: \.self) { subject in
                NavigationLink(value: Route.results(query: subject.query, sort: .relevance, title: subject.label)) {
                    Chip(text: subject.label, tone: .onSheet)
                        .frame(maxWidth: 280, alignment: .leading)
                        .padding(.vertical, Theme.Spacing.xs / 2)
                        .contentShape(Rectangle())
                }
                .buttonStyle(PressScaleStyle())
                .accessibilityLabel(subject.label)
                .accessibilityHint(Copy.Detail.subjectHint)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Copy.Detail.subjects)
    }
}

// MARK: - Details

private struct DetailFactsList: View {
    let facts: [DetailFact]
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.s) {
            SectionHeader(title: Copy.Detail.details, onSheet: true)
            VStack(spacing: 0) {
                ForEach(Array(facts.enumerated()), id: \.offset) { index, fact in
                    row(fact)
                        .padding(.vertical, Theme.Spacing.s)
                    if index < facts.count - 1 {
                        Rectangle()
                            .fill(Theme.Colors.chipOnSheet)
                            .frame(height: 1)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func row(_ fact: DetailFact) -> some View {
        let layout = dynamicTypeSize.isAccessibilitySize
            ? AnyLayout(VStackLayout(alignment: .leading, spacing: Theme.Spacing.xs))
            : AnyLayout(HStackLayout(alignment: .firstTextBaseline, spacing: Theme.Spacing.m))
        layout {
            Text(fact.label)
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
                .frame(width: dynamicTypeSize.isAccessibilitySize ? nil : 104, alignment: .leading)
            Text(fact.value)
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.onSheet)
                .lineLimit(5)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityElement(children: .combine)
    }
}

// MARK: - Copies

/// An edition-level item's copies (§5.10): location, status (green when on the shelf), the
/// library's note and the call number, one row per copy.
private struct CopiesList: View {
    let state: Loadable<[CopyRow]>
    let retry: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.s) {
            SectionHeader(title: Copy.Detail.copies, onSheet: true)
            switch state {
            case .idle, .loading:
                HStack(spacing: Theme.Spacing.s) {
                    ProgressView()
                        .controlSize(.small)
                        .tint(Theme.Colors.onSheetSecondary)
                    Text(Copy.Detail.loadingCopies)
                }
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
                .padding(.vertical, Theme.Spacing.s)
                .accessibilityElement(children: .combine)
            case .failed(let error):
                HStack(spacing: Theme.Spacing.s) {
                    Text(Copy.Detail.copiesFailed)
                        .foregroundStyle(Theme.Colors.onSheetSecondary)
                    if error.canRetry {
                        Button(Copy.Common.retry, action: retry)
                            .foregroundStyle(Theme.Colors.primary)
                            .buttonStyle(.borderless)
                            .accessibilityHint(Copy.Detail.retryCopiesHint)
                    }
                }
                .themeFont(.caption)
                .padding(.vertical, Theme.Spacing.s)
            case .loaded(let rows):
                VStack(spacing: 0) {
                    ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                        CopyRowView(row: row)
                            .padding(.vertical, Theme.Spacing.s)
                        if index < rows.count - 1 {
                            Rectangle()
                                .fill(Theme.Colors.chipOnSheet)
                                .frame(height: 1)
                        }
                    }
                }
            }
        }
    }
}

private struct CopyRowView: View {
    let row: CopyRow

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.xs / 2) {
            Text(row.location)
                .themeFont(.bodyStrong)
                .foregroundStyle(Theme.Colors.onSheet)
            if let status = row.status {
                Text(status)
                    .themeFont(.body)
                    .foregroundStyle(row.available ? Theme.Colors.success : Theme.Colors.onSheet)
            }
            if let description = row.description {
                Text(description)
                    .themeFont(.body)
                    .foregroundStyle(Theme.Colors.onSheetSecondary)
            }
            if let callNumber = row.callNumber {
                Text(callNumber)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.onSheetSecondary)
                    .accessibilityLabel(Copy.Detail.callNumber(number: callNumber))
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .combine)
    }
}

// MARK: - Related carousels

extension RelatedPanel: Identifiable {
    var id: String { rawValue }
}

/// One related panel: "Similar books", "More by this author", "In this series", "Other editions".
private struct RelatedCarousel: View {
    let panel: RelatedPanel
    let state: Loadable<[BookRef]>

    private let coverWidth = Theme.Cover.shelfWidth

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            SectionHeader(title: panel.title, onSheet: true)
            ScrollView(.horizontal) {
                LazyHStack(alignment: .top, spacing: Theme.Spacing.l) {
                    if let refs = state.value {
                        ForEach(refs) { ref in
                            NavigationLink(value: Route.detail(ref)) {
                                ShelfCover(title: DetailText.displayTitle(ref.title), author: ref.author,
                                           coverUrl: ref.coverUrl, caption: caption(for: ref),
                                           captionColor: Theme.Colors.onSheetSecondary, width: coverWidth,
                                           showsTitle: true, onSheet: true)
                            }
                            .buttonStyle(PressScaleStyle())
                        }
                    } else {
                        ForEach(0..<4, id: \.self) { _ in
                            RoundedRectangle(cornerRadius: Theme.Radius.cover, style: .continuous)
                                .fill(Theme.Colors.chipOnSheet)
                                .frame(width: coverWidth, height: Theme.Cover.height(forWidth: coverWidth))
                        }
                        .accessibilityHidden(true)
                    }
                }
                .padding(.horizontal, Theme.Spacing.xl)
                .padding(.vertical, Theme.Spacing.s)
            }
            .scrollIndicators(.hidden)
            .padding(.horizontal, -Theme.Spacing.xl)
            .accessibilityLabel(state.isLoading ? Copy.Detail.loadingPanel(panel: panel.title.lowercased()) : panel.title)
        }
    }

    private func caption(for ref: BookRef) -> String? {
        Formatting.authorName(ref.author) ?? ref.year
    }
}

// MARK: - States on the white sheet

/// A spinner with a message, in sheet colours.
struct SheetLoadingView: View {
    let message: String

    var body: some View {
        HStack(spacing: Theme.Spacing.m) {
            ProgressView()
                .tint(Theme.Colors.onSheetSecondary)
            Text(message)
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, Theme.Spacing.xxl)
        .accessibilityElement(children: .combine)
    }
}

/// `ErrorStateView` for the white sheet: what went wrong, Retry, and the detail in a disclosure.
struct SheetErrorView: View {
    let error: UserFacingError
    var retry: (() -> Void)?

    @State private var showsDetail = false

    var body: some View {
        VStack(spacing: Theme.Spacing.m) {
            Image(systemName: error.systemImage)
                .font(.system(size: 36, weight: .light))
                .foregroundStyle(Theme.Colors.warning)
                .accessibilityHidden(true)
            Text(error.title)
                .themeFont(.section)
                .foregroundStyle(Theme.Colors.onSheet)
            Text(error.message)
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
                .multilineTextAlignment(.center)
            if let retry, error.canRetry {
                PrimaryPillButton(title: Copy.Common.retry, systemImage: "arrow.clockwise", action: retry)
                    .padding(.top, Theme.Spacing.s)
            }
            if let detail = error.detail {
                DisclosureGroup(Copy.Errors.details, isExpanded: $showsDetail) {
                    Text(detail)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.onSheetSecondary)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.top, Theme.Spacing.xs)
                }
                .themeFont(.caption)
                .tint(Theme.Colors.onSheetSecondary)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
                .frame(maxWidth: 320)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, Theme.Spacing.xl)
    }
}

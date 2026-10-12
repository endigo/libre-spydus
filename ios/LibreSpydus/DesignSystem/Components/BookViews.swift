import SwiftUI

/// A shelf item: cover with a one- or two-line caption under it (Home carousels, Favourites grid,
/// detail "Similar books"). `caption` replaces the title line, e.g. "3 days left".
struct ShelfCover: View {
    let title: String
    var author: String?
    var coverUrl: String?
    var caption: String?
    var captionColor: Color = Theme.Colors.textSecondary
    var width: CGFloat = Theme.Cover.shelfWidth
    var showsTitle = false
    /// Text colours for the white detail sheet.
    var onSheet = false

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.s) {
            CoverImage(url: coverUrl, title: title, author: author, width: width)
            if showsTitle {
                Text(title)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(onSheet ? Theme.Colors.onSheet : Theme.Colors.textPrimary)
                    .lineLimit(2)
            }
            if let caption {
                Text(caption)
                    .themeFont(.caption)
                    .foregroundStyle(captionColor)
                    .lineLimit(1)
            }
        }
        .frame(width: width, alignment: .leading)
        .accessibilityElement(children: .combine)
    }
}

extension ShelfCover {
    init(_ ref: BookRef, caption: String? = nil, captionColor: Color = Theme.Colors.textSecondary,
         width: CGFloat = Theme.Cover.shelfWidth, showsTitle: Bool = false, onSheet: Bool = false) {
        self.init(title: ref.title, author: ref.author, coverUrl: ref.coverUrl, caption: caption,
                  captionColor: captionColor, width: width, showsTitle: showsTitle, onSheet: onSheet)
    }
}

/// A Borrowed-style card on `surface`: cover, title, "By author", meta line, chips, trailing
/// coral text actions. Also fits search results.
struct BookRowCard: View {
    struct Action: Identifiable {
        let title: String
        var role: ButtonRole?
        let perform: () -> Void
        var id: String { title }
    }

    let title: String
    var author: String?
    var coverUrl: String?
    /// "Due 27 Oct · 3 days left", "Ready for pickup · Riverton", "2021".
    var meta: String?
    var metaColor: Color = Theme.Colors.textSecondary
    var chips: [Chip] = []
    var actions: [Action] = []
    /// Ready-for-pickup reservations: accent outline.
    var highlighted = false

    var body: some View {
        HStack(alignment: .top, spacing: Theme.Spacing.l) {
            CoverImage(url: coverUrl, title: title, author: author, width: Theme.Cover.smallWidth)
            VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                Text(title)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .lineLimit(2)
                if let byline = Formatting.byline(author) {
                    Text(byline)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .lineLimit(1)
                }
                if !chips.isEmpty {
                    FlowRow(spacing: Theme.Spacing.xs) {
                        ForEach(Array(chips.enumerated()), id: \.offset) { _, chip in chip }
                    }
                    .padding(.top, Theme.Spacing.xs)
                }
                Spacer(minLength: Theme.Spacing.xs)
                if let meta {
                    Text(meta)
                        .themeFont(.caption)
                        .foregroundStyle(metaColor)
                        .lineLimit(2)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if !actions.isEmpty {
                VStack(alignment: .trailing, spacing: Theme.Spacing.m) {
                    ForEach(actions) { action in
                        Button(action.title, role: action.role, action: action.perform)
                            .themeFont(.bodyStrong)
                            .foregroundStyle(Theme.Colors.primary)
                            .buttonStyle(.borderless)
                    }
                }
                .frame(maxHeight: .infinity)
            }
        }
        .padding(Theme.Spacing.l)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
                .fill(highlighted ? Theme.Colors.surfaceElevated : Theme.Colors.surface)
        )
        .overlay {
            if highlighted {
                RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
                    .strokeBorder(Theme.Colors.accent, lineWidth: 1.5)
            }
        }
        .accessibilityElement(children: .contain)
    }
}

extension BookRowCard {
    init(_ ref: BookRef, meta: String? = nil, metaColor: Color = Theme.Colors.textSecondary, chips: [Chip] = [],
         actions: [Action] = [], highlighted: Bool = false) {
        self.init(title: ref.title, author: ref.author, coverUrl: ref.coverUrl, meta: meta, metaColor: metaColor,
                  chips: chips, actions: actions, highlighted: highlighted)
    }
}

/// Lays children out in rows, wrapping when a row is full (chips).
struct FlowRow: Layout {
    var spacing: CGFloat = Theme.Spacing.s

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let rows = arrange(subviews: subviews, width: proposal.width ?? .infinity)
        let height = rows.map(\.height).reduce(0, +) + spacing * CGFloat(max(rows.count - 1, 0))
        let width = rows.map(\.width).max() ?? 0
        return CGSize(width: proposal.width ?? width, height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var y = bounds.minY
        for row in arrange(subviews: subviews, width: bounds.width) {
            var x = bounds.minX
            for index in row.indices {
                let size = subviews[index].sizeThatFits(.unspecified)
                subviews[index].place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
                x += size.width + spacing
            }
            y += row.height + spacing
        }
    }

    private struct Row {
        var indices: [Int] = []
        var width: CGFloat = 0
        var height: CGFloat = 0
    }

    private func arrange(subviews: Subviews, width: CGFloat) -> [Row] {
        var rows: [Row] = []
        var current = Row()
        for index in subviews.indices {
            let size = subviews[index].sizeThatFits(.unspecified)
            let needed = current.indices.isEmpty ? size.width : current.width + spacing + size.width
            if needed > width, !current.indices.isEmpty {
                rows.append(current)
                current = Row()
            }
            current.width = current.indices.isEmpty ? size.width : current.width + spacing + size.width
            current.height = max(current.height, size.height)
            current.indices.append(index)
        }
        if !current.indices.isEmpty { rows.append(current) }
        return rows
    }
}

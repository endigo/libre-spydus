import LibreSpydusCore
import SwiftUI

/// Home's "Featured" section (§5.10): the library's own collections as a horizontal row of tiles.
struct FeaturedRow: View {
    let links: [FeaturedLink]
    let open: (FeaturedLink) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            SectionHeader(title: Copy.Featured.title)
                .padding(.horizontal, Theme.Spacing.l)
            ScrollView(.horizontal) {
                HStack(alignment: .top, spacing: Theme.Spacing.m) {
                    ForEach(Array(links.enumerated()), id: \.offset) { index, link in
                        FeaturedTile(link: link, color: Theme.Colors.genre[index % Theme.Colors.genre.count]) {
                            open(link)
                        }
                    }
                }
                .padding(.horizontal, Theme.Spacing.l)
            }
            .scrollIndicators(.hidden)
            .scrollClipDisabled()
        }
    }
}

/// One collection: a rounded `surface` card with the icon on a tinted circle and the title under
/// it (two lines at most).
private struct FeaturedTile: View {
    let link: FeaturedLink
    let color: Color
    let action: () -> Void

    static let width: CGFloat = 128
    private static let iconDiameter: CGFloat = 44

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: Theme.Spacing.m) {
                Image(systemName: FeaturedIcon.systemImage(for: link.icon))
                    .font(.system(size: Self.iconDiameter * 0.42, weight: .semibold))
                    .foregroundStyle(color)
                    .frame(width: Self.iconDiameter, height: Self.iconDiameter)
                    .background(Circle().fill(color.opacity(0.18)))
                Text(link.title)
                    .fontWeight(.semibold)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .multilineTextAlignment(.leading)
                    .lineLimit(2, reservesSpace: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(Theme.Spacing.m)
            .frame(width: Self.width, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
                .fill(Theme.Colors.surface))
            .contentShape(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous))
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(link.title)
        .accessibilityHint(link.description.nonEmpty ?? hint)
        .accessibilityAddTraits(.isButton)
    }

    private var hint: String {
        switch link.kind {
        case .collection, .search: Copy.Featured.collectionHint
        case .web: Copy.Featured.websiteHint
        }
    }
}

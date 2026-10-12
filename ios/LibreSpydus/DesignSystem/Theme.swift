import SwiftUI

/// Design tokens, transcribed from `design/tokens.json` (the source of truth; ThemeTokensTests
/// checks the two still agree). Navy screens, white tab bar, white sheet on book detail.
enum Theme {
    enum Colors {
        static let background = Color(hex: 0x2B2D42)
        static let surface = Color(hex: 0x35374E)
        static let surfaceElevated = Color(hex: 0x3E405A)
        static let field = Color(hex: 0x3A3C55)
        static let divider = Color(hex: 0x44465F)
        static let textPrimary = Color(hex: 0xFFFFFF)
        static let textSecondary = Color(hex: 0xA4A7C1)
        static let textMuted = Color(hex: 0x7D809B)
        static let primary = Color(hex: 0xF2665E)
        static let primaryGradientStart = Color(hex: 0xF7806F)
        static let primaryGradientEnd = Color(hex: 0xEF5A5C)
        static let onPrimary = Color(hex: 0xFFFFFF)
        static let accent = Color(hex: 0x4B5BD6)
        static let onAccent = Color(hex: 0xFFFFFF)
        static let sheet = Color(hex: 0xFFFFFF)
        static let onSheet = Color(hex: 0x24263A)
        static let onSheetSecondary = Color(hex: 0x5F627A)
        static let chipOnSheet = Color(hex: 0xEEF0F7)
        static let tabBar = Color(hex: 0xFFFFFF)
        static let tabActive = Color(hex: 0x2B2D42)
        static let tabInactive = Color(hex: 0xB4B6C6)
        static let success = Color(hex: 0x3CC48A)
        static let warning = Color(hex: 0xF5A524)
        static let danger = Color(hex: 0xF2665E)
        static let digital = Color(hex: 0x2EC4B6)
        static let genre: [Color] = [0xF26B5B, 0x9B59D0, 0x4F6BE3, 0xF29F4B, 0x2EC4B6, 0xE85D9B, 0x5DB85C, 0x7A6FF0]
            .map { Color(hex: $0) }

        /// The coral pill gradient.
        static var primaryGradient: LinearGradient {
            LinearGradient(colors: [primaryGradientStart, primaryGradientEnd], startPoint: .leading, endPoint: .trailing)
        }

        /// A genre colour picked stably from `key` (same key → same colour on every launch).
        static func genre(for key: String) -> Color {
            genre[Int(StableHash.fnv1a(key) % UInt64(genre.count))]
        }
    }

    enum Radius {
        static let card: CGFloat = 16
        static let cover: CGFloat = 10
        static let field: CGFloat = 14
        static let tile: CGFloat = 18
        static let sheet: CGFloat = 28
        static let pill: CGFloat = 999
    }

    enum Spacing {
        static let xs: CGFloat = 4
        static let s: CGFloat = 8
        static let m: CGFloat = 12
        static let l: CGFloat = 16
        static let xl: CGFloat = 24
        static let xxl: CGFloat = 32
    }

    enum Cover {
        /// width / height.
        static let aspect: CGFloat = 0.68
        static let shelfWidth: CGFloat = 120
        static let smallWidth: CGFloat = 72
        static let detailWidth: CGFloat = 140

        static func height(forWidth width: CGFloat) -> CGFloat { (width / aspect).rounded() }
    }

    enum TabBar {
        static let height: CGFloat = 64
        static let centerButtonDiameter: CGFloat = 64
        static let centerButtonLift: CGFloat = 22
    }

    struct Shadow: Sendable {
        var color: Color
        var opacity: Double
        var radius: CGFloat
        var y: CGFloat

        static let card = Shadow(color: Color(hex: 0x000000), opacity: 0.25, radius: 12, y: 6)
        static let tabCenter = Shadow(color: Color(hex: 0x2B2D42), opacity: 0.18, radius: 10, y: 4)
    }
}

/// Type tokens. Use `.themeFont(.body)`: sizes scale with Dynamic Type.
enum ThemeFont: CaseIterable, Sendable {
    case greeting, title, section, body, bodyStrong, caption, tab

    var size: CGFloat {
        switch self {
        case .greeting: 28
        case .title: 22
        case .section: 18
        case .body, .bodyStrong: 15
        case .caption: 12
        case .tab: 11
        }
    }

    var weight: Font.Weight {
        switch self {
        case .greeting, .title, .section, .bodyStrong: .semibold
        case .body, .caption: .regular
        case .tab: .medium
        }
    }

    /// The Dynamic Type style each size scales with.
    var textStyle: Font.TextStyle {
        switch self {
        case .greeting: .largeTitle
        case .title: .title2
        case .section: .title3
        case .body, .bodyStrong: .body
        case .caption: .caption
        case .tab: .caption2
        }
    }
}

private struct ThemeFontModifier: ViewModifier {
    let token: ThemeFont
    @ScaledMetric private var size: CGFloat

    init(_ token: ThemeFont) {
        self.token = token
        _size = ScaledMetric(wrappedValue: token.size, relativeTo: token.textStyle)
    }

    func body(content: Content) -> some View {
        content.font(.system(size: size, weight: token.weight))
    }
}

extension View {
    /// A type token from `design/tokens.json`, scaled with Dynamic Type.
    func themeFont(_ token: ThemeFont) -> some View {
        modifier(ThemeFontModifier(token))
    }

    func themeShadow(_ shadow: Theme.Shadow) -> some View {
        self.shadow(color: shadow.color.opacity(shadow.opacity), radius: shadow.radius / 2, x: 0, y: shadow.y)
    }

    /// Navy full-screen background (every tab root).
    func screenBackground() -> some View {
        background(Theme.Colors.background.ignoresSafeArea())
    }

    /// The white rounded content sheet (book detail's lower half).
    func sheetSurface() -> some View {
        background(
            UnevenRoundedRectangle(topLeadingRadius: Theme.Radius.sheet, topTrailingRadius: Theme.Radius.sheet)
                .fill(Theme.Colors.sheet)
                .ignoresSafeArea(edges: .bottom)
        )
    }

    /// Navy navigation bar with white titles for a pushed or root screen.
    func navyNavigationBar() -> some View {
        toolbarBackground(Theme.Colors.background, for: .navigationBar)
            .toolbarBackground(.visible, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
    }
}

extension Color {
    /// `Color(hex: 0x2B2D42)`, sRGB.
    init(hex: UInt32, opacity: Double = 1) {
        self.init(.sRGB,
                  red: Double((hex >> 16) & 0xFF) / 255,
                  green: Double((hex >> 8) & 0xFF) / 255,
                  blue: Double(hex & 0xFF) / 255,
                  opacity: opacity)
    }
}

/// FNV-1a: a hash that's stable across launches (unlike `Hasher`), for colour picks.
enum StableHash {
    static func fnv1a(_ text: String) -> UInt64 {
        var hash: UInt64 = 0xcbf2_9ce4_8422_2325
        for byte in text.utf8 {
            hash ^= UInt64(byte)
            hash = hash &* 0x0000_0100_0000_01B3
        }
        return hash
    }
}

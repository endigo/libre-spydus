import Foundation
import SwiftUI
import Testing
@testable import LibreSpydus

/// `Theme` is transcribed from design/tokens.json; this keeps the two in step
/// (skipped when the repo checkout isn't readable from the simulator).
@MainActor
@Suite(.enabled(if: TestData.repoAvailable, "needs the repo checkout (design/tokens.json)"))
struct ThemeTokensTests {
    private func tokens() throws -> [String: Any] {
        let data = try Data(contentsOf: TestData.repoRoot.appending(path: "design/tokens.json"))
        return try #require(try JSONSerialization.jsonObject(with: data) as? [String: Any])
    }

    private func hex(_ color: Color) -> String {
        let resolved = color.resolve(in: EnvironmentValues())
        func byte(_ value: Float) -> Int { Int((value * 255).rounded()) }
        return String(format: "#%02X%02X%02X", byte(resolved.red), byte(resolved.green), byte(resolved.blue))
    }

    @Test func coloursMatchTokens() throws {
        let colors = try #require(try tokens()["color"] as? [String: Any])
        let theme: [String: Color] = [
            "background": Theme.Colors.background, "surface": Theme.Colors.surface,
            "surfaceElevated": Theme.Colors.surfaceElevated, "field": Theme.Colors.field,
            "divider": Theme.Colors.divider, "textPrimary": Theme.Colors.textPrimary,
            "textSecondary": Theme.Colors.textSecondary, "textMuted": Theme.Colors.textMuted,
            "primary": Theme.Colors.primary, "primaryGradientStart": Theme.Colors.primaryGradientStart,
            "primaryGradientEnd": Theme.Colors.primaryGradientEnd, "onPrimary": Theme.Colors.onPrimary,
            "accent": Theme.Colors.accent, "onAccent": Theme.Colors.onAccent, "sheet": Theme.Colors.sheet,
            "onSheet": Theme.Colors.onSheet, "onSheetSecondary": Theme.Colors.onSheetSecondary,
            "chipOnSheet": Theme.Colors.chipOnSheet, "tabBar": Theme.Colors.tabBar,
            "tabActive": Theme.Colors.tabActive, "tabInactive": Theme.Colors.tabInactive,
            "success": Theme.Colors.success, "warning": Theme.Colors.warning, "danger": Theme.Colors.danger,
            "digital": Theme.Colors.digital,
        ]
        for (name, value) in colors {
            if let list = value as? [String] {
                #expect(list == Theme.Colors.genre.map(hex), "color.\(name)")
            } else {
                #expect(theme[name].map(hex) == (value as? String)?.uppercased(), "color.\(name)")
            }
        }
    }

    @Test func metricsMatchTokens() throws {
        let tokens = try tokens()
        func number(_ group: String, _ key: String) -> CGFloat? {
            ((tokens[group] as? [String: Any])?[key] as? NSNumber).map { CGFloat($0.doubleValue) }
        }
        #expect(number("radius", "card") == Theme.Radius.card)
        #expect(number("radius", "cover") == Theme.Radius.cover)
        #expect(number("radius", "field") == Theme.Radius.field)
        #expect(number("radius", "tile") == Theme.Radius.tile)
        #expect(number("radius", "sheet") == Theme.Radius.sheet)
        #expect(number("spacing", "l") == Theme.Spacing.l)
        #expect(number("spacing", "xxl") == Theme.Spacing.xxl)
        #expect(number("cover", "aspect") == Theme.Cover.aspect)
        #expect(number("cover", "shelfWidth") == Theme.Cover.shelfWidth)
        #expect(number("cover", "smallWidth") == Theme.Cover.smallWidth)
        #expect(number("cover", "detailWidth") == Theme.Cover.detailWidth)
        #expect(number("tabBar", "height") == Theme.TabBar.height)
        #expect(number("tabBar", "centerButtonDiameter") == Theme.TabBar.centerButtonDiameter)
        #expect(number("tabBar", "centerButtonLift") == Theme.TabBar.centerButtonLift)
        let type = try #require(tokens["type"] as? [String: [String: Any]])
        for token in ThemeFont.allCases {
            let size = (type[String(describing: token)]?["size"] as? NSNumber).map { CGFloat($0.doubleValue) }
            #expect(size == token.size, "type.\(token)")
        }
    }
}

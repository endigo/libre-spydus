import SwiftUI

/// "New arrivals ……… View All".
struct SectionHeader: View {
    let title: String
    var actionTitle = Copy.Common.viewAll
    var onSheet = false
    var action: (() -> Void)?

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(title)
                .themeFont(.section)
                .foregroundStyle(onSheet ? Theme.Colors.onSheet : Theme.Colors.textPrimary)
                .accessibilityAddTraits(.isHeader)
            Spacer()
            if let action {
                Button(actionTitle, action: action)
                    .themeFont(.caption)
                    .foregroundStyle(onSheet ? Theme.Colors.onSheetSecondary : Theme.Colors.textSecondary)
                    .buttonStyle(.borderless)
                    // "View All New arrivals"; any other action reads as shown, after its header.
                    .accessibilityLabel(actionTitle == Copy.Common.viewAll ? Copy.A11y.viewAllSection(section: title) : actionTitle)
            }
        }
    }
}

/// The rounded search field on navy. Apply `.focused(_:)` to it to control focus.
struct SearchField: View {
    @Binding var text: String
    var placeholder = Copy.Search.placeholder
    var onSubmit: () -> Void = {}

    var body: some View {
        HStack(spacing: Theme.Spacing.s) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(Theme.Colors.textMuted)
            TextField(text: $text) {
                Text(placeholder).foregroundStyle(Theme.Colors.textMuted)
            }
            .themeFont(.body)
            .foregroundStyle(Theme.Colors.textPrimary)
            .tint(Theme.Colors.primary)
            .submitLabel(.search)
            .autocorrectionDisabled()
            .textInputAutocapitalization(.never)
            .onSubmit(onSubmit)
            if !text.isEmpty {
                Button {
                    text = ""
                } label: {
                    Image(systemName: "xmark.circle.fill").foregroundStyle(Theme.Colors.textMuted)
                }
                .buttonStyle(.borderless)
                .accessibilityLabel(Copy.Search.clearSearch)
            }
        }
        .padding(.horizontal, Theme.Spacing.l)
        .frame(minHeight: 52)
        .background(RoundedRectangle(cornerRadius: Theme.Radius.field, style: .continuous).fill(Theme.Colors.field))
    }
}

/// Looks like `SearchField` but is a button (Home: tap → Search tab).
struct SearchFieldButton: View {
    var placeholder = Copy.Search.placeholder
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: Theme.Spacing.s) {
                Image(systemName: "magnifyingglass")
                Text(placeholder).themeFont(.body)
                Spacer()
            }
            .foregroundStyle(Theme.Colors.textMuted)
            .padding(.horizontal, Theme.Spacing.l)
            .frame(minHeight: 52)
            .background(RoundedRectangle(cornerRadius: Theme.Radius.field, style: .continuous).fill(Theme.Colors.field))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(Copy.Home.searchField)
        .accessibilityHint(Copy.Home.searchFieldHint)
    }
}

/// The coral gradient pill ("Reserve", "Sign in").
struct PrimaryPillButton: View {
    let title: String
    var systemImage: String?
    var isLoading = false
    var fillsWidth = false
    let action: () -> Void

    @Environment(\.isEnabled) private var isEnabled

    var body: some View {
        Button(action: action) {
            HStack(spacing: Theme.Spacing.s) {
                if isLoading {
                    ProgressView().tint(Theme.Colors.onPrimary)
                } else if let systemImage {
                    Image(systemName: systemImage)
                }
                Text(title).themeFont(.bodyStrong)
            }
            .foregroundStyle(Theme.Colors.onPrimary)
            .padding(.horizontal, Theme.Spacing.xl + Theme.Spacing.s)
            .frame(minHeight: 50)
            .frame(maxWidth: fillsWidth ? .infinity : nil)
            .background(Capsule().fill(Theme.Colors.primaryGradient))
            .shadow(color: Theme.Colors.primary.opacity(isEnabled ? 0.35 : 0), radius: 8, y: 4)
            .opacity(isEnabled ? 1 : 0.5)
            .contentShape(Capsule())
        }
        .buttonStyle(PressScaleStyle())
        .disabled(isLoading)
        .accessibilityLabel(title)
    }
}

/// The quieter pill beside a `PrimaryPillButton` ("Sign in with Face ID" under Sign in).
struct SecondaryPillButton: View {
    let title: String
    var systemImage: String?
    var fillsWidth = false
    let action: () -> Void

    @Environment(\.isEnabled) private var isEnabled

    var body: some View {
        Button(action: action) {
            HStack(spacing: Theme.Spacing.s) {
                if let systemImage {
                    Image(systemName: systemImage)
                }
                Text(title).themeFont(.bodyStrong)
            }
            .foregroundStyle(Theme.Colors.textPrimary)
            .padding(.horizontal, Theme.Spacing.xl + Theme.Spacing.s)
            .frame(minHeight: 50)
            .frame(maxWidth: fillsWidth ? .infinity : nil)
            .background(Capsule().fill(Theme.Colors.surfaceElevated))
            .opacity(isEnabled ? 1 : 0.5)
            .contentShape(Capsule())
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityLabel(title)
    }
}

/// The blue circular icon button (bookmark / favourite toggle).
struct CircleIconButton: View {
    let systemImage: String
    var accessibilityLabel: String
    var diameter: CGFloat = 50
    var tint: Color = Theme.Colors.accent
    var foreground: Color = Theme.Colors.onAccent
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: diameter * 0.4, weight: .semibold))
                .foregroundStyle(foreground)
                .frame(width: diameter, height: diameter)
                .background(Circle().fill(tint))
                .shadow(color: tint.opacity(0.35), radius: 8, y: 4)
                .contentShape(Circle())
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityLabel(accessibilityLabel)
    }
}

/// A rounded colour tile with an icon and a caption under it (Home "Popular genres").
struct GenreTile: View {
    let title: String
    let systemImage: String
    var color: Color?
    var size: CGFloat = 76
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: Theme.Spacing.s) {
                let fill = color ?? Theme.Colors.genre(for: title)
                Image(systemName: systemImage)
                    .font(.system(size: size * 0.36, weight: .semibold))
                    .foregroundStyle(.white)
                    .frame(width: size, height: size)
                    .background(
                        RoundedRectangle(cornerRadius: Theme.Radius.tile, style: .continuous)
                            .fill(LinearGradient(colors: [fill.opacity(0.8), fill], startPoint: .topLeading, endPoint: .bottomTrailing))
                    )
                Text(title)
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .lineLimit(1)
            }
            .frame(width: size + Theme.Spacing.m)
        }
        .buttonStyle(PressScaleStyle())
        .accessibilityLabel(title)
    }
}

/// A small rounded label: format chips, subject chips, status.
struct Chip: View {
    enum Tone {
        /// On navy.
        case neutral
        /// Digital formats ("Available online").
        case digital
        /// On the white detail sheet.
        case onSheet
        /// Selected format.
        case selected
        case success
        case warning
        case danger
    }

    let text: String
    var tone: Tone = .neutral
    var systemImage: String?

    var body: some View {
        HStack(spacing: Theme.Spacing.xs) {
            if let systemImage { Image(systemName: systemImage) }
            Text(text).lineLimit(1)
        }
        .themeFont(.caption)
        .foregroundStyle(foreground)
        .padding(.horizontal, Theme.Spacing.s + 2)
        .padding(.vertical, Theme.Spacing.xs + 1)
        .background(Capsule().fill(background))
        .accessibilityElement(children: .combine)
    }

    private var foreground: Color {
        switch tone {
        case .neutral: Theme.Colors.textPrimary
        case .digital: Theme.Colors.digital
        case .onSheet: Theme.Colors.onSheetSecondary
        case .selected: Theme.Colors.onPrimary
        case .success: Theme.Colors.success
        case .warning: Theme.Colors.warning
        case .danger: Theme.Colors.danger
        }
    }

    private var background: Color {
        switch tone {
        case .neutral: Theme.Colors.surfaceElevated
        case .digital: Theme.Colors.digital.opacity(0.16)
        case .onSheet: Theme.Colors.chipOnSheet
        case .selected: Theme.Colors.primary
        case .success: Theme.Colors.success.opacity(0.16)
        case .warning: Theme.Colors.warning.opacity(0.16)
        case .danger: Theme.Colors.danger.opacity(0.16)
        }
    }
}

extension Chip {
    /// A format chip; digital formats get the `digital` tint.
    init(format: FormatRef, selected: Bool = false) {
        self.init(text: format.displayLabel, tone: selected ? .selected : (format.isDigital ? .digital : .neutral),
                  systemImage: format.isDigital ? "cloud" : nil)
    }
}

/// An inline result banner (reservation placed / refused, cancel result).
struct Banner: View {
    enum Tone { case success, info, error }

    let message: String
    var tone: Tone = .info

    var body: some View {
        HStack(alignment: .top, spacing: Theme.Spacing.s) {
            Image(systemName: icon)
            Text(message).frame(maxWidth: .infinity, alignment: .leading)
        }
        .themeFont(.body)
        .foregroundStyle(Theme.Colors.textPrimary)
        .padding(Theme.Spacing.m)
        .background(RoundedRectangle(cornerRadius: Theme.Radius.field, style: .continuous).fill(color.opacity(0.22)))
        .overlay(RoundedRectangle(cornerRadius: Theme.Radius.field, style: .continuous).strokeBorder(color.opacity(0.6)))
        .accessibilityElement(children: .combine)
    }

    private var icon: String {
        switch tone {
        case .success: "checkmark.circle.fill"
        case .info: "info.circle.fill"
        case .error: "exclamationmark.triangle.fill"
        }
    }

    private var color: Color {
        switch tone {
        case .success: Theme.Colors.success
        case .info: Theme.Colors.accent
        case .error: Theme.Colors.danger
        }
    }
}

/// Slight shrink on press.
struct PressScaleStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.96 : 1)
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}

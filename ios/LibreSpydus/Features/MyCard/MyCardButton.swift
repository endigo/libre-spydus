import SwiftUI

/// Home's "My card" capsule, right of the greeting while signed in (docs/native-apps.md §5.9).
struct MyCardButton: View {
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: Theme.Spacing.s) {
                Image(systemName: "barcode")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(Theme.Colors.primary)
                Text(Copy.MyCard.button)
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .lineLimit(1)
            }
            .padding(.horizontal, Theme.Spacing.l)
            .frame(minHeight: 44)
            .background(Capsule().fill(Theme.Colors.surfaceElevated))
            .contentShape(Capsule())
        }
        .buttonStyle(PressScaleStyle())
        .fixedSize()
        .accessibilityLabel(Copy.MyCard.button)
        .accessibilityHint(Copy.MyCard.buttonHint)
    }
}

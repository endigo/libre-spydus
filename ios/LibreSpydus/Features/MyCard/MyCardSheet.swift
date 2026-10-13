import LibreSpydusCore
import SwiftUI

/// "My card" (docs/native-apps.md §5.9): the library card as a barcode to scan at the desk or
/// self-checkout, on a white card whatever the theme. Asks for the number when none is saved;
/// "Edit card number" opens the same form. The screen is at full brightness while it's up.
struct MyCardSheet: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var card: MyCardModel
    @FocusState private var fieldFocused: Bool

    /// `editing`: start on the entry form, filled in with the number.
    init(cardNumber: String?, knownSymbology: BarcodeSymbology?, editing: Bool = false) {
        _card = State(initialValue: MyCardModel(cardNumber: cardNumber, knownSymbology: knownSymbology, editing: editing))
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                Group {
                    if card.mode == .card, let number = model.cardNumber {
                        cardContent(number)
                    } else {
                        entryForm
                    }
                }
                .padding(.horizontal, Theme.Spacing.l)
                .padding(.top, Theme.Spacing.s)
                .padding(.bottom, Theme.Spacing.xxl)
            }
            .scrollDismissesKeyboard(.interactively)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Theme.Colors.background.ignoresSafeArea())
            .navigationTitle("My card")
            .navigationBarTitleDisplayMode(.inline)
            .navyNavigationBar()
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .presentationDetents([.large])
        .presentationDragIndicator(.visible)
        .presentationBackground(Theme.Colors.background)
        .fullScreenBrightness()
        .task { await card.loadSymbology(model) }
    }

    // MARK: Card

    private func cardContent(_ number: String) -> some View {
        VStack(spacing: Theme.Spacing.l) {
            LibraryCardView(libraryName: model.libraryName, logoUrl: model.library?.logoUrl,
                            borrowerName: model.accountName, number: number, expires: model.cardExpires,
                            barcode: card.barcode(for: number))
            Text("Show this at the library desk or self-checkout.")
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.textSecondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            Button {
                card.edit(current: number)
                fieldFocused = true
            } label: {
                Label("Edit card number", systemImage: "pencil")
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.textPrimary)
                    .padding(.horizontal, Theme.Spacing.xl)
                    .frame(minHeight: 44)
                    .background(Capsule().fill(Theme.Colors.surfaceElevated))
                    .contentShape(Capsule())
            }
            .buttonStyle(PressScaleStyle())
        }
    }

    // MARK: Entry form

    private var entryForm: some View {
        @Bindable var card = card
        return VStack(alignment: .leading, spacing: Theme.Spacing.l) {
            VStack(alignment: .leading, spacing: Theme.Spacing.s) {
                Text("Library card number")
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .accessibilityHidden(true)
                TextField(text: $card.draft) {
                    Text("Card number").foregroundStyle(Theme.Colors.textMuted)
                }
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.textPrimary)
                .tint(Theme.Colors.primary)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.characters)
                .submitLabel(.done)
                .focused($fieldFocused)
                .onSubmit(save)
                .accessibilityLabel("Library card number")
                .padding(.horizontal, Theme.Spacing.l)
                .frame(minHeight: 52)
                .background(RoundedRectangle(cornerRadius: Theme.Radius.field, style: .continuous).fill(Theme.Colors.field))
                Text("Enter the number on your library card or in your library account's Personal details (e.g. CL114D…).")
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            PrimaryPillButton(title: "Save", fillsWidth: true, action: save)
                .disabled(!card.canSave)
            if card.isEditingExisting {
                Button("Cancel") {
                    fieldFocused = false
                    card.cancelEditing()
                }
                .themeFont(.bodyStrong)
                .foregroundStyle(Theme.Colors.textSecondary)
                .frame(maxWidth: .infinity, minHeight: 44)
                .buttonStyle(.borderless)
            }
        }
        .padding(.top, Theme.Spacing.s)
        .onAppear { fieldFocused = true }
    }

    private func save() {
        guard card.canSave else { return }
        fieldFocused = false
        card.save(to: model)
    }
}

/// The white library card: logo and name, the borrower, the barcode and the number under it.
/// White in any theme, because scanners need black on white.
private struct LibraryCardView: View {
    let libraryName: String
    let logoUrl: String?
    let borrowerName: String?
    let number: String
    /// "Expires 12 Mar 2027" under the number, when the account gives it.
    let expires: String?
    let barcode: MyCardModel.Barcode

    private static let barcodeHeight: CGFloat = 100
    private let padding: CGFloat = 20

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Theme.Spacing.m) {
                LibraryLogo(name: libraryName, logoUrl: logoUrl, size: 40)
                VStack(alignment: .leading, spacing: 2) {
                    Text(libraryName)
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.onSheet)
                        .fixedSize(horizontal: false, vertical: true)
                    Text("Library card")
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.onSheetSecondary)
                }
            }
            .accessibilityElement(children: .combine)
            .padding(.horizontal, padding)
            .padding(.top, padding)
            if let borrowerName {
                Text(borrowerName)
                    .themeFont(.section)
                    .foregroundStyle(Theme.Colors.onSheet)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, padding)
                    .padding(.top, Theme.Spacing.l)
            }
            barcodeArea
                .padding(.top, Theme.Spacing.xl)
                .padding(.bottom, padding)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        // The shadow is the card's own, so no bar or digit gets a soft edge.
        .background(RoundedRectangle(cornerRadius: Theme.Radius.card, style: .continuous)
            .fill(Color.white)
            .themeShadow(.card))
    }

    @ViewBuilder
    private var barcodeArea: some View {
        switch barcode {
        case .loading:
            VStack(spacing: Theme.Spacing.s) {
                ProgressView()
                    .tint(Theme.Colors.onSheetSecondary)
                    .frame(maxWidth: .infinity)
                    .frame(height: Self.barcodeHeight)
                numberText(number)
                expiryText
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("Loading barcode for card \(number)")
        case .bars(let barcode):
            // Edge to edge: the card's white is part of the quiet zone, so the bars can be wider.
            VStack(spacing: Theme.Spacing.s) {
                BarcodeView(modules: barcode.modules)
                    .frame(height: Self.barcodeHeight)
                numberText(barcode.text)
                expiryText
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("Barcode for card \(number)")
            .accessibilityAddTraits(.isImage)
        case .unencodable(let number):
            VStack(spacing: Theme.Spacing.s) {
                Text(number)
                    .font(.system(size: 30, weight: .semibold, design: .monospaced))
                    .tracking(2)
                    .foregroundStyle(Theme.Colors.onSheet)
                    .multilineTextAlignment(.center)
                    .minimumScaleFactor(0.5)
                    .lineLimit(2)
                Text("This number can't be shown as a barcode.")
                    .themeFont(.caption)
                    .foregroundStyle(Theme.Colors.onSheetSecondary)
                    .multilineTextAlignment(.center)
            }
            .frame(maxWidth: .infinity)
            .padding(.horizontal, padding)
            .accessibilityElement(children: .combine)
        }
    }

    @ViewBuilder
    private var expiryText: some View {
        if let expires {
            Text("Expires \(expires)")
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.onSheetSecondary)
                .frame(maxWidth: .infinity)
        }
    }

    private func numberText(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 17, weight: .medium, design: .monospaced))
            .tracking(3)
            .foregroundStyle(Theme.Colors.onSheet)
            .lineLimit(1)
            .minimumScaleFactor(0.6)
            .frame(maxWidth: .infinity)
            .padding(.horizontal, padding)
    }
}

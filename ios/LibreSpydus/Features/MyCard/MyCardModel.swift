import Foundation
import LibreSpydusCore
import Observation

/// The card sheet's state (docs/native-apps.md §5.9): the card, or the number entry form when no
/// number is saved or the patron edits it, and the library's barcode symbology. The number itself
/// lives in `AppModel.cardNumber`.
@MainActor
@Observable
final class MyCardModel {
    enum Mode: Equatable {
        case card
        case entry
    }

    /// What the card's barcode area shows.
    enum Barcode: Equatable {
        /// The library's symbology isn't known yet (first open per library).
        case loading
        case bars(CardBarcode)
        /// `cardBarcode` can't encode the number (non-ASCII): it's shown as text instead.
        case unencodable(String)
    }

    private(set) var mode: Mode
    /// The entry form's text.
    var draft = ""
    /// The library's symbology: known from an earlier open, else nil until `loadSymbology`.
    private(set) var symbology: BarcodeSymbology?
    /// The form edits a number already on the card, so it offers Cancel back to the card.
    private(set) var isEditingExisting = false

    /// Opens on the card when there's a number, else on the entry form. `editing`: on the form,
    /// filled in with the number.
    init(cardNumber: String?, knownSymbology: BarcodeSymbology?, editing: Bool = false) {
        symbology = knownSymbology
        mode = .card
        if cardNumber == nil || editing { edit(current: cardNumber) }
    }

    var canSave: Bool { !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    /// "Edit card number": the entry form, filled in with the current number.
    func edit(current: String?) {
        draft = current ?? ""
        isEditingExisting = current != nil
        mode = .entry
    }

    /// Back to the card without saving (only when there's a number to show).
    func cancelEditing() {
        guard isEditingExisting else { return }
        isEditingExisting = false
        mode = .card
    }

    /// Saves the draft for this library and shows the card; false (nothing saved) while blank.
    @discardableResult
    func save(to model: AppModel) -> Bool {
        guard canSave else { return false }
        model.setCardNumber(draft)
        isEditingExisting = false
        mode = .card
        return true
    }

    /// Reads the library's symbology (instant after the first time: the core caches it), showing a
    /// remembered one meanwhile.
    func loadSymbology(_ model: AppModel) async {
        symbology = await model.loadCardSymbology()
    }

    /// The barcode area for `number`.
    func barcode(for number: String) -> Barcode {
        Self.barcode(for: number, symbology: symbology)
    }

    static func barcode(for number: String, symbology: BarcodeSymbology?) -> Barcode {
        guard let symbology else {
            // Code 128 is every symbology's fallback: a number it can't encode needn't wait.
            return cardBarcode(cardNumber: number, symbology: .code128) == nil ? .unencodable(number) : .loading
        }
        return cardBarcode(cardNumber: number, symbology: symbology).map(Barcode.bars) ?? .unencodable(number)
    }
}

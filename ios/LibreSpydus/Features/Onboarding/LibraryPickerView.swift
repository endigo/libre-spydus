import LibreSpydusCore
import SwiftUI

/// Library picker (I2): search + list grouped by region. Shown full screen on first launch and
/// as a sheet from Profile. Wave-1 stub: offers Canning only.
struct LibraryPickerView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        PlaceholderScreen(title: "Choose your library", systemImage: "building.columns",
                          message: "The full library directory arrives with the onboarding screens.") {
            PrimaryPillButton(title: "Use \(LibraryInfo.canning.name)") {
                model.selectLibrary(LibraryInfo.canning)
            }
        }
    }
}

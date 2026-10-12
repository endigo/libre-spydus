import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

@MainActor
struct FavouritesTests {
    private let hutt = LibraryInfo(id: "hutt", name: "Hutt City Libraries", origin: "https://hutt.spydus.co.nz",
                                   logoUrl: nil, country: "New Zealand", region: "NZ", isTest: false)

    @Test func theGridShowsTheSelectedLibrarysFavourites() {
        let model = AppModel(dependencies: .testing())
        model.selectLibrary(.canning)
        model.favourites.add(BookRef(id: "brn:1", title: "Canning book"), now: Date(timeIntervalSince1970: 1))
        model.favourites.add(BookRef(id: "brn:2", title: "Newer Canning book"), now: Date(timeIntervalSince1970: 2))

        model.selectLibrary(hutt)
        #expect(model.favourites.items.isEmpty)
        model.favourites.add(BookRef(id: "brn:9", title: "Hutt book"))
        #expect(model.favourites.items.map(\.title) == ["Hutt book"])

        model.selectLibrary(.canning)
        #expect(model.favourites.items.map(\.title) == ["Newer Canning book", "Canning book"])
    }

    @Test func removingFromTheGridForgetsTheBook() {
        let model = AppModel(dependencies: .testing())
        model.selectLibrary(.canning)
        let ref = BookRef(id: "brn:1", title: "Book")
        model.favourites.add(ref)
        model.favourites.remove(id: ref.id)
        #expect(!model.favourites.contains(ref))
    }

    @Test func twoColumnsFitTheScreenGutters() {
        // iPhone 17 Pro: 402 pt wide, 16 pt gutters and gap.
        #expect(FavouritesView.columnWidth(for: 402) == 177)
        #expect(FavouritesView.columnWidth(for: 0) == 80)
    }
}

import Foundation
import Testing
@testable import LibreSpydus

@MainActor
struct FavouritesStoreTests {
    let directory = TestData.temporaryDirectory()
    let origin = "https://canning.spydus.com"

    private func ref(_ id: String, title: String = "Title") -> BookRef {
        BookRef(id: id, title: title, author: "Author, An", coverUrl: "https://example.org/c.jpg",
                url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?BRN=\(id)", isbn: "978", year: "2026",
                formats: [FormatRef(code: "BK", label: "Book", reserveUrl: "r", availabilityUrl: "a")])
    }

    @Test func toggleAddsThenRemoves() {
        let store = FavouritesStore(directory: directory, origin: origin)
        #expect(store.items.isEmpty)
        #expect(store.toggle(ref("1")) == true)
        #expect(store.contains(id: "1"))
        #expect(store.items.first?.savedAt != nil)
        #expect(store.toggle(ref("1")) == false)
        #expect(!store.contains(id: "1"))
        #expect(store.items.isEmpty)
    }

    @Test func listIsNewestFirstAndReAddMovesToTop() {
        let store = FavouritesStore(directory: directory, origin: origin)
        let t0 = Date(timeIntervalSince1970: 1_000)
        store.add(ref("a"), now: t0)
        store.add(ref("b"), now: t0.addingTimeInterval(10))
        store.add(ref("c"), now: t0.addingTimeInterval(20))
        #expect(store.items.map(\.id) == ["c", "b", "a"])
        store.add(ref("a"), now: t0.addingTimeInterval(30))
        #expect(store.items.map(\.id) == ["a", "c", "b"])
        store.remove(id: "c")
        #expect(store.items.map(\.id) == ["a", "b"])
    }

    @Test func persistsAcrossInstancesWithAllFields() throws {
        let saved = ref("42", title: "The long night")
        let first = FavouritesStore(directory: directory, origin: origin)
        first.add(saved, now: Date(timeIntervalSince1970: 1_700_000_000))

        let second = FavouritesStore(directory: directory, origin: origin)
        let loaded = try #require(second.items.first)
        #expect(second.items.count == 1)
        #expect(loaded.id == "42")
        #expect(loaded.title == "The long night")
        #expect(loaded.formats == saved.formats)
        #expect(loaded.url == saved.url)
        #expect(loaded.savedAt == Date(timeIntervalSince1970: 1_700_000_000))
    }

    @Test func listsAreKeptPerOrigin() {
        let store = FavouritesStore(directory: directory, origin: origin)
        store.add(ref("canning"))
        store.load(origin: "https://hutt.spydus.co.nz")
        #expect(store.items.isEmpty)
        store.add(ref("hutt"))
        store.load(origin: origin)
        #expect(store.items.map(\.id) == ["canning"])
    }

    @Test func noOriginMeansEmptyAndUnsaved() {
        let store = FavouritesStore(directory: directory, origin: nil)
        store.add(ref("x"))
        let reloaded = FavouritesStore(directory: directory, origin: nil)
        #expect(reloaded.items.isEmpty)
    }

    @Test func deleteAllForgetsEveryOrigin() {
        let store = FavouritesStore(directory: directory, origin: origin)
        store.add(ref("1"))
        FavouritesStore.deleteAll(in: directory)
        #expect(FavouritesStore(directory: directory, origin: origin).items.isEmpty)
    }
}

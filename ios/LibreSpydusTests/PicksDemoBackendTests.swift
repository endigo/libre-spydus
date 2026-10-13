import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Demo mode's My Picks (§5.11): an in-memory list seeded with a work and an edition-level card,
/// a genuine `noResults` when emptied, and counts from save/remove.
struct PicksDemoBackendTests {
    private func makeDemo(publicBackend: FakeBackend = FakeBackend()) throws -> DemoBackend {
        DemoBackend(publicBackend: publicBackend, fixtures: try DemoFixtures.bundled(), mutationDelay: .zero)
    }

    private let session = DemoBackend.session

    @Test func seededWithAWorkAndAnEditionLevelCard() async throws {
        let demo = try makeDemo()
        let page = try await demo.picks(session: session, url: nil)
        #expect(!page.noResults)
        #expect(page.totalResults == 2)
        #expect(page.nextPageUrl == nil)
        #expect(page.records.map(\.workIrn) == [DemoBackend.seedPickIrns.work, DemoBackend.seedPickIrns.edition])

        let work = page.records[0]
        #expect(work.title == "The Midnight Library")
        #expect(work.formats.map(\.label) == ["Book", "Large Print"])

        let edition = page.records[1]
        #expect(edition.title == "The Love Library")
        #expect(edition.author == "Manning, Sarra")
        #expect(edition.formats.map(FormatRef.init).isEditionLevel)
        // Shown without a chip.
        #expect(SearchResultItem(edition, key: "work:\(DemoBackend.seedPickIrns.edition)", position: 1).formats.isEmpty)
    }

    @Test func theEditionLevelCardOpensOffline() async throws {
        let publicBackend = FakeBackend()
        let demo = try makeDemo(publicBackend: publicBackend)
        let edition = try await demo.picks(session: session, url: nil).records[1]
        let url = try #require(edition.formats.first?.availabilityUrl)
        let availability = try await demo.availability(url: url)
        #expect(availability.bibRecordIrn == DemoBackend.seedPickIrns.edition)
        let record = try await demo.record(url: url)
        #expect(record.bibIrn == DemoBackend.seedPickIrns.edition)
        #expect(publicBackend.searches.isEmpty)
    }

    @Test func removeCountsDownToAGenuineEmptyList() async throws {
        let demo = try makeDemo()
        #expect(try await demo.removePick(irn: DemoBackend.seedPickIrns.work, session: session) == 1)
        #expect(try await demo.removePick(irn: DemoBackend.seedPickIrns.edition, session: session) == 0)
        // Removing again changes nothing.
        #expect(try await demo.removePick(irn: DemoBackend.seedPickIrns.edition, session: session) == 0)
        let page = try await demo.picks(session: session, url: nil)
        #expect(page.noResults)
        #expect(page.records.isEmpty)
        #expect(page.totalResults == 0)
    }

    @Test func saveListsWhatTheBackendHasSeen() async throws {
        let demo = try makeDemo()
        // A search result's work.
        _ = try await demo.search(query: "baby-sitters")
        #expect(try await demo.savePick(irn: "15974354", session: session) == 3)
        // Saving it again doesn't add it twice.
        #expect(try await demo.savePick(irn: "15974354", session: session) == 3)
        // A loan's bib (a book page opened from Your Books): an edition-level card.
        #expect(try await demo.savePick(irn: "19243435", session: session) == 4)
        // A demo work by its work IRN.
        #expect(try await demo.savePick(irn: "15975964", session: session) == 5)

        let records = try await demo.picks(session: session, url: nil).records
        #expect(records.map(\.workIrn) == [DemoBackend.seedPickIrns.work, DemoBackend.seedPickIrns.edition,
                                           "15974354", "19243435", "15975964"])
        #expect(records[2].title == TestData.searchRecord.title)
        #expect(records[3].title == "Whistler")
        #expect(records[3].formats.map(FormatRef.init).isEditionLevel)
        #expect(records[4].title == "The Long Night")
        #expect(records.map(\.rank) == [1, 2, 3, 4, 5])
    }

    @Test func aRecordLoadedOnABookPageCanBeSaved() async throws {
        let demo = try makeDemo()
        // Hollow Bones' session-free link, as a related item or a link would load it.
        let record = try await demo.record(url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&IRN=19243431")
        let irn = try #require(record.bibIrn)
        #expect(try await demo.savePick(irn: irn, session: session) == 3)
    }

    @Test func refusesWhatTheLibraryWould() async throws {
        let demo = try makeDemo()
        await #expect(throws: SpydusError.Upstream(message: "None found")) {
            try await demo.savePick(irn: "99999999", session: session)
        }
        await #expect(throws: SpydusError.Parse(message: "not an IRN: 12a")) {
            try await demo.savePick(irn: "12a", session: session)
        }
        await #expect(throws: SpydusError.NotLoggedIn(message: "not logged in")) {
            try await demo.picks(session: "someone-else", url: nil)
        }
        await #expect(throws: SpydusError.NotLoggedIn(message: "not logged in")) {
            try await demo.removePick(irn: DemoBackend.seedPickIrns.work, session: "")
        }
    }

    @Test func longListsComeInPagesOfTwenty() async throws {
        let records = (0..<25).map { index in
            var record = TestData.searchRecord
            record.workIrn = String(20_000_000 + index)
            record.title = "Book \(index)"
            return record
        }
        let demo = try makeDemo(publicBackend: FakeBackend(records: records))
        _ = try await demo.search(query: "books")
        for record in records {
            _ = try await demo.savePick(irn: record.workIrn!, session: session)
        }
        let first = try await demo.picks(session: session, url: nil)
        #expect(first.records.count == 20)
        #expect(first.totalResults == 27)
        let next = try #require(first.nextPageUrl)
        let second = try await demo.picks(session: session, url: next)
        #expect(second.records.count == 7)
        #expect(second.records.first?.rank == 21)
        #expect(second.nextPageUrl == nil)
    }
}

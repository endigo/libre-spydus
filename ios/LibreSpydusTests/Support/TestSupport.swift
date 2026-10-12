import Foundation
import LibreSpydusCore
@testable import LibreSpydus

/// An offline `LibraryBackend`: canned public reads, records calls, never touches the network.
final class FakeBackend: LibraryBackend, @unchecked Sendable {
    let origin: String
    var isDemo: Bool { false }

    private let lock = NSLock()
    private var _verificationCookies: [String?] = []
    private var _searches: [String] = []
    var searchResult: SearchResults
    var accountResult: Result<AccountLinks, SpydusError> = .success(AccountLinks(
        name: "Fake Borrower", loansUrl: nil, reservationsReadyUrl: nil, reservationsPendingUrl: nil))

    init(origin: String = "https://canning.spydus.com", records: [SearchRecord] = [TestData.searchRecord]) {
        self.origin = origin
        searchResult = SearchResults(query: "q", setId: "1", totalResults: Int64(records.count), showing: nil,
                                     nextPageUrl: nil, prevPageUrl: nil, records: records)
    }

    var verificationCookies: [String?] { lock.withLock { _verificationCookies } }
    var searches: [String] { lock.withLock { _searches } }

    func setVerificationCookie(_ cookie: String?) { lock.withLock { _verificationCookies.append(cookie) } }

    func search(query: String) async throws -> SearchResults {
        lock.withLock { _searches.append(query) }
        return searchResult
    }
    func searchSorted(query: String, sort: SearchSortOption) async throws -> SearchResults { try await search(query: query) }
    func page(url: String) async throws -> SearchResults { searchResult }
    func availability(url: String) async throws -> Availability { throw SpydusError.Upstream(message: "fake") }
    func record(url: String) async throws -> RecordDetail { throw SpydusError.Upstream(message: "fake") }
    func related(url: String) async throws -> RelatedList { RelatedList(items: []) }
    func login(username: String, password: String) async throws -> String {
        guard password == "ok" else { throw SpydusError.CredentialsRejected(message: "credentials rejected") }
        return "fake-session"
    }
    func account(session: String) async throws -> AccountLinks { try accountResult.get() }
    func loans(session: String) async throws -> PatronList { PatronList(total: 0, items: []) }
    func reservations(session: String) async throws -> Reservations {
        Reservations(ready: PatronList(total: 0, items: []), pending: PatronList(total: 0, items: []))
    }
    func reserveOptions(url: String, session: String) async throws -> ReserveOptions { throw SpydusError.NotLoggedIn(message: "") }
    func placeReservation(url: String, session: String, xdays: String?, pickup: String?) async throws -> ReserveResult {
        ReserveResult(ok: false, message: nil)
    }
    func cancelReservation(url: String, session: String) async throws -> ReserveResult { ReserveResult(ok: false, message: nil) }
    func editOptions(url: String, session: String) async throws -> EditOptions { throw SpydusError.NotLoggedIn(message: "") }
    func editReservation(url: String, session: String, overrides: EditOverrides) async throws -> ReserveResult {
        ReserveResult(ok: false, message: nil)
    }
}

enum TestData {
    static let reserveUrl = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78529863/0/V/0/R/9/ALLWRKENQ?SVL=15974354&RECFMT=BK"

    static let searchRecord = SearchRecord(
        rank: 1, workIrn: "15974354", title: "Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter",
        detailUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/78529863/15974354,1",
        author: "Crenshaw, Ellen T.", authorRole: "author", authorUrl: nil, year: "2025", language: "English",
        summary: nil, isbn: "9781546144717",
        coverUrls: ["https://secure.syndetics.com/index.aspx?isbn=9781546144717/lc.gif&client=citycanning"],
        formats: [
            RecordFormat(code: "BK", label: "Book", reserveUrl: reserveUrl,
                         availabilityUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=BK"),
            RecordFormat(code: "EBK", label: "eBook", reserveUrl: nil,
                         availabilityUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78529863?FMT=RECFMT&QRY=EBK"),
        ]
    )

    /// A fresh, empty directory under the test's temp dir.
    static func temporaryDirectory() -> URL {
        let url = FileManager.default.temporaryDirectory.appending(path: "LibreSpydusTests-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    /// The repo checkout this test file was compiled from (tests read design/ and core/ goldens).
    static let repoRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()

    /// Whether the simulator can read the checkout (it can, on the Mac that built the tests).
    static let repoAvailable = FileManager.default.fileExists(atPath: repoRoot.appending(path: "design/tokens.json").path)
}

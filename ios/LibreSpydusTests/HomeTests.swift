import Foundation
import LibreSpydusCore
import Testing
import UIKit
@testable import LibreSpydus

struct HomeGreetingTests {
    @Test(arguments: [
        ("Borrower, Test", "Test"),
        ("Test Borrower", "Test"),
        ("Smith, Jane Mary", "Jane"),
        ("SMITH, JANE", "Jane"),
        ("jane smith", "Jane"),
        ("Watson, MARY-JANE", "Mary-Jane"),
        ("O'Brien, Siobhán", "Siobhán"),
        ("Mrs Jane Smith", "Jane"),
        ("Smith, Dr. Jane", "Jane"),
        ("Smith, J. Jane", "Jane"),
        ("  Jane   Smith  ", "Jane"),
        ("Jane", "Jane"),
        ("McKenzie, DeShawn", "DeShawn"),
    ])
    func firstNames(accountName: String, expected: String) {
        #expect(HomeGreeting.firstName(from: accountName) == expected)
    }

    @Test(arguments: [nil, "", "   ", "Smith,", "J. Smith", "Mr", "12345678"] as [String?])
    func noUsableFirstName(accountName: String?) {
        #expect(HomeGreeting.firstName(from: accountName) == nil)
        #expect(HomeGreeting.title(accountName: accountName) == "Hello!")
    }

    @Test func greetingTitle() {
        #expect(HomeGreeting.title(accountName: "Borrower, Test") == "Hello Test!")
        #expect(HomeGreeting.title(accountName: "Test Borrower") == "Hello Test!")
    }
}

struct HomeLoanShelfTests {
    private var calendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Australia/Perth")!
        return calendar
    }

    private func loan(_ title: String, due: String?, bib: String? = nil) -> PatronItem {
        PatronItem(irn: nil, bibIrn: bib ?? title, title: title, bibUrl: nil, author: "Haig, Matt", year: nil,
                   coverUrls: [], isbn: nil, fields: due.map { ["Due": $0] } ?? [:], actions: [])
    }

    @Test func captionsAndTones() throws {
        let today = try #require(DueDate.parse("24 Oct 2026", calendar: calendar))
        func item(_ due: String?) -> LoanShelfItem { LoanShelfItem(loan("Book", due: due), today: today, calendar: calendar) }

        #expect(item("22 Oct 2026").caption == "2 days overdue")
        #expect(item("22 Oct 2026").tone == .overdue)
        #expect(item("23 Oct 2026").caption == "1 day overdue")
        #expect(item("24 Oct 2026").caption == "Due today")
        #expect(item("24 Oct 2026").tone == .soon)
        #expect(item("27 Oct 2026").caption == "3 days left")
        #expect(item("27 Oct 2026").tone == .soon)
        #expect(item("28 Oct 2026").tone == .normal)
        #expect(item("7 Nov 2026").caption == "14 days left")
        // Beyond two weeks the date reads better than a count.
        let far = item("20 Nov 2026")
        #expect(far.caption == "Due \(Formatting.shortDate(far.dueDate!))")
        #expect(far.tone == .normal)
        // Unreadable dates are shown as the library wrote them; no date, no caption.
        #expect(item("Recalled").caption == "Due Recalled")
        #expect(item("Recalled").dueDate == nil)
        #expect(item(nil).caption == nil)
    }

    @Test func loansAreSortedByDueDateWithUndatedLast() throws {
        let today = try #require(DueDate.parse("24 Oct 2026", calendar: calendar))
        let items = LoanShelfItem.make(from: [
            loan("Later", due: "10 Nov 2026"),
            loan("Undated A", due: nil),
            loan("Overdue", due: "20 Oct 2026"),
            loan("Soon", due: "26 Oct 2026"),
            loan("Undated B", due: "?"),
            loan("Soon", due: "26 Oct 2026"), // the same loan listed twice
        ], today: today, calendar: calendar)
        #expect(items.map(\.ref.title) == ["Overdue", "Soon", "Later", "Undated A", "Undated B"])
    }
}

@MainActor
struct HomeViewModelTests {
    private func model(backend: PagingFakeBackend, demo: Bool) -> AppModel {
        let dependencies = AppModel.Dependencies(
            defaults: UserDefaults(suiteName: "LibreSpydusTests-home-\(UUID().uuidString)")!,
            sessions: InMemorySecretStore(), verificationCookies: InMemorySecretStore(),
            favouritesDirectory: TestData.temporaryDirectory(), isDemo: demo,
            makeBackend: { _ in backend }
        )
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        return model
    }

    @Test func shelvesLoadIndependently() async {
        let records = (1...14).map { SearchFixtures.record(String($0)) } + [SearchFixtures.record("1")]
        let backend = PagingFakeBackend(first: .success(SearchFixtures.page(records)))
        backend.loansResult = .failure(.Network(message: "timed out"))
        let app = model(backend: backend, demo: true) // demo: starts signed in
        let home = HomeViewModel()

        await home.loadIfNeeded(app)
        #expect(home.arrivals.value?.count == HomeViewModel.shelfLimit)
        #expect(home.loans.error?.kind == .network)
        // New arrivals is the first shelf, newest first.
        #expect(backend.searchCalls == ["\(homeShelves()[0].query)|newest"])

        // Same key again (back from a pushed screen): nothing reloads.
        await home.loadIfNeeded(app)
        #expect(backend.searchCalls.count == 1)

        backend.loansResult = .success(PatronList(total: 1, items: [
            PatronItem(irn: "1", bibIrn: "1", title: "Whistler", bibUrl: nil, author: nil, year: nil, coverUrls: [],
                       isbn: nil, fields: ["Due": DueDate.format(.now)], actions: []),
        ]))
        await home.retryLoans(app)
        #expect(home.loans.value?.map(\.caption) == ["Due today"])
    }

    @Test func aLoadWhoseTaskWasCancelledStillCounts() async {
        // Pushing a screen mid-load cancels Home's task, but core calls finish anyway; coming
        // back must not reload shelves that did load.
        let backend = PagingFakeBackend(first: .success(SearchFixtures.page([SearchFixtures.record("1")])))
        let app = model(backend: backend, demo: true)
        let home = HomeViewModel()
        let task = Task { await home.loadIfNeeded(app) }
        task.cancel()
        await task.value
        #expect(home.arrivals.value?.count == 1)
        await home.loadIfNeeded(app)
        #expect(backend.searchCalls.count == 1)
    }

    @Test func signedOutHasNoLoansAndSignInReloads() async {
        let backend = PagingFakeBackend(first: .success(SearchFixtures.page([SearchFixtures.record("1")])))
        let app = model(backend: backend, demo: false)
        let home = HomeViewModel()
        await home.loadIfNeeded(app)
        #expect(!app.isSignedIn)
        #expect(home.loans.isIdle)
        #expect(home.arrivals.value?.count == 1)

        app.report(SpydusError.NotLoggedIn(message: "")) // bumps accountGeneration
        await home.loadIfNeeded(app)
        #expect(backend.searchCalls.count == 2)
    }

    @Test func anUnrecognisedArrivalsPageIsAnError() async {
        let backend = PagingFakeBackend(first: .success(SearchFixtures.page([], noResults: false)))
        let home = HomeViewModel()
        await home.loadIfNeeded(model(backend: backend, demo: false))
        #expect(home.arrivals.error == SearchResultsModel.unrecognisedPage)
    }

    @Test func everyGenreHasAnIconAndARoute() {
        let genres = homeGenres()
        #expect(genres.count == 8)
        for genre in genres {
            let symbol = GenreStyle.systemImage(for: genre.key)
            #expect(symbol != "books.vertical.fill", "no icon for \(genre.key)")
            #expect(UIImage(systemName: symbol) != nil, "missing SF Symbol \(symbol)")
            #expect(HomeViewModel.route(for: genre) == .results(query: genre.query, sort: .relevance, title: genre.label))
        }
        #expect(UIImage(systemName: GenreStyle.systemImage(for: "unknown")) != nil)
    }
}

private extension Loadable {
    var isIdle: Bool {
        if case .idle = self { return true }
        return false
    }
}

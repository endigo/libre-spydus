import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

struct FormattingTests {
    @Test(arguments: [
        ("Haig, Matt, author", "Matt Haig"),
        ("Crenshaw, Ellen T.", "Ellen T. Crenshaw"),
        ("Galligan, Gale, 1964- illustrator", "Gale Galligan"),
        ("Wood, Mary, 1945-", "Mary Wood"),
        ("Rowling, J. K., author", "J. K. Rowling"),
        ("Smith, author", "Smith"),
        ("Disney Enterprises", "Disney Enterprises"),
    ])
    func authorNames(raw: String, expected: String) {
        #expect(Formatting.authorName(raw) == expected)
    }

    @Test func greetingAndFirstName() {
        #expect(Formatting.greeting(accountName: "Test Borrower") == "Hello Test!")
        #expect(Formatting.greeting(accountName: "Borrower, Test") == "Hello Test!")
        #expect(Formatting.greeting(accountName: nil) == "Hello!")
    }

    @Test func dueStatus() throws {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Australia/Perth")!
        let today = try #require(DueDate.parse("24 Oct 2026", calendar: calendar))
        let status = try #require(DueStatus(dueText: "27 Oct 2026", today: today, calendar: calendar))
        #expect(status.daysLeft == 3)
        #expect(status.isUrgent && !status.isOverdue)
        #expect(status.daysText == "3 days left")
        #expect(DueStatus(dueText: "22 Oct 2026", today: today, calendar: calendar)?.daysText == "2 days overdue")
        #expect(DueStatus(dueText: "24 Oct 2026", today: today, calendar: calendar)?.daysText == "Due today")
        #expect(DueStatus(dueText: "not a date", today: today) == nil)
        #expect(DueDate.format(today, calendar: calendar) == "24 Oct 2026")
    }

    @Test func userFacingErrors() {
        #expect(UserFacingError(SpydusError.Network(message: "x"), libraryName: "Canning").message
            == "Can't reach Canning. Check your connection.")
        #expect(UserFacingError(SpydusError.CredentialsRejected(message: ""), libraryName: "Canning").message
            == "Card number or PIN not recognised.")
        let parse = UserFacingError(SpydusError.Parse(message: "parse error: bad"), libraryName: "Canning")
        #expect(parse.message == "The library site returned something unexpected.")
        #expect(parse.detail == "parse error: bad")
        #expect(parse.canRetry)
        #expect(!UserFacingError(SpydusError.NotLoggedIn(message: ""), libraryName: "C").canRetry)
    }

    @Test func noResultsComesFromTheCore() {
        let empty = SearchResults(query: "zzz", setId: nil, totalResults: nil, showing: nil, nextPageUrl: nil,
                                  prevPageUrl: nil, noResults: true, records: [])
        #expect(empty.isNoResults)
        // An unrecognised page (no records, no "did not find" message) is not "no results".
        let unrecognised = SearchResults(query: "q", setId: nil, totalResults: nil, showing: nil,
                                         nextPageUrl: nil, prevPageUrl: nil, noResults: false, records: [])
        #expect(!unrecognised.isNoResults)
    }
}

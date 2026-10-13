import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Key screens take their words from the shared catalogue (design/copy.json → Copy.swift,
/// docs/native-apps.md §7.1), so iOS and Android say the same thing. Each test checks that a screen's
/// text is the `Copy` string, and pins a few catalogue strings the glossary decides.
@MainActor
struct CopyUsageTests {
    // MARK: My Picks

    @Test func myPicksIsTheTitleSignedOutAndSignedIn() async throws {
        let harness = PicksAppModelTests.Harness(irns: [LibraryInfo.canning.origin: ["1"]])
        let model = harness.makeModel()
        model.selectLibrary(.canning)

        let signedOut = FavouritesView.Content(model)
        #expect(signedOut == .local)
        #expect(signedOut.title == Copy.Picks.title)
        #expect(signedOut.subtitle == Copy.Picks.subtitlePhone)

        try await model.signIn(username: "card", password: "ok")
        let signedIn = FavouritesView.Content(model)
        #expect(signedIn == .myPicks)
        #expect(signedIn.title == Copy.Picks.title)
        #expect(signedIn.subtitle == Copy.Picks.subtitleAccount)

        // The tab says the same as the screen; never "Favourites".
        #expect(AppTab.favourites.title == Copy.Tabs.myPicks)
        #expect(Copy.Tabs.myPicks == Copy.Picks.title)
        #expect(Copy.Picks.title == "My Picks")
        #expect(Copy.Picks.subtitlePhone == "Saved on this phone")
        #expect(Copy.Picks.subtitleAccount == "Saved on your library account")
    }

    // MARK: Your Books

    @Test func aReservationsStatusLineComesFromTheCatalogue() throws {
        let ready = ReservationRow(item: Self.item(["Status": "Please pickup from: RIVERTON LIBRARY"]), kind: .ready)
        #expect(ready.statusLabel == Copy.YourBooks.readyToCollect)
        #expect(ready.meta == Copy.YourBooks.collectFrom(branch: "Riverton Library"))
        #expect(ready.meta == "Collect from Riverton Library")

        let placed = try #require(DueDate.parse("02 Oct 2026", calendar: .libraryDates))
        let pending = ReservationRow(item: Self.item(["Status": "Pending", "Rank": "4", "Placed": "02 Oct 2026"]),
                                     kind: .pending)
        let placedLine = Copy.YourBooks.placed(date: Formatting.shortDate(placed))
        #expect(pending.statusLabel == Copy.YourBooks.pending)
        #expect(pending.meta == Copy.YourBooks.queuePlace(place: "4") + Copy.Common.separator + placedLine)
        #expect(pending.spokenMeta == Copy.YourBooks.queuePlaceLabel(place: "4") + Copy.Common.separator + placedLine)

        let textPlace = ReservationRow(item: Self.item(["Status": "Pending", "Rank": "1 of 3"]), kind: .pending)
        #expect(textPlace.meta == Copy.YourBooks.queuePlaceText(place: "1 of 3"))
        #expect(textPlace.meta == "1 of 3 in queue") // §7.1: never "Rank"
    }

    @Test func theCancelDialogComesFromTheCatalogue() {
        let ready = ReservationRow(item: Self.item(["Pickup location": "RIVERTON LIBRARY"], title: "Whistler"), kind: .ready)
        #expect(CancelConfirm.message(for: ready) == Copy.YourBooks.cancelReadyMessage(title: "Whistler", branch: "Riverton Library"))
        let readyNoBranch = ReservationRow(item: Self.item([:], title: "Whistler"), kind: .ready)
        #expect(CancelConfirm.message(for: readyNoBranch) == Copy.YourBooks.cancelReadyMessageNoBranch(title: "Whistler"))
        let pending = ReservationRow(item: Self.item(["Status": "Pending"], title: "Whistler"), kind: .pending)
        #expect(CancelConfirm.message(for: pending) == Copy.YourBooks.cancelPendingMessage(title: "Whistler"))
        #expect(CancelConfirm.message(for: pending)
            == "“Whistler” will be removed from your reservations and you’ll lose your place in the queue.")

        // The menu item and the dialog's destructive button are one string, in sentence case.
        #expect(ReservationAction.cancel.menuTitle == Copy.YourBooks.cancelReservation)
        #expect(ReservationAction.edit.menuTitle == Copy.YourBooks.editReservation)
        #expect(Copy.YourBooks.cancelTitle == "Cancel this reservation?")
        #expect(Copy.YourBooks.cancelReservation == "Cancel reservation")
        #expect(Copy.YourBooks.keepReservation == "Keep reservation")
        #expect(SwipeReveal.hint(swipes: true).text == Copy.YourBooks.swipeHint)
    }

    @Test func aLoansDueLineComesFromTheCatalogue() throws {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Australia/Perth")!
        let today = try #require(DueDate.parse("12 Oct 2026", calendar: calendar))
        func line(_ due: String) throws -> String {
            LoanRow.dueLine(try #require(DueStatus(dueText: due, today: today, calendar: calendar)))
        }
        let date = { (text: String) in Formatting.shortDate(DueDate.parse(text, calendar: calendar)!) }
        #expect(try line("15 Oct 2026") == Copy.Loans.lineDays(date: date("15 Oct 2026"), days: Copy.Loans.daysLeft(count: 3)))
        #expect(try line("13 Oct 2026") == Copy.Loans.lineTomorrow(date: date("13 Oct 2026")))
        #expect(try line("12 Oct 2026") == Copy.Loans.lineToday(date: date("12 Oct 2026")))
        #expect(try line("10 Oct 2026") == Copy.Loans.lineDays(date: date("10 Oct 2026"), days: Copy.Loans.daysOverdue(count: 2)))
        #expect(Copy.Loans.daysOverdue(count: 1) == "1 day overdue")
        #expect(Copy.Loans.daysLeft(count: 2) == "2 days left")
    }

    // MARK: Book page

    @Test func theAlreadyYoursPillComesFromTheCatalogue() {
        let queued = HeldStatus(AccountHolding(kind: .reservationPending, item: Self.item(["Status": "Pending", "Rank": "2"])))
        #expect(queued.text == Copy.Detail.reservedQueue(place: "2"))
        #expect(queued.accessibilityLabel == Copy.Detail.reservedQueueLabel(place: "2"))
        let ready = HeldStatus(AccountHolding(kind: .reservationReady,
                                              item: Self.item(["Pickup location": "RIVERTON LIBRARY"])))
        #expect(ready.text == Copy.Detail.readyToCollectAt(branch: "Riverton Library"))
        #expect(ready.accessibilityLabel == Copy.Detail.readyToCollectAtLabel(branch: "Riverton Library"))
    }

    // MARK: Errors

    @Test func errorsComeFromTheCatalogue() {
        let network = UserFacingError(SpydusError.Network(message: "x"), libraryName: "Canning")
        #expect(network.title == Copy.Errors.cantConnectTitle)
        #expect(network.message == Copy.Errors.network(library: "Canning"))
        let unexpected = UserFacingError(SpydusError.Parse(message: "x"), libraryName: "Canning")
        #expect(unexpected.title == Copy.Errors.unexpectedTitle)
        #expect(unexpected.message == Copy.Errors.unexpected)
    }

    // MARK: Helpers

    private static func item(_ fields: [String: String], title: String = "A book") -> PatronItem {
        PatronItem(irn: "1", bibIrn: "2", title: title, bibUrl: nil, author: nil, year: nil, coverUrls: [],
                   isbn: nil, fields: fields, actions: [])
    }
}

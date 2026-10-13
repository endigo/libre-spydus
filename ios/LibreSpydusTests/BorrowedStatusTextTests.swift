import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// Canning puts the pickup branch in a reservation's Status field (real list pages, 2026-10-12).
struct BorrowedStatusTextTests {
    @Test func pendingStatusNamesTheBranch() {
        let text = StatusText("Requested pickup location: RIVERTON LIBRARY")
        #expect(text == StatusText(status: nil, pickup: "RIVERTON LIBRARY", pickupBy: nil))
    }

    @Test func readyStatusNamesTheBranchAndDate() {
        let text = StatusText("Please pickup from: RIVERTON LIBRARY by 17 Oct 2026")
        #expect(text == StatusText(status: nil, pickup: "RIVERTON LIBRARY", pickupBy: "17 Oct 2026"))
    }

    @Test func otherStatusesAreKept() {
        #expect(StatusText("In transit") == StatusText(status: "In transit"))
        #expect(StatusText("  ") == StatusText())
        #expect(StatusText(nil) == StatusText())
    }

    @Test func pendingRowShowsPendingAndTheBranch() {
        let row = ReservationRow(item: item(status: "Requested pickup location: RIVERTON LIBRARY",
                                            extra: ["Rank": "5", "Placed": "22 Sep 2026"]), kind: .pending)
        #expect(row.statusLabel == "Pending")
        #expect(row.meta?.contains("Collect from Riverton Library") == true)
        #expect(row.meta?.contains("#5 in queue") == true)
    }

    @Test func readyRowShowsCollectByDate() {
        let row = ReservationRow(item: item(status: "Please pickup from: RIVERTON LIBRARY by 17 Oct 2026"), kind: .ready)
        #expect(row.statusLabel == "Ready to collect")
        // The date follows the device locale ("17 Oct" / "Oct 17").
        #expect(row.meta?.hasPrefix("Collect from Riverton Library by ") == true)
        #expect(row.meta?.contains("17") == true)
    }

    private func item(status: String, extra: [String: String] = [:]) -> PatronItem {
        PatronItem(irn: "1", bibIrn: "2", title: "A book", bibUrl: nil, author: nil, year: nil, coverUrls: [],
                   isbn: nil, fields: extra.merging(["Status": status]) { a, _ in a }, actions: [])
    }
}

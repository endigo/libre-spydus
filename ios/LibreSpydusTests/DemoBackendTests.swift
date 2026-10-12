import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

struct DemoBackendTests {
    private func makeDemo(delay: Duration = .zero, publicBackend: FakeBackend = FakeBackend()) throws -> DemoBackend {
        DemoBackend(publicBackend: publicBackend, fixtures: try DemoFixtures.bundled(), mutationDelay: delay)
    }

    @Test func loginAcceptsOnlyDemoDemo() async throws {
        let demo = try makeDemo()
        #expect(try await demo.login(username: " Demo ", password: "demo") == DemoBackend.session)
        await #expect(throws: SpydusError.CredentialsRejected(message: "credentials rejected")) {
            try await demo.login(username: "demo", password: "1234")
        }
        await #expect(throws: SpydusError.self) {
            try await demo.login(username: "2000123", password: "demo")
        }
    }

    @Test func accountCallsNeedTheDemoSession() async throws {
        let demo = try makeDemo()
        await #expect(throws: SpydusError.NotLoggedIn(message: "not logged in")) {
            try await demo.loans(session: "someone-else")
        }
        let account = try await demo.account(session: DemoBackend.session)
        #expect(account.name == "Test Borrower")
    }

    @Test func loansAreDueRelativeToToday() async throws {
        let demo = try makeDemo()
        let loans = try await demo.loans(session: DemoBackend.session)
        #expect(loans.items.count == 5)
        let statuses = loans.items.compactMap { DueStatus(dueText: $0.fields["Due"]) }
        #expect(statuses.count == 5)
        #expect(statuses.map(\.daysLeft) == [-2, 2, 5, 12, 19])
        #expect(statuses[0].isOverdue)
        #expect(statuses[1].isUrgent)
        #expect(!statuses[2].isUrgent)
    }

    @Test func reservationsComeFromTheSyntheticGolden() async throws {
        let demo = try makeDemo()
        let reservations = try await demo.reservations(session: DemoBackend.session)
        #expect(reservations.ready.items.map(\.title) == ["The midnight library"])
        #expect(reservations.ready.items[0].fields["Status"] == "Awaiting pickup")
        #expect(reservations.pending.items.count == 3)
    }

    @Test func cancelRemovesTheRowAfterTheDelay() async throws {
        let demo = try makeDemo(delay: .milliseconds(150))
        let before = try await demo.reservations(session: DemoBackend.session)
        let row = before.ready.items[0]
        let cancel = try #require(row.actions.first { $0.label.range(of: "cancel", options: .caseInsensitive) != nil })

        let clock = ContinuousClock()
        let start = clock.now
        let result = try await demo.cancelReservation(url: cancel.href, session: DemoBackend.session)
        #expect(clock.now - start >= .milliseconds(150))
        #expect(result.ok)

        let after = try await demo.reservations(session: DemoBackend.session)
        #expect(after.ready.items.isEmpty)
        #expect(after.pending.items.count == before.pending.items.count)

        let again = try await demo.cancelReservation(url: cancel.href, session: DemoBackend.session)
        #expect(!again.ok)
    }

    @Test func cancelNeedsTheSession() async throws {
        let demo = try makeDemo()
        await #expect(throws: SpydusError.self) {
            try await demo.cancelReservation(url: "x", session: "")
        }
    }

    @Test func reserveAddsAPendingRowForASearchedRecord() async throws {
        let demo = try makeDemo()
        _ = try await demo.search(query: "baby-sitters")
        let result = try await demo.placeReservation(url: TestData.reserveUrl, session: DemoBackend.session,
                                                     xdays: "30", pickup: nil)
        #expect(result.ok)
        let pending = try await demo.reservations(session: DemoBackend.session).pending.items
        #expect(pending.first?.title == TestData.searchRecord.title)
        #expect(pending.first?.fields["Pickup location"] == "RIVERTON LIBRARY")

        let duplicate = try await demo.placeReservation(url: TestData.reserveUrl, session: DemoBackend.session,
                                                        xdays: nil, pickup: nil)
        #expect(!duplicate.ok)
    }

    @Test func editChangesThePickupLocation() async throws {
        let demo = try makeDemo()
        let row = try await demo.reservations(session: DemoBackend.session).ready.items[0]
        let edit = try #require(row.actions.first { $0.label.range(of: "edit", options: .caseInsensitive) != nil })
        let options = try await demo.editOptions(url: edit.href, session: DemoBackend.session)
        #expect(options.pickupSelected == "25363")
        let result = try await demo.editReservation(
            url: edit.href, session: DemoBackend.session,
            overrides: EditOverrides(pickup: "25353", delayUntil: nil, notRequiredAfter: nil, clearDelay: false, clearExpiry: false))
        #expect(result.ok)
        let updated = try await demo.reservations(session: DemoBackend.session).ready.items[0]
        #expect(updated.fields["Pickup location"] == "CANNINGTON LIBRARY")
    }

    @Test func publicReadsGoToTheRealBackend() async throws {
        let fake = FakeBackend()
        let demo = try makeDemo(publicBackend: fake)
        let results = try await demo.search(query: "cats")
        #expect(results.records.count == 1)
        #expect(fake.searches == ["cats"])
        demo.setVerificationCookie("aws-waf-token=x")
        #expect(fake.verificationCookies == ["aws-waf-token=x"])
        #expect(demo.isDemo)
        #expect(demo.origin == fake.origin)
    }

    @Test func fixturesFollowTheBackendOrigin() throws {
        let fixtures = try DemoFixtures.bundled(origin: "https://hutt.spydus.co.nz")
        let urls = (fixtures.loans + fixtures.ready + fixtures.pending).compactMap(\.bibUrl)
        #expect(!urls.isEmpty)
        #expect(urls.allSatisfy { $0.hasPrefix("https://hutt.spydus.co.nz/") })
    }

    /// The bundled demo JSON is the synthetic goldens verbatim (skipped if the repo isn't readable).
    @Test(.enabled(if: TestData.repoAvailable, "needs the repo checkout (core/tests/golden)"))
    func demoJsonMatchesTheSyntheticGoldens() throws {
        let golden = TestData.repoRoot.appending(path: "core/tests/golden/synthetic")
        let pairs = [
            ("account-links.json", "demo-account"),
            ("reserve-options.json", "demo-reserve-options"),
            ("edit-options.json", "demo-edit-options"),
        ]
        for (goldenName, demoName) in pairs {
            let goldenData = try Data(contentsOf: golden.appending(path: goldenName))
            let demoData = try Data(contentsOf: try #require(Bundle.main.url(forResource: demoName, withExtension: "json")))
            let lhs = try JSONSerialization.jsonObject(with: goldenData) as? NSDictionary
            let rhs = try JSONSerialization.jsonObject(with: demoData) as? NSDictionary
            #expect(lhs == rhs, "\(demoName).json differs from \(goldenName)")
        }
    }
}

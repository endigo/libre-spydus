import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

private func row(_ title: String, irn: String, kind: ReservationRow.Kind = .pending,
                 actions: [(String, String)] = []) -> ReservationRow {
    ReservationRow(item: PatronItem(irn: irn, bibIrn: nil, title: title, bibUrl: nil, author: nil, year: nil,
                                    coverUrls: [], isbn: nil, fields: [:],
                                    actions: actions.map { PatronAction(label: $0.0, href: $0.1) }),
                   kind: kind)
}

private let cancel = ("Cancel reservation", "https://canning.spydus.com/cancel/1")
private let edit = ("Edit", "https://canning.spydus.com/edit/1")

// MARK: - Which actions a card offers

struct ReservationActionTests {
    @Test func onlyActionsWithALinkAreOffered() {
        #expect(ReservationAction.available(for: row("Both", irn: "1", actions: [cancel, edit])) == [.edit, .cancel])
        #expect(ReservationAction.available(for: row("Cancel only", irn: "2", actions: [cancel])) == [.cancel])
        #expect(ReservationAction.available(for: row("Edit only", irn: "3", actions: [edit])) == [.edit])
        #expect(ReservationAction.available(for: row("Neither", irn: "4")).isEmpty)
        #expect(ReservationAction.available(for: row("Empty link", irn: "5", actions: [("Cancel", "")])).isEmpty)
    }

    @Test func aBusyRowOffersNothing() {
        #expect(ReservationAction.available(for: row("Both", irn: "1", actions: [cancel, edit]), busy: true).isEmpty)
    }

    @Test func wording() {
        #expect(ReservationAction.edit.title == "Edit")
        #expect(ReservationAction.cancel.title == "Cancel")
        #expect(ReservationAction.edit.menuTitle == "Edit reservation")
        #expect(ReservationAction.cancel.menuTitle == "Cancel reservation")
        #expect(ReservationAction.cancel.role == .destructive)
        #expect(ReservationAction.edit.role == nil)
    }

    @Test func swipeHintOnlyWhenSomethingCanBeChanged() throws {
        let fixtures = try DemoFixtures.bundled()
        let demo = ReservationSections(Reservations(
            ready: PatronList(total: nil, items: fixtures.ready), pending: PatronList(total: nil, items: fixtures.pending)))
        #expect(demo.hasActions)
        #expect(!ReservationSections(ready: [row("Ready", irn: "1", kind: .ready)], pending: [row("In transit", irn: "2")]).hasActions)
        #expect(ReservationSections(pending: [row("In transit", irn: "2"), row("Queued", irn: "3", actions: [edit])]).hasActions)
        #expect(!ReservationSections().hasActions)
    }

    @Test func hintNamesTheGestureThisDeviceHas() {
        #expect(SwipeReveal.hint(swipes: true).text == "Swipe left on a reservation to change or cancel it.")
        #expect(SwipeReveal.hint(swipes: false).text == "Touch and hold a reservation to change or cancel it.")
        #expect(SwipeReveal.isSupported) // the test runtime is iOS 18 or later
    }
}

// MARK: - Swipe geometry

struct SwipeRevealTests {
    private let width = SwipeReveal.openWidth(tiles: 2)

    @Test func openWidthFitsEachTileAndItsGap() {
        #expect(SwipeReveal.openWidth(tiles: 0) == 0)
        #expect(SwipeReveal.openWidth(tiles: 1) == SwipeReveal.tileWidth + SwipeReveal.spacing)
        #expect(width == 2 * (SwipeReveal.tileWidth + SwipeReveal.spacing))
    }

    @Test func theCardFollowsTheFingerBetweenClosedAndOpen() {
        #expect(SwipeReveal.offset(start: 0, translation: -40, openWidth: width) == -40)
        #expect(SwipeReveal.offset(start: -width, translation: 30, openWidth: width) == -width + 30)
    }

    @Test func pastEitherEndItResists() {
        let far = SwipeReveal.offset(start: 0, translation: -1000, openWidth: width)
        #expect(far < -width)
        #expect(far > -2 * width) // a long swipe never runs away
        let further = SwipeReveal.offset(start: 0, translation: -1500, openWidth: width)
        #expect(further < far) // but still moves with the finger

        let right = SwipeReveal.offset(start: 0, translation: 80, openWidth: width)
        #expect(right > 0 && right < 80)
    }

    @Test func releaseSettlesByDistanceOrFlick() {
        #expect(!SwipeReveal.settlesOpen(offset: -30, velocity: 0, openWidth: width))
        #expect(SwipeReveal.settlesOpen(offset: -width * 0.6, velocity: 0, openWidth: width))
        #expect(SwipeReveal.settlesOpen(offset: -20, velocity: -800, openWidth: width)) // short flick left
        #expect(!SwipeReveal.settlesOpen(offset: -width, velocity: 800, openWidth: width)) // flick right closes
        #expect(!SwipeReveal.settlesOpen(offset: -width, velocity: 0, openWidth: 0)) // nothing to show
    }

    @Test func aFullSwipeOnlyOpens() {
        // The card's only resting states are open and closed; a run-away swipe just opens it.
        let offset = SwipeReveal.offset(start: 0, translation: -2000, openWidth: width)
        #expect(SwipeReveal.settlesOpen(offset: offset, velocity: -5000, openWidth: width))
        #expect(SwipeReveal.progress(offset: offset, openWidth: width) == 1)
    }

    @Test func progress() {
        #expect(SwipeReveal.progress(offset: 0, openWidth: width) == 0)
        #expect(SwipeReveal.progress(offset: -width / 2, openWidth: width) == 0.5)
        #expect(SwipeReveal.progress(offset: 20, openWidth: width) == 0)
        #expect(SwipeReveal.progress(offset: -20, openWidth: 0) == 0)
    }

    @Test func onlySidewaysDragsSwipe() {
        #expect(SwipeReveal.isSwipe(CGSize(width: -30, height: 5), isOpen: false))
        #expect(!SwipeReveal.isSwipe(CGSize(width: -10, height: 40), isOpen: false)) // a scroll
        #expect(!SwipeReveal.isSwipe(CGSize(width: 30, height: 2), isOpen: false)) // nothing on the leading side
        #expect(SwipeReveal.isSwipe(CGSize(width: 30, height: 2), isOpen: true)) // swiping an open card shut
        #expect(!SwipeReveal.isSwipe(CGSize(width: 20, height: 20), isOpen: true))
    }
}

// MARK: - View model: the open card

@MainActor
struct BorrowedSwipeViewModelTests {
    private func load() async throws -> (BorrowedViewModel, AppModel, ReservationSections) {
        let demo = DemoBackend(publicBackend: FakeBackend(), fixtures: try DemoFixtures.bundled(),
                               mutationDelay: .milliseconds(80), snapshotDelay: .milliseconds(10))
        let dependencies = AppModel.Dependencies(
            defaults: UserDefaults(suiteName: "LibreSpydusTests-\(UUID().uuidString)")!,
            sessions: InMemorySecretStore(), verificationCookies: InMemorySecretStore(),
            favouritesDirectory: TestData.temporaryDirectory(), isDemo: true,
            makeBackend: { _ in demo }
        )
        let model = AppModel(dependencies: dependencies)
        model.selectLibrary(.canning)
        let viewModel = BorrowedViewModel()
        await viewModel.appear(model)
        return (viewModel, model, try #require(viewModel.lists(model)?.reservations))
    }

    @Test func editClosesTheSwipeAndOpensTheSheet() async throws {
        let (viewModel, _, sections) = try await load()
        let row = try #require(sections.pending.first { $0.editHref != nil })
        viewModel.swipedRowID = row.id
        viewModel.edit(row)
        #expect(viewModel.editing == row)
        #expect(viewModel.swipedRowID == nil)
    }

    @Test func cancelAsksFirstWithTheCardStillOpen() async throws {
        let (viewModel, _, sections) = try await load()
        let row = try #require(sections.ready.first)
        viewModel.swipedRowID = row.id
        viewModel.requestCancel(row)
        #expect(viewModel.confirmingCancel == row)
        #expect(viewModel.swipedRowID == row.id)
    }

    @Test func aRowWithoutACancelLinkCantBeAskedToCancel() async throws {
        let (viewModel, _, sections) = try await load()
        let inTransit = try #require(sections.pending.first { $0.cancelHref == nil })
        viewModel.requestCancel(inTransit)
        #expect(viewModel.confirmingCancel == nil)
    }

    @Test func aBusyRowClosesAndCantBeActedOnAgain() async throws {
        let (viewModel, model, sections) = try await load()
        let row = try #require(sections.ready.first)
        viewModel.swipedRowID = row.id

        let cancelling = Task { await viewModel.cancel(row, model: model) }
        try await Task.sleep(for: .milliseconds(20))
        #expect(viewModel.isBusy(row))
        #expect(viewModel.swipedRowID == nil)
        #expect(ReservationAction.available(for: row, busy: viewModel.isBusy(row)).isEmpty)
        viewModel.requestCancel(row)
        viewModel.edit(row)
        #expect(viewModel.confirmingCancel == nil)
        #expect(viewModel.editing == nil)
        await cancelling.value
        #expect(viewModel.lists(model)?.reservations.row(id: row.id) == nil)
    }

    @Test func signingOutForgetsTheOpenCard() async throws {
        let (viewModel, model, sections) = try await load()
        viewModel.swipedRowID = try #require(sections.ready.first).id
        model.signOut()
        await viewModel.appear(model)
        #expect(viewModel.swipedRowID == nil)
    }
}

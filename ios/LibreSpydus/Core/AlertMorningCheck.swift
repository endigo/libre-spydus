import Foundation

/// The alert switches, a tapped alert and the morning check (docs/native-apps.md §5.8) on `AppModel`.
extension AppModel {
    /// The morning check gives up waiting after this; iOS allows an app-refresh task about 30 s.
    static let morningCheckLimit: Duration = .seconds(25)

    /// Profile's "Ready to collect" switch, for the selected library's saved account.
    func setReadyAlerts(_ on: Bool) async {
        await alerts.setReady(on, origin: library == nil ? nil : backend.origin, saved: account.snapshot)
        rescheduleMorningCheck()
    }

    /// Profile's "Due-date reminders" switch, for the selected library's saved account.
    func setDueAlerts(_ on: Bool) async {
        await alerts.setDue(on, origin: library == nil ? nil : backend.origin, saved: account.snapshot)
        rescheduleMorningCheck()
    }

    /// A tapped alert (§5.8 Tap): Your Books on its list (Reservations / Loans) with the book page of
    /// the reservation or loan it names pushed on top, built from the saved account the way tapping
    /// that row opens it, so Back lands on the list. Just the list when the alert names several
    /// loans, an item no longer in the account or none (an older build's due reminder), or belongs
    /// to another library than the selected one. At launch the saved account is already loaded
    /// (`AccountStore.use` reads it in `init`), so a tap that launches the app opens the book too.
    func openAlert(_ tap: AlertTap) {
        openYourBooks(on: tap.segment)
        guard library != nil, tap.originHash == AlertContent.originHash(backend.origin),
              let snapshot = account.snapshot, let ref = AlertContent.book(for: tap, in: snapshot) else {
            AlertLog.print("tap: Your Books › \(tap.segment.rawValue)")
            return
        }
        AlertLog.print("tap: Your Books › \(tap.segment.rawValue) › \(ref.id)")
        push(.detail(ref), on: .borrowed)
    }

    /// Submits the morning check for the next 07:45 while signed in with a switch on, else cancels
    /// it: at launch, on going to the background, on sign-in, sign-out and a library change, when
    /// a switch changes and after each check.
    func rescheduleMorningCheck() {
        alerts.scheduleMorningCheck(signedIn: isSignedIn)
    }

    /// The morning check (the app-refresh task `com.librespydus.morning-check`, or DEBUG
    /// `-LSMorningCheckNow`): fetch the account with the stored session and save it like any
    /// refresh, so the app opens with fresh data; the alerts process it on the way
    /// (`AccountStore.didFetch`). Then the next one is scheduled. A failure is silent (the error
    /// routes as in the app: an expired session signs out). Returns within `morningCheckLimit`, or
    /// sooner when the task is cancelled; a fetch still running then finishes unseen.
    func runMorningCheck() async {
        defer { rescheduleMorningCheck() }
        guard isSignedIn, alerts.anyEnabled else {
            AlertLog.print("morning check: nothing to do (signed in: \(isSignedIn), alerts on: \(alerts.anyEnabled))")
            return
        }
        AlertLog.print("morning check: fetching the account")
        let account = self.account
        let finished = await waitAtMost(Self.morningCheckLimit) { await account.refreshNow() } != nil
        if !finished {
            AlertLog.print("morning check: gave up waiting")
        } else if let error = account.lastRefreshError {
            AlertLog.print("morning check: refresh failed (\(error.message))")
        } else {
            AlertLog.print("morning check: done")
        }
    }
}

/// Waits for `operation` for at most `limit`, and no longer once the calling task is cancelled (a
/// background task that expires). Returns nil when it stopped waiting. The operation itself isn't
/// cancelled, since the core's calls can't be: it finishes unseen.
@MainActor
func waitAtMost<T: Sendable>(_ limit: Duration, _ operation: @escaping @MainActor () async -> T) async -> T? {
    let race = FirstResult<T>()
    Task { race.finish(await operation()) }
    let timer = Task {
        try? await Task.sleep(for: limit)
        race.finish(nil)
    }
    defer { timer.cancel() }
    return await withTaskCancellationHandler {
        await race.value()
    } onCancel: {
        Task { @MainActor in race.finish(nil) }
    }
}

/// The first of several racing results, handed to one waiter.
@MainActor
private final class FirstResult<T: Sendable> {
    private var result: T?
    private var isFinished = false
    private var waiter: CheckedContinuation<T?, Never>?

    func finish(_ value: T?) {
        guard !isFinished else { return }
        isFinished = true
        result = value
        waiter?.resume(returning: value)
        waiter = nil
    }

    func value() async -> T? {
        if isFinished { return result }
        return await withCheckedContinuation { waiter = $0 }
    }
}

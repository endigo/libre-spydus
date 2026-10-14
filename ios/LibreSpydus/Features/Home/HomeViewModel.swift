import Foundation
import LibreSpydusCore
import Observation

/// Home's shelves: "New arrivals" (the first `homeShelves()` entry) and, signed in, the current
/// loans. New arrivals load here; the loans come from the saved account (`AppModel.account`), so
/// a slow or failing shelf never blocks the other.
@MainActor
@Observable
final class HomeViewModel {
    /// How many covers the "New arrivals" shelf shows.
    static let shelfLimit = 10

    let arrivalsShelf: Shelf?
    let genres: [Genre]

    private(set) var arrivals: Loadable<[BookRef]> = .idle

    /// The `AppModel.reloadKey` the shelf was last loaded for.
    @ObservationIgnored private var loadedKey: AppModel.ReloadKey?
    /// Bumped by each load: a late answer for an older load is dropped.
    @ObservationIgnored private var arrivalsGeneration = 0
    @ObservationIgnored private let today: () -> Date
    /// The last snapshot's loans, so a redraw doesn't rebuild them.
    @ObservationIgnored private var builtLoans: (snapshot: AccountSnapshot, items: [LoanShelfItem])?

    init(shelves: [Shelf] = homeShelves(), genres: [Genre] = homeGenres(), today: @escaping () -> Date = { .now }) {
        arrivalsShelf = shelves.first
        self.genres = genres
        self.today = today
    }

    /// `.task(id: model.reloadKey)`: loads New arrivals once per key, so coming back from a pushed
    /// screen doesn't reload; a new library, verification or sign-in/out does. The account is
    /// fetched in the background when it's out of date.
    func loadIfNeeded(_ model: AppModel) async {
        async let account: Bool = model.refreshAccount(force: false)
        if model.reloadKey != loadedKey { await load(model, keepContent: false) }
        _ = await account
    }

    /// Pull to refresh keeps the current covers on screen while reloading, and fetches the
    /// account too.
    func refresh(_ model: AppModel) async {
        async let account: Bool = model.refreshAccount(force: true)
        await load(model, keepContent: true)
        _ = await account
    }

    func retryArrivals(_ model: AppModel) async {
        await loadArrivals(model, keepContent: false)
    }

    func retryLoans(_ model: AppModel) async {
        await model.refreshAccount(force: true)
    }

    /// "Your loans" from the saved account: `.idle` signed out, `.loading` until the
    /// first snapshot (or while retrying one that failed), else its loans in due order.
    func loans(_ model: AppModel) -> Loadable<[LoanShelfItem]> {
        guard model.isSignedIn else { return .idle }
        if let snapshot = model.account.snapshot {
            if let builtLoans, builtLoans.snapshot == snapshot { return .loaded(builtLoans.items) }
            let items = LoanShelfItem.make(from: snapshot.loans.items, today: today())
            builtLoans = (snapshot, items)
            return .loaded(items)
        }
        if let error = model.account.lastRefreshError, !model.account.isRefreshing { return .failed(error) }
        return .loading
    }

    private func load(_ model: AppModel, keepContent: Bool) async {
        let key = model.reloadKey
        await loadArrivals(model, keepContent: keepContent)
        // Keyed on what the shelf shows, not on task cancellation: core calls finish even when
        // the view's task is cancelled (a screen pushed mid-load), and their results still count.
        // A shelf left `.loading` by a real cancellation loads again when Home reappears.
        if model.reloadKey == key, !arrivals.isLoading { loadedKey = key }
    }

    /// The shelf's covers. Without `keepContent` (a new library, verification, sign-in/out), the
    /// covers saved on the device show at once while fresh ones load (§5.13); with it (pull to
    /// refresh) the covers on screen stay.
    private func loadArrivals(_ model: AppModel, keepContent: Bool) async {
        guard let shelf = arrivalsShelf else {
            arrivals = .loaded([])
            return
        }
        arrivalsGeneration += 1
        let generation = arrivalsGeneration
        let backend = model.backend
        let showsSaved = !keepContent && showSavedArrivals(shelf, backend: backend)
        if !showsSaved, !(keepContent && arrivals.value != nil) { arrivals = .loading }
        do {
            let results = try await backend.searchSorted(query: shelf.query, sort: SearchSortOption(shelf.sort))
            guard generation == arrivalsGeneration else { return }
            if results.records.isEmpty, !results.isNoResults {
                if !showsSaved { arrivals = .failed(SearchResultsModel.unrecognisedPage) }
                return
            }
            arrivals = .loaded(Self.items(results))
            await backend.saveShelf(key: shelf.key, results: results)
        } catch {
            guard generation == arrivalsGeneration, let userError = model.report(error) else { return }
            if showsSaved || keepContent, arrivals.value != nil { return }
            arrivals = .failed(userError)
        }
    }

    /// Shows the shelf's covers saved on the device, if any that the shelf can show.
    private func showSavedArrivals(_ shelf: Shelf, backend: any LibraryBackend) -> Bool {
        guard let saved = backend.savedShelf(key: shelf.key), !saved.records.isEmpty || saved.isNoResults else {
            return false
        }
        arrivals = .loaded(Self.items(saved))
        return true
    }

    /// One cover per book, at most `shelfLimit`: `ref.id` is the shelf's identity (records without a
    /// work IRN or URL fall back to their title).
    private static func items(_ results: SearchResults) -> [BookRef] {
        var seen = Set<String>()
        let items = SearchResultsModel.merge([], with: results.records).map(\.ref)
            .filter { seen.insert($0.id).inserted }
        return Array(items.prefix(shelfLimit))
    }

    // MARK: Routes

    var arrivalsRoute: Route? {
        arrivalsShelf.map { .results(query: $0.query, sort: SearchSortOption($0.sort), title: $0.title) }
    }

    static func route(for genre: Genre) -> Route {
        .results(query: genre.query, sort: .relevance, title: genre.label)
    }
}

// MARK: - Greeting

/// "Hello {first name}!" from `AppModel.accountName`.
enum HomeGreeting {
    /// "Hello Jane!", or "Hello!" when there's no usable first name (signed out, still loading).
    static func title(accountName: String?) -> String {
        firstName(from: accountName).map { Copy.Home.greeting(name: $0) } ?? Copy.Home.greetingSignedOut
    }

    /// The given name from an account name as libraries write it: "Smith, Jane" (surname first)
    /// or "Jane Smith". Titles ("Mrs", "Dr.") are skipped, ALL-CAPS and all-lowercase names are
    /// capitalised ("SMITH, JANE" → "Jane"), and a bare initial ("J. Smith") gives nil.
    static func firstName(from accountName: String?) -> String? {
        guard let name = accountName.nonEmpty else { return nil }
        let comma = name.firstIndex(of: ",")
        let given = comma.map { name[name.index(after: $0)...] } ?? Substring(name)
        var words = given.split(whereSeparator: { $0.isWhitespace })
            .map { String($0).trimmingCharacters(in: .punctuationCharacters.subtracting(["-", "'"])) }
            .filter { !$0.isEmpty && !honorifics.contains($0.lowercased()) }
        // After a comma every word is a given name, so "Smith, J. Jane" can skip the initial;
        // without one, the word after an initial is likely the surname.
        if comma != nil { words.removeAll { $0.count == 1 } }
        guard let first = words.first, first.count > 1, first.contains(where: \.isLetter) else { return nil }
        return normalisedCase(first)
    }

    private static let honorifics: Set<String> = [
        "mr", "mrs", "ms", "miss", "mx", "dr", "prof", "professor", "sir", "dame", "rev", "fr", "master",
    ]

    /// "JANE" / "jane" → "Jane", "MARY-JANE" → "Mary-Jane"; mixed case ("McKenzie") is kept.
    private static func normalisedCase(_ word: String) -> String {
        guard word == word.uppercased() || word == word.lowercased() else { return word }
        return word.lowercased().capitalized
    }
}

// MARK: - Loans shelf

/// A current loan on Home: cover plus "3 days left" / "Due 27 Oct" / "2 days overdue".
struct LoanShelfItem: Identifiable, Hashable, Sendable {
    enum Tone: Hashable, Sendable {
        case normal
        /// Due within `soonDays` days.
        case soon
        case overdue
    }

    /// Within this many days the caption counts down ("3 days left"); later it shows the date.
    static let countdownDays = 14
    /// Due within this many days (or today): `soon`.
    static let soonDays = 3

    let ref: BookRef
    let caption: String?
    let tone: Tone
    let dueDate: Date?

    var id: String { ref.id }

    init(_ item: PatronItem, today: Date = .now, calendar: Calendar = .library) {
        ref = BookRef(item)
        let dueText = item.fields["Due"].nonEmpty
        guard let status = DueStatus(dueText: dueText, today: today, calendar: calendar) else {
            dueDate = nil
            tone = .normal
            caption = dueText.map { Copy.Loans.dueOn(date: $0) }
            return
        }
        dueDate = status.dueDate
        if status.isOverdue {
            tone = .overdue
            caption = status.daysText
        } else {
            tone = status.daysLeft <= Self.soonDays ? .soon : .normal
            caption = status.daysLeft <= Self.countdownDays ? status.daysText : status.dueText
        }
    }

    /// Loans in due order (overdue first); loans without a readable date last, in list order.
    static func make(from items: [PatronItem], today: Date = .now, calendar: Calendar = .library) -> [LoanShelfItem] {
        var seen = Set<String>()
        let loans = items.map { LoanShelfItem($0, today: today, calendar: calendar) }
            .filter { seen.insert($0.id).inserted }
        return loans.enumerated().sorted { lhs, rhs in
            switch (lhs.element.dueDate, rhs.element.dueDate) {
            case let (left?, right?) where left != right: left < right
            case (.some, nil): true
            case (nil, .some): false
            default: lhs.offset < rhs.offset
            }
        }.map(\.element)
    }
}

// MARK: - Genres

enum GenreStyle {
    /// An SF Symbol for a `homeGenres()` key; unknown keys get a generic books icon.
    static func systemImage(for key: String) -> String {
        switch key {
        case "biography": "person.bust.fill"
        case "fiction": "book.fill"
        case "science": "atom"
        case "history": "building.columns.fill"
        case "fantasy": "wand.and.stars"
        case "crime": "eyeglasses"
        case "cooking": "fork.knife"
        case "children": "teddybear.fill"
        default: "books.vertical.fill"
        }
    }
}

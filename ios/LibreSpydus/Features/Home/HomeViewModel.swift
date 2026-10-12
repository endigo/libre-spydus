import Foundation
import LibreSpydusCore
import Observation

/// Home's shelves: "New arrivals" (the first `homeShelves()` entry) and, signed in, the current
/// loans. Each loads concurrently with its own state, so one slow or failing shelf never blocks
/// the other.
@MainActor
@Observable
final class HomeViewModel {
    /// How many covers the "New arrivals" shelf shows.
    static let shelfLimit = 10

    let arrivalsShelf: Shelf?
    let genres: [Genre]

    private(set) var arrivals: Loadable<[BookRef]> = .idle
    /// `.idle` while signed out.
    private(set) var loans: Loadable<[LoanShelfItem]> = .idle

    /// The `AppModel.reloadKey` the shelves were last loaded for.
    @ObservationIgnored private var loadedKey: AppModel.ReloadKey?
    /// Per shelf, bumped by each load of it: a late answer for an older load is dropped.
    @ObservationIgnored private var arrivalsGeneration = 0
    @ObservationIgnored private var loansGeneration = 0

    init(shelves: [Shelf] = homeShelves(), genres: [Genre] = homeGenres()) {
        arrivalsShelf = shelves.first
        self.genres = genres
    }

    /// `.task(id: model.reloadKey)`: loads once per key, so coming back from a pushed screen
    /// doesn't reload; a new library, verification or sign-in/out does.
    func loadIfNeeded(_ model: AppModel) async {
        guard model.reloadKey != loadedKey else { return }
        await load(model, keepContent: false)
    }

    /// Pull to refresh keeps the current covers on screen while reloading.
    func refresh(_ model: AppModel) async {
        await load(model, keepContent: true)
    }

    func retryArrivals(_ model: AppModel) async {
        await loadArrivals(model, keepContent: false)
    }

    func retryLoans(_ model: AppModel) async {
        await loadLoans(model, keepContent: false)
    }

    /// Both shelves at once; each settles on its own.
    private func load(_ model: AppModel, keepContent: Bool) async {
        let key = model.reloadKey
        async let arrivalsDone: Void = loadArrivals(model, keepContent: keepContent)
        async let loansDone: Void = loadLoans(model, keepContent: keepContent)
        _ = await (arrivalsDone, loansDone)
        // Keyed on what the shelves show, not on task cancellation: core calls finish even when
        // the view's task is cancelled (a screen pushed mid-load), and their results still count.
        // A shelf left `.loading` by a real cancellation loads again when Home reappears.
        if model.reloadKey == key, !arrivals.isLoading, !loans.isLoading { loadedKey = key }
    }

    private func loadArrivals(_ model: AppModel, keepContent: Bool) async {
        guard let shelf = arrivalsShelf else {
            arrivals = .loaded([])
            return
        }
        arrivalsGeneration += 1
        let generation = arrivalsGeneration
        if !(keepContent && arrivals.value != nil) { arrivals = .loading }
        do {
            let results = try await model.backend.searchSorted(query: shelf.query, sort: SearchSortOption(shelf.sort))
            guard generation == arrivalsGeneration else { return }
            if results.records.isEmpty, !results.isNoResults {
                arrivals = .failed(SearchResultsModel.unrecognisedPage)
                return
            }
            // One cover per book: `ref.id` is the shelf's identity (records without a work IRN or
            // URL fall back to their title).
            var seen = Set<String>()
            let items = SearchResultsModel.merge([], with: results.records).map(\.ref)
                .filter { seen.insert($0.id).inserted }
            arrivals = .loaded(Array(items.prefix(Self.shelfLimit)))
        } catch {
            guard generation == arrivalsGeneration, let userError = model.report(error) else { return }
            if keepContent, arrivals.value != nil { return }
            arrivals = .failed(userError)
        }
    }

    private func loadLoans(_ model: AppModel, keepContent: Bool) async {
        loansGeneration += 1
        let generation = loansGeneration
        guard let session = model.session else {
            loans = .idle
            return
        }
        if !(keepContent && loans.value != nil) { loans = .loading }
        do {
            let list = try await model.backend.loans(session: session)
            guard generation == loansGeneration else { return }
            loans = .loaded(LoanShelfItem.make(from: list.items))
        } catch {
            guard generation == loansGeneration, let userError = model.report(error) else { return }
            if keepContent, loans.value != nil { return }
            loans = .failed(userError)
        }
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
        firstName(from: accountName).map { "Hello \($0)!" } ?? "Hello!"
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

    init(_ item: PatronItem, today: Date = .now, calendar: Calendar = .current) {
        ref = BookRef(item)
        let dueText = item.fields["Due"].nonEmpty
        guard let status = DueStatus(dueText: dueText, today: today, calendar: calendar) else {
            dueDate = nil
            tone = .normal
            caption = dueText.map { "Due \($0)" }
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
    static func make(from items: [PatronItem], today: Date = .now, calendar: Calendar = .current) -> [LoanShelfItem] {
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

import Foundation
import Observation

/// The Search tab's recent queries: local only (UserDefaults), newest first, at most `limit`.
@MainActor
@Observable
final class RecentSearchStore {
    static let limit = 8
    nonisolated static let defaultsKey = "LibreSpydus.recentSearches"

    private(set) var items: [String]

    @ObservationIgnored private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        let stored = defaults.stringArray(forKey: Self.defaultsKey) ?? []
        items = Array(stored.prefix(Self.limit))
    }

    /// Moves `query` (trimmed, inner whitespace collapsed) to the top; a case-insensitive repeat
    /// replaces the older entry. Blank queries are ignored.
    func record(_ query: String) {
        let cleaned = query.split(whereSeparator: \.isWhitespace).joined(separator: " ")
        guard !cleaned.isEmpty else { return }
        items.removeAll { $0.caseInsensitiveCompare(cleaned) == .orderedSame }
        items.insert(cleaned, at: 0)
        if items.count > Self.limit { items.removeLast(items.count - Self.limit) }
        save()
    }

    func remove(_ query: String) {
        items.removeAll { $0 == query }
        save()
    }

    func clear() {
        items = []
        save()
    }

    private func save() {
        if items.isEmpty {
            defaults.removeObject(forKey: Self.defaultsKey)
        } else {
            defaults.set(items, forKey: Self.defaultsKey)
        }
    }
}

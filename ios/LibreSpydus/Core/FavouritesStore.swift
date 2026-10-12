import Foundation
import Observation

/// Local favourites, one list per library origin (docs/native-apps.md §5.3), saved as JSON in
/// Application Support. Newest first.
@MainActor
@Observable
final class FavouritesStore {
    private(set) var origin: String?
    /// Saved books, newest first.
    private(set) var items: [BookRef] = []
    /// The last load/save failure, if any (the list still works in memory).
    private(set) var lastError: String?

    @ObservationIgnored private let directory: URL

    nonisolated static var defaultDirectory: URL {
        URL.applicationSupportDirectory.appending(path: "Favourites", directoryHint: .isDirectory)
    }

    init(directory: URL = FavouritesStore.defaultDirectory, origin: String? = nil) {
        self.directory = directory
        load(origin: origin)
    }

    /// Switches to `origin`'s list (nil: empty, nothing saved).
    func load(origin: String?) {
        self.origin = origin
        lastError = nil
        guard let file = fileURL(for: origin), FileManager.default.fileExists(atPath: file.path) else {
            items = []
            return
        }
        do {
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            items = Self.sorted(try decoder.decode([BookRef].self, from: Data(contentsOf: file)))
        } catch {
            items = []
            lastError = "Couldn't read favourites: \(error.localizedDescription)"
        }
    }

    func contains(id: String) -> Bool {
        items.contains { $0.id == id }
    }

    func contains(_ ref: BookRef) -> Bool {
        contains(id: ref.id)
    }

    /// Saves `ref` (stamped `savedAt = now`, moved to the top) or replaces the saved copy.
    func add(_ ref: BookRef, now: Date = .now) {
        var saved = ref
        saved.savedAt = now
        items.removeAll { $0.id == ref.id }
        items.insert(saved, at: 0)
        save()
    }

    func remove(id: String) {
        let before = items.count
        items.removeAll { $0.id == id }
        if items.count != before { save() }
    }

    /// Adds or removes `ref`; returns whether it's a favourite afterwards.
    @discardableResult
    func toggle(_ ref: BookRef, now: Date = .now) -> Bool {
        if contains(ref) {
            remove(id: ref.id)
            return false
        }
        add(ref, now: now)
        return true
    }

    /// Deletes every origin's favourites (`-LSReset`).
    nonisolated static func deleteAll(in directory: URL = FavouritesStore.defaultDirectory) {
        try? FileManager.default.removeItem(at: directory)
    }

    private func save() {
        guard let file = fileURL(for: origin) else { return }
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let encoder = JSONEncoder()
            encoder.dateEncodingStrategy = .iso8601
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            try encoder.encode(items).write(to: file, options: .atomic)
            lastError = nil
        } catch {
            lastError = "Couldn't save favourites: \(error.localizedDescription)"
        }
    }

    private func fileURL(for origin: String?) -> URL? {
        guard let origin, !origin.isEmpty else { return nil }
        let safe = String(origin.map { $0.isLetter || $0.isNumber ? $0 : "_" })
        return directory.appending(path: "favourites-\(safe).json")
    }

    private static func sorted(_ items: [BookRef]) -> [BookRef] {
        items.sorted { ($0.savedAt ?? .distantPast) > ($1.savedAt ?? .distantPast) }
    }
}

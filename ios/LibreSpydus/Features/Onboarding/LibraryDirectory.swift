import Foundation
import LibreSpydusCore

/// The library picker's pure logic: regions, search and sections (no I/O; unit-tested).
enum LibraryDirectory {
    /// Picker regions in display order (docs/native-apps.md §6). Unknown region codes are `.other`.
    enum Region: String, CaseIterable, Identifiable, Sendable {
        case au = "AU"
        case nz = "NZ"
        case us = "US"
        case uk = "UK"
        case sg = "SG"
        case other = "OTHER"

        var id: String { rawValue }

        init(code: String) {
            self = Region(rawValue: code.uppercased()) ?? .other
        }

        /// Section header.
        var title: String {
            switch self {
            case .au: "Australia"
            case .nz: "New Zealand"
            case .us: "United States"
            case .uk: "United Kingdom"
            case .sg: "Singapore"
            case .other: "Other"
            }
        }

        /// What a row in this region should warn about (core/README.md "Current regional support").
        var caveat: String? {
            switch self {
            case .uk: "Asks you to verify you're human"
            case .sg: "May be unreachable outside school networks"
            case .au, .nz, .us, .other: nil
            }
        }
    }

    struct Section: Identifiable, Equatable, Sendable {
        let region: Region
        let libraries: [LibraryInfo]
        var id: Region { region }
    }

    /// `libraries` matching `query`, grouped by region in `Region` order and sorted by name;
    /// empty regions are left out.
    static func sections(_ libraries: [LibraryInfo], query: String = "") -> [Section] {
        let matching = filter(libraries, query: query)
        let grouped = Dictionary(grouping: matching) { Region(code: $0.region) }
        return Region.allCases.compactMap { region in
            guard let members = grouped[region], !members.isEmpty else { return nil }
            return Section(region: region, libraries: sorted(members))
        }
    }

    /// Every whitespace-separated word of `query` must appear in the name, the country or the
    /// region's name (case- and diacritic-insensitive), or be the region code: "city aus",
    /// "wellington", "kingdom", "uk".
    static func filter(_ libraries: [LibraryInfo], query: String) -> [LibraryInfo] {
        let words = query.split(whereSeparator: \.isWhitespace).map(String.init)
        guard !words.isEmpty else { return libraries }
        return libraries.filter { library in
            let haystack = [library.name, library.country ?? "", Region(code: library.region).title]
            return words.allSatisfy { word in
                library.region.caseInsensitiveCompare(word) == .orderedSame
                    || haystack.contains { $0.localizedStandardContains(word) }
            }
        }
    }

    /// By name as a person reads it ("Library 2" before "Library 10"), then by origin.
    static func sorted(_ libraries: [LibraryInfo]) -> [LibraryInfo] {
        libraries.sorted { lhs, rhs in
            switch lhs.name.localizedStandardCompare(rhs.name) {
            case .orderedAscending: true
            case .orderedDescending: false
            case .orderedSame: lhs.origin < rhs.origin
            }
        }
    }

    /// Up to two initials for the logo placeholder: "City of Canning" → "CC".
    static func monogram(for name: String) -> String {
        let skipped: Set<String> = ["of", "the", "and", "&", "-", "–"]
        let words = name.split(whereSeparator: { $0.isWhitespace || $0 == "/" })
            .filter { !skipped.contains($0.lowercased()) }
        let letters = words.prefix(2).compactMap { $0.first(where: \.isLetter) ?? $0.first }
        return letters.isEmpty ? "?" : String(letters).uppercased()
    }

    /// `https://canning.spydus.com` → `canning.spydus.com`.
    static func host(of origin: String) -> String {
        URL(string: origin)?.host() ?? origin
    }
}

/// The directory on disk (Caches), so the picker opens instantly and offline after the first
/// fetch. Refreshed when older than `maxAge` or on pull-to-refresh.
struct LibraryDirectoryCache: Sendable {
    static let maxAge: TimeInterval = 7 * 24 * 60 * 60

    struct Snapshot: Equatable, Sendable {
        var fetchedAt: Date
        var libraries: [LibraryInfo]
    }

    let fileURL: URL

    /// `Caches/LibreSpydus/library-directory.json`.
    static var `default`: LibraryDirectoryCache {
        let caches = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        return LibraryDirectoryCache(fileURL: caches.appending(path: "LibreSpydus/library-directory.json"))
    }

    /// Fresh when fetched less than `maxAge` ago. A date in the future (the clock moved back) is
    /// stale, so a wrong clock can't pin an old list forever.
    static func isFresh(fetchedAt: Date, now: Date, maxAge: TimeInterval = maxAge) -> Bool {
        let age = now.timeIntervalSince(fetchedAt)
        return age >= 0 && age < maxAge
    }

    /// The cached directory, or nil when there's none or it can't be read.
    func load() -> Snapshot? {
        guard let data = try? Data(contentsOf: fileURL),
              let stored = try? JSONDecoder().decode(StoredSnapshot.self, from: data),
              stored.version == StoredSnapshot.currentVersion else { return nil }
        return Snapshot(fetchedAt: stored.fetchedAt, libraries: stored.libraries.map(\.info))
    }

    func save(_ snapshot: Snapshot) throws {
        let stored = StoredSnapshot(fetchedAt: snapshot.fetchedAt, libraries: snapshot.libraries.map(StoredLibraryInfo.init))
        try FileManager.default.createDirectory(at: fileURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        try JSONEncoder().encode(stored).write(to: fileURL, options: .atomic)
    }

    func remove() {
        try? FileManager.default.removeItem(at: fileURL)
    }

    private struct StoredSnapshot: Codable {
        static let currentVersion = 1
        var version = currentVersion
        var fetchedAt: Date
        var libraries: [StoredLibraryInfo]
    }

    /// `LibraryInfo` as JSON (UniFFI records aren't Codable).
    private struct StoredLibraryInfo: Codable {
        var id: String
        var name: String
        var origin: String
        var logoUrl: String?
        var country: String?
        var region: String
        var isTest: Bool

        init(_ info: LibraryInfo) {
            id = info.id
            name = info.name
            origin = info.origin
            logoUrl = info.logoUrl
            country = info.country
            region = info.region
            isTest = info.isTest
        }

        var info: LibraryInfo {
            LibraryInfo(id: id, name: name, origin: origin, logoUrl: logoUrl, country: country, region: region,
                        isTest: isTest)
        }
    }
}

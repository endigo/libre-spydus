import CryptoKit
import Foundation
import LibreSpydusCore

/// The saved account, one file per library origin (docs/native-apps.md §5.2a):
/// `Application Support/AccountCache/<sha256(origin)>.json`, holding the core's
/// `encodeAccountSnapshot` JSON. Written atomically, readable after the first unlock
/// (`.completeUntilFirstUserAuthentication`) and excluded from backups. A file that doesn't decode
/// (corrupt, or another `ACCOUNT_SNAPSHOT_VERSION`) is deleted and treated as missing.
struct AccountSnapshotCache: Sendable {
    let directory: URL

    nonisolated static var defaultDirectory: URL {
        URL.applicationSupportDirectory.appending(path: "AccountCache", directoryHint: .isDirectory)
    }

    init(directory: URL = AccountSnapshotCache.defaultDirectory) {
        self.directory = directory
    }

    /// `origin`'s saved snapshot, or nil when there's none or it can't be read.
    func load(origin: String) -> AccountSnapshot? {
        let file = fileURL(for: origin)
        guard let data = try? Data(contentsOf: file) else { return nil }
        do {
            return try decodeAccountSnapshot(json: String(decoding: data, as: UTF8.self))
        } catch {
            // Malformed or written by another snapshot version: fetch a new one instead.
            try? FileManager.default.removeItem(at: file)
            return nil
        }
    }

    func save(_ snapshot: AccountSnapshot, origin: String) throws {
        let manager = FileManager.default
        if !manager.fileExists(atPath: directory.path) {
            try manager.createDirectory(at: directory, withIntermediateDirectories: true)
            try Self.excludeFromBackup(directory)
        }
        let file = fileURL(for: origin)
        try Data(encodeAccountSnapshot(snapshot: snapshot).utf8)
            .write(to: file, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        // An atomic write replaces the file, so the flag is set again each time.
        try Self.excludeFromBackup(file)
    }

    func remove(origin: String) {
        try? FileManager.default.removeItem(at: fileURL(for: origin))
    }

    /// Deletes every origin's saved account (`-LSReset`).
    static func deleteAll(in directory: URL = AccountSnapshotCache.defaultDirectory) {
        try? FileManager.default.removeItem(at: directory)
    }

    /// The origin is hashed so the file name says nothing about which library it is.
    func fileURL(for origin: String) -> URL {
        let digest = SHA256.hash(data: Data(origin.utf8)).map { String(format: "%02x", $0) }.joined()
        return directory.appending(path: "\(digest).json", directoryHint: .notDirectory)
    }

    private static func excludeFromBackup(_ url: URL) throws {
        var url = url
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try url.setResourceValues(values)
    }
}

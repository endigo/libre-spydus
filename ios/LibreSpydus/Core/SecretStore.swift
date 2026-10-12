import Foundation
import Security

/// Small string secrets keyed by library origin (sessions, verification cookies).
protocol SecretStore: Sendable {
    func value(for key: String) -> String?
    func setValue(_ value: String?, for key: String)
    func removeAll()
}

/// Generic-password Keychain items under one service, readable after first unlock, this device only.
struct KeychainSecretStore: SecretStore {
    let service: String

    func value(for key: String) -> String? {
        var query = baseQuery(key)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: AnyObject?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    func setValue(_ value: String?, for key: String) {
        guard let value else {
            SecItemDelete(baseQuery(key) as CFDictionary)
            return
        }
        let data = Data(value.utf8)
        let update: [String: Any] = [kSecValueData as String: data]
        if SecItemUpdate(baseQuery(key) as CFDictionary, update as CFDictionary) == errSecItemNotFound {
            var add = baseQuery(key)
            add[kSecValueData as String] = data
            add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            SecItemAdd(add as CFDictionary, nil)
        }
    }

    func removeAll() {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service]
        SecItemDelete(query as CFDictionary)
    }

    private func baseQuery(_ key: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
    }
}

/// In-memory store for demo mode and tests (nothing touches the Keychain).
final class InMemorySecretStore: SecretStore, @unchecked Sendable {
    private let lock = NSLock()
    private var values: [String: String]

    init(_ values: [String: String] = [:]) {
        self.values = values
    }

    func value(for key: String) -> String? {
        lock.withLock { values[key] }
    }

    func setValue(_ value: String?, for key: String) {
        lock.withLock { values[key] = value }
    }

    func removeAll() {
        lock.withLock { values.removeAll() }
    }
}

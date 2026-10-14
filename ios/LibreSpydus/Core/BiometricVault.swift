import Foundation
import LocalAuthentication
import Security

// Sign back in with Face ID (docs/native-apps.md §5.14): where a library's sign-in details are
// kept, readable only after the phone's face or fingerprint check.

/// The phone's face or fingerprint check.
enum Biometry: Equatable, Sendable {
    case faceID
    case touchID
    case opticID

    /// `{method}` in `Copy.Biometrics`: "Face ID", "Touch ID", "Optic ID".
    var name: String {
        switch self {
        case .faceID: Copy.Biometrics.faceId
        case .touchID: Copy.Biometrics.touchId
        case .opticID: Copy.Biometrics.opticId
        }
    }

    /// Its SF Symbol.
    var systemImage: String {
        switch self {
        case .faceID: "faceid"
        case .touchID: "touchid"
        case .opticID: "opticid"
        }
    }
}

/// The username / card number and PIN of a typed sign-in that succeeded.
struct SignInDetails: Codable, Equatable, Sendable {
    var username: String
    var pin: String
}

/// Why a check didn't save or hand back the details.
enum BiometricFailure: Error, Equatable {
    /// Cancelled, "Enter PIN" chosen, or the face or finger wasn't recognised.
    case declined
    /// The check can't run now (nothing enrolled, locked out, the app in the background).
    case unavailable
    /// Nothing readable for that library (a face or finger added since): deleted.
    case missing
}

/// A library's sign-in details per origin, behind the check.
protocol BiometricVault: Sendable {
    /// The check this phone can run now, or nil.
    func biometry() -> Biometry?
    /// The username of `origin`'s details, nil when it has none (never asks for the check).
    func username(origin: String) -> String?
    /// Runs the check (`reason` shown with it), then keeps `details` for `origin`. Throws
    /// `BiometricFailure`.
    func save(_ details: SignInDetails, origin: String, reason: String) async throws
    /// Runs the check, then hands back `origin`'s details. Throws `BiometricFailure`.
    func read(origin: String, reason: String) async throws -> SignInDetails
    func delete(origin: String)
    func deleteAll()
}

/// Keychain generic passwords under one service, account = origin, that only a check with the faces
/// or fingers enrolled now can read (`.biometryCurrentSet`); this device only, never backed up. The
/// username is also kept in a plain item of its own (service `<service>.username`): on a phone, even
/// an attributes-only query of the protected item needs the check, so it can't say whether there
/// are details without asking.
struct KeychainBiometricVault: BiometricVault {
    let service: String

    private var usernames: KeychainSecretStore { KeychainSecretStore(service: "\(service).username") }

    func biometry() -> Biometry? {
        BiometricCheck.biometry()
    }

    func username(origin: String) -> String? {
        usernames.value(for: origin)
    }

    func save(_ details: SignInDetails, origin: String, reason: String) async throws {
        let context = try await BiometricCheck.run(reason: reason)
        guard let access = SecAccessControlCreateWithFlags(nil, kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly,
                                                           .biometryCurrentSet, nil),
              let data = try? JSONEncoder().encode(details) else { throw BiometricFailure.unavailable }
        SecItemDelete(baseQuery(origin) as CFDictionary)
        var add = baseQuery(origin)
        add[kSecValueData as String] = data
        add[kSecAttrAccessControl as String] = access
        add[kSecUseAuthenticationContext as String] = context
        let status = SecItemAdd(add as CFDictionary, nil)
        Self.log("save", status)
        guard status == errSecSuccess else { throw BiometricFailure.unavailable }
        usernames.setValue(details.username, for: origin)
    }

    func read(origin: String, reason: String) async throws -> SignInDetails {
        guard username(origin: origin) != nil else { throw BiometricFailure.missing }
        let context = try await BiometricCheck.run(reason: reason)
        // The context has passed the check, so the Keychain doesn't ask again.
        var query = baseQuery(origin)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        query[kSecUseAuthenticationContext as String] = context
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        Self.log("read", status)
        switch status {
        case errSecSuccess:
            guard let data = result as? Data, let details = try? JSONDecoder().decode(SignInDetails.self, from: data) else {
                delete(origin: origin)
                throw BiometricFailure.missing
            }
            return details
        case errSecUserCanceled:
            throw BiometricFailure.declined
        case errSecItemNotFound, errSecAuthFailed:
            // Unreadable: a face or finger was added since (or the item is gone).
            delete(origin: origin)
            throw BiometricFailure.missing
        default:
            throw BiometricFailure.unavailable
        }
    }

    func delete(origin: String) {
        SecItemDelete(baseQuery(origin) as CFDictionary)
        usernames.setValue(nil, for: origin)
    }

    func deleteAll() {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service]
        SecItemDelete(query as CFDictionary)
        usernames.removeAll()
    }

    private func baseQuery(_ origin: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: origin,
        ]
    }

    /// DEBUG: the Keychain's answer, for the device console.
    private static func log(_ operation: String, _ status: OSStatus) {
        #if DEBUG
        print("🔐 sign-in details \(operation): \(status == errSecSuccess ? "ok" : "OSStatus \(status)")")
        #endif
    }
}

/// Demo mode: the details stay in memory (gone on relaunch), but the real check still runs.
final class MemoryBiometricVault: BiometricVault, @unchecked Sendable {
    private let lock = NSLock()
    private var details: [String: SignInDetails] = [:]

    func biometry() -> Biometry? {
        BiometricCheck.biometry()
    }

    func username(origin: String) -> String? {
        lock.withLock { details[origin]?.username }
    }

    func save(_ details: SignInDetails, origin: String, reason: String) async throws {
        _ = try await BiometricCheck.run(reason: reason)
        lock.withLock { self.details[origin] = details }
    }

    func read(origin: String, reason: String) async throws -> SignInDetails {
        guard username(origin: origin) != nil else { throw BiometricFailure.missing }
        _ = try await BiometricCheck.run(reason: reason)
        guard let saved = lock.withLock({ details[origin] }) else { throw BiometricFailure.missing }
        return saved
    }

    func delete(origin: String) {
        lock.withLock { _ = details.removeValue(forKey: origin) }
    }

    func deleteAll() {
        lock.withLock { details.removeAll() }
    }
}

/// No check on this phone (the default in tests): nothing is offered or kept.
struct NoBiometricVault: BiometricVault {
    func biometry() -> Biometry? { nil }
    func username(origin: String) -> String? { nil }
    func save(_ details: SignInDetails, origin: String, reason: String) async throws { throw BiometricFailure.unavailable }
    func read(origin: String, reason: String) async throws -> SignInDetails { throw BiometricFailure.unavailable }
    func delete(origin: String) {}
    func deleteAll() {}
}

/// The system's face or fingerprint check (`LocalAuthentication`).
enum BiometricCheck {
    static func biometry() -> Biometry? {
        let context = LAContext()
        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: nil) else { return nil }
        switch context.biometryType {
        case .faceID: return .faceID
        case .touchID: return .touchID
        case .opticID: return .opticID
        default: return nil
        }
    }

    /// Asks for the check; the context that passed it. Throws `BiometricFailure`.
    static func run(reason: String) async throws -> LAContext {
        let context = LAContext()
        context.localizedFallbackTitle = Copy.Biometrics.enterPin
        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: nil) else {
            throw BiometricFailure.unavailable
        }
        do {
            try await context.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, localizedReason: reason)
            return context
        } catch let error as LAError {
            switch error.code {
            case .userCancel, .userFallback, .authenticationFailed, .systemCancel, .appCancel:
                throw BiometricFailure.declined
            default:
                throw BiometricFailure.unavailable
            }
        } catch {
            throw BiometricFailure.unavailable
        }
    }
}

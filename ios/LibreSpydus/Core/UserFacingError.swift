import Foundation
import LibreSpydusCore

/// What a screen shows for a failed call (docs/native-apps.md §5.7). Get one from
/// `AppModel.report(_:)`, which also performs the side effects (verification sheet, sign-out).
struct UserFacingError: Error, Equatable, Sendable {
    enum Kind: Equatable, Sendable {
        case network
        /// The verification sheet is (being) shown; the screen retries on `verificationGeneration`.
        case verificationRequired
        /// Verification was completed twice and the library still challenges: offer its website.
        case verificationUnavailable
        case notLoggedIn
        case credentialsRejected
        /// OriginForbidden / Upstream / Parse / anything else.
        case unexpected
    }

    var kind: Kind
    var message: String
    /// Technical detail for a disclosure (the core's error text).
    var detail: String?

    init(kind: Kind, message: String, detail: String? = nil) {
        self.kind = kind
        self.message = message
        self.detail = detail
    }

    init(_ error: any Error, libraryName: String) {
        let library = libraryName.isEmpty ? Copy.Common.yourLibrary : libraryName
        if let error = error as? UserFacingError {
            self = error
            return
        }
        if let error = error as? SpydusError {
            switch error {
            case .Network(let message):
                self.init(kind: .network, message: Copy.Errors.network(library: library), detail: message)
            case .HumanVerificationRequired(let message):
                self.init(kind: .verificationRequired,
                          message: Copy.Errors.verification(library: library).capitalisedFirst, detail: message)
            case .NotLoggedIn(let message):
                self.init(kind: .notLoggedIn, message: Copy.Errors.signInAgain, detail: message)
            case .CredentialsRejected(let message):
                self.init(kind: .credentialsRejected, message: Copy.Errors.credentialsRejected, detail: message)
            case .OriginForbidden(let message), .Upstream(let message), .Parse(let message):
                self.init(kind: .unexpected, message: Self.unexpectedMessage, detail: message)
            }
            return
        }
        if let error = error as? URLError {
            self.init(kind: .network, message: Copy.Errors.network(library: library),
                      detail: error.localizedDescription)
            return
        }
        self.init(kind: .unexpected, message: Self.unexpectedMessage, detail: String(describing: error))
    }

    static let unexpectedMessage = Copy.Errors.unexpected

    static func verificationUnavailable(libraryName: String) -> UserFacingError {
        let library = libraryName.isEmpty ? Copy.Common.yourLibrary : libraryName
        return UserFacingError(kind: .verificationUnavailable,
                               message: Copy.Errors.libraryUnavailable(library: library).capitalisedFirst)
    }

    var title: String {
        switch kind {
        case .network: Copy.Errors.cantConnectTitle
        case .verificationRequired: Copy.Errors.verificationNeededTitle
        case .verificationUnavailable: Copy.Errors.libraryUnavailableTitle
        case .notLoggedIn: Copy.Errors.signedOutTitle
        case .credentialsRejected: Copy.Errors.signInFailedTitle
        case .unexpected: Copy.Errors.unexpectedTitle
        }
    }

    var systemImage: String {
        switch kind {
        case .network: "wifi.exclamationmark"
        case .verificationRequired, .verificationUnavailable: "checkmark.shield"
        case .notLoggedIn, .credentialsRejected: "person.crop.circle.badge.exclamationmark"
        case .unexpected: "exclamationmark.triangle"
        }
    }

    /// Whether a Retry button makes sense (sign-in problems are fixed by signing in instead).
    var canRetry: Bool {
        switch kind {
        case .network, .verificationRequired, .unexpected: true
        case .verificationUnavailable, .notLoggedIn, .credentialsRejected: false
        }
    }
}

/// Typical screen state: `.task(id: model.reloadKey) { … }` sets it; views switch over it.
enum Loadable<Value> {
    case idle
    case loading
    case loaded(Value)
    case failed(UserFacingError)

    var value: Value? {
        if case .loaded(let value) = self { return value }
        return nil
    }

    var isLoading: Bool {
        if case .loading = self { return true }
        return false
    }

    var error: UserFacingError? {
        if case .failed(let error) = self { return error }
        return nil
    }
}

extension Loadable: Sendable where Value: Sendable {}
extension Loadable: Equatable where Value: Equatable {}

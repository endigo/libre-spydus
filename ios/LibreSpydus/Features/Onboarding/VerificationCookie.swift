import Foundation

/// AWS WAF human verification, the pure parts (core/README.md "Human verification"): find the
/// library's `aws-waf-token` among the web view's cookies, and tell a WAF challenge response from
/// the library's own page. Nothing here touches the challenge itself.
enum VerificationCookie {
    static let name = "aws-waf-token"

    /// What the matcher needs from an `HTTPCookie` (tests build these directly).
    struct Candidate: Equatable, Sendable {
        var name: String
        var value: String
        var domain: String
        var expiresDate: Date?

        init(name: String, value: String, domain: String, expiresDate: Date? = nil) {
            self.name = name
            self.value = value
            self.domain = domain
            self.expiresDate = expiresDate
        }

        init(_ cookie: HTTPCookie) {
            self.init(name: cookie.name, value: cookie.value, domain: cookie.domain, expiresDate: cookie.expiresDate)
        }
    }

    /// `"aws-waf-token=<value>"` for the library at `host`, or nil when there's no usable token.
    ///
    /// The cookie must be named `aws-waf-token`, have a value, not have expired, and its domain
    /// must match `host` (RFC 6265 domain-match: the host itself, or a parent domain, with or
    /// without a leading dot). A host-only cookie wins over a parent-domain one; among equals the
    /// one that expires last wins.
    static func header(in cookies: [Candidate], host: String, now: Date = Date()) -> String? {
        let usable = cookies.filter { cookie in
            cookie.name == name
                && !cookie.value.isEmpty
                && (cookie.expiresDate.map { $0 > now } ?? true)
                && domainMatches(cookie.domain, host: host)
        }
        let best = usable.max { lhs, rhs in
            let lhsExact = normalized(lhs.domain) == normalized(host)
            let rhsExact = normalized(rhs.domain) == normalized(host)
            if lhsExact != rhsExact { return !lhsExact }
            return (lhs.expiresDate ?? .distantFuture) < (rhs.expiresDate ?? .distantFuture)
        }
        return best.map { "\(name)=\($0.value)" }
    }

    static func header(in cookies: [HTTPCookie], host: String, now: Date = Date()) -> String? {
        header(in: cookies.map(Candidate.init), host: host, now: now)
    }

    /// Whether a cookie set for `domain` is sent to `host`: `canning.spydus.com` and
    /// `.spydus.com` match `canning.spydus.com`; `other.spydus.com`, `spydus.co.uk` and
    /// `nning.spydus.com` don't. IP addresses only match exactly.
    static func domainMatches(_ domain: String, host: String) -> Bool {
        let domain = normalized(domain)
        let host = normalized(host)
        guard !domain.isEmpty, !host.isEmpty else { return false }
        if domain == host { return true }
        guard !isIPAddress(host), domain.contains(".") else { return false }
        return host.hasSuffix("." + domain)
    }

    /// Cookies the "Clear verification" action deletes from the web view's store.
    static func isVerificationCookie(_ cookie: Candidate, host: String) -> Bool {
        cookie.name == name && domainMatches(cookie.domain, host: host)
    }

    /// Whether a page response is the WAF's challenge rather than the library's own page: the
    /// core treats any response carrying `x-amzn-waf-action` as a challenge, whatever its status.
    static func isChallengeResponse(headers: [AnyHashable: Any]) -> Bool {
        headers.keys.contains { ($0 as? String)?.caseInsensitiveCompare("x-amzn-waf-action") == .orderedSame }
    }

    /// Lower-cased, without a leading or trailing dot.
    private static func normalized(_ domain: String) -> String {
        var text = domain.trimmingCharacters(in: .whitespaces).lowercased()
        while text.hasPrefix(".") { text.removeFirst() }
        while text.hasSuffix(".") { text.removeLast() }
        return text
    }

    private static func isIPAddress(_ host: String) -> Bool {
        host.contains(":") || host.split(separator: ".").allSatisfy { !$0.isEmpty && $0.allSatisfy(\.isNumber) }
    }
}

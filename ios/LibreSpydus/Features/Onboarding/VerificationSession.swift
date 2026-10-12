import Foundation
import Observation
import WebKit

/// One verification attempt in the sheet's web view: page state, and when to hand the
/// `aws-waf-token` to the app.
///
/// The token is taken only once the web view shows the library's own page again (the last
/// main-frame response isn't a WAF challenge), so a token the challenge sets before the user has
/// finished, or an expired one the library just rejected, isn't used.
@MainActor
@Observable
final class VerificationSession {
    let origin: String
    let host: String
    let libraryName: String

    private(set) var isLoading = true
    private(set) var loadError: UserFacingError?
    /// Whether the last page response was the WAF challenge; nil before the first response.
    private(set) var showingChallenge: Bool?
    /// The `aws-waf-token=<value>` handed to the app (at most once).
    private(set) var completedCookie: String?
    /// The library's own page is showing and its cookies held no token.
    private(set) var foundNoToken = false

    @ObservationIgnored weak var webView: WKWebView?

    init(origin: String, libraryName: String) {
        self.origin = origin
        self.libraryName = libraryName
        host = LibraryDirectory.host(of: origin)
    }

    /// `client.origin() + "/"` (core/README.md "Human verification").
    var pageURL: URL? { URL(string: origin + "/") }

    /// The library's page loaded without a challenge and there's no token: nothing to complete.
    var finishedWithoutCheck: Bool {
        showingChallenge == false && !isLoading && completedCookie == nil && foundNoToken
    }

    // MARK: Web view events

    func didStartLoading() {
        isLoading = true
        loadError = nil
        foundNoToken = false
    }

    func didReceiveMainFrameResponse(_ response: URLResponse) {
        guard let http = response as? HTTPURLResponse else { return }
        showingChallenge = VerificationCookie.isChallengeResponse(headers: http.allHeaderFields)
    }

    func didFinishLoading() {
        isLoading = false
    }

    func didFailLoading(_ error: any Error) {
        isLoading = false
        let nsError = error as NSError
        // A new navigation replacing this one (reload, the challenge's own redirect) isn't a failure.
        if nsError.domain == NSURLErrorDomain, nsError.code == NSURLErrorCancelled { return }
        if nsError.domain == "WebKitErrorDomain", nsError.code == 102 { return } // frame load interrupted
        loadError = UserFacingError(kind: .network, message: "Can't reach \(libraryName). Check your connection.",
                                    detail: nsError.localizedDescription)
    }

    /// The cookie to complete verification with, at most once: a usable token for the library's
    /// host while the library's own page (not the challenge) is showing.
    func verifiedCookie(from cookies: [VerificationCookie.Candidate], now: Date = Date()) -> String? {
        guard completedCookie == nil, showingChallenge == false else { return nil }
        guard let header = VerificationCookie.header(in: cookies, host: host, now: now) else {
            foundNoToken = true
            return nil
        }
        completedCookie = header
        return header
    }

    // MARK: Actions

    func reload() {
        loadError = nil
        guard let webView else { return }
        if webView.url == nil, let pageURL {
            webView.load(URLRequest(url: pageURL))
        } else {
            webView.reload()
        }
    }

    /// "Clear verification": deletes the library's `aws-waf-token` cookies from the web view's
    /// persistent store, so the next check starts fresh.
    static func clearStoredToken(origin: String) async {
        let host = LibraryDirectory.host(of: origin)
        let store = WKWebsiteDataStore.default().httpCookieStore
        for cookie in await store.allCookies()
        where VerificationCookie.isVerificationCookie(.init(cookie), host: host) {
            await store.deleteCookie(cookie)
        }
    }
}

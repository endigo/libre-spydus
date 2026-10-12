import CoreGraphics
import Foundation
import Testing
@testable import LibreSpydus

struct OnboardingVerificationCookieTests {
    typealias Candidate = VerificationCookie.Candidate
    let host = "aberdeencity.spydus.co.uk"
    let now = Date(timeIntervalSince1970: 1_800_000_000)

    @Test(arguments: [
        ("aberdeencity.spydus.co.uk", true),
        (".aberdeencity.spydus.co.uk", true),
        ("AberdeenCity.Spydus.co.uk", true),
        ("aberdeencity.spydus.co.uk.", true),
        (".spydus.co.uk", true),
        ("spydus.co.uk", true),
        ("..spydus.co.uk", true),
        ("aberdeenshire.spydus.co.uk", false),
        ("deencity.spydus.co.uk", false),
        ("city.spydus.co.uk", false),
        ("spydus.com", false),
        ("uk", false),
        (".uk", false),
        ("", false),
        (".", false),
    ])
    func domainMatching(domain: String, matches: Bool) {
        #expect(VerificationCookie.domainMatches(domain, host: host) == matches)
    }

    @Test func ipHostsOnlyMatchExactly() {
        #expect(VerificationCookie.domainMatches("127.0.0.1", host: "127.0.0.1"))
        #expect(!VerificationCookie.domainMatches("0.0.1", host: "127.0.0.1"))
        #expect(!VerificationCookie.domainMatches("", host: ""))
    }

    @Test func findsTheLibrarysToken() {
        let cookies = [
            Candidate(name: "SESSION", value: "s", domain: host),
            Candidate(name: "aws-waf-token", value: "abc:def==", domain: host, expiresDate: now + 3600),
        ]
        #expect(VerificationCookie.header(in: cookies, host: host, now: now) == "aws-waf-token=abc:def==")
    }

    @Test func leadingDotParentDomainCookieCounts() {
        let cookies = [Candidate(name: "aws-waf-token", value: "parent", domain: ".spydus.co.uk")]
        #expect(VerificationCookie.header(in: cookies, host: host, now: now) == "aws-waf-token=parent")
    }

    @Test func ignoresOtherHostsNamesExpiredAndEmptyTokens() {
        let cookies = [
            Candidate(name: "aws-waf-token", value: "other", domain: "aberdeenshire.spydus.co.uk"),
            Candidate(name: "aws-waf-token", value: "expired", domain: host, expiresDate: now - 1),
            Candidate(name: "aws-waf-token", value: "", domain: host),
            Candidate(name: "AWS-WAF-TOKEN", value: "case", domain: host),
            Candidate(name: "aws-waf-token-x", value: "prefix", domain: host),
        ]
        #expect(VerificationCookie.header(in: cookies, host: host, now: now) == nil)
        #expect(VerificationCookie.header(in: [Candidate](), host: host, now: now) == nil)
    }

    @Test func hostOnlyCookieWinsOverParentDomain() {
        let cookies = [
            Candidate(name: "aws-waf-token", value: "parent", domain: ".spydus.co.uk", expiresDate: now + 9000),
            Candidate(name: "aws-waf-token", value: "host", domain: host, expiresDate: now + 60),
        ]
        #expect(VerificationCookie.header(in: cookies, host: host, now: now) == "aws-waf-token=host")
        #expect(VerificationCookie.header(in: Array(cookies.reversed()), host: host, now: now) == "aws-waf-token=host")
    }

    @Test func latestExpiryWinsAmongEquals() {
        let cookies = [
            Candidate(name: "aws-waf-token", value: "older", domain: host, expiresDate: now + 60),
            Candidate(name: "aws-waf-token", value: "session", domain: ".\(host)"),
            Candidate(name: "aws-waf-token", value: "newer", domain: host, expiresDate: now + 600),
        ]
        // A session cookie (no expiry) lasts longest; it's host-matching with a leading dot.
        #expect(VerificationCookie.header(in: cookies, host: host, now: now) == "aws-waf-token=session")
        #expect(VerificationCookie.header(in: Array(cookies.prefix(1) + cookies.suffix(1)), host: host, now: now)
            == "aws-waf-token=newer")
    }

    @Test func readsRealHTTPCookies() throws {
        let cookie = try #require(HTTPCookie(properties: [
            .name: "aws-waf-token", .value: "real", .domain: ".aberdeencity.spydus.co.uk", .path: "/",
            .expires: now + 3600,
        ]))
        #expect(VerificationCookie.header(in: [cookie], host: host, now: now) == "aws-waf-token=real")
        #expect(VerificationCookie.isVerificationCookie(.init(cookie), host: host))
        #expect(!VerificationCookie.isVerificationCookie(.init(cookie), host: "canning.spydus.com"))
    }

    @Test func challengeResponsesAreRecognisedByTheWAFHeader() {
        #expect(VerificationCookie.isChallengeResponse(headers: ["x-amzn-waf-action": "captcha"]))
        #expect(VerificationCookie.isChallengeResponse(headers: ["X-Amzn-Waf-Action": "challenge"]))
        #expect(!VerificationCookie.isChallengeResponse(headers: ["Content-Type": "text/html"]))
        #expect(!VerificationCookie.isChallengeResponse(headers: [:]))
    }
}

@MainActor
struct OnboardingVerificationSessionTests {
    let origin = "https://aberdeencity.spydus.co.uk"
    let token = VerificationCookie.Candidate(name: "aws-waf-token", value: "t1", domain: "aberdeencity.spydus.co.uk")

    func response(waf: Bool, status: Int = 200) -> HTTPURLResponse {
        HTTPURLResponse(url: URL(string: origin + "/")!, statusCode: waf ? 405 : status, httpVersion: "HTTP/1.1",
                        headerFields: waf ? ["x-amzn-waf-action": "captcha"] : ["Content-Type": "text/html"])!
    }

    @Test func loadsTheOriginRoot() {
        let session = VerificationSession(origin: origin, libraryName: "Aberdeen City Libraries")
        #expect(session.pageURL == URL(string: "https://aberdeencity.spydus.co.uk/"))
        #expect(session.host == "aberdeencity.spydus.co.uk")
        #expect(session.isLoading)
        #expect(session.showingChallenge == nil)
    }

    @Test func waitsForTheLibrarysOwnPageBeforeTakingTheToken() {
        let session = VerificationSession(origin: origin, libraryName: "Aberdeen")
        // Before any page response, and while the challenge shows, a token isn't taken.
        #expect(session.verifiedCookie(from: [token]) == nil)
        session.didReceiveMainFrameResponse(response(waf: true))
        #expect(session.showingChallenge == true)
        #expect(session.verifiedCookie(from: [token]) == nil)
        #expect(!session.foundNoToken)

        // The challenge reloads into the library's page: now the token completes, once.
        session.didStartLoading()
        session.didReceiveMainFrameResponse(response(waf: false))
        session.didFinishLoading()
        #expect(session.verifiedCookie(from: [token]) == "aws-waf-token=t1")
        #expect(session.completedCookie == "aws-waf-token=t1")
        #expect(session.verifiedCookie(from: [token]) == nil)
        #expect(!session.finishedWithoutCheck)
    }

    @Test func libraryPageWithoutATokenIsReported() {
        let session = VerificationSession(origin: origin, libraryName: "Aberdeen")
        session.didReceiveMainFrameResponse(response(waf: false))
        session.didFinishLoading()
        #expect(session.verifiedCookie(from: []) == nil)
        #expect(session.finishedWithoutCheck)
        session.didStartLoading()
        #expect(!session.finishedWithoutCheck)
    }

    @Test func loadFailuresBecomeANetworkErrorButCancellationsDont() {
        let session = VerificationSession(origin: origin, libraryName: "Aberdeen City Libraries")
        session.didFailLoading(URLError(.cancelled))
        #expect(session.loadError == nil)
        #expect(!session.isLoading)
        session.didFailLoading(URLError(.notConnectedToInternet))
        #expect(session.loadError?.kind == .network)
        #expect(session.loadError?.message == "Can't reach Aberdeen City Libraries. Check your connection.")
        session.didStartLoading()
        #expect(session.loadError == nil)
    }
}

/// Logos that are light on a transparent background get a navy tile.
@MainActor
struct OnboardingLogoToneTests {
    /// A 32×32 image: transparent, with a centred square of `rgba` (or filled when `opaque`).
    func image(red: CGFloat, green: CGFloat, blue: CGFloat, opaque: Bool = false) -> CGImage {
        let context = CGContext(data: nil, width: 32, height: 32, bitsPerComponent: 8, bytesPerRow: 0,
                                space: CGColorSpaceCreateDeviceRGB(),
                                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        context.setFillColor(red: red, green: green, blue: blue, alpha: 1)
        context.fill(opaque ? CGRect(x: 0, y: 0, width: 32, height: 32) : CGRect(x: 8, y: 8, width: 16, height: 16))
        return context.makeImage()!
    }

    @Test func whiteOnTransparentIsLight() {
        #expect(LogoTone.measure(image(red: 1, green: 1, blue: 1)))
    }

    @Test func darkOrOpaqueLogosStayOnWhite() {
        #expect(!LogoTone.measure(image(red: 0.1, green: 0.1, blue: 0.3)))
        #expect(!LogoTone.measure(image(red: 1, green: 1, blue: 1, opaque: true)))
        #expect(!LogoTone.measure(image(red: 0.9, green: 0.2, blue: 0.2)))
    }
}

package com.librespydus.app.feature.onboarding

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.librespydus.app.core.Copy
import com.librespydus.app.core.UserFacingError

/**
 * AWS WAF human verification, the pure parts (core/README.md "Human verification"): find the
 * library's `aws-waf-token` in the web view's cookies, and tell a WAF challenge response from the
 * library's own page. Nothing here touches the challenge itself.
 */
object VerificationCookie {
    const val NAME = "aws-waf-token"
    private const val WAF_HEADER = "x-amzn-waf-action"

    /**
     * `"aws-waf-token=<value>"` from a `Cookie` header such as `CookieManager.getCookie(origin)`
     * returns (`"a=1; aws-waf-token=abc:def==; b=2"`), or null when there's no usable token.
     * The cookie manager has already matched domain, path and expiry for that URL; with a
     * host-only and a parent-domain token both present, the first listed wins. Names are
     * case-sensitive, and an empty value doesn't count.
     */
    fun header(cookieHeader: String?): String? {
        if (cookieHeader.isNullOrBlank()) return null
        return cookieHeader.split(';').firstNotNullOfOrNull { pair ->
            val separator = pair.indexOf('=')
            if (separator < 0) return@firstNotNullOfOrNull null
            val name = pair.substring(0, separator).trim()
            val value = pair.substring(separator + 1).trim()
            if (name == NAME && value.isNotEmpty()) "$NAME=$value" else null
        }
    }

    /**
     * Whether a page response is the WAF's challenge rather than the library's own page: the core
     * treats any response carrying `x-amzn-waf-action` as a challenge, whatever its status.
     */
    fun isChallengeResponse(headers: Map<String, String>?): Boolean =
        headers?.keys?.any { it.equals(WAF_HEADER, ignoreCase = true) } == true

    /**
     * Whether a cookie set for [domain] is sent to [host] (RFC 6265 domain-match):
     * `canning.spydus.com` and `.spydus.com` match `canning.spydus.com`; `other.spydus.com`,
     * `spydus.co.uk` and `nning.spydus.com` don't. IP addresses only match exactly.
     */
    fun domainMatches(domain: String, host: String): Boolean {
        val d = normalized(domain)
        val h = normalized(host)
        if (d.isEmpty() || h.isEmpty()) return false
        if (d == h) return true
        if (isIpAddress(h) || '.' !in d) return false
        return h.endsWith(".$d")
    }

    /**
     * "Clear verification": `Set-Cookie` values that expire the library's token in the web view's
     * cookie store (Android's `CookieManager` can't delete one cookie). One for a host-only
     * cookie, then one per domain the token could have been set for: the host and its parent
     * domains. The cookie store ignores public suffixes such as `co.uk`.
     */
    fun expiringCookies(host: String): List<String> {
        val expired = "$NAME=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; Path=/"
        val labels = normalized(host).split('.').filter { it.isNotEmpty() }
        if (labels.isEmpty()) return emptyList()
        if (isIpAddress(normalized(host))) return listOf(expired)
        val domains = (0..labels.size - 2).map { labels.drop(it).joinToString(".") }
        return listOf(expired) + domains.map { "$expired; Domain=$it" }
    }

    /** Lower-cased, without a leading or trailing dot. */
    private fun normalized(domain: String): String = domain.trim().lowercase().trim('.')

    private fun isIpAddress(host: String): Boolean =
        ':' in host || host.split('.').all { part -> part.isNotEmpty() && part.all { it.isDigit() } }
}

/**
 * One verification attempt in the sheet's web view: page state, and when to hand the
 * `aws-waf-token` to the app.
 *
 * The token is taken only once the web view shows the library's own page again (the last
 * main-frame page wasn't a WAF challenge), so a token the challenge sets before the user has
 * finished, or an expired one the library just rejected, isn't used.
 *
 * Android's `WebViewClient` only exposes a main-frame response's headers for HTTP errors
 * (`onReceivedHttpError`), which is how the WAF answers (HTTP 405 + `x-amzn-waf-action`): a page
 * that finishes loading without such an error is the library's own.
 */
class VerificationSession(val origin: String, val libraryName: String) {
    val host: String = LibraryPickerLogic.host(origin)

    /** `client.origin() + "/"` (core/README.md "Human verification"). */
    val pageUrl: String = origin.trimEnd('/') + "/"

    var isLoading by mutableStateOf(true)
        private set
    var loadError by mutableStateOf<UserFacingError?>(null)
        private set

    /** Whether the last page was the WAF challenge; null before the first page has loaded. */
    var showingChallenge by mutableStateOf<Boolean?>(null)
        private set

    /** The `aws-waf-token=<value>` handed to the app (at most once). */
    var completedCookie by mutableStateOf<String?>(null)
        private set

    /** The library's own page is showing and its cookies held no token. */
    var foundNoToken by mutableStateOf(false)
        private set

    /**
     * A WAF response arrived since the last page finished. Chromium may report a main-frame HTTP
     * error before or after `onPageStarted`, so it's tied to the next finish, not to a start.
     */
    private var challengeSinceFinish = false

    /** A page started since the last finish (a repeated `onPageFinished` is ignored). */
    private var pageStarted = false

    /**
     * The page being loaded failed. WebView still reports `onPageFinished` for a failed load; that
     * finish isn't the library's page, so it doesn't make a token usable.
     */
    private var failedSinceStart = false

    /** The library's page loaded without a challenge and there's no token: nothing to complete. */
    val finishedWithoutCheck: Boolean
        get() = showingChallenge == false && !isLoading && completedCookie == null && foundNoToken

    // region Web view events

    fun didStartLoading() {
        isLoading = true
        pageStarted = true
        failedSinceStart = false
        loadError = null
        foundNoToken = false
    }

    /** A main-frame HTTP error response (status ≥ 400) with its headers. */
    fun didReceiveMainFrameHttpError(headers: Map<String, String>?) {
        if (VerificationCookie.isChallengeResponse(headers)) {
            challengeSinceFinish = true
            showingChallenge = true
        }
    }

    fun didFinishLoading() {
        isLoading = false
        if (!pageStarted) return
        pageStarted = false
        if (failedSinceStart) {
            // Not the library's page: keep whatever was known before (null before any page).
            if (challengeSinceFinish) showingChallenge = true
        } else {
            showingChallenge = challengeSinceFinish
        }
        challengeSinceFinish = false
    }

    /** A main-frame load failed (no connection, DNS, TLS…). An aborted navigation isn't a failure. */
    fun didFailLoading(description: String?) {
        isLoading = false
        if (description != null && "ERR_ABORTED" in description) return
        failedSinceStart = true
        loadError = UserFacingError(
            UserFacingError.Kind.NETWORK,
            Copy.Errors.network(libraryName),
            description,
        )
    }

    // endregion

    /**
     * The cookie to complete verification with, at most once: a usable token for the library
     * while the library's own page (not the challenge) is showing. [cookieHeader] is
     * `CookieManager.getCookie(pageUrl)`.
     */
    fun verifiedCookie(cookieHeader: String?): String? {
        if (completedCookie != null || showingChallenge != false || loadError != null || isLoading) return null
        val header = VerificationCookie.header(cookieHeader)
        if (header == null) {
            foundNoToken = true
            return null
        }
        completedCookie = header
        return header
    }
}

package com.librespydus.app.feature.onboarding

import com.librespydus.app.core.UserFacingError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VerificationCookieTest {
    private val host = "aberdeencity.spydus.co.uk"

    @Test
    fun domainMatching() {
        val cases = listOf(
            "aberdeencity.spydus.co.uk" to true,
            ".aberdeencity.spydus.co.uk" to true,
            "AberdeenCity.Spydus.co.uk" to true,
            "aberdeencity.spydus.co.uk." to true,
            ".spydus.co.uk" to true,
            "spydus.co.uk" to true,
            "..spydus.co.uk" to true,
            "aberdeenshire.spydus.co.uk" to false,
            "deencity.spydus.co.uk" to false,
            "city.spydus.co.uk" to false,
            "spydus.com" to false,
            "uk" to false,
            ".uk" to false,
            "" to false,
            "." to false,
        )
        for ((domain, matches) in cases) assertEquals(domain, matches, VerificationCookie.domainMatches(domain, host))
    }

    @Test
    fun ipHostsOnlyMatchExactly() {
        assertTrue(VerificationCookie.domainMatches("127.0.0.1", "127.0.0.1"))
        assertFalse(VerificationCookie.domainMatches("0.0.1", "127.0.0.1"))
        assertFalse(VerificationCookie.domainMatches("", ""))
    }

    @Test
    fun findsTheLibrarysTokenInTheCookieHeader() {
        assertEquals(
            "aws-waf-token=abc:def==",
            VerificationCookie.header("SESSION=s; aws-waf-token=abc:def==; other=1"),
        )
        assertEquals("aws-waf-token=only", VerificationCookie.header("aws-waf-token=only"))
        assertEquals("aws-waf-token=spaced", VerificationCookie.header("  aws-waf-token = spaced ;x=y"))
    }

    @Test
    fun theFirstOfSeveralTokensWins() {
        // CookieManager lists a host-only and a parent-domain cookie both; the first is used.
        assertEquals("aws-waf-token=first", VerificationCookie.header("aws-waf-token=first; aws-waf-token=second"))
        assertEquals("aws-waf-token=second", VerificationCookie.header("aws-waf-token=; aws-waf-token=second"))
    }

    @Test
    fun ignoresOtherNamesEmptyAndMissingTokens() {
        assertNull(VerificationCookie.header(null))
        assertNull(VerificationCookie.header(""))
        assertNull(VerificationCookie.header("   "))
        assertNull(VerificationCookie.header("aws-waf-token="))
        assertNull(VerificationCookie.header("AWS-WAF-TOKEN=case"))
        assertNull(VerificationCookie.header("aws-waf-token-x=prefix; x-aws-waf-token=suffix"))
        assertNull(VerificationCookie.header("aws-waf-token"))
    }

    @Test
    fun challengeResponsesAreRecognisedByTheWafHeader() {
        assertTrue(VerificationCookie.isChallengeResponse(mapOf("x-amzn-waf-action" to "captcha")))
        assertTrue(VerificationCookie.isChallengeResponse(mapOf("X-Amzn-Waf-Action" to "challenge")))
        assertFalse(VerificationCookie.isChallengeResponse(mapOf("Content-Type" to "text/html")))
        assertFalse(VerificationCookie.isChallengeResponse(emptyMap()))
        assertFalse(VerificationCookie.isChallengeResponse(null))
    }

    @Test
    fun clearingExpiresTheTokenForTheHostAndItsParents() {
        val cookies = VerificationCookie.expiringCookies(host)
        assertEquals(4, cookies.size)
        assertTrue(cookies.all { it.startsWith("aws-waf-token=; Max-Age=0;") && "Path=/" in it })
        assertFalse("Domain=" in cookies[0]) // the host-only cookie
        assertEquals(
            listOf("Domain=aberdeencity.spydus.co.uk", "Domain=spydus.co.uk", "Domain=co.uk"),
            cookies.drop(1).map { it.substringAfterLast("; ") },
        )
        assertEquals(1, VerificationCookie.expiringCookies("127.0.0.1").size)
        assertTrue(VerificationCookie.expiringCookies("").isEmpty())
    }
}

class VerificationSessionTest {
    private val origin = "https://aberdeencity.spydus.co.uk"
    private val token = "SESSION=x; aws-waf-token=t1"
    private val waf = mapOf("x-amzn-waf-action" to "captcha")

    @Test
    fun loadsTheOriginRoot() {
        val session = VerificationSession(origin, "Aberdeen City Libraries")
        assertEquals("https://aberdeencity.spydus.co.uk/", session.pageUrl)
        assertEquals("https://aberdeencity.spydus.co.uk/", VerificationSession("$origin/", "A").pageUrl)
        assertEquals("aberdeencity.spydus.co.uk", session.host)
        assertTrue(session.isLoading)
        assertNull(session.showingChallenge)
    }

    @Test
    fun waitsForTheLibrarysOwnPageBeforeTakingTheToken() {
        val session = VerificationSession(origin, "Aberdeen")
        // Before any page, and while the challenge shows, a token isn't taken.
        assertNull(session.verifiedCookie(token))
        session.didStartLoading()
        session.didReceiveMainFrameHttpError(waf)
        assertEquals(true, session.showingChallenge)
        session.didFinishLoading()
        assertEquals(true, session.showingChallenge)
        assertNull(session.verifiedCookie(token))
        assertFalse(session.foundNoToken)

        // The challenge reloads into the library's page: now the token completes, once.
        session.didStartLoading()
        session.didFinishLoading()
        assertEquals(false, session.showingChallenge)
        assertEquals("aws-waf-token=t1", session.verifiedCookie(token))
        assertEquals("aws-waf-token=t1", session.completedCookie)
        assertNull(session.verifiedCookie(token))
        assertFalse(session.finishedWithoutCheck)
    }

    @Test
    fun aWafResponseReportedBeforeThePageStartsStillCounts() {
        val session = VerificationSession(origin, "Aberdeen")
        session.didReceiveMainFrameHttpError(waf)
        session.didStartLoading()
        session.didFinishLoading()
        assertEquals(true, session.showingChallenge)
        assertNull(session.verifiedCookie(token))
    }

    @Test
    fun aRepeatedFinishForTheChallengeIsIgnored() {
        val session = VerificationSession(origin, "Aberdeen")
        session.didStartLoading()
        session.didReceiveMainFrameHttpError(waf)
        session.didFinishLoading()
        session.didFinishLoading()
        assertEquals(true, session.showingChallenge)
        assertNull(session.verifiedCookie(token))
    }

    @Test
    fun otherHttpErrorsArentChallenges() {
        val session = VerificationSession(origin, "Aberdeen")
        session.didStartLoading()
        session.didReceiveMainFrameHttpError(mapOf("Content-Type" to "text/html"))
        session.didFinishLoading()
        assertEquals(false, session.showingChallenge)
    }

    @Test
    fun libraryPageWithoutATokenIsReported() {
        val session = VerificationSession(origin, "Aberdeen")
        session.didStartLoading()
        session.didFinishLoading()
        assertNull(session.verifiedCookie("SESSION=x"))
        assertTrue(session.finishedWithoutCheck)
        session.didStartLoading()
        assertFalse(session.finishedWithoutCheck)
    }

    @Test
    fun aFailedLoadIsntTheLibrarysPage() {
        // WebView reports onPageFinished after onReceivedError: the (stale) token must not be taken.
        val first = VerificationSession(origin, "Aberdeen")
        first.didStartLoading()
        first.didFailLoading("net::ERR_INTERNET_DISCONNECTED")
        first.didFinishLoading()
        assertNull(first.showingChallenge)
        assertNull(first.verifiedCookie(token))
        assertFalse(first.finishedWithoutCheck)

        // The library's page showed, then a reload failed: still no token while the error shows.
        val later = VerificationSession(origin, "Aberdeen")
        later.didStartLoading()
        later.didReceiveMainFrameHttpError(waf)
        later.didFinishLoading()
        later.didStartLoading()
        later.didFailLoading("net::ERR_CONNECTION_RESET")
        later.didFinishLoading()
        assertEquals(true, later.showingChallenge)
        assertNull(later.verifiedCookie(token))

        // Retry loads the library's page: now it counts.
        later.didStartLoading()
        later.didFinishLoading()
        assertEquals("aws-waf-token=t1", later.verifiedCookie(token))
    }

    @Test
    fun noTokenIsTakenWhileAPageIsLoading() {
        val session = VerificationSession(origin, "Aberdeen")
        session.didStartLoading()
        session.didFinishLoading()
        session.didStartLoading() // the library page navigates on
        assertNull(session.verifiedCookie(token))
        session.didFinishLoading()
        assertEquals("aws-waf-token=t1", session.verifiedCookie(token))
    }

    @Test
    fun loadFailuresBecomeANetworkErrorButAbortsDont() {
        val session = VerificationSession(origin, "Aberdeen City Libraries")
        session.didFailLoading("net::ERR_ABORTED")
        assertNull(session.loadError)
        assertFalse(session.isLoading)
        session.didFailLoading("net::ERR_INTERNET_DISCONNECTED")
        assertEquals(UserFacingError.Kind.NETWORK, session.loadError?.kind)
        assertEquals("Can’t reach Aberdeen City Libraries. Check your connection.", session.loadError?.message)
        assertEquals("net::ERR_INTERNET_DISCONNECTED", session.loadError?.detail)
        session.didStartLoading()
        assertNull(session.loadError)
    }
}

/** Logos that are light on a transparent background get a navy tile. */
class LogoToneTest {
    /** 32×32 ARGB pixels: transparent with a centred 16×16 square of [rgb] (or all of it when [opaque]). */
    private fun pixels(rgb: Int, opaque: Boolean = false): IntArray = IntArray(32 * 32) { index ->
        val x = index % 32
        val y = index / 32
        val inside = opaque || (x in 8 until 24 && y in 8 until 24)
        if (inside) (0xFF shl 24) or rgb else 0
    }

    @Test
    fun whiteOnTransparentIsLight() {
        assertTrue(LogoTone.measure(pixels(0xFFFFFF)))
    }

    @Test
    fun darkOrOpaqueLogosStayOnWhite() {
        assertFalse(LogoTone.measure(pixels(0x1A1A4D)))
        assertFalse(LogoTone.measure(pixels(0xFFFFFF, opaque = true)))
        assertFalse(LogoTone.measure(pixels(0xE63333)))
        assertFalse(LogoTone.measure(IntArray(0)))
        assertFalse(LogoTone.measure(IntArray(16))) // fully transparent
    }
}

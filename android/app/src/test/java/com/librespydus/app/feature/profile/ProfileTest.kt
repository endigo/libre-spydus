package com.librespydus.app.feature.profile

import com.librespydus.app.core.AppState
import com.librespydus.app.feature.onboarding.CANNING
import com.librespydus.app.feature.onboarding.accountFixture
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.librespydus_core.LibraryInfo

class ProfileTest {
    @get:Rule val tmp = TemporaryFolder()
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    @Test
    fun versionTextNamesVersionAndBuild() {
        assertEquals("Version 0.1.0 (1)", ProfileText.versionText("0.1.0", 1))
        assertEquals("Version ? (7)", ProfileText.versionText(null, 7))
        val text = ProfileText.versionText()
        assertTrue(text.startsWith("Version "))
        assertFalse("?" in text)
    }

    @Test
    fun accountInitials() {
        assertEquals("TB", ProfileText.initials("Test Borrower"))
        assertEquals("TB", ProfileText.initials("Borrower, Test"))
        assertEquals("M", ProfileText.initials("Madonna"))
        assertEquals("", ProfileText.initials(null))
        assertEquals("", ProfileText.initials("  "))
    }

    @Test
    fun theLibraryCardShowsTheCountryElseTheRegion() {
        assertEquals("Australia", ProfileText.place(CANNING))
        val bare = LibraryInfo("x", "X", "https://x.spydus.co.uk", null, null, "UK", false)
        assertEquals("United Kingdom", ProfileText.place(bare))
        assertEquals("Other", ProfileText.place(bare.copy(region = "ZZ", country = " ")))
    }

    @Test
    fun clearingVerificationForgetsTheCookie() = runTest {
        val f = accountFixture(tmp.root).also { scopes += it.scope }
        f.state.completeVerification("aws-waf-token=abc")
        assertTrue(f.state.hasVerificationCookie.value)
        assertEquals("aws-waf-token=abc", f.secrets.get(AppState.verifyKey(CANNING.origin)))
        val generation = f.state.verificationGeneration.value

        f.state.clearVerification()
        assertFalse(f.state.hasVerificationCookie.value)
        assertNull(f.secrets.get(AppState.verifyKey(CANNING.origin)))
        assertEquals(generation + 1, f.state.verificationGeneration.value)
    }
}

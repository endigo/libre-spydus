package com.librespydus.app.feature.onboarding

import com.librespydus.app.core.UserFacingError
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/** The sign-in form against DemoBackend ("demo"/"demo" signs in; anything else is rejected). No network. */
class LoginFormTest {
    @get:Rule val tmp = TemporaryFolder()
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    @Test
    fun needsACardNumberAndAPin() {
        val form = LoginFormState()
        assertFalse(form.canSubmit)
        form.cardNumber = "   "
        form.pin = "1234"
        assertFalse(form.canSubmit)
        form.cardNumber = " 2000123 "
        assertTrue(form.canSubmit)
        form.pin = ""
        assertFalse(form.canSubmit)
    }

    @Test
    fun rejectedCredentialsShowInline() = runTest {
        val f = accountFixture(tmp.root).also { scopes += it.scope }
        f.state.requestLogin()
        val form = LoginFormState().apply {
            cardNumber = "2000123"
            pin = "wrong"
        }
        assertFalse(form.submit(f.state))
        assertEquals(UserFacingError.Kind.CREDENTIALS, form.error?.kind)
        assertEquals("Card number or PIN not recognised.", form.error?.message)
        assertFalse(form.isSigningIn)
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
        form.clearError()
        assertNull(form.error)
    }

    @Test
    fun successSignsInClosesTheSheetAndLoadsTheAccount() = runTest {
        val f = accountFixture(tmp.root).also { scopes += it.scope }
        f.state.requestLogin()
        val form = LoginFormState().apply {
            cardNumber = " demo\n"
            pin = "demo"
        }
        assertTrue(form.submit(f.state))
        advanceUntilIdle()
        assertNull(form.error)
        assertEquals("", form.pin)
        assertTrue(f.state.isSignedIn.value)
        assertFalse(f.state.loginNeeded.value)
        assertEquals("Test Borrower", f.state.accountName.value)
    }

    @Test
    fun emptyFormDoesNothing() = runTest {
        val f = accountFixture(tmp.root).also { scopes += it.scope }
        val form = LoginFormState()
        assertFalse(form.submit(f.state))
        assertNull(form.error)
    }
}

package com.librespydus.app.core

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.SpydusException
import java.io.File

/** Signing back in with the fingerprint check (§5.14), through AppState with a fake vault. */
class BiometricSignInTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = LibraryInfo("canning", "City of Canning", "https://canning.spydus.com", null, "Australia", "AU", false)
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    /** The library: `login` answers `session-<username>` (or [loginFails]); sessions in [expired] are over. */
    private class SessionBackend(origin: String) : LibraryBackend by FakeBackend(origin) {
        val logins = mutableListOf<String>()
        var loginFails: Exception? = null
        val expired = mutableSetOf<String>()

        override suspend fun login(username: String, password: String): String {
            logins += username
            delay(100)
            loginFails?.let { throw it }
            return "session-$username"
        }

        override suspend fun accountSnapshot(session: String): AccountSnapshot {
            delay(100)
            if (session in expired) throw SpydusException.NotLoggedIn("not logged in")
            return accountSnapshot(name = "Reader for $session") // the top-level fixture, not this member
        }
    }

    /** Keeps details in memory; [readAnswer] / [savePasses] stand in for the reader at the prompt. */
    private class FakeVault(var available: Boolean = true) : BiometricVault {
        val kept = mutableMapOf<String, SignInDetails>()
        var readAnswer: BiometricRead? = null
        var savePasses = true
        var reads = 0
        var saves = 0

        /** When set, a save waits for it (the prompt is up). */
        var saveGate: CompletableDeferred<Unit>? = null

        /** When set, a read waits for it (the prompt is up). */
        var gate: CompletableDeferred<Unit>? = null

        override fun isAvailable() = available
        override fun username(origin: String) = kept[origin]?.username

        override suspend fun save(origin: String, details: SignInDetails, libraryName: String): Boolean {
            saves++
            saveGate?.await()
            if (savePasses) kept[origin] = details
            return savePasses
        }

        override suspend fun read(origin: String, libraryName: String): BiometricRead {
            reads++
            gate?.await()
            readAnswer?.let { return it }
            return kept[origin]?.let { BiometricRead.Unlocked(it) } ?: BiometricRead.Invalidated
        }

        override fun delete(origin: String) {
            kept.remove(origin)
        }

        override fun deleteAll() = kept.clear()
    }

    private class Fixture(
        val state: AppState,
        val backend: () -> SessionBackend,
        val vault: FakeVault,
        val offers: InMemoryBiometricOfferStore,
        val secrets: InMemorySecretStore,
        val foreground: MutableStateFlow<Boolean>,
    )

    private fun TestScope.fixture(
        session: String? = null,
        details: SignInDetails? = null,
        foreground: Boolean = true,
        expired: Set<String> = emptySet(),
    ): Fixture {
        val io = StandardTestDispatcher(testScheduler)
        val secrets = InMemorySecretStore().apply { session?.let { put(AppState.sessionKey(canning.origin), it) } }
        val vault = FakeVault().apply { details?.let { kept[canning.origin] = it } }
        val offers = InMemoryBiometricOfferStore()
        val fg = MutableStateFlow(foreground)
        var backend: SessionBackend? = null
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        val state = AppState(
            scope = scope,
            libraryStore = InMemoryLibraryStore(canning),
            secrets = secrets,
            favourites = FavouritesStore(File(tmp.root, "fav.json"), io = io),
            directory = { listOf(canning) },
            makeBackend = { origin -> SessionBackend(origin).also { it.expired += expired; backend = it } },
            io = io,
            biometrics = vault,
            biometricOffers = offers,
            foreground = fg,
        )
        return Fixture(state, { backend!! }, vault, offers, secrets, fg)
    }

    private val reader = SignInDetails("cl114d", "1234")

    // region The offer

    @Test
    fun aTypedSignInOffersTheCheck() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()

        f.state.signIn(" cl114d ", "1234")
        val offer = f.state.biometricOffer.value!!
        assertEquals(canning.origin, offer.origin)
        assertEquals("Sign in with fingerprint next time?", offer.title)
        assertEquals(
            "When City of Canning signs you out, sign back in with fingerprint instead of typing your PIN. " +
                "Your card number and PIN stay on this phone, locked by fingerprint.",
            offer.message,
        )

        f.state.answerBiometricOffer(use = true)
        advanceUntilIdle()
        assertNull(f.state.biometricOffer.value)
        assertEquals(1, f.vault.saves)
        assertEquals(SignInDetails("cl114d", "1234"), f.vault.kept[canning.origin])
        assertTrue(f.state.biometricOn.value)
    }

    @Test
    fun aRestoredSessionOffersNothing() = runTest {
        val f = fixture(session = "tok")
        f.state.start()
        advanceUntilIdle()
        assertTrue(f.state.isSignedIn.value)
        assertNull(f.state.biometricOffer.value)
    }

    @Test
    fun noOfferWithoutTheCheck() = runTest {
        val f = fixture()
        f.vault.available = false
        f.state.start()
        advanceUntilIdle()
        f.state.signIn("cl114d", "1234")
        assertNull(f.state.biometricOffer.value)
        assertFalse(f.state.biometricAvailable.value)
    }

    @Test
    fun aCancelledCheckKeepsNothingAndIsNotRemembered() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        f.state.signIn("cl114d", "1234")
        f.vault.savePasses = false
        f.state.answerBiometricOffer(use = true)
        advanceUntilIdle()
        assertTrue(f.vault.kept.isEmpty())
        assertFalse(f.state.biometricOn.value)
        assertFalse(f.offers.wasDeclined(canning.origin))

        f.state.signIn("cl114d", "1234")
        assertNotNull(f.state.biometricOffer.value) // asked again next time
    }

    @Test
    fun notNowIsRememberedUntilSignOut() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        f.state.signIn("cl114d", "1234")
        f.state.answerBiometricOffer(use = false)
        assertTrue(f.offers.wasDeclined(canning.origin))
        assertEquals(0, f.vault.saves)

        f.state.signIn("cl114d", "1234")
        assertNull(f.state.biometricOffer.value)

        f.state.signOut()
        assertFalse(f.offers.wasDeclined(canning.origin))
        f.state.signIn("cl114d", "1234")
        assertNotNull(f.state.biometricOffer.value)
    }

    @Test
    fun thePicksOfferWaitsForThisOne() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        f.state.toggleFavourite(BookRef(id = "work:1", title = "Saved"))
        f.state.signIn("cl114d", "1234")
        advanceUntilIdle()
        assertNotNull(f.state.picks.offer.value)
        assertNotNull(f.state.biometricOffer.value)
        assertNull(f.state.picksOfferToShow.value)

        f.state.answerBiometricOffer(use = false)
        advanceUntilIdle()
        assertNotNull(f.state.picksOfferToShow.value)
    }

    @Test
    fun thePicksOfferWaitsForTheCheckToEnd() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        f.state.toggleFavourite(BookRef(id = "work:1", title = "Saved"))
        f.state.signIn("cl114d", "1234")
        advanceUntilIdle()

        val gate = CompletableDeferred<Unit>().also { f.vault.saveGate = it }
        f.state.answerBiometricOffer(use = true)
        advanceUntilIdle()
        assertNull(f.state.biometricOffer.value)
        assertNull(f.state.picksOfferToShow.value) // the check is up

        gate.complete(Unit)
        advanceUntilIdle()
        assertTrue(f.state.biometricOn.value)
        assertNotNull(f.state.picksOfferToShow.value)
    }

    @Test
    fun aDifferentUsernameDeletesTheDetailsAndTheSameOneKeepsThem() = runTest {
        val f = fixture(details = reader)
        f.state.start()
        advanceUntilIdle()

        f.state.signIn(" CL114D ", "1234")
        assertEquals(reader, f.vault.kept[canning.origin])
        assertTrue(f.state.biometricOn.value)
        assertNull(f.state.biometricOffer.value)

        f.state.signIn("someone", "9999")
        assertNull(f.vault.kept[canning.origin]) // the check never signs in to the previous account
        assertFalse(f.state.biometricOn.value)
        assertEquals("someone", f.state.biometricOffer.value?.details?.username) // offered as usual
    }

    // endregion
    // region Signing back in

    @Test
    fun anExpiredSessionSignsBackInWithoutTheSignInSheet() = runTest {
        val f = fixture(session = "tok", details = reader, expired = setOf("tok"))
        val gate = CompletableDeferred<Unit>().also { f.vault.gate = it }
        f.state.start()
        advanceUntilIdle()

        // The check is up: the session and the saved account stay until it ends.
        assertEquals(1, f.vault.reads)
        assertEquals("tok", f.state.session.value)
        assertFalse(f.state.loginNeeded.value)
        assertTrue(f.state.biometricSigningIn.value)

        // More NotLoggedIns meanwhile are ignored.
        f.state.report(SpydusException.NotLoggedIn("not logged in"))
        runCurrent()
        assertEquals(1, f.vault.reads)
        assertEquals("tok", f.state.session.value)

        gate.complete(Unit)
        advanceUntilIdle()
        assertEquals(listOf("cl114d"), f.backend().logins)
        assertEquals("session-cl114d", f.state.session.value)
        assertEquals("session-cl114d", f.secrets.get(AppState.sessionKey(canning.origin)))
        assertFalse(f.state.loginNeeded.value)
        assertFalse(f.state.biometricSigningIn.value)
        assertEquals("Reader for session-cl114d", f.state.account.snapshot.value?.accountName)
        assertNull(f.state.biometricOffer.value) // nothing offered after a biometric sign-in
    }

    @Test
    fun theSavedAccountStaysOnScreenWhileSigningBackIn() = runTest {
        val f = fixture(session = "tok", details = reader)
        f.state.start()
        advanceUntilIdle()
        assertEquals("Reader for tok", f.state.account.snapshot.value?.accountName)

        val gate = CompletableDeferred<Unit>().also { f.vault.gate = it }
        f.backend().expired += "tok"
        f.state.account.refresh(force = true)
        advanceUntilIdle()
        assertEquals("Reader for tok", f.state.account.snapshot.value?.accountName)

        gate.complete(Unit)
        advanceUntilIdle()
        assertEquals("Reader for session-cl114d", f.state.account.snapshot.value?.accountName)
    }

    @Test
    fun aCancelledCheckShowsTheFormAndDoesNotCheckAgainByItself() = runTest {
        val f = fixture(session = "tok", details = reader, expired = setOf("tok"))
        f.vault.readAnswer = BiometricRead.Declined
        f.state.start()
        advanceUntilIdle()

        assertNull(f.state.session.value)
        assertNull(f.secrets.get(AppState.sessionKey(canning.origin)))
        assertTrue(f.state.loginNeeded.value)
        assertNull(f.state.loginError.value)
        assertEquals(reader, f.vault.kept[canning.origin]) // kept for the sheet's button
        assertTrue(f.state.biometricOn.value)

        // The sheet opening doesn't start another check…
        assertNull(f.state.autoBiometricSignIn())
        advanceUntilIdle()
        assertEquals(1, f.vault.reads)

        // …its button does.
        f.vault.readAnswer = null
        f.state.signInWithBiometrics()
        advanceUntilIdle()
        assertEquals(2, f.vault.reads)
        assertEquals("session-cl114d", f.state.session.value)
        assertFalse(f.state.loginNeeded.value)
    }

    @Test
    fun theSheetStartsTheCheckWhenItOpens() = runTest {
        val f = fixture(details = reader)
        f.state.start()
        advanceUntilIdle()
        f.state.requestLogin()
        assertNotNull(f.state.autoBiometricSignIn())
        advanceUntilIdle()
        assertEquals("session-cl114d", f.state.session.value)
        assertFalse(f.state.loginNeeded.value)
    }

    @Test
    fun theSheetChecksAgainOnceReopened() = runTest {
        val f = fixture(session = "tok", details = reader, expired = setOf("tok"))
        f.vault.readAnswer = BiometricRead.Declined
        f.state.start()
        advanceUntilIdle()
        f.state.dismissLogin()

        f.state.requestLogin()
        assertNotNull(f.state.autoBiometricSignIn())
        advanceUntilIdle()
        assertEquals(2, f.vault.reads)
    }

    @Test
    fun rejectedDetailsAreDeletedAndTheFormSaysWhy() = runTest {
        val f = fixture(session = "tok", details = reader, expired = setOf("tok"))
        f.state.start()
        runCurrent() // the library is set up; its refresh is on its way
        f.backend().loginFails = SpydusException.CredentialsRejected("credentials rejected")
        advanceUntilIdle()

        assertNull(f.vault.kept[canning.origin])
        assertFalse(f.state.biometricOn.value)
        assertNull(f.state.session.value)
        assertTrue(f.state.loginNeeded.value)
        assertEquals(UserFacingError.Kind.CREDENTIALS, f.state.loginError.value?.kind)
        assertEquals(Copy.Errors.credentialsRejected, f.state.loginError.value?.message)
    }

    @Test
    fun anotherErrorKeepsTheDetails() = runTest {
        val f = fixture(session = "tok", details = reader, expired = setOf("tok"))
        f.state.start()
        runCurrent() // the library is set up; its refresh is on its way
        f.backend().loginFails = SpydusException.Network("offline")
        advanceUntilIdle()

        assertEquals(reader, f.vault.kept[canning.origin])
        assertTrue(f.state.biometricOn.value)
        assertNull(f.state.session.value)
        assertTrue(f.state.loginNeeded.value)
        assertEquals(UserFacingError.Kind.NETWORK, f.state.loginError.value?.kind)
    }

    @Test
    fun invalidatedDetailsShowTheForm() = runTest {
        val f = fixture(session = "tok", details = reader, expired = setOf("tok"))
        f.vault.readAnswer = BiometricRead.Invalidated
        f.state.start()
        advanceUntilIdle()
        assertFalse(f.state.biometricOn.value)
        assertNull(f.state.session.value)
        assertTrue(f.state.loginNeeded.value)
    }

    @Test
    fun withoutDetailsAnExpiredSessionAsksForThePinAsBefore() = runTest {
        val f = fixture(session = "tok", expired = setOf("tok"))
        f.state.start()
        advanceUntilIdle()
        assertEquals(0, f.vault.reads)
        assertNull(f.state.session.value)
        assertTrue(f.state.loginNeeded.value)
    }

    @Test
    fun inTheBackgroundItWaitsForTheForeground() = runTest {
        val f = fixture(session = "tok", details = reader, expired = setOf("tok"), foreground = false)
        f.state.start()
        advanceUntilIdle()
        assertEquals(0, f.vault.reads)
        assertEquals("tok", f.state.session.value)
        assertFalse(f.state.loginNeeded.value)

        f.foreground.value = true
        advanceUntilIdle()
        assertEquals(1, f.vault.reads)
        assertEquals("session-cl114d", f.state.session.value)
    }

    // endregion
    // region Profile and sign-out

    @Test
    fun profileOffDeletesTheDetails() = runTest {
        val f = fixture(session = "tok", details = reader)
        f.state.start()
        advanceUntilIdle()
        assertTrue(f.state.biometricOn.value)

        f.state.setBiometricSignIn(false)
        assertNull(f.vault.kept[canning.origin])
        assertFalse(f.state.biometricOn.value)
        assertEquals(0, f.vault.reads) // no check to turn it off
    }

    @Test
    fun profileOnOpensTheSheetAndSavesWithoutAsking() = runTest {
        val f = fixture(session = "tok")
        f.state.start()
        advanceUntilIdle()
        f.offers.markDeclined(canning.origin) // even after Not now

        f.state.setBiometricSignIn(true)
        assertTrue(f.state.loginNeeded.value)
        assertTrue(f.state.biometricTurnOn.value)
        assertNull(f.state.autoBiometricSignIn())

        f.state.signIn("cl114d", "1234")
        advanceUntilIdle()
        // No question: the offer only carries the moment for the check (once the sheet has gone).
        assertEquals(false, f.state.biometricOffer.value?.asks)
        assertEquals(0, f.vault.saves)
        f.state.answerBiometricOffer(use = true)
        advanceUntilIdle()
        assertNull(f.state.biometricOffer.value)
        assertEquals(1, f.vault.saves)
        assertEquals(SignInDetails("cl114d", "1234"), f.vault.kept[canning.origin])
        assertTrue(f.state.biometricOn.value)
        assertFalse(f.state.biometricTurnOn.value)
    }

    @Test
    fun closingTheTurnOnSheetLeavesItOff() = runTest {
        val f = fixture(session = "tok")
        f.state.start()
        advanceUntilIdle()
        f.state.setBiometricSignIn(true)
        f.state.dismissLogin()
        assertFalse(f.state.biometricTurnOn.value)

        f.state.signIn("cl114d", "1234")
        advanceUntilIdle()
        assertEquals(0, f.vault.saves)
        assertEquals(true, f.state.biometricOffer.value?.asks) // the usual question instead
    }

    @Test
    fun signOutDeletesTheDetails() = runTest {
        val f = fixture(session = "tok", details = reader)
        f.state.start()
        advanceUntilIdle()
        f.offers.markDeclined("elsewhere")

        f.state.signOut()
        assertNull(f.vault.kept[canning.origin])
        assertFalse(f.state.biometricOn.value)
        assertTrue(f.offers.wasDeclined("elsewhere")) // other libraries keep theirs
    }

    // endregion
}

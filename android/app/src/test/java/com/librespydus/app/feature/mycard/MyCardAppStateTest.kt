package com.librespydus.app.feature.mycard

import com.librespydus.app.core.AppState
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.FavouritesStore
import com.librespydus.app.core.InMemoryAccountSnapshotStorage
import com.librespydus.app.core.InMemoryLibraryStore
import com.librespydus.app.core.InMemorySecretStore
import com.librespydus.app.core.LaunchOptions
import com.librespydus.app.core.SnapshotBackend
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
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
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.time.LocalDate

/**
 * AppState and the card number (§5.9 as amended): entered on the card and saved per library in the
 * secure store, replaced by an edit, deleted on sign-out, kept when the session expires; never
 * taken from sign-in (the sign-in ID may be a username); a saved number wins over the account's.
 */
class MyCardAppStateTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = LibraryInfo("canning", "City of Canning", "https://canning.spydus.com", null, "Australia", "AU", false)
    private val hutt = LibraryInfo("hutt", "Hutt City Libraries", "https://hutt.spydus.co.nz", null, "New Zealand", "NZ", false)
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private class Fixture(val state: AppState, val secrets: InMemorySecretStore, val backends: Map<String, SnapshotBackend>)

    private fun TestScope.fixture(
        library: LibraryInfo? = canning,
        sessions: Map<String, String> = mapOf(canning.origin to "tok", hutt.origin to "tok2"),
        secrets: InMemorySecretStore = InMemorySecretStore(),
        accountCardNumber: (AccountSnapshot) -> String? = { null },
    ): Fixture {
        val io = StandardTestDispatcher(testScheduler)
        sessions.forEach { (origin, token) -> secrets.put(AppState.sessionKey(origin), token) }
        val backends = mutableMapOf<String, SnapshotBackend>()
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        val state = AppState(
            scope = scope,
            libraryStore = InMemoryLibraryStore(library),
            secrets = secrets,
            favourites = FavouritesStore(File(tmp.root, "fav.json"), io = io),
            directory = { listOf(canning, hutt) },
            makeBackend = { origin -> SnapshotBackend(origin, delayMs = 100).also { backends[origin] = it } },
            loadDemoData = { origin -> DemoData.load({ File("src/main/assets/demo/$it").readText() }, origin, LocalDate.of(2026, 10, 13)) },
            io = io,
            accountStorage = InMemoryAccountSnapshotStorage(),
            accountCardNumber = accountCardNumber,
        )
        return Fixture(state, secrets, backends)
    }

    private fun TestScope.started(f: Fixture): Fixture {
        f.state.start()
        advanceUntilIdle()
        return f
    }

    @Test
    fun noNumberUntilOneIsEntered() = runTest {
        val f = started(fixture())
        assertTrue(f.state.isSignedIn.value)
        assertNull(f.state.cardNumber.value)
    }

    @Test
    fun enteringSavesTheTrimmedNumberPerLibraryInTheSecureStore() = runTest {
        val f = started(fixture())
        f.state.setCardNumber("  CL114D0001234 ")
        advanceUntilIdle()
        assertEquals("CL114D0001234", f.state.cardNumber.value)
        assertEquals("CL114D0001234", f.secrets.get("card.https://canning.spydus.com"))

        // Another library has its own (none yet); coming back shows Canning's again.
        f.state.selectLibrary(hutt)
        advanceUntilIdle()
        assertNull(f.state.cardNumber.value)
        f.state.setCardNumber("H123")
        advanceUntilIdle()
        f.state.selectLibrary(canning)
        advanceUntilIdle()
        assertEquals("CL114D0001234", f.state.cardNumber.value)
        assertEquals("H123", f.secrets.get(AppState.cardKey(hutt.origin)))
    }

    @Test
    fun aSavedNumberIsRestoredAtLaunch() = runTest {
        val secrets = InMemorySecretStore().apply { put(AppState.cardKey(canning.origin), "CL114D0009999") }
        val f = started(fixture(secrets = secrets))
        assertEquals("CL114D0009999", f.state.cardNumber.value)
    }

    @Test
    fun anEditReplacesTheNumber() = runTest {
        val f = started(fixture())
        f.state.setCardNumber("CL114D0001234")
        advanceUntilIdle()
        f.state.setCardNumber("CL114D0005678")
        advanceUntilIdle()
        assertEquals("CL114D0005678", f.state.cardNumber.value)
        assertEquals("CL114D0005678", f.secrets.get(AppState.cardKey(canning.origin)))

        // Blank (never offered by the form, Save is disabled) deletes rather than saving "".
        f.state.setCardNumber("   ")
        advanceUntilIdle()
        assertNull(f.state.cardNumber.value)
        assertNull(f.secrets.get(AppState.cardKey(canning.origin)))
    }

    @Test
    fun signInSavesNothing() = runTest {
        val f = started(fixture(sessions = emptyMap()))
        f.state.signIn(" CL114D0001234 ", "pin")
        advanceUntilIdle()
        assertTrue(f.state.isSignedIn.value)
        assertNull(f.state.cardNumber.value)
        assertNull(f.secrets.get(AppState.cardKey(canning.origin)))
    }

    @Test
    fun signOutDeletesTheNumber() = runTest {
        val f = started(fixture())
        f.state.setCardNumber("CL114D0001234")
        advanceUntilIdle()
        f.state.selectLibrary(hutt)
        advanceUntilIdle()
        f.state.setCardNumber("H123")
        advanceUntilIdle()

        f.state.signOut()
        advanceUntilIdle()
        assertNull(f.state.cardNumber.value)
        assertNull(f.secrets.get(AppState.cardKey(hutt.origin)))
        assertEquals("CL114D0001234", f.secrets.get(AppState.cardKey(canning.origin))) // other libraries keep theirs
    }

    @Test
    fun anExpiredSessionKeepsTheNumber() = runTest {
        val secrets = InMemorySecretStore().apply { put(AppState.cardKey(canning.origin), "CL114D0001234") }
        val f = fixture(secrets = secrets)
        f.state.start()
        advanceTimeBy(1)
        f.backends.getValue(canning.origin).fail = SpydusException.NotLoggedIn("not logged in")
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
        assertNull(f.secrets.get(AppState.sessionKey(canning.origin)))
        assertEquals("CL114D0001234", f.state.cardNumber.value)
        assertEquals("CL114D0001234", f.secrets.get(AppState.cardKey(canning.origin)))
    }

    @Test
    fun theAccountsNumberShowsUntilOneIsSavedWhichThenWins() = runTest {
        val f = started(fixture(accountCardNumber = { snapshot -> snapshot.accountName?.let { "ACC-$it" } }))
        assertEquals("ACC-Reader for tok", f.state.cardNumber.value)

        f.state.setCardNumber("CL114D0001234")
        advanceUntilIdle()
        assertEquals("CL114D0001234", f.state.cardNumber.value)

        f.state.setCardNumber(null) // edited away: back to the account's
        advanceUntilIdle()
        assertEquals("ACC-Reader for tok", f.state.cardNumber.value)
    }

    @Test
    fun theAccountGivesNoNumberYet() = runTest {
        // The default stub: the core doesn't read Personal details yet.
        val f = started(fixture())
        assertTrue(f.state.account.snapshot.value != null)
        assertNull(f.state.cardNumber.value)
    }

    @Test
    fun demoModeHasItsNumberInMemoryOnly() = runTest {
        val f = started(fixture(library = null, sessions = emptyMap()))
        f.state.applyLaunchOptions(LaunchOptions(origin = canning.origin, demo = true))
        advanceUntilIdle()
        assertTrue(f.state.backend.value?.isDemo == true)
        assertEquals(DemoBackend.CARD_NUMBER, f.state.cardNumber.value)
        assertEquals("DEMO0012345", f.state.cardNumber.value)

        f.state.setCardNumber("CL114D0001234")
        advanceUntilIdle()
        assertEquals("CL114D0001234", f.state.cardNumber.value)
        assertNull(f.secrets.get(AppState.cardKey(canning.origin)))

        // Signed out, the demo number is gone too; signing in again doesn't bring it back.
        f.state.signOut()
        advanceUntilIdle()
        assertNull(f.state.cardNumber.value)
        f.state.signIn("demo", "demo")
        advanceUntilIdle()
        assertNull(f.state.cardNumber.value)
        assertNull(f.secrets.get(AppState.cardKey(canning.origin)))
    }
}

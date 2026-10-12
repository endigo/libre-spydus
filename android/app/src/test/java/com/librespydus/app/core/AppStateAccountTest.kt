package com.librespydus.app.core

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
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
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.time.LocalDate

/** AppState and the saved account (§5.2a): when it is restored, refreshed, saved and deleted. */
class AppStateAccountTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = LibraryInfo("canning", "City of Canning", "https://canning.spydus.com", null, "Australia", "AU", false)
    private val hutt = LibraryInfo("hutt", "Hutt City Libraries", "https://hutt.spydus.co.nz", null, "New Zealand", "NZ", false)
    private val scopes = mutableListOf<CoroutineScope>()
    private var clock = 1_800_000_000_000L

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private class Fixture(
        val state: AppState,
        val storage: InMemoryAccountSnapshotStorage,
        val secrets: InMemorySecretStore,
        val backends: Map<String, SnapshotBackend>,
    )

    private fun TestScope.fixture(
        library: LibraryInfo? = canning,
        sessions: Map<String, String> = mapOf(canning.origin to "tok"),
        storage: InMemoryAccountSnapshotStorage = InMemoryAccountSnapshotStorage(),
        wrapStorage: (InMemoryAccountSnapshotStorage) -> AccountSnapshotStorage = { it },
    ): Fixture {
        val io = StandardTestDispatcher(testScheduler)
        val secrets = InMemorySecretStore().apply { sessions.forEach { (origin, token) -> put(AppState.sessionKey(origin), token) } }
        val backends = mutableMapOf<String, SnapshotBackend>()
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        val state = AppState(
            scope = scope,
            libraryStore = InMemoryLibraryStore(library),
            secrets = secrets,
            favourites = FavouritesStore(File(tmp.root, "fav.json"), io = io),
            directory = { listOf(canning, hutt) },
            makeBackend = { origin -> SnapshotBackend(origin, delayMs = 1_000, clock = { clock }).also { backends[origin] = it } },
            loadDemoData = { origin -> DemoData.load({ File("src/main/assets/demo/$it").readText() }, origin, LocalDate.of(2026, 10, 12)) },
            io = io,
            accountStorage = wrapStorage(storage),
            now = { clock },
        )
        return Fixture(state, storage, secrets, backends)
    }

    @Test
    fun launchShowsTheSavedAccountAtOnceThenRefreshesIt() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply { save(canning.origin, accountSnapshot("Saved Reader", fetchedAtMs = 7)) }
        val f = fixture(storage = storage)
        f.state.start()
        advanceTimeBy(500) // restored, the refresh still running
        assertTrue(f.state.ready.value)
        assertEquals("Saved Reader", f.state.account.snapshot.value?.accountName)
        assertEquals("Saved Reader", f.state.accountName.value)
        assertTrue(f.state.account.isRefreshing.value)

        advanceUntilIdle()
        assertEquals(1, f.backends.getValue(canning.origin).calls)
        assertEquals("Reader for tok", f.state.account.snapshot.value?.accountName)
        assertEquals("Reader for tok", f.state.accountName.value)
        assertEquals(clock, storage.load(canning.origin)?.fetchedAtMs)
    }

    @Test
    fun signedOutLaunchesFetchNothing() = runTest {
        val f = fixture(sessions = emptyMap())
        f.state.start()
        advanceUntilIdle()
        assertEquals(0, f.backends.getValue(canning.origin).calls)
        assertNull(f.state.account.snapshot.value)
    }

    @Test
    fun signInRefreshesAtOnceAndSignOutDeletesTheSavedAccount() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply {
            save(canning.origin, accountSnapshot("Someone Else")) // left behind somehow: never shown
            save(hutt.origin, accountSnapshot("Hutt Reader"))
        }
        val f = fixture(sessions = emptyMap(), storage = storage)
        f.state.start()
        advanceUntilIdle()

        f.state.signIn("1234", "pin")
        assertNull(f.state.account.snapshot.value)
        assertNull(storage.load(canning.origin))
        assertTrue(f.state.account.isRefreshing.value)
        advanceUntilIdle()
        assertEquals("Reader for session-1234", f.state.account.snapshot.value?.accountName)
        assertNotNull(storage.load(canning.origin))

        f.state.signOut()
        assertNull(f.state.account.snapshot.value)
        assertNull(f.state.accountName.value)
        assertNull(storage.load(canning.origin))
        assertEquals("Hutt Reader", storage.load(hutt.origin)?.accountName) // other libraries keep theirs
    }

    @Test
    fun anExpiredSessionDeletesTheSavedAccount() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply { save(canning.origin, accountSnapshot("Saved Reader")) }
        val f = fixture(storage = storage)
        f.state.start()
        advanceTimeBy(1)
        f.backends.getValue(canning.origin).fail = SpydusException.NotLoggedIn("not logged in")
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
        assertNull(f.state.account.snapshot.value)
        assertTrue(storage.keys().isEmpty())
        assertNull(f.secrets.get(AppState.sessionKey(canning.origin)))
    }

    @Test
    fun eachLibraryShowsItsOwnSavedAccount() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply {
            save(canning.origin, accountSnapshot("Canning Reader"))
            save(hutt.origin, accountSnapshot("Hutt Reader"))
        }
        val f = fixture(sessions = mapOf(canning.origin to "tok", hutt.origin to "hutt-tok"), storage = storage)
        f.state.start()
        advanceTimeBy(1)
        assertEquals("Canning Reader", f.state.account.snapshot.value?.accountName)

        f.state.selectLibrary(hutt)
        // Switched mid-refresh: Hutt shows its own saved account, never Canning's fresh one.
        assertEquals("Hutt Reader", f.state.account.snapshot.value?.accountName)
        advanceUntilIdle()
        assertEquals("Reader for hutt-tok", f.state.account.snapshot.value?.accountName)
        assertEquals("Canning Reader", storage.load(canning.origin)?.accountName)
        assertEquals(setOf(canning.origin, hutt.origin), storage.keys())
    }

    @Test
    fun demoModeNeverSavesTheAccount() = runTest {
        val f = fixture(library = null, sessions = emptyMap())
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = hutt.origin, demo = true))
        advanceUntilIdle()
        assertEquals("Test Borrower", f.state.account.snapshot.value?.accountName)
        assertEquals(4, f.state.account.snapshot.value?.loans?.items?.size)
        assertTrue(f.storage.keys().isEmpty())

        f.state.signOut()
        f.state.signIn("demo", "demo")
        advanceUntilIdle()
        assertNotNull(f.state.account.snapshot.value)
        assertTrue(f.storage.keys().isEmpty())
    }

    @Test
    fun theDebugDemoPersistOptionSavesUnderItsOwnKey() = runTest {
        val f = fixture(library = null, sessions = emptyMap())
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = hutt.origin, demo = true, demoPersist = true))
        advanceUntilIdle()
        assertEquals(setOf("demo:${hutt.origin}"), f.storage.keys())
        assertEquals("Test Borrower", f.storage.load("demo:${hutt.origin}")?.accountName)
    }

    @Test
    fun theDebugRefreshFailureKeepsTheSavedDemoAccount() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply { save("demo:${hutt.origin}", accountSnapshot("Saved Demo")) }
        val f = fixture(library = null, sessions = emptyMap(), storage = storage)
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = hutt.origin, demo = true, demoPersist = true, demoRefreshFails = true))
        advanceUntilIdle()
        assertEquals("Saved Demo", f.state.account.snapshot.value?.accountName)
        assertEquals(UserFacingError.Kind.NETWORK, f.state.account.lastRefreshError.value?.kind)
        assertFalse(f.state.loginNeeded.value)
    }

    @Test
    fun foregroundRefreshesOnlyAStaleAccount() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        val backend = f.backends.getValue(canning.origin)
        assertEquals(1, backend.calls)

        clock += 60_000
        f.state.onForeground()
        advanceUntilIdle()
        assertEquals(1, backend.calls)

        clock += AccountStore.STALE_AFTER_MS
        f.state.onForeground()
        advanceUntilIdle()
        assertEquals(2, backend.calls)
    }

    @Test
    fun aReservationChangeRefreshesTheAccount() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        f.state.accountDidChange() // e.g. a reservation placed on Book detail
        advanceUntilIdle()
        assertEquals(2, f.backends.getValue(canning.origin).calls)
    }

    @Test
    fun aVerificationRetriesTheFailedRefresh() = runTest {
        val f = fixture()
        val backend = { f.backends.getValue(canning.origin) }
        f.state.start()
        advanceTimeBy(1)
        backend().fail = SpydusException.HumanVerificationRequired("human verification required")
        advanceUntilIdle()
        assertTrue(f.state.verificationNeeded.value)
        assertEquals(UserFacingError.Kind.VERIFICATION, f.state.account.lastRefreshError.value?.kind)

        backend().fail = null
        f.state.completeVerification("aws-waf-token=abc")
        advanceUntilIdle()
        assertEquals(2, backend().calls)
        assertNull(f.state.account.lastRefreshError.value)
        assertNotNull(f.state.account.snapshot.value)
    }

    @Test
    fun resetDeletesEverySavedAccount() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply {
            save(canning.origin, accountSnapshot())
            save(hutt.origin, accountSnapshot())
        }
        val f = fixture(storage = storage)
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(reset = true))
        advanceUntilIdle()
        assertTrue(storage.keys().isEmpty())
        assertNull(f.state.account.snapshot.value)
    }

    @Test
    fun aSessionExpiringWhileTheNextLibraryLoadsDeletesTheRightFile() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply {
            save(canning.origin, accountSnapshot("Canning Reader"))
            save(hutt.origin, accountSnapshot("Hutt Reader"))
        }
        lateinit var state: AppState
        var expireCanning = false
        // Canning's session turns out to have expired just as Hutt's saved account is being read.
        val f = fixture(sessions = mapOf(canning.origin to "tok", hutt.origin to "hutt-tok"), storage = storage) { inner ->
            object : AccountSnapshotStorage by inner {
                override fun load(key: String) = inner.load(key).also {
                    if (key == hutt.origin && expireCanning) state.report(SpydusException.NotLoggedIn("not logged in"))
                }
            }
        }
        state = f.state
        f.state.start()
        advanceUntilIdle()

        expireCanning = true
        f.state.selectLibrary(hutt)
        assertNull(storage.load(canning.origin))
        assertNull(f.secrets.get(AppState.sessionKey(canning.origin)))
        assertEquals(hutt, f.state.library.value)
        assertEquals("hutt-tok", f.state.session.value)
        assertEquals("Hutt Reader", f.state.account.snapshot.value?.accountName)
        advanceUntilIdle()
        assertEquals(1, f.backends.getValue(hutt.origin).calls)
        assertEquals("Reader for hutt-tok", f.state.account.snapshot.value?.accountName)
        assertEquals("Reader for hutt-tok", storage.load(hutt.origin)?.accountName)
    }

    @Test
    fun aSignInRacingALibrarySwitchLeavesTheStoreOnTheShownLibrary() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply { save(hutt.origin, accountSnapshot("Hutt Reader")) }
        val f = fixture(sessions = mapOf(hutt.origin to "hutt-tok"), storage = storage)
        f.state.start()
        advanceUntilIdle()

        val signIn = launch { f.state.signIn("1234", "pin") } // at Canning
        val switch = launch { f.state.selectLibrary(hutt) }
        advanceUntilIdle()
        signIn.join()
        switch.join()

        assertEquals(hutt, f.state.library.value)
        assertEquals("hutt-tok", f.state.session.value)
        assertEquals("Reader for hutt-tok", f.state.account.snapshot.value?.accountName)
        assertEquals("Reader for hutt-tok", storage.load(hutt.origin)?.accountName)
        // Canning's account never reached Hutt's file, nor Hutt's Canning's.
        assertTrue(storage.load(canning.origin)?.accountName.let { it == null || it == "Reader for session-1234" })
    }
}

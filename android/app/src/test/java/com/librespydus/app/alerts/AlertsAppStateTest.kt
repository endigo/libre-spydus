package com.librespydus.app.alerts

import com.librespydus.app.core.Alerts
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.FavouritesStore
import com.librespydus.app.core.InMemoryAccountSnapshotStorage
import com.librespydus.app.core.InMemoryAlertStore
import com.librespydus.app.core.InMemoryLibraryStore
import com.librespydus.app.core.InMemorySecretStore
import com.librespydus.app.core.LaunchOptions
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.Tab
import com.librespydus.app.core.UserFacingError
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.time.LocalDate

/** AppState and the alerts (§5.8): every fresh snapshot is processed; sign-out forgets; the morning check. */
class AlertsAppStateTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = LibraryInfo("canning", "City of Canning", "https://canning.spydus.com", null, "Australia", "AU", false)
    private val today = LocalDate.of(2026, 10, 13)
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    /** A signed-in library whose next `accountSnapshot` is [next] (or [fail]). */
    private class AccountBackend(origin: String) : LibraryBackend by FakeBackend(origin) {
        var next: AccountSnapshot = snapshot()
        var fail: Exception? = null
        var calls = 0

        override suspend fun accountSnapshot(session: String): AccountSnapshot {
            calls++
            delay(100)
            fail?.let { throw it }
            return next
        }
    }

    private class Fixture(
        val state: AppState,
        val store: InMemoryAlertStore,
        val notifier: RecordingNotifier,
        val scheduler: RecordingScheduler,
        val backend: AccountBackend,
        val storage: InMemoryAccountSnapshotStorage,
    )

    private fun TestScope.fixture(
        library: LibraryInfo? = canning,
        session: String? = "tok",
        storage: InMemoryAccountSnapshotStorage = InMemoryAccountSnapshotStorage(),
        store: InMemoryAlertStore = InMemoryAlertStore(),
    ): Fixture {
        val io = StandardTestDispatcher(testScheduler)
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        val notifier = RecordingNotifier()
        val scheduler = RecordingScheduler()
        val backend = AccountBackend(canning.origin)
        val state = AppState(
            scope = scope,
            libraryStore = InMemoryLibraryStore(library),
            secrets = InMemorySecretStore().apply { session?.let { put(AppState.sessionKey(canning.origin), it) } },
            favourites = FavouritesStore(File(tmp.root, "fav.json"), io = io),
            directory = { listOf(canning) },
            makeBackend = { backend },
            loadDemoData = { origin -> DemoData.load({ File("src/main/assets/demo/$it").readText() }, origin, today) },
            io = io,
            accountStorage = storage,
            alerts = Alerts(store, notifier, scheduler, today = { today }, io = io),
        )
        return Fixture(state, store, notifier, scheduler, backend, storage)
    }

    private val dune = item("19000001", "Dune", mapOf("Pickup location" to "RIVERTON LIBRARY"))
    private val emma = item("19000002", "Emma", mapOf("Status" to "Please pickup from: CANNINGTON LIBRARY by 20 Oct 2026"))

    @Test
    fun aForegroundRefreshPostsWhatBecameReady() = runTest {
        val f = fixture()
        f.backend.next = snapshot(ready = listOf(dune))
        f.state.start()
        advanceUntilIdle()
        f.state.setReadyAlerts(true)
        advanceUntilIdle()
        assertEquals(listOf("19000001"), f.store.readyKeys(Alerts.originHash(canning.origin)))
        assertTrue(f.notifier.posted.isEmpty())

        f.backend.next = snapshot(ready = listOf(dune, emma))
        f.state.account.refresh(force = true)
        advanceUntilIdle()
        val posted = f.notifier.posted.single()
        assertEquals("ready:${Alerts.originHash(canning.origin)}:19000002", posted.id)
        assertEquals("Emma · Cannington Library", posted.body)
    }

    @Test
    fun theDemoAccountsNewReadyReservationIsAlertedOnItsNextRefresh() = runTest {
        val f = fixture(library = null, session = null)
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = canning.origin, demo = true, demoNewReady = true))
        advanceUntilIdle()
        f.state.setReadyAlerts(true) // seeds "Tomorrow, and tomorrow, and tomorrow"
        advanceUntilIdle()
        assertTrue(f.notifier.posted.isEmpty())

        f.state.account.refresh(force = true)
        advanceUntilIdle()
        val posted = f.notifier.posted.single()
        assertEquals("ready:${Alerts.originHash("demo:${canning.origin}")}:19000002", posted.id)
        assertEquals("Ready to collect", posted.title)
        assertEquals("Baby-sitters club. 18, Boy-crazy Stacey · Riverton Library", posted.body)
        assertEquals(2, f.state.account.snapshot.value?.reservations?.ready?.items?.size)
        assertTrue(f.storage.keys().isEmpty()) // demo data still isn't saved
    }

    @Test
    fun theMorningCheckRefreshesThenPostsTodaysDueReminders() = runTest {
        val f = fixture(library = null, session = null)
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = canning.origin, demo = true, demoNewReady = true))
        advanceUntilIdle()
        f.state.setReadyAlerts(true)
        f.state.setDueReminders(true)
        advanceUntilIdle()
        assertEquals("ensure", f.scheduler.calls.last())

        launch { f.state.morningCheck() }
        advanceUntilIdle()
        assertEquals(
            listOf("Ready to collect" to "Baby-sitters club. 18, Boy-crazy Stacey · Riverton Library", "Due Thursday 15 Oct" to "Project Hail Mary"),
            f.notifier.posted.map { it.title to it.body },
        )
        f.state.scheduleNextMorningCheck()
        assertEquals("next", f.scheduler.calls.last())

        // Again the same morning: nothing new.
        launch { f.state.morningCheck() }
        advanceUntilIdle()
        assertEquals(2, f.notifier.posted.size)
    }

    @Test
    fun aFailedMorningCheckStillPostsTheDueRemindersFromTheSavedAccount() = runTest {
        val storage = InMemoryAccountSnapshotStorage().apply {
            save(canning.origin, snapshot(loans = listOf(loan("1", "Project Hail Mary", "15 Oct 2026"))))
        }
        val store = InMemoryAlertStore().apply { dueEnabled = true }
        val f = fixture(storage = storage, store = store)
        f.backend.fail = SpydusException.Network("offline")
        f.state.start()
        launch { f.state.morningCheck() }
        advanceUntilIdle()
        assertEquals(1, f.backend.calls) // joined the refresh the launch started
        assertEquals(UserFacingError.Kind.NETWORK, f.state.account.lastRefreshError.value?.kind)
        assertEquals(listOf("Due Thursday 15 Oct"), f.notifier.posted.map { it.title })
    }

    @Test
    fun noMorningCheckWithTheSwitchesOff() = runTest {
        val f = fixture()
        f.backend.next = snapshot(loans = listOf(loan("1", "Project Hail Mary", "15 Oct 2026")), ready = listOf(dune))
        f.state.start()
        advanceUntilIdle()
        assertEquals("cancel", f.scheduler.calls.last())
        launch { f.state.morningCheck() }
        advanceUntilIdle()
        assertEquals(1, f.backend.calls) // just the launch's own refresh
        assertTrue(f.notifier.posted.isEmpty())
        f.state.scheduleNextMorningCheck()
        assertEquals("cancel", f.scheduler.calls.last())
    }

    @Test
    fun signingOutForgetsTheLibrarysAlertsAndCancelsTheCheck() = runTest {
        val f = fixture()
        f.backend.next = snapshot(ready = listOf(dune))
        f.state.start()
        advanceUntilIdle()
        f.state.setReadyAlerts(true)
        advanceUntilIdle()
        assertEquals("ensure", f.scheduler.calls.last())

        f.state.signOut()
        advanceUntilIdle()
        val hash = Alerts.originHash(canning.origin)
        assertNull(f.store.readyKeys(hash))
        assertEquals(listOf(hash), f.notifier.removed)
        assertEquals("cancel", f.scheduler.calls.last())
        assertTrue(f.state.alerts.readyEnabled.value) // app-wide: still on for the next sign-in
    }

    @Test
    fun anExpiredSessionForgetsTheLibrarysAlerts() = runTest {
        val f = fixture()
        f.backend.next = snapshot(ready = listOf(dune))
        f.state.start()
        advanceUntilIdle()
        f.state.setReadyAlerts(true)
        advanceUntilIdle()

        f.backend.fail = SpydusException.NotLoggedIn("not logged in")
        f.state.account.refresh(force = true)
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertNull(f.store.readyKeys(Alerts.originHash(canning.origin)))
        assertEquals("cancel", f.scheduler.calls.last())
    }

    @Test
    fun theMorningCheckHookRunsOnceTheLaunchRefreshHasSettled() = runTest {
        val f = fixture(library = null, session = null)
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = canning.origin, demo = true, morningCheckNow = true))
        advanceUntilIdle()
        assertEquals("now", f.scheduler.calls.last())
        assertFalse(f.state.account.isRefreshing.value)
    }

    @Test
    fun aTappedAlertOpensYourBooksOnItsSegment() = runTest {
        val f = fixture()
        f.state.start()
        f.state.openFromAlert(BorrowedSegment.RESERVATIONS)
        advanceUntilIdle()
        assertEquals(Tab.BORROWED, f.state.selectedTab.value)
        assertEquals(BorrowedSegment.RESERVATIONS, f.state.consumePendingBorrowedSegment())
    }

    @Test
    fun theDebugResetTurnsTheAlertsOff() = runTest {
        val store = InMemoryAlertStore().apply {
            readyEnabled = true
            dueEnabled = true
        }
        val f = fixture(store = store)
        f.state.start()
        advanceUntilIdle()
        f.state.applyLaunchOptions(LaunchOptions(reset = true))
        advanceUntilIdle()
        assertFalse(f.state.alerts.readyEnabled.value)
        assertFalse(store.dueEnabled)
    }
}

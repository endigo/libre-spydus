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
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.SpydusException

/** AccountStore: restore, background refresh (coalesced, dropped when stale), save, discard. */
class AccountStoreTest {
    private val scopes = mutableListOf<CoroutineScope>()
    private var clock = 1_800_000_000_000L
    private val reported = mutableListOf<Throwable>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private fun TestScope.store(storage: AccountSnapshotStorage = InMemoryAccountSnapshotStorage()): AccountStore {
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        return AccountStore(
            scope = scope,
            storage = storage,
            report = { e -> reported += e; UserFacingError.from(e, "City of Canning") },
            now = { clock },
            io = StandardTestDispatcher(testScheduler),
        )
    }

    private fun backend(origin: String = "https://canning.spydus.com") = SnapshotBackend(origin, clock = { clock })

    /** What AppState does on launch / library switch: read the saved copy, then bind with it. */
    private suspend fun AccountStore.restore(backend: LibraryBackend, session: String, cacheKey: String) =
        bind(backend, session, cacheKey, loadSaved(cacheKey))

    @Test
    fun concurrentRefreshesShareOneRequest() = runTest {
        val store = store()
        val backend = backend()
        store.bind(backend, "tok", cacheKey = null)

        val first = store.refresh(force = true)
        assertNotNull(first)
        assertTrue(store.isRefreshing.value)
        assertSame(first, store.refresh(force = true))
        assertSame(first, store.refresh())
        advanceUntilIdle()

        assertEquals(1, backend.calls)
        assertEquals("Reader for tok", store.snapshot.value?.accountName)
        assertFalse(store.isRefreshing.value)
        assertNull(store.lastRefreshError.value)
    }

    @Test
    fun aResultForAnEarlierBindingIsDropped() = runTest {
        val store = store()
        val canning = backend()
        val hutt = backend("https://hutt.spydus.co.nz")
        store.bind(canning, "tok", cacheKey = null)
        val stale = store.refresh(force = true)!!
        advanceTimeBy(50)

        // Library switch mid-refresh: Hutt never shows Canning's account.
        store.bind(hutt, "hutt-tok", cacheKey = null)
        assertTrue(stale.isCancelled)
        assertFalse(store.isRefreshing.value)
        advanceUntilIdle()
        assertNull(store.snapshot.value)

        store.refresh(force = true)
        advanceUntilIdle()
        assertEquals("Reader for hutt-tok", store.snapshot.value?.accountName)

        // Signed in again (a new session) mid-refresh: the old session's result is dropped too.
        store.refresh(force = true)
        advanceTimeBy(50)
        store.bind(hutt, "new-tok", cacheKey = null)
        store.refresh(force = true)
        advanceUntilIdle()
        assertEquals("Reader for new-tok", store.snapshot.value?.accountName)
    }

    @Test
    fun aFailedRefreshKeepsTheSnapshotAndRoutesTheError() = runTest {
        val store = store()
        val backend = backend()
        store.bind(backend, "tok", cacheKey = null)
        store.refresh(force = true)
        advanceUntilIdle()
        val saved = store.snapshot.value!!

        val offline = SpydusException.Network("network error: connect timed out after 10s")
        backend.fail = offline
        clock += 5 * 60_000
        store.refresh()
        advanceUntilIdle()
        assertSame(saved, store.snapshot.value)
        assertEquals(UserFacingError.Kind.NETWORK, store.lastRefreshError.value?.kind)
        assertEquals(listOf<Throwable>(offline), reported)

        backend.fail = null
        store.refresh(force = true)
        advanceUntilIdle()
        assertNull(store.lastRefreshError.value)
        assertEquals(clock, store.snapshot.value?.fetchedAtMs)
    }

    @Test
    fun automaticRefreshesWaitTwoMinutes() = runTest {
        val store = store()
        val backend = backend()
        store.bind(backend, "tok", cacheKey = null)
        assertTrue(store.isStale()) // nothing yet
        store.refresh()
        advanceUntilIdle()
        assertEquals(1, backend.calls)
        val fetchedAt = clock

        assertFalse(store.isStale(fetchedAt + AccountStore.STALE_AFTER_MS - 1))
        assertTrue(store.isStale(fetchedAt + AccountStore.STALE_AFTER_MS))
        assertTrue(store.isStale(fetchedAt - 1)) // fetched "in the future": the clock moved back

        clock += 90_000
        assertNull(store.refresh()) // fresh: opening a screen or coming back does nothing
        assertEquals(1, backend.calls)
        assertNotNull(store.refresh(force = true)) // pull to refresh always runs
        advanceUntilIdle()
        assertEquals(2, backend.calls)

        clock += AccountStore.STALE_AFTER_MS
        assertTrue(store.isDue())
        backend.fail = SpydusException.Network("offline")
        store.refresh()
        advanceUntilIdle()
        assertEquals(3, backend.calls)
        // Still stale, but it just failed: no automatic retry for two minutes (Retry still works).
        clock += 60_000
        assertTrue(store.isStale())
        assertFalse(store.isDue())
        assertNull(store.refresh())
        clock += 60_000
        assertTrue(store.isDue())
    }

    @Test
    fun bindRestoresTheSavedSnapshotAndRefreshesSaveIt() = runTest {
        val storage = InMemoryAccountSnapshotStorage()
        storage.save("https://canning.spydus.com", accountSnapshot("Saved Reader", fetchedAtMs = 42))
        val store = store(storage)
        val backend = backend()

        store.restore(backend, "tok", cacheKey = "https://canning.spydus.com")
        assertEquals("Saved Reader", store.snapshot.value?.accountName) // before any refresh
        assertEquals(0, backend.calls)

        store.refresh()
        advanceUntilIdle()
        assertEquals("Reader for tok", store.snapshot.value?.accountName)
        assertEquals("Reader for tok", storage.load("https://canning.spydus.com")?.accountName)
    }

    @Test
    fun withoutACacheKeyNothingTouchesTheDisk() = runTest {
        val storage = InMemoryAccountSnapshotStorage()
        storage.save("https://canning.spydus.com", accountSnapshot("Saved Reader"))
        val store = store(storage)
        store.bind(backend(), DemoBackend.SESSION, cacheKey = null) // demo mode
        assertNull(store.snapshot.value)
        store.refresh()
        advanceUntilIdle()
        assertNotNull(store.snapshot.value)
        store.discard().join()
        assertEquals(setOf("https://canning.spydus.com"), storage.keys())
        assertEquals("Saved Reader", storage.load("https://canning.spydus.com")?.accountName)
    }

    @Test
    fun discardDeletesOnlyThisLibrarysCopy() = runTest {
        val storage = InMemoryAccountSnapshotStorage()
        storage.save("https://canning.spydus.com", accountSnapshot("Canning Reader"))
        storage.save("https://hutt.spydus.co.nz", accountSnapshot("Hutt Reader"))
        val store = store(storage)
        store.restore(backend(), "tok", cacheKey = "https://canning.spydus.com")
        store.refresh()
        store.discard().join()
        advanceUntilIdle()
        assertNull(store.snapshot.value)
        assertFalse(store.isRefreshing.value)
        assertEquals(setOf("https://hutt.spydus.co.nz"), storage.keys())
        assertNull(store.refresh(force = true)) // signed out
    }

    @Test
    fun aSignOutDuringASaveLeavesNoFile() = runTest {
        val storage = InMemoryAccountSnapshotStorage()
        lateinit var store: AccountStore
        // The sign-out lands while the fresh snapshot is being written.
        val hooked = object : AccountSnapshotStorage by storage {
            override fun save(key: String, snapshot: uniffi.librespydus_core.AccountSnapshot) {
                storage.save(key, snapshot)
                store.discard()
            }
        }
        store = store(hooked)
        store.bind(backend(), "tok", cacheKey = "https://canning.spydus.com")
        store.refresh()
        advanceUntilIdle()
        assertNull(store.snapshot.value)
        assertTrue(storage.keys().isEmpty())
    }

    @Test
    fun deletingEverythingWaitsForASaveUnderWay() = runTest {
        val storage = InMemoryAccountSnapshotStorage()
        lateinit var store: AccountStore
        lateinit var reset: kotlinx.coroutines.Job
        // `ls_reset` arrives while the fresh snapshot is being written.
        val hooked = object : AccountSnapshotStorage by storage {
            override fun save(key: String, snapshot: uniffi.librespydus_core.AccountSnapshot) {
                reset = scopes.first().launch { store.deleteAllSaved() }
                storage.save(key, snapshot)
            }
        }
        store = store(hooked)
        store.bind(backend(), "tok", cacheKey = "https://canning.spydus.com")
        store.refresh()
        advanceUntilIdle()
        assertTrue(reset.isCompleted)
        assertTrue(storage.keys().isEmpty())
    }

    @Test
    fun aBindWithoutASessionShowsNothing() = runTest {
        val store = store()
        store.bind(backend(), null, cacheKey = "https://canning.spydus.com", saved = accountSnapshot())
        assertNull(store.snapshot.value)
        assertNull(store.refresh(force = true))
    }

    @Test
    fun aChangeReplacesTheRunningRefresh() = runTest {
        val store = store()
        val backend = backend()
        store.bind(backend, "tok", cacheKey = null)
        val before = store.refresh(force = true)!!
        advanceTimeBy(50)
        val after = store.refreshAfterChange()!!
        assertNotSame(before, after)
        assertTrue(before.isCancelled)
        assertTrue(store.isRefreshing.value)
        advanceUntilIdle()
        assertEquals(2, backend.calls)
        assertNotNull(store.snapshot.value)
        assertFalse(store.isRefreshing.value)
    }

    @Test
    fun aCancelledReservationLeavesTheSnapshotAtOnce() = runTest {
        val storage = InMemoryAccountSnapshotStorage()
        storage.save("https://canning.spydus.com", accountSnapshot())
        val store = store(storage)
        store.restore(backend(), "tok", cacheKey = "https://canning.spydus.com")
        assertEquals(1, store.snapshot.value?.reservations?.pending?.items?.size)

        store.removeReservation("https://canning.spydus.com/other")
        assertEquals(1, store.snapshot.value?.reservations?.pending?.items?.size)

        store.removeReservation(CANCEL_HREF)
        val pending = store.snapshot.value!!.reservations.pending
        assertTrue(pending.items.isEmpty())
        assertEquals(0L, pending.total)
        assertEquals(1_000L, store.snapshot.value?.fetchedAtMs) // still as old as it was
        advanceUntilIdle()
        assertTrue(storage.load("https://canning.spydus.com")!!.reservations.pending.items.isEmpty())
    }
}

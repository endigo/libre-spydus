package com.librespydus.app.picks

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.InMemoryPicksOfferStore
import com.librespydus.app.core.PicksList
import com.librespydus.app.core.PicksStore
import com.librespydus.app.core.PicksText
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.favourites.MyPicksContent
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.SpydusException

class PicksStoreTest {
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private class Fixture(val store: PicksStore, val backend: PicksBackend, val reported: MutableList<Throwable>, val offers: InMemoryPicksOfferStore)

    private fun TestScope.fixture(irns: List<String> = listOf("1", "2", "3"), bind: Boolean = true): Fixture {
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        val reported = mutableListOf<Throwable>()
        val offers = InMemoryPicksOfferStore()
        val store = PicksStore(scope, offers, report = { reported += it; UserFacingError.from(it) }, now = { testScheduler.currentTime })
        val backend = PicksBackend(session = "tok").apply { irns.forEach { list += pickRecord(it) } }
        if (bind) store.bind(backend, "tok")
        return Fixture(store, backend, reported, offers)
    }

    // region Loading

    @Test
    fun reloadLoadsTheSetFromAtMostTenPages() = runTest {
        val f = fixture(irns = (1..250).map { "$it" })
        f.store.reload()
        advanceUntilIdle()
        assertEquals(10, f.backend.pageRequests.size)
        assertEquals((1..200).map { "$it" }.toSet(), f.store.irns.value)
        val list = f.store.list.value
        assertEquals(PicksList.Phase.Loaded, list.phase)
        assertEquals(200, list.records.size)
        assertEquals("https://canning.spydus.com/picks?page=10", list.nextPageUrl)
        assertFalse(list.loadingMore)

        // Past the tenth page the list pages as it scrolls, and the set follows.
        f.store.loadMore()
        advanceUntilIdle()
        assertEquals(220, f.store.list.value.records.size)
        assertTrue("220" in f.store.irns.value)
    }

    @Test
    fun theFirstPageShowsWhileTheRestLoad() = runTest {
        val f = fixture(irns = (1..45).map { "$it" })
        f.store.reload()
        advanceTimeBy(150)
        val list = f.store.list.value
        assertEquals(PicksList.Phase.Loaded, list.phase)
        assertEquals(20, list.records.size)
        assertTrue(list.loadingMore)
        assertTrue(MyPicksContent.of(list).let { it is MyPicksContent.Rows && it.footer is MyPicksContent.Footer.More })
        advanceUntilIdle()
        assertEquals(45, f.store.list.value.records.size)
        assertEquals(MyPicksContent.Footer.None, (MyPicksContent.of(f.store.list.value) as MyPicksContent.Rows).footer)
    }

    @Test
    fun anEmptyListIsNoPicksYet() = runTest {
        val f = fixture(irns = emptyList())
        f.store.reload()
        advanceUntilIdle()
        assertEquals(PicksList.Phase.Loaded, f.store.list.value.phase)
        assertEquals(MyPicksContent.Empty, MyPicksContent.of(f.store.list.value))
        assertTrue(f.store.irns.value.isEmpty())
    }

    @Test
    fun anUnrecognisedPageOrAFailureShowsAnErrorWithRetry() = runTest {
        val f = fixture()
        f.backend.unrecognised = true
        f.store.reload()
        advanceUntilIdle()
        val failed = MyPicksContent.of(f.store.list.value) as MyPicksContent.Failed
        assertEquals(UserFacingError.Kind.UNEXPECTED, failed.error.kind)

        f.backend.unrecognised = false
        f.backend.pageFails = SpydusException.Network("offline")
        f.store.reload()
        advanceUntilIdle()
        assertEquals(UserFacingError.Kind.NETWORK, (MyPicksContent.of(f.store.list.value) as MyPicksContent.Failed).error.kind)

        // Favourites coming into view retries a failed list.
        f.store.refreshIfStale()
        advanceUntilIdle()
        assertEquals(3, f.store.list.value.records.size)
    }

    @Test
    fun aFailedRefreshKeepsTheListAndSaysWhy() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()
        f.backend.pageFails = SpydusException.Network("offline")
        f.store.reload()
        assertTrue(f.store.list.value.refreshing)
        advanceUntilIdle()
        assertEquals(3, f.store.list.value.records.size)
        assertFalse(f.store.list.value.refreshing)
        assertTrue(f.store.notice.value!!.isError)
    }

    @Test
    fun theListNeedsASessionAndAnExpiredOneIsReported() = runTest {
        val f = fixture(bind = false)
        assertNull(f.store.reload())
        f.store.bind(f.backend, "old")
        f.store.reload()
        advanceUntilIdle()
        assertTrue(f.reported.single() is SpydusException.NotLoggedIn)
    }

    @Test
    fun bindingAgainDropsTheListAndALateAnswer() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()
        assertEquals(setOf("1", "2", "3"), f.store.irns.value)

        f.store.reload()
        f.store.clear() // signed out while the reload was running
        advanceUntilIdle()
        assertTrue(f.store.irns.value.isEmpty())
        assertEquals(PicksList(), f.store.list.value)
        assertFalse(f.store.isPicked("1"))
    }

    @Test
    fun favouritesReloadsOnlyWhenStale() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()
        assertNull(f.store.refreshIfStale())

        // A save the list doesn't show yet.
        f.store.toggle("9")
        advanceUntilIdle()
        f.store.refreshIfStale()
        advanceUntilIdle()
        assertEquals("9", f.store.list.value.records.first().workIrn)
        assertNull(f.store.refreshIfStale())

        // Older than two minutes.
        advanceTimeBy(PicksStore.STALE_AFTER_MS + 1)
        val before = f.backend.pageRequests.size
        f.store.refreshIfStale()
        advanceUntilIdle()
        assertEquals(before + 1, f.backend.pageRequests.size)
    }

    // endregion
    // region Save / remove

    @Test
    fun theBookmarkFlipsAtOnceThenConfirms() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()

        f.store.toggle("7")
        assertTrue(f.store.isPicked("7"))
        assertEquals(setOf("7"), f.store.pending.value)
        advanceUntilIdle()
        assertEquals(listOf("7"), f.backend.saved)
        assertEquals(PicksText.SAVED, f.store.notice.value!!.message)
        assertTrue(f.store.pending.value.isEmpty())

        f.store.toggle("7")
        assertFalse(f.store.isPicked("7"))
        advanceUntilIdle()
        assertEquals(listOf("7"), f.backend.removed)
        assertEquals(PicksText.REMOVED, f.store.notice.value!!.message)
        assertFalse(f.store.notice.value!!.isError)
    }

    @Test
    fun aFailedSaveFlipsBackWithAMessage() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()
        f.backend.saveFails = SpydusException.Upstream("None found")
        f.store.toggle("7")
        assertTrue(f.store.isPicked("7"))
        advanceUntilIdle()
        assertFalse(f.store.isPicked("7"))
        assertEquals(PicksText.FAILED, f.store.notice.value!!.message)
        assertTrue(f.store.notice.value!!.isError)
        assertTrue(f.reported.isEmpty())

        f.backend.removeFails = SpydusException.Network("offline")
        f.store.toggle("2")
        assertFalse(f.store.isPicked("2"))
        advanceUntilIdle()
        assertTrue(f.store.isPicked("2"))
        assertEquals("Couldn't update My Picks.", f.store.notice.value!!.message)
    }

    @Test
    fun anExpiredSessionTakesTheSessionExpiredPath() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()
        f.backend.saveFails = SpydusException.NotLoggedIn("not logged in")
        f.store.toggle("7")
        advanceUntilIdle()
        assertTrue(f.reported.single() is SpydusException.NotLoggedIn)
        assertFalse(f.store.isPicked("7"))
        assertNull(f.store.notice.value) // AppState asks to sign in again instead
    }

    @Test
    fun aSecondTapWaitsForTheFirst() = runTest {
        val f = fixture()
        f.store.toggle("7")
        assertNull(f.store.toggle("7"))
        advanceUntilIdle()
        assertEquals(listOf("7"), f.backend.saved)
        assertTrue(f.backend.removed.isEmpty())
        assertTrue(f.store.isPicked("7"))
    }

    @Test
    fun removingFromTheListTakesTheRowAwayAtOnceAndBackOnFailure() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()

        f.backend.removeFails = SpydusException.Network("offline")
        f.store.remove("2")
        assertEquals(listOf("1", "3"), f.store.list.value.records.map { it.workIrn })
        assertFalse(f.store.isPicked("2"))
        advanceUntilIdle()
        assertEquals(listOf("1", "2", "3"), f.store.list.value.records.map { it.workIrn })
        assertTrue(f.store.isPicked("2"))
        assertEquals(PicksText.FAILED, f.store.notice.value!!.message)

        f.backend.removeFails = null
        f.store.remove("2")
        advanceUntilIdle()
        assertEquals(listOf("1", "3"), f.store.list.value.records.map { it.workIrn })
        assertEquals(PicksText.REMOVED, f.store.notice.value!!.message)

        f.store.remove("1")
        f.store.remove("3")
        advanceUntilIdle()
        assertEquals(MyPicksContent.Empty, MyPicksContent.of(f.store.list.value))
    }

    @Test
    fun aReloadKeepsChangesStillInFlight() = runTest {
        val f = fixture()
        f.store.reload()
        advanceUntilIdle()
        f.backend.delayMs = 300
        f.store.remove("2")
        f.store.toggle("8")
        runCurrent() // both requests are on their way
        f.backend.delayMs = 50
        f.store.reload() // answers before the remove and the save reach the list
        advanceTimeBy(60)
        runCurrent()
        assertEquals(listOf("1", "3"), f.store.list.value.records.map { it.workIrn })
        assertEquals(setOf("1", "3", "8"), f.store.irns.value)
        advanceUntilIdle()
        assertEquals(setOf("1", "3", "8"), f.store.irns.value)
    }

    @Test
    fun nothingHappensSignedOut() = runTest {
        val f = fixture(bind = false)
        assertNull(f.store.toggle("1"))
        assertNull(f.store.remove("1"))
        assertFalse(f.store.isPicked("1"))
        assertFalse(f.store.isPicked(null))
    }

    // endregion
    // region Copy offer

    private fun ref(id: String, savedAt: Long, url: String? = null) = BookRef(id = id, title = id, url = url, savedAt = savedAt)

    @Test
    fun theOfferNeedsFavouritesAndIsAnsweredOnce() = runTest {
        val f = fixture()
        f.store.offerAfterSignIn("lib", emptyList())
        assertNull(f.store.offer.value)

        val favourites = listOf(ref("work:1", 2), ref("bib:2", 1))
        f.store.offerAfterSignIn("lib", favourites)
        val offer = f.store.offer.value!!
        assertEquals("2 books saved on this phone can be added to your library account.", offer.message)

        assertNull(f.store.answerOffer(add = false))
        assertNull(f.store.offer.value)
        assertTrue(f.backend.saved.isEmpty())
        f.store.offerAfterSignIn("lib", favourites)
        assertNull(f.store.offer.value)

        // Another library is asked separately.
        f.store.offerAfterSignIn("other", favourites)
        assertEquals("other", f.store.offer.value!!.scope)
    }

    @Test
    fun addCopiesOldestFirstLookingUpRecordsAndSummarises() = runTest {
        val f = fixture(irns = emptyList())
        val origin = "https://canning.spydus.com/cgi-bin/spydus.exe"
        f.backend.records["$origin/loan"] = recordDetail(bibIrn = "900")
        f.backend.records["$origin/related"] = recordDetail(bibIrn = null)
        val favourites = listOf(
            ref("title:No link", savedAt = 5),
            ref("brn:12", savedAt = 4, url = "$origin/related"),
            ref("work:100", savedAt = 3),
            ref("url:$origin/loan", savedAt = 2, url = "$origin/loan"),
            ref("bib:200", savedAt = 1),
        )
        f.store.offerAfterSignIn("lib", favourites)
        f.store.answerOffer(add = true)
        advanceUntilIdle()

        assertEquals(listOf("200", "900", "100"), f.backend.saved)
        assertEquals(listOf("$origin/loan", "$origin/related"), f.backend.recordRequests)
        assertEquals("Added 3 books to My Picks · 2 couldn't be added", f.store.notice.value!!.message)
        assertTrue(f.offers.wasAsked("lib"))
        // The set is reloaded from the list.
        assertEquals(setOf("100", "200", "900"), f.store.irns.value)
        assertEquals(3, f.store.list.value.records.size)
    }

    @Test
    fun aFailedSaveCountsAsCouldntBeAdded() = runTest {
        val f = fixture(irns = emptyList())
        f.backend.saveFails = SpydusException.Upstream("None found")
        f.store.offerAfterSignIn("lib", listOf(ref("work:1", 1)))
        f.store.answerOffer(add = true)
        advanceUntilIdle()
        assertEquals("Added 0 books to My Picks · 1 couldn't be added", f.store.notice.value!!.message)
        assertTrue(f.store.notice.value!!.isError)
    }

    @Test
    fun oneBookIsSingular() {
        assertEquals("1 book saved on this phone can be added to your library account.", PicksText.offerMessage(1))
        assertEquals("Added 1 book to My Picks", PicksText.copied(1, 0))
        assertEquals("Added 2 books to My Picks · 1 couldn't be added", PicksText.copied(2, 1))
    }

    @Test
    fun anExpiredSessionStopsTheCopyWithoutASummary() = runTest {
        val f = fixture(irns = emptyList())
        f.backend.saveFails = SpydusException.NotLoggedIn("not logged in")
        f.store.offerAfterSignIn("lib", listOf(ref("work:1", 1), ref("work:2", 2)))
        f.store.answerOffer(add = true)
        advanceUntilIdle()
        assertTrue(f.reported.single() is SpydusException.NotLoggedIn)
        assertNull(f.store.notice.value)
    }

    // endregion
}

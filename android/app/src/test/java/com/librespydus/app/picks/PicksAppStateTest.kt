package com.librespydus.app.picks

import com.librespydus.app.core.AppState
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.FavouritesStore
import com.librespydus.app.core.InMemoryLibraryStore
import com.librespydus.app.core.InMemoryPicksOfferStore
import com.librespydus.app.core.InMemorySecretStore
import com.librespydus.app.core.LaunchOptions
import com.librespydus.app.core.PicksList
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
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
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.time.LocalDate

/** My Picks wired into AppState (§5.11): loading, clearing, the session-expired path and the copy offer. */
class PicksAppStateTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = LibraryInfo("canning", "City of Canning", "https://canning.spydus.com", null, "Australia", "AU", false)
    private val hutt = LibraryInfo("hutt", "Hutt City Libraries", "https://hutt.spydus.co.nz", null, "New Zealand", "NZ", false)

    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private class Fixture(val state: AppState, val backends: MutableMap<String, PicksBackend>, val offers: InMemoryPicksOfferStore)

    private fun TestScope.fixture(
        library: LibraryInfo? = canning,
        secrets: InMemorySecretStore = InMemorySecretStore(),
        offers: InMemoryPicksOfferStore = InMemoryPicksOfferStore(),
    ): Fixture {
        val io = StandardTestDispatcher(testScheduler)
        val backends = mutableMapOf<String, PicksBackend>()
        var clock = 1_000L
        val appScope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        val state = AppState(
            scope = appScope,
            libraryStore = InMemoryLibraryStore(library),
            secrets = secrets,
            // Distinct savedAt stamps, so "oldest first" is well defined.
            favourites = FavouritesStore(File(tmp.root, "fav.json"), now = { clock++ }, io = io),
            directory = { listOf(canning, hutt) },
            makeBackend = { origin ->
                PicksBackend(origin).apply { listOf("1", "2", "3").forEach { list += pickRecord(it, origin = origin) } }.also { backends[origin] = it }
            },
            loadDemoData = { origin -> DemoData.load({ File("src/main/assets/demo/$it").readText() }, origin, LocalDate.of(2026, 10, 12)) },
            io = io,
            picksOffers = offers,
        )
        return Fixture(state, backends, offers)
    }

    private fun signedIn(origin: String = canning.origin) = InMemorySecretStore().apply { put(AppState.sessionKey(origin), "tok") }

    private suspend fun AppState.favour(vararg ids: String) = ids.forEach { toggleFavourite(BookRef(id = it, title = it)) }

    @Test
    fun aRestoredSessionLoadsTheSetAndNeverOffers() = runTest {
        // Favourites saved on this phone before the app restarts signed in.
        FavouritesStore(File(tmp.root, "fav.json"), io = StandardTestDispatcher(testScheduler)).apply {
            load()
            toggle(canning.origin, BookRef(id = "work:9", title = "Saved"))
        }
        val f = fixture(secrets = signedIn())
        f.state.start()
        advanceUntilIdle()
        assertEquals(1, f.state.currentFavourites.value.size)
        assertEquals(setOf("1", "2", "3"), f.state.picks.irns.value)
        assertEquals(3, f.state.picks.list.value.records.size)
        assertNull(f.state.picks.offer.value)
    }

    @Test
    fun anInteractiveSignInWithLocalFavouritesOffersOncePerLibrary() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        assertTrue(f.state.picks.irns.value.isEmpty())
        f.state.favour("work:10", "bib:20")

        f.state.signIn("reader", "pin")
        advanceUntilIdle()
        val offer = f.state.picks.offer.value!!
        assertEquals(canning.origin, offer.scope)
        assertEquals("2 books saved on this phone can be added to your library account.", offer.message)
        assertEquals(setOf("1", "2", "3"), f.state.picks.irns.value)

        f.state.picks.answerOffer(add = true)
        advanceUntilIdle()
        assertEquals(listOf("10", "20"), f.backends[canning.origin]!!.saved) // oldest first
        assertEquals("Added 2 books to My Picks", f.state.picks.notice.value!!.message)
        assertEquals(setOf("1", "2", "3", "10", "20"), f.state.picks.irns.value)
        // The local list is kept: it's what Favourites shows signed out.
        assertEquals(2, f.state.currentFavourites.value.size)

        // Never asked again for this library…
        f.state.signOut()
        f.state.signIn("reader", "pin")
        advanceUntilIdle()
        assertNull(f.state.picks.offer.value)

        // …but another library is asked on its own.
        f.state.selectLibrary(hutt)
        f.state.favour("work:30")
        f.state.signIn("reader", "pin")
        advanceUntilIdle()
        assertEquals(hutt.origin, f.state.picks.offer.value!!.scope)
        f.state.picks.answerOffer(add = false)
        assertTrue(f.offers.wasAsked(hutt.origin))
        assertTrue(f.backends[hutt.origin]!!.saved.isEmpty())
    }

    @Test
    fun noLocalFavouritesNoOffer() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        f.state.signIn("reader", "pin")
        advanceUntilIdle()
        assertNull(f.state.picks.offer.value)
        assertFalse(f.offers.wasAsked(canning.origin))
    }

    @Test
    fun signingOutClearsTheSet() = runTest {
        val f = fixture(secrets = signedIn())
        f.state.start()
        advanceUntilIdle()
        assertTrue(f.state.picks.isPicked("2"))
        f.state.signOut()
        advanceUntilIdle()
        assertTrue(f.state.picks.irns.value.isEmpty())
        assertEquals(PicksList(), f.state.picks.list.value)
        assertNull(f.state.picks.toggle("2")) // signed out: the bookmark is local favourites again
    }

    @Test
    fun switchingLibraryClearsTheSetAndLoadsThatLibrarys() = runTest {
        val secrets = signedIn().apply { put(AppState.sessionKey(hutt.origin), "tok") }
        val f = fixture(secrets = secrets)
        f.state.start()
        advanceUntilIdle()
        f.state.picks.toggle("7")
        advanceUntilIdle()
        assertTrue(f.state.picks.isPicked("7"))

        f.state.selectLibrary(hutt)
        assertFalse(f.state.picks.isPicked("7"))
        advanceUntilIdle()
        assertEquals(setOf("1", "2", "3"), f.state.picks.irns.value)
        assertEquals(hutt.origin, f.state.picks.list.value.records.first().detailUrl!!.substringBefore("/cgi-bin"))
    }

    @Test
    fun anExpiredSessionOnSaveSignsOutAndAsksToSignIn() = runTest {
        val f = fixture(secrets = signedIn())
        f.state.start()
        advanceUntilIdle()
        f.backends[canning.origin]!!.saveFails = SpydusException.NotLoggedIn("not logged in")
        f.state.picks.toggle("7")
        advanceUntilIdle()
        assertNull(f.state.session.value)
        assertTrue(f.state.loginNeeded.value)
        assertTrue(f.state.picks.irns.value.isEmpty())
        assertNull(f.state.picks.notice.value)
    }

    @Test
    fun anExpiredSessionOnTheListSignsOut() = runTest {
        val f = fixture(secrets = signedIn())
        f.state.start()
        advanceUntilIdle()
        f.backends[canning.origin]!!.pageFails = SpydusException.NotLoggedIn("not logged in")
        f.state.picks.reload()
        advanceUntilIdle()
        assertNull(f.state.session.value)
        assertTrue(f.state.loginNeeded.value)
        assertEquals(PicksList(), f.state.picks.list.value)
    }

    @Test
    fun demoModeHasTheSeededListAndOffersAfterADemoSignIn() = runTest {
        val f = fixture()
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = canning.origin, demo = true))
        advanceUntilIdle()
        assertEquals(DemoBackend.SESSION, f.state.session.value)
        assertEquals(2, f.state.picks.list.value.records.size)
        assertNull(f.state.picks.offer.value) // signed in by the launch option, not interactively

        // A demo book saved on the phone (the Project Hail Mary loan), then a demo sign-in.
        f.state.signOut()
        f.state.favour("bib:17665994")
        f.state.signIn("demo", "demo")
        advanceUntilIdle()
        val offer = f.state.picks.offer.value!!
        assertEquals("demo:${canning.origin}", offer.scope)
        f.state.picks.answerOffer(add = true)
        advanceUntilIdle()
        assertEquals("Added 1 book to My Picks", f.state.picks.notice.value!!.message)
        assertEquals(3, f.state.picks.list.value.records.size)
        assertTrue(f.state.picks.isPicked("17665994"))
        assertFalse(f.offers.wasAsked(canning.origin)) // the real library's offer is separate
    }

    @Test
    fun resetForgetsTheOffers() = runTest {
        val offers = InMemoryPicksOfferStore().apply { markAsked(canning.origin) }
        val f = fixture(offers = offers)
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(reset = true))
        advanceUntilIdle()
        assertFalse(offers.wasAsked(canning.origin))
    }
}

package com.librespydus.app.core

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
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.time.LocalDate

class AppStateTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = LibraryInfo("canning", "City of Canning", "https://canning.spydus.com", null, "Australia", "AU", false)
    private val hutt = LibraryInfo("hutt", "Hutt City Libraries", "https://hutt.spydus.co.nz", null, "New Zealand", "NZ", false)

    // AppState's own scope on the test scheduler. Not backgroundScope: advanceUntilIdle() doesn't
    // run background-only work, and AppState's start/launch-option jobs must run to completion.
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private class Fixture(val state: AppState, val secrets: InMemorySecretStore, val store: InMemoryLibraryStore, val made: MutableMap<String, FakeBackend>)

    private fun TestScope.fixture(library: LibraryInfo? = null, secrets: InMemorySecretStore = InMemorySecretStore()): Fixture {
        val io = StandardTestDispatcher(testScheduler)
        val made = mutableMapOf<String, FakeBackend>()
        val store = InMemoryLibraryStore(library)
        val appScope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler)).also { scopes += it }
        val state = AppState(
            scope = appScope,
            libraryStore = store,
            secrets = secrets,
            favourites = FavouritesStore(File(tmp.root, "fav.json"), io = io),
            directory = { listOf(canning, hutt) },
            makeBackend = { origin -> FakeBackend(origin).also { made[origin] = it } },
            loadDemoData = { origin ->
                DemoData.load({ File("src/main/assets/demo/$it").readText() }, origin, LocalDate.of(2026, 10, 12))
            },
            io = io,
        )
        return Fixture(state, secrets, store, made)
    }

    @Test
    fun startRestoresLibrarySessionAndVerificationCookie() = runTest {
        val secrets = InMemorySecretStore().apply {
            put(AppState.sessionKey(canning.origin), "tok")
            put(AppState.verifyKey(canning.origin), "aws-waf-token=abc")
        }
        val f = fixture(canning, secrets)
        f.state.start()
        advanceUntilIdle()
        assertTrue(f.state.ready.value)
        assertEquals(canning, f.state.library.value)
        assertEquals("tok", f.state.session.value)
        assertTrue(f.state.isSignedIn.value)
        assertEquals("Real Person", f.state.accountName.value)
        assertEquals("aws-waf-token=abc", f.made[canning.origin]!!.appliedCookie)
        assertTrue(f.state.hasVerificationCookie.value)
    }

    @Test
    fun noStoredLibraryMeansOnboarding() = runTest {
        val f = fixture()
        f.state.start()
        advanceUntilIdle()
        assertTrue(f.state.ready.value)
        assertNull(f.state.library.value)
        assertNull(f.state.backend.value)
    }

    @Test
    fun signInStoresSessionPerOriginAndSwitchingLibraryReusesClients() = runTest {
        val f = fixture(canning)
        f.state.start()
        advanceUntilIdle()
        f.state.signIn(" 1234 ", "pin")
        assertEquals("session-1234", f.secrets.get(AppState.sessionKey(canning.origin)))
        val generation = f.state.accountGeneration.value

        f.state.selectLibrary(hutt)
        assertEquals(hutt, f.store.get())
        assertNull(f.state.session.value)
        assertTrue(f.state.accountGeneration.value > generation)

        f.state.selectLibrary(canning)
        assertEquals("session-1234", f.state.session.value)
        assertSame(f.made[canning.origin], f.state.backend.value)
        assertEquals(2, f.made.size)

        f.state.signOut()
        assertNull(f.secrets.get(AppState.sessionKey(canning.origin)))
        assertFalse(f.state.isSignedIn.value)
    }

    @Test
    fun reportRoutesVerificationAndExpiredSessions() = runTest {
        val f = fixture(canning, InMemorySecretStore().apply { put(AppState.sessionKey(canning.origin), "tok") })
        f.state.start()
        advanceUntilIdle()

        val verify = f.state.report(SpydusException.HumanVerificationRequired("human verification required"))
        assertEquals(UserFacingError.Kind.VERIFICATION, verify.kind)
        assertTrue(f.state.verificationNeeded.value)

        val generation = f.state.verificationGeneration.value
        f.state.completeVerification("aws-waf-token=xyz")
        assertFalse(f.state.verificationNeeded.value)
        assertEquals("aws-waf-token=xyz", f.made[canning.origin]!!.appliedCookie)
        assertEquals("aws-waf-token=xyz", f.secrets.get(AppState.verifyKey(canning.origin)))
        assertEquals(generation + 1, f.state.verificationGeneration.value)

        val expired = f.state.report(SpydusException.NotLoggedIn("not logged in"))
        advanceUntilIdle()
        assertEquals("Please sign in again.", expired.message)
        assertNull(f.state.session.value)
        assertTrue(f.state.loginNeeded.value)
        assertNull(f.secrets.get(AppState.sessionKey(canning.origin)))

        val network = f.state.report(SpydusException.Network("network error: connect timed out after 10s"))
        assertEquals("Can't reach City of Canning. Check your connection.", network.message)
        assertEquals("network error: connect timed out after 10s", network.detail)
    }

    @Test
    fun demoLaunchOptionsSignInAsTestBorrowerWithoutPersistingTheSession() = runTest {
        val f = fixture()
        f.state.start()
        f.state.applyLaunchOptions(LaunchOptions(origin = "https://hutt.spydus.co.nz/", route = LaunchRoute.Borrowed, demo = true))
        advanceUntilIdle()
        assertEquals(hutt, f.state.library.value)
        assertTrue(f.state.isDemo.value)
        assertTrue(f.state.backend.value!!.isDemo)
        assertEquals(DemoBackend.SESSION, f.state.session.value)
        assertEquals("Test Borrower", f.state.accountName.value)
        assertEquals(Tab.BORROWED, f.state.selectedTab.value)
        assertNull(f.secrets.get(AppState.sessionKey(hutt.origin)))
    }

    @Test
    fun launchRoutesAndReset() = runTest {
        val f = fixture(canning, InMemorySecretStore().apply { put(AppState.sessionKey(canning.origin), "tok") })
        f.state.start()
        advanceUntilIdle()
        f.state.favourites.toggle(canning.origin, BookRef("bib:1", "Saved"))

        f.state.applyLaunchOptions(LaunchOptions(route = LaunchRoute.Detail("https://canning.spydus.com/x")))
        advanceUntilIdle()
        assertEquals(Tab.HOME, f.state.selectedTab.value)
        assertEquals(Route.Detail(BookRef.fromUrl("https://canning.spydus.com/x")), f.state.consumePendingNavigation(Tab.HOME))
        assertNull(f.state.consumePendingNavigation(Tab.HOME))

        f.state.applyLaunchOptions(LaunchOptions(route = LaunchRoute.Search("dune")))
        advanceUntilIdle()
        assertEquals(Tab.SEARCH, f.state.selectedTab.value)
        assertEquals("dune", f.state.consumePendingSearch())

        f.state.applyLaunchOptions(LaunchOptions(reset = true))
        advanceUntilIdle()
        assertNull(f.state.library.value)
        assertNull(f.store.get())
        assertNull(f.secrets.get(AppState.sessionKey(canning.origin)))
        assertTrue(f.state.currentFavourites.value.isEmpty())
    }

    @Test
    fun unknownOriginGetsAFallbackLibrary() {
        val info = AppState.fallbackLibrary("https://bromley.spydus.co.uk")
        assertEquals("Bromley", info.name)
        assertEquals("UK", info.region)
        assertEquals("https://bromley.spydus.co.uk", info.origin)
    }
}

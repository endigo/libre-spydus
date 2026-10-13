package com.librespydus.app.feature.onboarding

import com.librespydus.app.core.AppState
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.FavouritesStore
import com.librespydus.app.core.InMemoryLibraryStore
import com.librespydus.app.core.InMemorySecretStore
import com.librespydus.app.core.LibraryBackend
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import uniffi.librespydus_core.LibraryInfo
import java.io.File
import java.time.LocalDate
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

/*
 * Shared by the A2 feature tests (onboarding, profile, borrowed, favourites): an AppState on the
 * test scheduler whose backends are DemoBackend over FakeBackend (fixture account data, no
 * network, nothing posted).
 */

internal val CANNING = LibraryInfo("canning", "City of Canning", "https://canning.spydus.com", null, "Australia", "AU", false)
internal val HUTT = LibraryInfo("hutt", "Hutt City Libraries", "https://hutt.spydus.co.nz", null, "New Zealand", "NZ", false)

/** The demo fixtures in `src/main/assets/demo`, dated relative to [today]. */
internal fun demoData(origin: String = CANNING.origin, today: LocalDate = TODAY): DemoData =
    DemoData.load({ File("src/main/assets/demo/$it").readText() }, origin, today)

internal val TODAY: LocalDate = LocalDate.of(2026, 10, 12)

internal class AccountFixture(
    val state: AppState,
    val secrets: InMemorySecretStore,
    val backends: Map<String, LibraryBackend>,
    val scope: CoroutineScope,
)

/**
 * An AppState with [library] restored (and [session] for it, if any), every backend a
 * [DemoBackend] (`login("demo", "demo")` succeeds) wrapped by [wrap]. Call `start()` is done here.
 * Cancel [AccountFixture.scope] at the end of the test.
 */
internal fun TestScope.accountFixture(
    favouritesDir: File,
    library: LibraryInfo? = CANNING,
    session: String? = null,
    mutationDelay: Duration = 80.milliseconds,
    wrap: (LibraryBackend) -> LibraryBackend = { it },
): AccountFixture {
    val io = StandardTestDispatcher(testScheduler)
    val secrets = InMemorySecretStore()
    if (library != null && session != null) secrets.put(AppState.sessionKey(library.origin), session)
    val backends = mutableMapOf<String, LibraryBackend>()
    var savedAt = 1_800_000_000_000L
    val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler))
    val state = AppState(
        scope = scope,
        libraryStore = InMemoryLibraryStore(library),
        secrets = secrets,
        // A ticking clock, so favourites saved one after another sort deterministically.
        favourites = FavouritesStore(File(favouritesDir, "favourites.json"), now = { ++savedAt }, io = io),
        directory = { listOf(CANNING, HUTT) },
        makeBackend = { origin ->
            wrap(DemoBackend(FakeBackend(origin), demoData(origin), today = TODAY, mutationDelay = mutationDelay))
                .also { backends[origin] = it }
        },
        io = io,
    )
    state.start()
    advanceUntilIdle()
    return AccountFixture(state, secrets, backends, scope)
}

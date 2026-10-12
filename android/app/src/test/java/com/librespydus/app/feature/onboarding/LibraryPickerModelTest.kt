package com.librespydus.app.feature.onboarding

import com.librespydus.app.core.UserFacingError
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
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
import java.util.concurrent.TimeUnit

class LibraryPickerModelTest {
    @get:Rule val tmp = TemporaryFolder()

    private val now = 1_800_000_000_000L
    private val day = TimeUnit.DAYS.toMillis(1)
    private val cached = listOf(LibraryPickerLogicTest.library("City of Canning", "AU", "Australia"))
    private val fetched = LibraryPickerLogicTest.sample

    /** Counts directory fetches and answers with [result]. */
    private class FetchStub(var result: Result<List<LibraryInfo>>) {
        var calls = 0
        suspend fun fetch(): List<LibraryInfo> {
            calls++
            return result.getOrThrow()
        }
    }

    private fun cache() = LibraryDirectoryCache(File(tmp.root, "nested/${LibraryDirectoryCache.FILE_NAME}"))

    private fun picker(stub: FetchStub, cache: LibraryDirectoryCache) =
        LibraryPickerModel(fetch = stub::fetch, cache = cache, now = { now }, io = Dispatchers.Unconfined)

    // region Freshness

    @Test
    fun freshForUpToSevenDays() {
        assertTrue(LibraryDirectoryCache.isFresh(now, now))
        assertTrue(LibraryDirectoryCache.isFresh(now - 6 * day, now))
        assertTrue(LibraryDirectoryCache.isFresh(now - 7 * day + 1, now))
        assertFalse(LibraryDirectoryCache.isFresh(now - 7 * day, now))
        assertFalse(LibraryDirectoryCache.isFresh(now - 30 * day, now))
    }

    @Test
    fun aFetchDateInTheFutureIsStale() {
        assertFalse(LibraryDirectoryCache.isFresh(now + 60_000, now))
    }

    // endregion
    // region Disk cache

    @Test
    fun cacheRoundTripsEveryField() {
        val cache = cache()
        assertNull(cache.load())
        val library = LibraryInfo("x", "Ünïcode Library", "https://x.spydus.co.uk", "https://x.spydus.co.uk/logo.png", null, "UK", true)
        cache.save(LibraryDirectoryCache.Snapshot(now, listOf(library)))
        val loaded = cache.load()!!
        assertEquals(now, loaded.fetchedAt)
        assertEquals(listOf(library), loaded.libraries)
        cache.remove()
        assertNull(cache.load())
    }

    @Test
    fun unreadableCacheIsIgnored() {
        val cache = cache()
        File(tmp.root, "nested").mkdirs()
        File(tmp.root, "nested/${LibraryDirectoryCache.FILE_NAME}").writeText("not json")
        assertNull(cache.load())
    }

    // endregion
    // region Picker model

    @Test
    fun freshCacheOpensWithoutFetching() = runTest {
        val cache = cache().apply { save(LibraryDirectoryCache.Snapshot(now - day, cached)) }
        val stub = FetchStub(Result.success(fetched))
        val picker = picker(stub, cache)
        picker.load()
        assertEquals(0, stub.calls)
        assertEquals(cached, picker.libraries.value)
        picker.load()
        assertEquals(0, stub.calls)
    }

    @Test
    fun staleCacheIsShownThenRefreshedAndSaved() = runTest {
        val cache = cache().apply { save(LibraryDirectoryCache.Snapshot(now - 8 * day, cached)) }
        val stub = FetchStub(Result.success(fetched))
        val picker = picker(stub, cache)
        picker.load()
        assertEquals(1, stub.calls)
        assertEquals(fetched, picker.libraries.value)
        assertEquals(now, picker.fetchedAt.value)
        assertEquals(LibraryDirectoryCache.Snapshot(now, fetched), cache.load())
    }

    @Test
    fun noCacheFetchesAndCaches() = runTest {
        val cache = cache()
        val stub = FetchStub(Result.success(fetched))
        val picker = picker(stub, cache)
        assertTrue(picker.libraries.value.isEmpty())
        // Loading (not "No libraries listed") until the first load has run.
        assertTrue(picker.isLoading)
        assertFalse(picker.loaded.value)
        picker.load()
        assertEquals(1, stub.calls)
        assertTrue(picker.loaded.value)
        assertFalse(picker.isLoading)
        assertEquals(PickerRegion.entries.toList(), picker.sections.map { it.region })
        assertEquals(fetched, cache.load()?.libraries)
    }

    @Test
    fun firstFetchFailureShowsAnErrorAndRetryRecovers() = runTest {
        val stub = FetchStub(Result.failure(SpydusException.Network("connect timed out after 10s")))
        val picker = picker(stub, cache())
        picker.load()
        assertTrue(picker.loaded.value)
        val error = picker.loadError.value!!
        assertEquals(UserFacingError.Kind.NETWORK, error.kind)
        assertEquals("Can't reach the library directory. Check your connection.", error.message)
        assertEquals("connect timed out after 10s", error.detail)
        assertFalse(picker.isLoading)

        stub.result = Result.success(fetched)
        picker.retry()
        assertNull(picker.loadError.value)
        assertEquals(fetched, picker.libraries.value)
    }

    @Test
    fun refreshFailureKeepsTheCachedList() = runTest {
        val cache = cache().apply { save(LibraryDirectoryCache.Snapshot(now - 8 * day, cached)) }
        val stub = FetchStub(Result.failure(SpydusException.Upstream("HTTP 500")))
        val picker = picker(stub, cache)
        picker.load()
        assertEquals(cached, picker.libraries.value)
        assertNull(picker.loadError.value)
        assertEquals("The library directory returned something unexpected.", picker.refreshError.value?.message)
        // The stale snapshot isn't overwritten.
        assertEquals(now - 8 * day, cache.load()?.fetchedAt)

        stub.result = Result.success(fetched)
        picker.refresh()
        assertNull(picker.refreshError.value)
        assertEquals(fetched, picker.libraries.value)
    }

    @Test
    fun searchingFiltersTheSections() = runTest {
        val picker = picker(FetchStub(Result.success(fetched)), cache())
        picker.load()
        picker.setQuery("kingdom")
        assertTrue(picker.isSearching)
        assertEquals(listOf(PickerRegion.UK), picker.sections.map { it.region })
        picker.setQuery(" ")
        assertFalse(picker.isSearching)
        assertEquals(6, picker.sections.size)
    }

    // endregion
}

package com.librespydus.app.picks

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.feature.detail.DetailRecordSource
import com.librespydus.app.feature.featured.EditionLevel
import com.librespydus.app.feature.search.SearchResultItem
import kotlinx.coroutines.test.currentTime
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.time.LocalDate

/** Demo mode's Pick list (§5.11): seeded, honest when emptied, counts from save/remove, offline. */
class DemoPicksTest {
    private val origin = "https://canning.spydus.com"

    private fun backend(public: LibraryBackend = FakeBackend(origin)) = DemoBackend(
        public,
        DemoData.load(read = { File("src/main/assets/demo/$it").readText() }, origin = origin, today = LocalDate.of(2026, 10, 12)),
        LocalDate.of(2026, 10, 12),
    )

    @Test
    fun seededWithAWorkAndAnEditionLevelCard() = runTest {
        val b = backend()
        val page = b.picks(DemoBackend.SESSION, null)
        assertFalse(page.noResults)
        assertEquals(2L, page.totalResults)
        assertEquals(listOf("The midnight library", "The Thursday Murder Club"), page.records.map { it.title })

        val work = SearchResultItem.from(page.records[0], null, 0)
        assertEquals(listOf("Book", "Large Print", "eBook"), work.formats.map { it.label })
        assertEquals("15642035", page.records[0].workIrn)

        // The edition: one format with an empty code and no label, so no chip.
        val edition = page.records[1]
        assertTrue(EditionLevel.isEditionLevel(edition.formats.map(FormatRef::from)))
        assertTrue(SearchResultItem.from(edition, null, 1).formats.isEmpty())
        assertEquals("99000601", edition.workIrn)
    }

    @Test
    fun theEditionLevelCardOpensOffline() = runTest {
        val b = backend()
        val edition = b.picks(DemoBackend.SESSION, null).records[1]
        val format = FormatRef.from(edition.formats.single())
        val url = DetailRecordSource.candidateUrls(BookRef.from(edition).url, format).first()
        assertEquals(format.availabilityUrl, url)
        val record = b.record(url)
        assertEquals("99000601", record.bibIrn)
        assertEquals("The Thursday Murder Club", record.title)
        val availability = b.availability(format.availabilityUrl!!)
        assertEquals("99000601", availability.bibRecordIrn)
        assertEquals(5L, availability.copies)
    }

    @Test
    fun saveAndRemoveCountTheListAndEmptyIsNoResults() = runTest {
        val b = backend()
        val session = DemoBackend.SESSION
        assertEquals(1L, b.removePick("15642035", session))
        assertEquals(0L, b.removePick("99000601", session))
        val empty = b.picks(session, null)
        assertTrue(empty.noResults)
        assertTrue(empty.records.isEmpty())
        assertEquals(0L, empty.totalResults)

        // A catalogue work and a loan's edition (its bib IRN), newest first; saving twice keeps one.
        assertEquals(1L, b.savePick("15643445", session))
        assertEquals(2L, b.savePick("8753080", session))
        assertEquals(2L, b.savePick("8753080", session))
        val page = b.picks(session, null)
        assertEquals(listOf("8753080", "15643445"), page.records.map { it.workIrn })
        assertEquals(listOf(1L, 2L), page.records.map { it.rank })
        assertFalse(page.noResults)
    }

    @Test
    fun mutationsTakeTheDemoDelay() = runTest {
        val b = backend()
        val start = currentTime
        b.savePick("15643445", DemoBackend.SESSION)
        assertEquals(600L, currentTime - start)
    }

    @Test
    fun aCardSeenInResultsOrARecordLoadedCanBeSaved() = runTest {
        val public = object : LibraryBackend by FakeBackend(origin) {
            override suspend fun search(query: String) = FakeBackend.results(pickRecord("777", "Dune", origin))
            override suspend fun record(url: String) = recordDetail(bibIrn = "555", title = "A real library book")
        }
        val b = backend(public)
        val session = DemoBackend.SESSION
        // Search results from the real library (public GETs).
        b.search("dune")
        assertEquals(3L, b.savePick("777", session))
        assertEquals("Dune", b.picks(session, null).records.first().title)
        // A record opened from a related item or loan outside the demo catalogue.
        b.record("$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=1")
        assertEquals(4L, b.savePick("555", session))
        val saved = b.picks(session, null).records.first()
        assertEquals("A real library book", saved.title)
        assertTrue(EditionLevel.isEditionLevel(saved.formats.map(FormatRef::from)))
    }

    @Test
    fun anUnknownIrnFailsLikeTheLibrary() = runTest {
        val b = backend()
        try {
            b.savePick("424242", DemoBackend.SESSION)
            fail("expected Upstream")
        } catch (_: SpydusException.Upstream) {
        }
        assertEquals(2L, b.picks(DemoBackend.SESSION, null).totalResults)
    }

    @Test
    fun theListNeedsTheDemoSession() = runTest {
        val b = backend()
        for (call in listOf<suspend () -> Unit>(
            { b.picks("real-session", null) },
            { b.savePick("15643445", "real-session") },
            { b.removePick("15642035", "real-session") },
        )) {
            try {
                call()
                fail("expected NotLoggedIn")
            } catch (_: SpydusException.NotLoggedIn) {
            }
        }
        assertEquals(2L, b.picks(DemoBackend.SESSION, null).totalResults)
    }
}

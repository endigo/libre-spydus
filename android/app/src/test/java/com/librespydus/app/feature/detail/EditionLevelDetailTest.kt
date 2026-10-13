package com.librespydus.app.feature.detail

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.featured.CopyRow
import com.librespydus.app.feature.featured.FeaturedBackend
import com.librespydus.app.feature.featured.FeaturedFixtures
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.valueOrNull
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.SpydusException

/** Edition-level items (§5.10): every item of a Featured collection's lists. */
class EditionLevelDetailTest {
    private val f = FeaturedFixtures
    private val report: BrowseReporter = { UserFacingError.from(it, "Canning") }
    private val irn = "10936881"
    private val ref = BookRef.from(f.editionRecord(irn))
    private val format = ref.formats.single()

    private fun backend() = FeaturedBackend().apply {
        records[format.availabilityUrl!!] = f.record(irn)
        availabilityByUrl[format.availabilityUrl!!] = f.availability(irn, copies = 2, available = 0)
        holdingsByUrl[f.holdingsUrl(irn)] = Result.success(
            listOf(
                f.holding("RIVERTON LIBRARY", "On loan - Due: 29 Oct 2026", available = false, callNumber = "LOT MUS004"),
                f.holding("RIVERTON LIBRARY", "Available", available = true, callNumber = "LOTMUS009"),
            ),
        )
    }

    @Test
    fun anEditionLevelPageHasNoFormatChipsAndNeverReFindsFormats() {
        val state = DetailState(ref = ref)
        assertTrue(state.isEditionLevel)
        assertFalse(state.showsFormatChips)
        assertEquals(FormatLookup.Provided, state.formatLookup)
        assertEquals(format, state.selectedFormat)
        assertEquals(format, state.reservableFormat) // Reserve uses the card's own reserve link

        val book = BookRef.from(DetailFixtures.searchRecord)
        val other = DetailState(ref = book)
        assertFalse(other.isEditionLevel)
        assertTrue(other.showsFormatChips)
        assertNull(other.copiesSection)
    }

    @Test
    fun theRecordLoadsThroughTheFormatUrlFirst() = runTest {
        assertEquals(listOf(format.availabilityUrl, ref.url), DetailRecordSource.candidateUrls(ref.url, format))

        val backend = backend()
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals(listOf(format.availabilityUrl), backend.recordUrls)
        assertTrue(backend.searchCalls.isEmpty())
        assertEquals("Mini Keyboard : Musical Instruments", model.state.value.record.valueOrNull?.title)
        assertNull(model.state.value.ref.author) // no author line
    }

    @Test
    fun theCopiesListLoadsFromTheAvailabilitysHoldings() = runTest {
        val backend = backend()
        val model = BookDetailModel(ref)
        model.load(backend, report)
        assertEquals(BrowseLoadable.Loading, model.state.value.copiesSection) // availability not in yet

        model.loadCopies(backend, report) // nothing to load before the availability
        assertTrue(backend.holdingsCalls.isEmpty())

        model.loadAvailability(backend, report)
        assertEquals(
            HeaderAvailability.Line(AvailabilityLine("0 of 2 available", AvailabilityLine.Tone.NORMAL)),
            model.state.value.headerAvailability,
        )
        assertEquals(f.holdingsUrl(irn), model.state.value.holdingsUrl)
        assertEquals(BrowseLoadable.Loading, model.state.value.copiesSection)

        model.loadCopies(backend, report)
        model.loadCopies(backend, report) // once per URL
        assertEquals(listOf(f.holdingsUrl(irn)), backend.holdingsCalls)
        assertEquals(
            BrowseLoadable.Loaded(
                listOf(
                    CopyRow("Riverton Library", "On loan - Due: 29 Oct 2026", false, null, "LOT MUS004"),
                    CopyRow("Riverton Library", "Available", true, null, "LOTMUS009"),
                ),
            ),
            model.state.value.copiesSection,
        )
    }

    @Test
    fun failedCopiesRetryAndNoCopiesHideTheSection() = runTest {
        val backend = backend()
        backend.holdingsByUrl[f.holdingsUrl(irn)] = Result.failure(SpydusException.Network("offline"))
        val model = BookDetailModel(ref)
        model.load(backend, report)
        model.loadAvailability(backend, report)
        model.loadCopies(backend, report)
        assertTrue(model.state.value.copiesSection is BrowseLoadable.Failed)

        backend.holdingsByUrl[f.holdingsUrl(irn)] = Result.success(emptyList())
        model.loadCopies(backend, report)
        assertEquals(2, backend.holdingsCalls.size)
        assertNull(model.state.value.copiesSection)
    }

    @Test
    fun noHoldingsLinkOrAFailedAvailabilityHidesTheCopies() = runTest {
        val backend = backend()
        backend.availabilityByUrl[format.availabilityUrl!!] = f.availability(irn, 2, 1).copy(holdingsUrl = null)
        val model = BookDetailModel(ref)
        model.loadAvailability(backend, report)
        assertNull(model.state.value.copiesSection)

        val failing = FeaturedBackend() // no availability at all
        val other = BookDetailModel(ref)
        other.loadAvailability(failing, report)
        assertEquals(HeaderAvailability.AvailabilityFailed, other.state.value.headerAvailability)
        assertNull(other.state.value.copiesSection)
    }

    @Test
    fun aFavouriteKeepsTheEditionLevelFormat() = runTest {
        val backend = backend()
        val model = BookDetailModel(ref)
        model.load(backend, report)
        val favourite = model.state.value.favouriteRef
        assertEquals(listOf(format), favourite.formats)
        assertEquals("work:$irn", favourite.id)
        // Reopened from Favourites: still edition-level, loaded the same way.
        val reopened = DetailState(ref = favourite.copy(savedAt = 1L))
        assertTrue(reopened.isEditionLevel)
        assertEquals(FormatLookup.Provided, reopened.formatLookup)
        assertEquals(listOf(format.availabilityUrl, favourite.url), DetailRecordSource.candidateUrls(favourite.url, reopened.selectedFormat))
    }
}

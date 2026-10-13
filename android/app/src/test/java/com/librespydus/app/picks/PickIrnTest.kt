package com.librespydus.app.picks

import com.librespydus.app.alerts.FakePreferences
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Copy
import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.PickIrn
import com.librespydus.app.core.PicksList
import com.librespydus.app.core.SharedPreferencesPicksOfferStore
import com.librespydus.app.feature.detail.DetailBookmark
import com.librespydus.app.feature.favourites.FavouritesMode
import com.librespydus.app.feature.favourites.MyPicksContent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.RecordFormat

/** The pick IRN (§5.11), the bookmark's state and the Favourites tab's two modes. */
class PickIrnTest {
    @Test
    fun aResultsCardIsKeyedOnItsWorkIrn() {
        assertEquals("15643445", PickIrn.of(FakeBackend.sampleRecord()))
        assertEquals("15643445", PickIrn.of(BookRef.from(FakeBackend.sampleRecord())))
        assertNull(PickIrn.of(FakeBackend.sampleRecord().copy(workIrn = " ")))
    }

    @Test
    fun workAndBibKeysAreTheirIrn() {
        assertEquals("123", PickIrn.of(BookRef(id = "work:123", title = "A")))
        assertEquals("8753080", PickIrn.of(BookRef(id = "bib:8753080", title = "A")))
        // The key wins over a loaded record's bib IRN (a work saves the work).
        assertEquals("123", PickIrn.of(BookRef(id = "work:123", title = "A"), recordDetail(bibIrn = "999")))
    }

    @Test
    fun otherRefsWaitForTheRecordsBibIrn() {
        for (id in listOf("brn:1520374", "url:https://canning.spydus.com/x", "title:Something")) {
            val ref = BookRef(id = id, title = "A")
            assertNull(id, PickIrn.of(ref))
            assertNull(id, PickIrn.of(ref, recordDetail(bibIrn = null)))
            assertEquals(id, "4321", PickIrn.of(ref, recordDetail(bibIrn = " 4321 ")))
        }
    }

    @Test
    fun onlyNumbersAreIrns() {
        assertNull(PickIrn.ofKey("bib:demo-1"))
        assertNull(PickIrn.ofKey("work:"))
        assertNull(PickIrn.of(BookRef(id = "brn:1", title = "A"), recordDetail(bibIrn = "12a")))
    }

    @Test
    fun theBookmarkSignedInIsMyPicks() {
        val waiting = DetailBookmark.myPicks(irn = null, picked = setOf("1"))
        assertFalse(waiting.enabled)
        assertFalse(waiting.saved)
        assertEquals("Save to My Picks", waiting.label)

        val saved = DetailBookmark.myPicks(irn = "1", picked = setOf("1"))
        assertTrue(saved.enabled)
        assertTrue(saved.saved)
        assertEquals("Remove from My Picks", saved.label)

        assertEquals(DetailBookmark(saved = false, enabled = true, label = Copy.Picks.save), DetailBookmark.myPicks("2", setOf("1")))
    }

    @Test
    fun theBookmarkSignedOutSaysMyPicksToo() {
        assertEquals(DetailBookmark(true, true, "Remove from My Picks"), DetailBookmark.local(true))
        assertEquals(DetailBookmark(false, true, "Save to My Picks"), DetailBookmark.local(false))
    }

    @Test
    fun favouritesIsMyPicksSignedInAndLocalSignedOut() {
        assertEquals(FavouritesMode.MY_PICKS, FavouritesMode.of(signedIn = true))
        assertEquals(FavouritesMode.LOCAL, FavouritesMode.of(signedIn = false))
    }

    @Test
    fun rowsAreSearchResultRowsAndAnEditionLevelItemHasNoChip() {
        val work = FakeBackend.sampleRecord()
        val edition = pickRecord("8753740", "The Thursday Murder Club").copy(
            formats = listOf(RecordFormat("", null, "https://x/reserve", "https://x/BIBENQ?SETLVL=&IRN=8753740&FMT=RECFMT")),
        )
        val content = MyPicksContent.of(PicksList(phase = PicksList.Phase.Loaded, records = listOf(work, edition, work)))
        val rows = (content as MyPicksContent.Rows).rows
        assertEquals(listOf("Book", "eBook"), rows[0].item.formats.map { it.label })
        assertEquals("15643445", rows[0].irn)
        assertTrue(rows[1].item.formats.isEmpty())
        assertEquals("8753740", rows[1].irn)
        assertEquals("work:8753740", rows[1].item.ref.id)
        // Lazy keys stay unique even if the list repeats a card.
        assertEquals(3, rows.map { it.item.id }.toSet().size)
    }

    @Test
    fun loadingAndIdleShowTheSkeleton() {
        assertEquals(MyPicksContent.Loading, MyPicksContent.of(PicksList()))
        assertEquals(MyPicksContent.Loading, MyPicksContent.of(PicksList(phase = PicksList.Phase.Loading)))
    }

    @Test
    fun theOfferAnswerIsKeptInPreferencesPerLibrary() {
        val prefs = FakePreferences()
        val store = SharedPreferencesPicksOfferStore(prefs)
        assertFalse(store.wasAsked("https://canning.spydus.com"))
        store.markAsked("https://canning.spydus.com")
        assertTrue(SharedPreferencesPicksOfferStore(prefs).wasAsked("https://canning.spydus.com"))
        assertFalse(store.wasAsked("demo:https://canning.spydus.com"))
        assertFalse(store.wasAsked("https://hutt.spydus.co.nz"))
        // Keys are hashed, like the alerts store's.
        assertTrue(prefs.all.keys.single().startsWith("copyOffered."))
        assertFalse(prefs.all.keys.single().contains("canning"))
        store.clear()
        assertFalse(store.wasAsked("https://canning.spydus.com"))
    }
}

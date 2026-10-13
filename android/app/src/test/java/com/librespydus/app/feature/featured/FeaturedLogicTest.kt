package com.librespydus.app.feature.featured

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.Route
import com.librespydus.app.core.Tab
import com.librespydus.app.feature.search.SearchResultItem
import com.librespydus.app.ui.nav.AppNavigator
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.FeaturedKind
import uniffi.librespydus_core.RecordFormat

class FeaturedLogicTest {
    private val f = FeaturedFixtures

    @Test
    fun iconsMapTheLibrarysFontAwesomeNames() {
        val expected = mapOf(
            "book" to FeaturedIcon.BOOK,
            "balloons" to FeaturedIcon.CELEBRATION,
            "computer" to FeaturedIcon.COMPUTER,
            "circle" to FeaturedIcon.CIRCLE,
            "globe" to FeaturedIcon.GLOBE,
            "game-board" to FeaturedIcon.PUZZLE,
            "map-location-dot" to FeaturedIcon.MAP,
            "user-circle" to FeaturedIcon.PERSON_CIRCLE,
        )
        expected.forEach { (name, icon) -> assertEquals(name, icon, FeaturedIcon.forName(name)) }
        assertEquals(FeaturedIcon.GLOBE, FeaturedIcon.forName(" Globe "))
        assertEquals(FeaturedIcon.BOOK, FeaturedIcon.forName("fa-book"))
    }

    @Test
    fun unknownOrMissingIconsFallBackToSparkles() {
        assertEquals(FeaturedIcon.SPARKLES, FeaturedIcon.forName(null))
        assertEquals(FeaturedIcon.SPARKLES, FeaturedIcon.forName(""))
        assertEquals(FeaturedIcon.SPARKLES, FeaturedIcon.forName("rocket"))
        // Every icon has a vector (the fallback included).
        FeaturedIcon.entries.forEach { it.vector() }
    }

    @Test
    fun tappingALinkRoutesByItsKind() {
        val collection = f.link("Library of Things", FeaturedKind.COLLECTION, f.LOT_URL)
        val search = f.link("Coming Soon", FeaturedKind.SEARCH, "${f.ORIGIN}/cgi-bin/spydus.exe/RUNSQRY/OPAC/BIBENQ?SQRYIRN=1")
        val web = f.link("eResources", FeaturedKind.WEB, "https://example.org/eresources")

        assertEquals(FeaturedDestination.Collection(Route.Collection("Library of Things", f.LOT_URL)), FeaturedRouting.destination(collection))
        assertEquals(FeaturedDestination.Results(Route.ListResults(search.url, "Coming Soon")), FeaturedRouting.destination(search))
        assertEquals(FeaturedDestination.Web("https://example.org/eresources"), FeaturedRouting.destination(web))
    }

    @Test
    fun openFeaturedCallsTheMatchingNavigatorAction() {
        val calls = mutableListOf<String>()
        val navigator = object : AppNavigator {
            override fun openBook(book: BookRef) = Unit
            override fun openResults(query: String, sort: ResultsSort, title: String?) = Unit
            override fun back() = Unit
            override fun switchTab(tab: Tab) = Unit
            override fun openSearch(query: String) = Unit
            override fun openCollection(title: String, url: String) { calls += "collection $title $url" }
            override fun openList(url: String, title: String) { calls += "list $title $url" }
            override fun openWeb(url: String) { calls += "web $url" }
        }
        navigator.openFeatured(f.link("LoT", FeaturedKind.COLLECTION, "c"))
        navigator.openFeatured(f.link("Soon", FeaturedKind.SEARCH, "s"))
        navigator.openFeatured(f.link("Web", FeaturedKind.WEB, "w"))
        assertEquals(listOf("collection LoT c", "list Soon s", "web w"), calls)
    }

    @Test
    fun editionLevelIsExactlyOneFormatWithAnEmptyCodeAndNoLabel() {
        val edition = FormatRef.from(f.editionFormat("1"))
        assertTrue(EditionLevel.isEditionLevel(edition))
        assertTrue(EditionLevel.isEditionLevel(listOf(edition)))
        assertTrue(EditionLevel.isEditionLevel(listOf(edition.copy(code = "  ", label = " "))))

        assertFalse(EditionLevel.isEditionLevel(emptyList()))
        assertFalse(EditionLevel.isEditionLevel(listOf(edition, edition)))
        assertFalse(EditionLevel.isEditionLevel(listOf(FormatRef("BK", "Book", "r", "a"))))
        assertFalse(EditionLevel.isEditionLevel(listOf(edition.copy(label = "Book"))))
        assertFalse(EditionLevel.isEditionLevel(listOf(edition.copy(code = "BK"))))
    }

    @Test
    fun editionLevelResultRowsHaveNoFormatChip() {
        val row = SearchResultItem.from(f.editionRecord("10936881"), "work:10936881", 0)
        assertTrue(row.formats.isEmpty())
        assertNull(row.ref.author)
        // The format stays on the ref: the book page loads through it, and favourites keep it.
        assertEquals(listOf(FormatRef.from(f.editionFormat("10936881"))), row.ref.formats)

        val book = f.editionRecord("2").copy(
            formats = listOf(RecordFormat("BK", "Book", "r", "a"), RecordFormat("LP", "Large print", "r2", "a2")),
        )
        assertEquals(listOf("Book", "Large print"), SearchResultItem.from(book, "work:2", 0).formats.map { it.label })
    }

    @Test
    fun copyRowsTitleCaseTheBranchAndKeepTheRest() {
        val rows = CopyRow.rows(
            listOf(
                f.holding("RIVERTON LIBRARY", "On loan - Due: 29 Oct 2026", available = false, callNumber = "LOT MUS004"),
                f.holding("Cannington Library", " Available ", available = true, callNumber = "LOT GAM012", description = "Pink body, teal handle"),
                f.holding(null, null, available = false), // nothing to show
            ),
        )
        assertEquals(
            listOf(
                CopyRow("Riverton Library", "On loan - Due: 29 Oct 2026", false, null, "LOT MUS004"),
                CopyRow("Cannington Library", "Available", true, "Pink body, teal handle", "LOT GAM012"),
            ),
            rows,
        )
        assertEquals("Cannington Library, Available, Pink body, teal handle, Call number LOT GAM012", rows[1].accessibilityLabel)
    }
}

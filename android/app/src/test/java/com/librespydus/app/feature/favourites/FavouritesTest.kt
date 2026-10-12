package com.librespydus.app.feature.favourites

import androidx.compose.ui.unit.dp
import com.librespydus.app.core.BookRef
import com.librespydus.app.feature.onboarding.CANNING
import com.librespydus.app.feature.onboarding.HUTT
import com.librespydus.app.feature.onboarding.accountFixture
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class FavouritesTest {
    @get:Rule val tmp = TemporaryFolder()
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    @Test
    fun theGridShowsTheSelectedLibrarysFavouritesNewestFirst() = runTest {
        val f = accountFixture(tmp.root, library = CANNING).also { scopes += it.scope }
        f.state.toggleFavourite(BookRef(id = "brn:1", title = "Canning book"))
        f.state.toggleFavourite(BookRef(id = "brn:2", title = "Newer Canning book"))
        advanceUntilIdle()

        f.state.selectLibrary(HUTT)
        advanceUntilIdle()
        assertTrue(f.state.currentFavourites.value.isEmpty())
        f.state.toggleFavourite(BookRef(id = "brn:9", title = "Hutt book"))
        advanceUntilIdle()
        assertEquals(listOf("Hutt book"), f.state.currentFavourites.value.map { it.title })

        f.state.selectLibrary(CANNING)
        advanceUntilIdle()
        assertEquals(listOf("Newer Canning book", "Canning book"), f.state.currentFavourites.value.map { it.title })
    }

    @Test
    fun removingFromTheGridForgetsTheBook() = runTest {
        val f = accountFixture(tmp.root).also { scopes += it.scope }
        val ref = BookRef(id = "brn:1", title = "Book")
        f.state.toggleFavourite(ref)
        advanceUntilIdle()
        assertTrue(f.state.isFavourite(ref.id))
        f.state.removeFavourite(ref.id)
        advanceUntilIdle()
        assertFalse(f.state.isFavourite(ref.id))
        assertTrue(f.state.currentFavourites.value.isEmpty())
    }

    @Test
    fun twoColumnsFitTheScreenGutters() {
        // Pixel 9: 411 dp wide, 16 dp gutters and gap.
        assertEquals(181.dp, FavouritesLayout.columnWidth(411.dp))
        assertEquals(177.dp, FavouritesLayout.columnWidth(402.dp))
        assertEquals(80.dp, FavouritesLayout.columnWidth(0.dp))
    }

    @Test
    fun headerCountsTheLibrarysBooks() {
        assertEquals("7 saved for City of Canning", FavouritesLayout.subtitle(7, "City of Canning"))
        assertEquals("Saved on this device for City of Canning", FavouritesLayout.subtitle(0, "City of Canning"))
    }
}

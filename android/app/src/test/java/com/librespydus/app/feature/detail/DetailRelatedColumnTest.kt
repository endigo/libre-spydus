package com.librespydus.app.feature.detail

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseLoadable
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordRelated

/**
 * When the wide book page shows its related column (docs/native-apps.md §6.1): while the book
 * loads, and once it has other books to show; not for a failed record or one without any.
 */
class DetailRelatedColumnTest {
    private val ref = BookRef(id = "work:275039", title = "The midnight library", author = "Haig, Matt", coverUrl = null, year = "2020", url = "https://canning.spydus.com/x")
    private val links = RecordRelated("https://canning.spydus.com/similar", "https://canning.spydus.com/author", null, null)
    private fun record(related: RecordRelated) =
        RecordDetail("8753080", "275039", "The midnight library", null, null, emptyList(), "A library.", emptyList(), null, related)

    @Test
    fun shownWhileTheBookLoads() {
        assertTrue(DetailRelatedColumn.isShown(DetailState(ref)))
        assertTrue(DetailRelatedColumn.isShown(DetailState(ref, record = BrowseLoadable.Loading)))
    }

    @Test
    fun shownForABookWithRelatedLinksBeforeAndAfterTheyLoad() {
        val loaded = DetailState(ref, record = BrowseLoadable.Loaded(record(links)))
        assertTrue(DetailRelatedColumn.isShown(loaded))
        val loading = loaded.copy(related = mapOf(RelatedPanel.SIMILAR to BrowseLoadable.Loading, RelatedPanel.AUTHOR to BrowseLoadable.Loading))
        assertTrue(DetailRelatedColumn.isShown(loading))
        val withBooks = loaded.copy(related = mapOf(RelatedPanel.SIMILAR to BrowseLoadable.Loaded(listOf(ref)), RelatedPanel.AUTHOR to BrowseLoadable.Loaded(emptyList())))
        assertTrue(DetailRelatedColumn.isShown(withBooks))
    }

    @Test
    fun hiddenOnceEveryPanelTurnsOutEmptyOrFails() {
        val loaded = DetailState(ref, record = BrowseLoadable.Loaded(record(links)))
        val empty = loaded.copy(
            related = mapOf(
                RelatedPanel.SIMILAR to BrowseLoadable.Loaded(emptyList()),
                RelatedPanel.AUTHOR to BrowseLoadable.Failed(UserFacingError(UserFacingError.Kind.NETWORK, "Offline")),
            ),
        )
        assertFalse(DetailRelatedColumn.isShown(empty))
    }

    @Test
    fun hiddenForABookWithoutRelatedLinks() {
        val none = DetailState(ref, record = BrowseLoadable.Loaded(record(RecordRelated(null, null, null, null))))
        assertFalse(DetailRelatedColumn.isShown(none))
    }

    @Test
    fun hiddenWhenTheBookFailedToLoad() {
        val failed = DetailState(ref, record = BrowseLoadable.Failed(UserFacingError(UserFacingError.Kind.NETWORK, "Offline")))
        assertFalse(DetailRelatedColumn.isShown(failed))
    }
}

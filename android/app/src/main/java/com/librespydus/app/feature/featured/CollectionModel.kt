package com.librespydus.app.feature.featured

import androidx.lifecycle.ViewModel
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.SearchResultsModel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import uniffi.librespydus_core.FeaturedCollection
import uniffi.librespydus_core.FeaturedSection

/**
 * The Collection screen (§5.10): `collection(url)`, then one shelf per section, each the first
 * `page(section.url)` as covers. Shelves load when they come into view ([loadShelf]), at most
 * [MAX_CONCURRENT] at a time; a section that comes back empty is hidden and one that fails keeps
 * its row with Retry. Calls run on the main thread (the screen launches them in the model's scope,
 * so pushing a book mid-load doesn't cancel them).
 */
class CollectionModel(
    /** The collection page (the Featured link's URL). */
    val url: String,
) : ViewModel() {
    /** What the page was loaded for: a new library, verification or Retry loads it again. */
    data class Request(val origin: String, val verification: Int, val attempt: Int = 0)

    sealed interface ShelfState {
        /** Not in view yet. */
        data object Idle : ShelfState

        /** In view: waiting for a slot or loading. */
        data object Loading : ShelfState
        data class Loaded(val books: List<BookRef>) : ShelfState

        /** The section has nothing: its shelf is hidden. */
        data object Empty : ShelfState
        data class Failed(val error: UserFacingError) : ShelfState
    }

    data class Shelf(val section: FeaturedSection, val state: ShelfState = ShelfState.Idle) {
        /** Unique on the page (two sections may share a title). */
        val id: String get() = section.url

        val isHidden: Boolean get() = state == ShelfState.Empty
    }

    data class State(
        val page: BrowseLoadable<FeaturedCollection> = BrowseLoadable.Idle,
        val shelves: List<Shelf> = emptyList(),
        /** Bumped with every page load, so the screen asks for the new shelves again. */
        val generation: Int = 0,
    ) {
        /** The page loaded and has no saved searches (eLibrary: links to outside services). */
        val hasNoSections: Boolean get() = page is BrowseLoadable.Loaded && shelves.isEmpty()

        /** Shelves to draw, in page order: empty sections are left out. */
        val visibleShelves: List<Shelf> get() = shelves.filterNot { it.isHidden }

        /** Every section came back empty: nothing to show but the library's website. */
        val allSectionsEmpty: Boolean get() = shelves.isNotEmpty() && shelves.all { it.isHidden }
    }

    private val _state = MutableStateFlow(State())
    val state: StateFlow<State> = _state.asStateFlow()

    private val permits = Semaphore(MAX_CONCURRENT)
    private var loadedRequest: Request? = null
    private var loadingRequest: Request? = null

    /** Loads the page once per request (coming back from a book doesn't reload it). */
    suspend fun load(request: Request, backend: LibraryBackend, report: BrowseReporter) {
        if (request == loadingRequest || (request == loadedRequest && _state.value.page !is BrowseLoadable.Idle)) return
        loadingRequest = request
        val generation = _state.value.generation + 1
        _state.value = State(page = BrowseLoadable.Loading, generation = generation)
        try {
            val collection = backend.collection(url)
            if (generation != _state.value.generation) return
            loadedRequest = request
            val shelves = collection.sections
                .filter { it.title.isNotBlank() && it.url.isNotBlank() }
                .distinctBy { it.url }
                .map { Shelf(it) }
            _state.update { it.copy(page = BrowseLoadable.Loaded(collection), shelves = shelves) }
        } catch (e: CancellationException) {
            if (generation == _state.value.generation) _state.update { it.copy(page = BrowseLoadable.Idle) }
            throw e
        } catch (e: Exception) {
            if (generation != _state.value.generation) return
            val shown = report(e)
            loadedRequest = request
            _state.update { it.copy(page = if (shown != null) BrowseLoadable.Failed(shown) else BrowseLoadable.Idle) }
        } finally {
            if (loadingRequest == request) loadingRequest = null
        }
    }

    /**
     * The shelf with [id] came into view: loads its first page once, waiting for one of the
     * [MAX_CONCURRENT] slots. With [retry], a failed shelf loads again.
     */
    suspend fun loadShelf(id: String, backend: LibraryBackend, report: BrowseReporter, retry: Boolean = false) {
        val generation = _state.value.generation
        val shelf = _state.value.shelves.firstOrNull { it.id == id } ?: return
        val startable = shelf.state == ShelfState.Idle || (retry && shelf.state is ShelfState.Failed)
        if (!startable) return
        setShelf(id, generation, ShelfState.Loading)
        try {
            val results = permits.withPermit {
                // The page was reloaded while this shelf waited for a slot.
                if (generation != _state.value.generation) return
                backend.page(shelf.section.url)
            }
            val books = SearchResultsModel.merge(emptyList(), results.records).map { it.ref }.distinctBy { it.id }.take(SHELF_LIMIT)
            setShelf(id, generation, if (books.isEmpty()) ShelfState.Empty else ShelfState.Loaded(books))
        } catch (e: CancellationException) {
            setShelf(id, generation, ShelfState.Idle)
            throw e
        } catch (e: Exception) {
            if (generation != _state.value.generation) return
            setShelf(id, generation, report(e)?.let { ShelfState.Failed(it) } ?: ShelfState.Idle)
        }
    }

    private fun setShelf(id: String, generation: Int, state: ShelfState) {
        _state.update { current ->
            if (current.generation != generation) current
            else current.copy(shelves = current.shelves.map { if (it.id == id) it.copy(state = state) else it })
        }
    }

    companion object {
        /** Section pages loading at once (What's New has 11 sections). */
        const val MAX_CONCURRENT = 3

        /** Covers per shelf: the section's first page (the OPAC gives 20). */
        const val SHELF_LIMIT = 20
    }
}

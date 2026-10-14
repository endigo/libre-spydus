package com.librespydus.app.feature.featured

import androidx.lifecycle.ViewModel
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.feature.home.HomeReloadKey
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import uniffi.librespydus_core.FeaturedLink

/**
 * Home's "Featured" row (§5.10): the library's own collections from `featured()`, loaded on its
 * own beside the other shelves. Most libraries have no such list, so the row is [links] only when
 * there are some: empty, still loading and failed all hide it, and a failure is never reported on
 * Home (a challenge or lost connection shows on New arrivals anyway). The tiles saved on the device
 * show while it loads and stay when it fails (§5.13).
 */
class HomeFeaturedModel : ViewModel() {
    private val _links = MutableStateFlow<List<FeaturedLink>>(emptyList())

    /** The tiles to show; empty → no "Featured" section. */
    val links: StateFlow<List<FeaturedLink>> = _links.asStateFlow()

    private var loadedKey: HomeReloadKey? = null
    private var loadingKey: HomeReloadKey? = null

    /** Bumped by each load: a late answer for an older load is dropped. */
    private var generation = 0

    /** Loads once per key (library, verification); coming back to Home doesn't reload. */
    suspend fun loadIfNeeded(backend: LibraryBackend, key: HomeReloadKey) {
        if (key != loadedKey && key != loadingKey) load(backend, key, keepContent = false)
    }

    /** Pull to refresh: the tiles stay while reloading, and a failure keeps them. */
    suspend fun refresh(backend: LibraryBackend, key: HomeReloadKey) = load(backend, key, keepContent = true)

    private suspend fun load(backend: LibraryBackend, key: HomeReloadKey, keepContent: Boolean) {
        val generation = ++generation
        loadingKey = key
        // Another library's tiles never show over this one's; its own saved ones do.
        if (!keepContent || key.origin != loadedKey?.origin) {
            _links.value = emptyList()
            val saved = backend.savedFeatured()
            if (saved != null && generation == this.generation) _links.value = usable(saved)
        }
        try {
            val links = backend.featured()
            if (generation != this.generation) return
            loadedKey = key
            _links.value = usable(links)
            backend.saveFeatured(links)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (generation != this.generation) return
            loadedKey = key
        } finally {
            if (generation == this.generation) loadingKey = null
        }
    }

    private fun usable(links: List<FeaturedLink>) = links.filter { it.title.isNotBlank() && it.url.isNotBlank() }
}

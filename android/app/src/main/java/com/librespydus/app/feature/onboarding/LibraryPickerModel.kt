package com.librespydus.app.feature.onboarding

import com.librespydus.app.core.StoredLibrary
import com.librespydus.app.core.UserFacingError
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import uniffi.librespydus_core.LibraryInfo
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * The directory on disk (the app's cache dir), so the picker opens instantly and offline after
 * the first fetch. Refreshed when older than [MAX_AGE_MILLIS] or on pull-to-refresh.
 */
class LibraryDirectoryCache(private val file: File) {
    data class Snapshot(val fetchedAt: Long, val libraries: List<LibraryInfo>)

    /** The cached directory, or null when there's none or it can't be read. */
    fun load(): Snapshot? = runCatching {
        val stored = json.decodeFromString(StoredSnapshot.serializer(), file.readText())
        if (stored.version != VERSION) return null
        Snapshot(stored.fetchedAt, stored.libraries.map { it.toInfo() })
    }.getOrNull()

    fun save(snapshot: Snapshot) {
        file.parentFile?.mkdirs()
        val text = json.encodeToString(
            StoredSnapshot.serializer(),
            StoredSnapshot(fetchedAt = snapshot.fetchedAt, libraries = snapshot.libraries.map(StoredLibrary::from)),
        )
        val tmp = File(file.parentFile, "${file.name}.tmp")
        tmp.writeText(text)
        if (!tmp.renameTo(file)) {
            file.writeText(text)
            tmp.delete()
        }
    }

    fun remove() {
        file.delete()
    }

    @Serializable
    private data class StoredSnapshot(val version: Int = VERSION, val fetchedAt: Long, val libraries: List<StoredLibrary>)

    companion object {
        const val FILE_NAME = "library-directory.json"
        val MAX_AGE_MILLIS: Long = TimeUnit.DAYS.toMillis(7)
        private const val VERSION = 1
        private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

        /**
         * Fresh when fetched less than [maxAge] ago. A date in the future (the clock moved back)
         * is stale, so a wrong clock can't pin an old list forever.
         */
        fun isFresh(fetchedAt: Long, now: Long, maxAge: Long = MAX_AGE_MILLIS): Boolean {
            val age = now - fetchedAt
            return age in 0 until maxAge
        }
    }
}

/**
 * The library picker's state: the Civica directory (cached on disk), the search text and the
 * sections to show.
 *
 * [load] shows the cached list at once and refreshes it in the background when it's older than
 * a week; without a cache it fetches (one public GET) and shows loading / error + Retry.
 */
class LibraryPickerModel(
    private val fetch: suspend () -> List<LibraryInfo>,
    private val cache: LibraryDirectoryCache,
    private val now: () -> Long = System::currentTimeMillis,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) {
    private val _libraries = MutableStateFlow<List<LibraryInfo>>(emptyList())
    private val _fetchedAt = MutableStateFlow<Long?>(null)
    private val _isFetching = MutableStateFlow(false)
    private val _loadError = MutableStateFlow<UserFacingError?>(null)
    private val _refreshError = MutableStateFlow<UserFacingError?>(null)
    private val _query = MutableStateFlow("")
    private val _loaded = MutableStateFlow(false)
    private var started = false

    val libraries: StateFlow<List<LibraryInfo>> = _libraries.asStateFlow()

    /** When [libraries] was fetched from the directory (epoch millis). */
    val fetchedAt: StateFlow<Long?> = _fetchedAt.asStateFlow()
    val isFetching: StateFlow<Boolean> = _isFetching.asStateFlow()

    /** The list couldn't be fetched and there's nothing cached to show. */
    val loadError: StateFlow<UserFacingError?> = _loadError.asStateFlow()

    /** A refresh failed while an older list is on screen (shown as a banner). */
    val refreshError: StateFlow<UserFacingError?> = _refreshError.asStateFlow()
    val query: StateFlow<String> = _query.asStateFlow()

    /** False until [load] has shown the cached list or finished its first fetch (show loading meanwhile). */
    val loaded: StateFlow<Boolean> = _loaded.asStateFlow()

    fun setQuery(text: String) {
        _query.value = text
    }

    val sections: List<PickerSection> get() = LibraryPickerLogic.sections(_libraries.value, _query.value)

    /** Nothing to show yet and the first load or a fetch is running. */
    val isLoading: Boolean get() = _libraries.value.isEmpty() && (_isFetching.value || !_loaded.value)

    val isSearching: Boolean get() = _query.value.isNotBlank()

    /** First appearance: the cached list, refreshed when stale or missing. Later calls do nothing. */
    suspend fun load() {
        if (started) return
        started = true
        val snapshot = withContext(io) { cache.load() }
        if (snapshot != null && snapshot.libraries.isNotEmpty()) {
            _libraries.value = snapshot.libraries
            _fetchedAt.value = snapshot.fetchedAt
            _loaded.value = true
            if (LibraryDirectoryCache.isFresh(snapshot.fetchedAt, now())) return
        }
        try {
            refresh()
        } finally {
            _loaded.value = true
        }
    }

    /**
     * Fetches the directory (pull-to-refresh, Retry, stale cache) and caches it. On failure the
     * list on screen stays.
     */
    suspend fun refresh() {
        if (_isFetching.value) return
        _isFetching.value = true
        try {
            val fetched = fetch()
            val snapshot = LibraryDirectoryCache.Snapshot(now(), fetched)
            _libraries.value = fetched
            _fetchedAt.value = snapshot.fetchedAt
            _loadError.value = null
            _refreshError.value = null
            withContext(io) { runCatching { cache.save(snapshot) } }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            val error = userFacingError(e)
            if (_libraries.value.isEmpty()) _loadError.value = error else _refreshError.value = error
        } finally {
            _isFetching.value = false
        }
    }

    suspend fun retry() {
        _loadError.value = null
        refresh()
    }

    companion object {
        /** The directory isn't a library, so the §5.7 messages name it instead. */
        fun userFacingError(error: Throwable): UserFacingError {
            val shown = UserFacingError.from(error, "the library directory")
            return when (shown.kind) {
                UserFacingError.Kind.NETWORK ->
                    shown.copy(message = "Can't reach the library directory. Check your connection.")
                else -> shown.copy(
                    kind = UserFacingError.Kind.UNEXPECTED,
                    message = "The library directory returned something unexpected.",
                )
            }
        }
    }
}

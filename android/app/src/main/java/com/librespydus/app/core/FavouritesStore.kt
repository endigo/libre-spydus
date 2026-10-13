package com.librespydus.app.core

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File

/**
 * Local favourites (§5.3): a list of [BookRef] per library origin, kept in one JSON file in the
 * app's files dir. Never synced anywhere. Lists are newest first ([BookRef.savedAt]).
 *
 * Call [load] once before use (AppState does it at start-up); mutations persist immediately.
 */
class FavouritesStore(
    private val file: File,
    private val now: () -> Long = System::currentTimeMillis,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) {
    private val mutex = Mutex()
    private val _all = MutableStateFlow<Map<String, List<BookRef>>>(emptyMap())

    /** Every library's favourites, keyed by origin. */
    val all: StateFlow<Map<String, List<BookRef>>> = _all.asStateFlow()

    suspend fun load() = mutex.withLock {
        _all.value = withContext(io) {
            if (!file.exists()) return@withContext emptyMap()
            decode(file.readText()) ?: run {
                // Keep an unreadable file aside instead of overwriting it with the next save.
                file.renameTo(File(file.parentFile, "${file.name}.unreadable"))
                emptyMap()
            }
        }
    }

    fun favourites(origin: String): Flow<List<BookRef>> =
        all.map { it[origin].orEmpty() }.distinctUntilChanged()

    fun list(origin: String): List<BookRef> = _all.value[origin].orEmpty()

    fun contains(origin: String, id: String): Boolean = list(origin).any { it.id == id }

    /** Adds [book] (stamped with `savedAt`) or removes it; returns true if it is now saved. */
    suspend fun toggle(origin: String, book: BookRef): Boolean = mutate(origin) { list ->
        if (list.any { it.id == book.id }) list.filterNot { it.id == book.id }
        else listOf(book.copy(savedAt = now())) + list
    }.let { saved -> saved.any { it.id == book.id } }

    suspend fun remove(origin: String, id: String) {
        mutate(origin) { list -> list.filterNot { it.id == id } }
    }

    /** Drops every library's favourites (debug `ls_reset`). */
    suspend fun clear() = mutex.withLock {
        _all.value = emptyMap()
        withContext(io) { file.delete() }
    }

    private suspend fun mutate(origin: String, change: (List<BookRef>) -> List<BookRef>): List<BookRef> =
        mutex.withLock {
            val updated = change(list(origin)).sortedByDescending { it.savedAt ?: 0L }
            val all = _all.value.toMutableMap().apply {
                if (updated.isEmpty()) remove(origin) else put(origin, updated)
            }
            withContext(io) { write(encode(all)) }
            _all.value = all
            updated
        }

    private fun write(text: String) {
        file.parentFile?.mkdirs()
        val tmp = File(file.parentFile, "${file.name}.tmp")
        tmp.writeText(text)
        if (!tmp.renameTo(file)) {
            file.writeText(text)
            tmp.delete()
        }
    }

    @Serializable
    internal data class FileFormat(val version: Int = VERSION, val libraries: Map<String, List<BookRef>> = emptyMap())

    companion object {
        const val FILE_NAME = "favourites.json"
        private const val VERSION = 1
        private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

        internal fun encode(all: Map<String, List<BookRef>>): String =
            json.encodeToString(FileFormat.serializer(), FileFormat(libraries = all))

        /** Null when [text] isn't a favourites file. */
        internal fun decode(text: String): Map<String, List<BookRef>>? =
            runCatching { json.decodeFromString(FileFormat.serializer(), text).libraries }.getOrNull()
    }
}

package com.librespydus.app.feature.search

import android.content.Context
import android.content.SharedPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.json.Json

/** The Search tab's recent queries: local only, newest first, at most [LIMIT]. */
class RecentSearchStore(private val storage: Storage) {
    /** Where the list lives (SharedPreferences in the app, memory in tests). */
    interface Storage {
        fun read(): List<String>
        fun write(items: List<String>)
    }

    private val _items = MutableStateFlow(storage.read().take(LIMIT))
    val items: StateFlow<List<String>> = _items.asStateFlow()

    /**
     * Moves [query] (trimmed, inner whitespace collapsed) to the top; a case-insensitive repeat
     * replaces the older entry. Blank queries are ignored.
     */
    fun record(query: String) {
        val cleaned = collapseWhitespace(query)
        if (cleaned.isEmpty()) return
        save(listOf(cleaned) + _items.value.filterNot { it.equals(cleaned, ignoreCase = true) })
    }

    fun remove(query: String) = save(_items.value - query)

    fun clear() = save(emptyList())

    private fun save(items: List<String>) {
        val trimmed = items.take(LIMIT)
        _items.value = trimmed
        storage.write(trimmed)
    }

    /** SharedPreferences storage: a JSON array under [KEY]; removed when empty. */
    class PreferencesStorage(private val prefs: SharedPreferences) : Storage {
        override fun read(): List<String> =
            prefs.getString(KEY, null)?.let { runCatching { json.decodeFromString(serializer, it) }.getOrNull() }.orEmpty()

        override fun write(items: List<String>) {
            prefs.edit().apply {
                if (items.isEmpty()) remove(KEY) else putString(KEY, json.encodeToString(serializer, items))
            }.apply()
        }

        private companion object {
            val json = Json
            val serializer = ListSerializer(String.serializer())
        }
    }

    companion object {
        const val LIMIT = 8
        const val KEY = "LibreSpydus.recentSearches"
        private const val PREFS = "librespydus.search"

        @Volatile private var shared: RecentSearchStore? = null

        /** The app's one store (the Search tab is its only user). */
        fun shared(context: Context): RecentSearchStore = shared ?: synchronized(this) {
            shared ?: RecentSearchStore(
                PreferencesStorage(context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)),
            ).also { shared = it }
        }

        /** "  harry   potter " → "harry potter". */
        fun collapseWhitespace(text: String): String = text.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }.joinToString(" ")
    }
}

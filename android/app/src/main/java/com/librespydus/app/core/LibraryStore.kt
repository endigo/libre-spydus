package com.librespydus.app.core

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import uniffi.librespydus_core.LibraryInfo

/** The selected library, persisted across launches (§5.2). Null → onboarding. */
interface LibraryStore {
    suspend fun get(): LibraryInfo?
    suspend fun set(library: LibraryInfo?)
}

private val Context.settingsDataStore: DataStore<Preferences> by preferencesDataStore(name = "settings")

/** [LibraryStore] in Preferences DataStore (`settings`), the library as JSON. */
class DataStoreLibraryStore(context: Context) : LibraryStore {
    private val store = context.applicationContext.settingsDataStore

    override suspend fun get(): LibraryInfo? =
        store.data.first()[KEY]?.let { text ->
            runCatching { json.decodeFromString(StoredLibrary.serializer(), text).toInfo() }.getOrNull()
        }

    override suspend fun set(library: LibraryInfo?) {
        store.edit { prefs ->
            if (library == null) prefs.remove(KEY)
            else prefs[KEY] = json.encodeToString(StoredLibrary.serializer(), StoredLibrary.from(library))
        }
    }

    private companion object {
        val KEY = stringPreferencesKey("library")
        val json = Json { ignoreUnknownKeys = true }
    }
}

class InMemoryLibraryStore(private var library: LibraryInfo? = null) : LibraryStore {
    override suspend fun get() = library
    override suspend fun set(library: LibraryInfo?) {
        this.library = library
    }
}

/** Serializable copy of the generated [LibraryInfo]. */
@Serializable
internal data class StoredLibrary(
    val id: String,
    val name: String,
    val origin: String,
    val logoUrl: String? = null,
    val country: String? = null,
    val region: String = "OTHER",
    val isTest: Boolean = false,
) {
    fun toInfo() = LibraryInfo(id, name, origin, logoUrl, country, region, isTest)

    companion object {
        fun from(info: LibraryInfo) =
            StoredLibrary(info.id, info.name, info.origin, info.logoUrl, info.country, info.region, info.isTest)
    }
}

package com.librespydus.app.feature.search

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class RecentSearchStoreTest {
    /** Stands in for SharedPreferences: null when the key is removed. */
    private class MemoryStorage : RecentSearchStore.Storage {
        var stored: List<String>? = null
        override fun read() = stored.orEmpty()
        override fun write(items: List<String>) {
            stored = items.ifEmpty { null }
        }
    }

    @Test
    fun recordsNewestFirstTrimmedAndWithoutCaseInsensitiveRepeats() {
        val store = RecentSearchStore(MemoryStorage())
        store.record("  harry   potter ")
        store.record("Matt Haig")
        store.record("HARRY POTTER")
        store.record("   ")
        assertEquals(listOf("HARRY POTTER", "Matt Haig"), store.items.value)
    }

    @Test
    fun keepsTheLastEight() {
        val store = RecentSearchStore(MemoryStorage())
        for (index in 1..10) store.record("query $index")
        assertEquals(RecentSearchStore.LIMIT, store.items.value.size)
        assertEquals("query 10", store.items.value.first())
        assertEquals("query 3", store.items.value.last())
    }

    @Test
    fun persistsRemovesAndClears() {
        val storage = MemoryStorage()
        val store = RecentSearchStore(storage)
        store.record("cooking")
        store.record("crime")
        store.record("history")
        assertEquals(listOf("history", "crime", "cooking"), RecentSearchStore(storage).items.value)

        store.remove("crime")
        assertEquals(listOf("history", "cooking"), RecentSearchStore(storage).items.value)

        store.clear()
        assertTrue(store.items.value.isEmpty())
        assertTrue(RecentSearchStore(storage).items.value.isEmpty())
        assertNull(storage.stored)
    }

    @Test
    fun aLongStoredListIsCutToTheLimit() {
        val storage = MemoryStorage().apply { stored = (1..12).map { "q$it" } }
        assertEquals(RecentSearchStore.LIMIT, RecentSearchStore(storage).items.value.size)
    }

    @Test
    fun collapsesWhitespace() {
        assertEquals("harry potter", RecentSearchStore.collapseWhitespace("  harry \t  potter \n"))
        assertEquals("", RecentSearchStore.collapseWhitespace("   "))
    }
}

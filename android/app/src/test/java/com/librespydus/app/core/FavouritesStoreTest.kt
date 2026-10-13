package com.librespydus.app.core

import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File

class FavouritesStoreTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = "https://canning.spydus.com"
    private val hutt = "https://hutt.spydus.co.nz"
    private var clock = 1_000L

    private fun book(id: String, title: String = "Book $id") = BookRef(
        id = id,
        title = title,
        author = "Author, An",
        coverUrl = "https://covers.example/$id.jpg",
        url = "$canning/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=$id",
        isbn = "978000000$id",
        year = "2024",
        formats = listOf(FormatRef("BK", "Book", "$canning/reserve/$id", "$canning/avail/$id")),
    )

    private fun store(file: File, dispatcher: kotlinx.coroutines.CoroutineDispatcher) =
        FavouritesStore(file, now = { clock++ }, io = dispatcher)

    @Test
    fun toggleAddsNewestFirstAndRemoves() = runTest {
        val s = store(File(tmp.root, "fav.json"), StandardTestDispatcher(testScheduler))
        s.load()
        assertTrue(s.toggle(canning, book("1")))
        assertTrue(s.toggle(canning, book("2")))
        assertEquals(listOf("2", "1"), s.list(canning).map { it.id })
        assertTrue(s.contains(canning, "1"))
        assertFalse(s.toggle(canning, book("1")))
        assertFalse(s.contains(canning, "1"))
        assertEquals(listOf("2"), s.list(canning).map { it.id })
    }

    @Test
    fun listsArePerOrigin() = runTest {
        val s = store(File(tmp.root, "fav.json"), StandardTestDispatcher(testScheduler))
        s.load()
        s.toggle(canning, book("1"))
        s.toggle(hutt, book("9"))
        assertEquals(listOf("1"), s.list(canning).map { it.id })
        assertEquals(listOf("9"), s.list(hutt).map { it.id })
        s.remove(hutt, "9")
        assertTrue(s.list(hutt).isEmpty())
        assertFalse(s.all.value.containsKey(hutt))
    }

    @Test
    fun persistsEveryFieldAcrossInstances() = runTest {
        val file = File(tmp.root, "fav.json")
        val dispatcher = StandardTestDispatcher(testScheduler)
        val first = store(file, dispatcher)
        first.load()
        first.toggle(canning, book("1", title = "Ünïcode — “quoted”"))

        val second = store(file, dispatcher)
        second.load()
        val saved = second.list(canning).single()
        assertEquals(book("1", title = "Ünïcode — “quoted”").copy(savedAt = 1_000L), saved)
    }

    @Test
    fun encodeDecodeRoundTripAndFormat() {
        val all = mapOf(canning to listOf(book("1").copy(savedAt = 5), book("2").copy(savedAt = 4)))
        val text = FavouritesStore.encode(all)
        assertTrue(text.contains("\"version\":1"))
        assertTrue(text.contains("\"libraries\""))
        assertEquals(all, FavouritesStore.decode(text))
    }

    @Test
    fun decodeToleratesUnknownKeysAndMissingOptionals() {
        val text = """{"version":2,"extra":true,"libraries":{"$canning":[{"id":"bib:1","title":"T","future":1}]}}"""
        val decoded = FavouritesStore.decode(text)!!
        assertEquals(BookRef(id = "bib:1", title = "T"), decoded[canning]!!.single())
    }

    @Test
    fun unreadableFileIsKeptAsideNotOverwritten() = runTest {
        val file = File(tmp.root, "fav.json").apply { writeText("not json") }
        val s = store(file, StandardTestDispatcher(testScheduler))
        s.load()
        assertTrue(s.list(canning).isEmpty())
        assertEquals("not json", File(tmp.root, "fav.json.unreadable").readText())
        assertNull(FavouritesStore.decode("not json"))
    }
}

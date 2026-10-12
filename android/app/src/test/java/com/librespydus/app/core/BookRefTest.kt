package com.librespydus.app.core

import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.librespydus_core.PatronAction
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.RelatedItem
import uniffi.librespydus_core.SearchRecord

class BookRefTest {
    private val origin = "https://canning.spydus.com"

    @Test
    fun fromSearchRecordKeepsFormatsAndUsesWorkIrn() {
        val ref = BookRef.from(FakeBackend.sampleRecord(origin))
        assertEquals("work:15643445", ref.id)
        assertEquals("Project Hail Mary", ref.title)
        assertEquals("Weir, Andy", ref.author)
        assertEquals("https://img.example/cover.jpg", ref.coverUrl)
        assertEquals("$origin/cgi-bin/spydus.exe/FULL/OPAC/ALLENQ/1/15643445,1", ref.url)
        assertEquals("9780593395561", ref.isbn)
        assertEquals("2021", ref.year)
        assertEquals(listOf("BK", "EBK"), ref.formats.map { it.code })
        assertEquals("$origin/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/BK", ref.formats[0].reserveUrl)
        assertNull(ref.formats[1].reserveUrl)
        assertNull(ref.savedAt)
    }

    @Test
    fun fromSearchRecordWithoutIdsFallsBackToUrlThenTitle() {
        val record = FakeBackend.sampleRecord(origin).copy(workIrn = null, title = "  ")
        assertEquals("url:${record.detailUrl}", BookRef.from(record).id)
        assertEquals(BookRef.UNTITLED, BookRef.from(record).title)
        val bare = SearchRecord(null, null, "Only a title", null, null, null, null, null, null, null, null, emptyList(), emptyList())
        assertEquals("title:Only a title", BookRef.from(bare).id)
    }

    @Test
    fun fromPatronItemUsesBibIrn() {
        val item = PatronItem(
            irn = "19000001",
            bibIrn = "8753080",
            title = "The midnight library",
            bibUrl = "$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78500000?QRY=IRN(8753080)&SETLVL=SET",
            author = "Haig, Matt, author",
            year = "2020",
            coverUrls = listOf("https://secure.syndetics.com/x.gif"),
            isbn = "9780525559474",
            fields = mapOf("Due" to "27 Oct 2026"),
            actions = listOf(PatronAction("Renew loan", "$origin/renew")),
        )
        val ref = BookRef.from(item)
        assertEquals("bib:8753080", ref.id)
        assertEquals("The midnight library", ref.title)
        assertEquals(item.bibUrl, ref.url)
        assertEquals("https://secure.syndetics.com/x.gif", ref.coverUrl)
        assertEquals("9780525559474", ref.isbn)
        assertEquals(emptyList<FormatRef>(), ref.formats)
    }

    @Test
    fun fromRelatedItemParsesIrnFromBibUrlThenBrn() {
        val withIrn = RelatedItem("Sequel", "$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?SETLVL=&IRN=17651573", "457540", "Weir, Andy", "2025", null, emptyList())
        assertEquals("bib:17651573", BookRef.from(withIrn).id)
        val withBrn = withIrn.copy(bibUrl = "$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=457540")
        assertEquals("brn:457540", BookRef.from(withBrn).id)
        assertNull(BookRef.from(withBrn).coverUrl)
    }

    @Test
    fun withDetailPrefersBookmarkUrlAndBibIrn() {
        val ref = BookRef.from(FakeBackend.sampleRecord(origin))
        val detail = RecordDetail(
            bibIrn = "17665994", brn = "457540", title = "Project Hail Mary", seriesTitle = null, isbn = "9781804953693",
            coverUrls = listOf("https://secure.syndetics.com/lc.gif"), summary = null, details = emptyList(),
            bookmarkUrl = "$origin/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=457540",
            related = RecordRelated(null, null, null, null),
        )
        val upgraded = ref.withDetail(detail)
        assertEquals("bib:17665994", upgraded.id)
        assertEquals(detail.bookmarkUrl, upgraded.url)
        assertEquals("https://secure.syndetics.com/lc.gif", upgraded.coverUrl)
        assertEquals("9781804953693", upgraded.isbn)
        assertEquals(ref.formats, upgraded.formats)
    }

    @Test
    fun serializesForNavigationAndFavourites() {
        val ref = BookRef.from(FakeBackend.sampleRecord(origin)).copy(savedAt = 42)
        val json = Json.encodeToString(BookRef.serializer(), ref)
        assertEquals(ref, Json.decodeFromString(BookRef.serializer(), json))
        val route: Route = Route.Detail(ref)
        assertEquals(route, Json.decodeFromString(Route.serializer(), Json.encodeToString(Route.serializer(), route)))
    }

    @Test
    fun fromUrl() {
        val ref = BookRef.fromUrl("$origin/x?BRN=1")
        assertEquals("url:$origin/x?BRN=1", ref.id)
        assertEquals("$origin/x?BRN=1", ref.url)
    }
}

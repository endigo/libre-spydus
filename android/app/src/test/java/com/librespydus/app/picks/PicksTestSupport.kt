package com.librespydus.app.picks

import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.LibraryBackend
import kotlinx.coroutines.delay
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusException

/**
 * A [FakeBackend] with a Pick list in memory: `picks` pages it [pageSize] at a time
 * (`…/picks?page=N` next links), `savePick` / `removePick` change it after [delayMs] and are
 * recorded, and each can be made to fail. Only [session] is signed in.
 */
internal class PicksBackend(
    override val origin: String = "https://canning.spydus.com",
    var session: String? = null,
) : LibraryBackend by FakeBackend(origin) {
    val list = mutableListOf<SearchRecord>()
    var pageSize = 20
    var delayMs = 100L

    /** The next `picks` call throws this (once). */
    var pageFails: Exception? = null

    /** `picks` answers a page with no records that isn't the "no items" page. */
    var unrecognised = false
    var saveFails: Exception? = null
    var removeFails: Exception? = null

    /** `record(url)` answers by URL (the copy's IRN lookup); anything else has no bib IRN. */
    val records = mutableMapOf<String, RecordDetail>()

    val pageRequests = mutableListOf<String?>()
    val saved = mutableListOf<String>()
    val removed = mutableListOf<String>()
    val recordRequests = mutableListOf<String>()

    override suspend fun picks(session: String, url: String?): SearchResults {
        pageRequests += url
        delay(delayMs)
        checkSession(session)
        pageFails?.let {
            pageFails = null
            throw it
        }
        if (unrecognised) return SearchResults(null, null, null, null, null, null, false, emptyList())
        val index = url?.substringAfter("page=")?.toInt() ?: 0
        val chunk = list.drop(index * pageSize).take(pageSize)
        val next = if ((index + 1) * pageSize < list.size) "$origin/picks?page=${index + 1}" else null
        return SearchResults(null, null, list.size.toLong(), null, next, null, list.isEmpty(), chunk)
    }

    override suspend fun savePick(irn: String, session: String): Long {
        delay(delayMs)
        checkSession(session)
        saveFails?.let { throw it }
        saved += irn
        if (list.none { it.workIrn == irn }) list.add(0, pickRecord(irn))
        return list.size.toLong()
    }

    override suspend fun removePick(irn: String, session: String): Long {
        delay(delayMs)
        checkSession(session)
        removeFails?.let { throw it }
        removed += irn
        list.removeAll { it.workIrn == irn }
        return list.size.toLong()
    }

    override suspend fun record(url: String): RecordDetail {
        recordRequests += url
        return records[url] ?: recordDetail(bibIrn = null)
    }

    private fun checkSession(session: String) {
        if (this.session != null && session != this.session) throw SpydusException.NotLoggedIn("not logged in")
    }
}

internal fun pickRecord(irn: String, title: String = "Book $irn", origin: String = "https://canning.spydus.com") = SearchRecord(
    rank = null,
    workIrn = irn,
    title = title,
    detailUrl = "$origin/cgi-bin/spydus.exe/FULL/OPAC/ALLENQ/1/$irn,1",
    author = "Author, A",
    authorRole = "author",
    authorUrl = null,
    year = "2024",
    language = "English",
    summary = null,
    isbn = null,
    coverUrls = emptyList(),
    formats = listOf(RecordFormat("BK", "Book", null, "$origin/avail/$irn")),
)

internal fun recordDetail(bibIrn: String?, title: String = "A record") =
    RecordDetail(bibIrn, null, title, null, null, emptyList(), null, emptyList(), null, RecordRelated(null, null, null, null))

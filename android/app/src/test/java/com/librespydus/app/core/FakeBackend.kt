package com.librespydus.app.core

import uniffi.librespydus_core.AccountLinks
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.EditOptions
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RecordRelated
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.ReserveOptions
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults

/** A public-reads-only [LibraryBackend] for JVM tests: no network, account calls fail loudly. */
class FakeBackend(override val origin: String = "https://canning.spydus.com") : LibraryBackend {
    var appliedCookie: String? = null
    var accountName: String = "Real Person"

    /** The `fetchedAtMs` of [accountSnapshot]s. */
    var now: () -> Long = System::currentTimeMillis
    val searches = mutableListOf<String>()

    override fun setVerificationCookie(cookie: String?) {
        appliedCookie = cookie
    }

    override suspend fun search(query: String): SearchResults {
        searches += query
        return results(sampleRecord(origin))
    }

    override suspend fun searchSorted(query: String, sort: ResultsSort) = search(query)
    override suspend fun page(url: String) = results()
    override suspend fun availability(url: String) = Availability(null, null, null, emptyMap(), null, null, null, null)
    override suspend fun record(url: String) =
        RecordDetail(null, null, "Title", null, null, emptyList(), null, emptyList(), url, RecordRelated(null, null, null, null))
    override suspend fun related(url: String) = RelatedList(emptyList())

    override suspend fun login(username: String, password: String) = "session-$username"
    override suspend fun account(session: String) = AccountLinks(accountName, null, null, null)
    override suspend fun loans(session: String) = PatronList(0, emptyList())
    override suspend fun reservations(session: String) = Reservations(PatronList(0, emptyList()), PatronList(0, emptyList()))
    override suspend fun accountSnapshot(session: String) =
        AccountSnapshot(DemoBackend.SNAPSHOT_VERSION, accountName, loans(session), reservations(session), now())
    override suspend fun reserveOptions(url: String, session: String): ReserveOptions = error("not public")
    override suspend fun placeReservation(url: String, session: String, xdays: String?, pickup: String?): ReserveResult = error("never POST in tests")
    override suspend fun cancelReservation(url: String, session: String): ReserveResult = error("never POST in tests")
    override suspend fun editOptions(url: String, session: String): EditOptions = error("not public")
    override suspend fun editReservation(url: String, session: String, overrides: EditOverrides): ReserveResult = error("never POST in tests")

    companion object {
        fun results(vararg records: SearchRecord) =
            SearchResults("q", "1", records.size.toLong(), null, null, null, records.isEmpty(), records.toList())

        fun sampleRecord(origin: String = "https://canning.spydus.com") = SearchRecord(
            rank = 1,
            workIrn = "15643445",
            title = "Project Hail Mary",
            detailUrl = "$origin/cgi-bin/spydus.exe/FULL/OPAC/ALLENQ/1/15643445,1",
            author = "Weir, Andy",
            authorRole = "author",
            authorUrl = null,
            year = "2021",
            language = "English",
            summary = null,
            isbn = "9780593395561",
            coverUrls = listOf("https://img.example/cover.jpg"),
            formats = listOf(
                RecordFormat("BK", "Book", "$origin/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/BK", "$origin/avail/BK"),
                RecordFormat("EBK", "eBook", null, "$origin/avail/EBK"),
            ),
        )
    }
}

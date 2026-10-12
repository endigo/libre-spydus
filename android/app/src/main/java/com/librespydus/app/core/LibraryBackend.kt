package com.librespydus.app.core

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.librespydus_core.AccountLinks
import uniffi.librespydus_core.Availability
import uniffi.librespydus_core.EditOptions
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RelatedList
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.ReserveOptions
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusClient

/**
 * Everything the screens may ask of a library (§5.1): the `SpydusClient` surface plus [origin].
 * Screens never touch `SpydusClient` directly; they get the current backend from
 * `AppState.backend`. Every suspend function throws `SpydusException` (route it through
 * `AppState.report`). Mutations return `ReserveResult` — `ok = false` is data to show.
 */
interface LibraryBackend {
    /** The library origin, normalised (`https://host`, no trailing `/`). */
    val origin: String

    /** True for [DemoBackend]: account data is fixture data and nothing is ever posted. */
    val isDemo: Boolean get() = false

    /** Sets (or with null clears) the `aws-waf-token=…` cookie for every later request. */
    fun setVerificationCookie(cookie: String?)

    suspend fun search(query: String): SearchResults

    /**
     * TODO(P): becomes the core's `searchSorted(query, sort)` once P lands; until then every
     * implementation falls back to relevance-ordered [search].
     */
    suspend fun searchSorted(query: String, sort: ResultsSort): SearchResults

    suspend fun page(url: String): SearchResults
    suspend fun availability(url: String): Availability
    suspend fun record(url: String): RecordDetail
    suspend fun related(url: String): RelatedList

    suspend fun login(username: String, password: String): String
    suspend fun account(session: String): AccountLinks
    suspend fun loans(session: String): PatronList
    suspend fun reservations(session: String): Reservations
    suspend fun reserveOptions(url: String, session: String): ReserveOptions
    suspend fun placeReservation(url: String, session: String, xdays: String?, pickup: String?): ReserveResult
    suspend fun cancelReservation(url: String, session: String): ReserveResult
    suspend fun editOptions(url: String, session: String): EditOptions
    suspend fun editReservation(url: String, session: String, overrides: EditOverrides): ReserveResult
}

/**
 * The real backend: one [SpydusClient] per library origin. Calls hop to [Dispatchers.IO] so the
 * JNA marshalling never runs on the main thread; the network work itself runs on the core's own
 * tokio runtime.
 */
class CoreBackend(private val client: SpydusClient) : LibraryBackend {
    override val origin: String = client.origin()

    override fun setVerificationCookie(cookie: String?) = client.setVerificationCookie(cookie)

    override suspend fun search(query: String) = io { client.search(query) }

    // TODO(P): `client.searchSorted(query, sort.toCore())` once SearchSort/searchSorted exist.
    override suspend fun searchSorted(query: String, sort: ResultsSort) = search(query)

    override suspend fun page(url: String) = io { client.page(url) }
    override suspend fun availability(url: String) = io { client.availability(url) }
    override suspend fun record(url: String) = io { client.record(url) }
    override suspend fun related(url: String) = io { client.related(url) }

    override suspend fun login(username: String, password: String) = io { client.login(username, password) }
    override suspend fun account(session: String) = io { client.account(session) }
    override suspend fun loans(session: String) = io { client.loans(session) }
    override suspend fun reservations(session: String) = io { client.reservations(session) }
    override suspend fun reserveOptions(url: String, session: String) = io { client.reserveOptions(url, session) }

    override suspend fun placeReservation(url: String, session: String, xdays: String?, pickup: String?) =
        io { client.placeReservation(url, session, xdays, pickup) }

    override suspend fun cancelReservation(url: String, session: String) =
        io { client.cancelReservation(url, session) }

    override suspend fun editOptions(url: String, session: String) = io { client.editOptions(url, session) }

    override suspend fun editReservation(url: String, session: String, overrides: EditOverrides) =
        io { client.editReservation(url, session, overrides) }

    private suspend inline fun <T> io(crossinline block: suspend () -> T): T =
        withContext(Dispatchers.IO) { block() }

    companion object {
        /** Builds the client off the main thread (it sets up the HTTP stack and TLS roots). */
        suspend fun create(origin: String): CoreBackend =
            withContext(Dispatchers.IO) { CoreBackend(SpydusClient(origin)) }
    }
}

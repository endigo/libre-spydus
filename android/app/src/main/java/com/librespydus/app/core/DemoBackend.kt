package com.librespydus.app.core

import kotlinx.coroutines.delay
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import uniffi.librespydus_core.AccountLinks
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.EditOptions
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.ExpiryChoice
import uniffi.librespydus_core.PatronAction
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.PickupChoice
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.ReserveOptions
import uniffi.librespydus_core.ReserveResult
import uniffi.librespydus_core.SearchRecord
import uniffi.librespydus_core.SearchResults
import uniffi.librespydus_core.SpydusException
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

/**
 * Demo mode (§5.4, DEBUG builds only). Public reads go to the real [public] backend (public GETs
 * are allowed); every account call is served from [DemoData] and never touches the network.
 * `login("demo", "demo")` succeeds, anything else is `CredentialsRejected`. Mutations succeed
 * after [mutationDelay] and change the in-memory lists: cancel removes the row, reserve adds a
 * pending row, edit updates the pickup location. [accountSnapshot] takes [snapshotDelay], so the
 * refreshing state is visible, and with [refreshFails] (DEBUG `ls_demo_refresh_fails`) it fails
 * like an unreachable library.
 */
class DemoBackend(
    private val public: LibraryBackend,
    data: DemoData,
    private val today: LocalDate = LocalDate.now(),
    private val mutationDelay: Duration = 600.milliseconds,
    private val snapshotDelay: Duration = 1_500.milliseconds,
    private val refreshFails: Boolean = false,
    private val now: () -> Long = System::currentTimeMillis,
) : LibraryBackend {
    override val origin: String get() = public.origin
    override val isDemo: Boolean get() = true

    private val accountLinks = data.accountLinks
    private val reserveOptions = data.reserveOptions
    private val editOptions = data.editOptions
    private val mutex = Mutex()
    private val loanItems = data.loans.toMutableList()
    private val readyItems = data.ready.toMutableList()
    private val pendingItems = data.pending.toMutableList()
    private var nextIrn = 1

    /** reserveUrl → the search record it came from, so a demo reservation shows a real title. */
    private val reservable = ConcurrentHashMap<String, SearchRecord>()

    override fun setVerificationCookie(cookie: String?) = public.setVerificationCookie(cookie)

    override suspend fun search(query: String) = public.search(query).also(::remember)
    override suspend fun searchSorted(query: String, sort: ResultsSort) =
        public.searchSorted(query, sort).also(::remember)
    override suspend fun page(url: String) = public.page(url).also(::remember)
    override suspend fun availability(url: String) = public.availability(url)
    override suspend fun record(url: String) = public.record(url)
    override suspend fun related(url: String) = public.related(url)

    override suspend fun login(username: String, password: String): String {
        delay(mutationDelay)
        if (username.trim() == DEMO_USERNAME && password == DEMO_PASSWORD) return SESSION
        throw SpydusException.CredentialsRejected("credentials rejected")
    }

    override suspend fun account(session: String): AccountLinks {
        requireSession(session)
        return accountLinks
    }

    override suspend fun loans(session: String): PatronList {
        requireSession(session)
        return mutex.withLock { PatronList(loanItems.size.toLong(), loanItems.toList()) }
    }

    override suspend fun reservations(session: String): Reservations {
        requireSession(session)
        return mutex.withLock {
            Reservations(
                ready = PatronList(readyItems.size.toLong(), readyItems.toList()),
                pending = PatronList(pendingItems.size.toLong(), pendingItems.toList()),
            )
        }
    }

    /** The demo account as it is now, including the in-memory reserve / cancel / edit changes. */
    override suspend fun accountSnapshot(session: String): AccountSnapshot {
        requireSession(session)
        delay(snapshotDelay)
        if (refreshFails) throw SpydusException.Network("network error: demo refresh set to fail (ls_demo_refresh_fails)")
        return AccountSnapshot(SNAPSHOT_VERSION, accountLinks.name, loans(session), reservations(session), now())
    }

    override suspend fun reserveOptions(url: String, session: String): ReserveOptions {
        requireSession(session)
        return reserveOptions
    }

    override suspend fun placeReservation(url: String, session: String, xdays: String?, pickup: String?): ReserveResult {
        requireSession(session)
        delay(mutationDelay)
        val record = reservable[url]
        mutex.withLock {
            val irn = "demo-${nextIrn++}"
            pendingItems.add(
                0,
                PatronItem(
                    irn = irn,
                    bibIrn = null,
                    title = record?.title ?: "Reserved title",
                    bibUrl = record?.detailUrl,
                    author = record?.author,
                    year = record?.year,
                    coverUrls = record?.coverUrls ?: emptyList(),
                    isbn = record?.isbn,
                    fields = linkedMapOf(
                        "Status" to "Pending",
                        "Placed" to DemoData.formatDate(today),
                        "Pickup location" to (pickupName(pickup) ?: reserveOptions.pickupLocation ?: "Your library"),
                    ),
                    actions = demoActions(irn),
                ),
            )
        }
        return ReserveResult(ok = true, message = "Reservation placed.")
    }

    override suspend fun cancelReservation(url: String, session: String): ReserveResult {
        requireSession(session)
        delay(mutationDelay)
        val removed = mutex.withLock {
            readyItems.removeAll { it.hasAction(url) } or pendingItems.removeAll { it.hasAction(url) }
        }
        return if (removed) ReserveResult(ok = true, message = "Reservation cancelled.")
        else ReserveResult(ok = false, message = "This reservation can't be cancelled.")
    }

    override suspend fun editOptions(url: String, session: String): EditOptions {
        requireSession(session)
        return editOptions
    }

    override suspend fun editReservation(url: String, session: String, overrides: EditOverrides): ReserveResult {
        requireSession(session)
        delay(mutationDelay)
        val edited = mutex.withLock {
            listOf(readyItems, pendingItems).any { list ->
                val index = list.indexOfFirst { it.hasAction(url) }
                if (index < 0) return@any false
                val item = list[index]
                val fields = LinkedHashMap(item.fields)
                pickupName(overrides.pickup)?.let { fields["Pickup location"] = it }
                list[index] = item.copy(fields = fields)
                true
            }
        }
        return if (edited) ReserveResult(ok = true, message = "Reservation updated.")
        else ReserveResult(ok = false, message = "This reservation can't be changed.")
    }

    private fun remember(results: SearchResults) {
        for (record in results.records) for (format in record.formats) {
            format.reserveUrl?.let { reservable[it] = record }
        }
    }

    private fun pickupName(code: String?): String? =
        code?.let { c -> editOptions.pickupChoices.firstOrNull { it.code == c }?.name }

    private fun demoActions(irn: String) = listOf(
        PatronAction("Cancel reservation", "$origin/demo/cancel/$irn"),
        PatronAction("Edit reservation", "$origin/demo/edit/$irn"),
    )

    private fun requireSession(session: String) {
        if (session != SESSION) throw SpydusException.NotLoggedIn("not logged in")
    }

    private fun PatronItem.hasAction(url: String) = actions.any { it.href == url }

    companion object {
        const val SESSION = "demo-session"
        const val DEMO_USERNAME = "demo"
        const val DEMO_PASSWORD = "demo"

        /** The core's `ACCOUNT_SNAPSHOT_VERSION` (core/src/app.rs); DemoBackendTest checks it still decodes. */
        const val SNAPSHOT_VERSION: UInt = 1u
    }
}

/**
 * Fixture account data for [DemoBackend], read from `assets/demo/` (built from the core's
 * synthetic goldens, "Test Borrower"). URLs in the files use [FIXTURE_ORIGIN] and are rewritten
 * to the selected library's origin so tapping a row stays on that origin. `loans.json` items
 * carry `dueInDays`, turned into a `Due` field relative to `today`.
 */
class DemoData(
    val accountLinks: AccountLinks,
    val loans: List<PatronItem>,
    val ready: List<PatronItem>,
    val pending: List<PatronItem>,
    val reserveOptions: ReserveOptions,
    val editOptions: EditOptions,
) {
    companion object {
        const val ASSET_DIR = "demo"
        const val FIXTURE_ORIGIN = "https://canning.spydus.com"
        val FILES = listOf("account-links.json", "loans.json", "reservations.json", "reserve-options.json", "edit-options.json")

        private val json = Json { ignoreUnknownKeys = true }
        private val DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM yyyy", Locale.ENGLISH)

        /** The OPAC's date style, e.g. "27 Oct 2026". */
        fun formatDate(date: LocalDate): String = DATE.format(date)

        /** [read] returns a file's text by name (from [FILES]). */
        fun load(read: (String) -> String, origin: String, today: LocalDate = LocalDate.now()): DemoData {
            fun obj(name: String): JsonObject =
                json.parseToJsonElement(read(name).replace(FIXTURE_ORIGIN, origin.trimEnd('/'))).jsonObject

            val links = obj("account-links.json")
            val loans = obj("loans.json")
            val reservations = obj("reservations.json")
            val reserve = obj("reserve-options.json")
            val edit = obj("edit-options.json")
            return DemoData(
                accountLinks = AccountLinks(
                    name = links.str("name"),
                    loansUrl = links.str("loansUrl"),
                    reservationsReadyUrl = links.str("reservationsReadyUrl"),
                    reservationsPendingUrl = links.str("reservationsPendingUrl"),
                ),
                loans = loans.items().map { item ->
                    val due = item["dueInDays"]?.jsonPrimitive?.intOrNull
                    val parsed = patronItem(item)
                    if (due == null) parsed
                    else parsed.copy(fields = linkedMapOf("Due" to formatDate(today.plusDays(due.toLong()))) + parsed.fields)
                },
                ready = reservations["ready"]!!.jsonObject.items().map(::patronItem),
                pending = reservations["pending"]!!.jsonObject.items().map(::patronItem),
                reserveOptions = ReserveOptions(
                    canReserve = reserve.bool("canReserve"),
                    pickupLocation = reserve.str("pickupLocation"),
                    availableSummary = reserve.str("availableSummary"),
                    expiryChoices = reserve.array("expiryChoices").map {
                        val o = it.jsonObject
                        ExpiryChoice(days = o.str("days").orEmpty(), label = o.str("label").orEmpty())
                    },
                    message = reserve.str("message"),
                ),
                editOptions = EditOptions(
                    canEdit = edit.bool("canEdit"),
                    pickupChoices = edit.array("pickupChoices").map {
                        val o = it.jsonObject
                        PickupChoice(code = o.str("code").orEmpty(), name = o.str("name").orEmpty())
                    },
                    pickupSelected = edit.str("pickupSelected"),
                    delayUntil = edit.str("delayUntil"),
                    notRequiredAfter = edit.str("notRequiredAfter"),
                    message = edit.str("message"),
                ),
            )
        }

        private fun patronItem(o: JsonObject) = PatronItem(
            irn = o.str("irn"),
            bibIrn = o.str("bibIrn"),
            title = o.str("title"),
            bibUrl = o.str("bibUrl"),
            author = o.str("author"),
            year = o.str("year"),
            coverUrls = o.array("coverUrls").mapNotNull { it.jsonPrimitive.contentOrNull },
            isbn = o.str("isbn"),
            fields = (o["fields"] as? JsonObject).orEmpty()
                .mapValuesTo(LinkedHashMap()) { it.value.jsonPrimitive.content },
            actions = o.array("actions").map {
                val a = it.jsonObject
                PatronAction(label = a.str("label").orEmpty(), href = a.str("href").orEmpty())
            },
        )

        private fun JsonObject.items(): List<JsonObject> = array("items").map { it.jsonObject }
        private fun JsonObject.str(key: String): String? =
            this[key]?.takeUnless { it is JsonNull }?.jsonPrimitive?.contentOrNull
        private fun JsonObject.bool(key: String): Boolean = this[key]?.jsonPrimitive?.content == "true"
        private fun JsonObject.array(key: String): JsonArray = (this[key] as? JsonArray) ?: JsonArray(emptyList())
    }
}

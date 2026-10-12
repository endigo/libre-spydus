package com.librespydus.app.core

import kotlinx.coroutines.delay
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.PatronAction
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.Reservations

/**
 * A [FakeBackend] whose `accountSnapshot` takes [delayMs], can [fail], counts its [calls] and
 * names the borrower after the session ("Reader for tok"), so a result for the wrong session shows.
 */
internal class SnapshotBackend(
    origin: String = "https://canning.spydus.com",
    var delayMs: Long = 100,
    var clock: () -> Long = { 0L },
) : LibraryBackend by FakeBackend(origin) {
    var calls = 0
    var fail: Exception? = null

    override suspend fun accountSnapshot(session: String): AccountSnapshot {
        calls++
        delay(delayMs)
        fail?.let { throw it }
        return accountSnapshot(name = "Reader for $session", fetchedAtMs = clock())
    }
}

/** A snapshot with one loan and one pending reservation (cancel link [CANCEL_HREF]). */
internal fun accountSnapshot(name: String? = "Test Borrower", fetchedAtMs: Long = 1_000L, title: String = "Project Hail Mary") =
    AccountSnapshot(
        DemoBackend.SNAPSHOT_VERSION,
        name,
        PatronList(1, listOf(patronItem("loan-1", title))),
        Reservations(
            ready = PatronList(0, emptyList()),
            pending = PatronList(1, listOf(patronItem("res-1", "The Women", PatronAction("Cancel reservation", CANCEL_HREF)))),
        ),
        fetchedAtMs,
    )

internal const val CANCEL_HREF = "https://canning.spydus.com/cgi-bin/spydus.exe/cancel/res-1"

private fun patronItem(irn: String, title: String, vararg actions: PatronAction) = PatronItem(
    irn = irn, bibIrn = irn, title = title, bibUrl = null, author = "Weir, Andy", year = "2021",
    coverUrls = emptyList(), isbn = null, fields = mapOf("Due" to "27 Oct 2026"), actions = actions.toList(),
)

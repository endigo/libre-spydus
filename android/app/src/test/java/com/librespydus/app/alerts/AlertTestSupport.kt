package com.librespydus.app.alerts

import com.librespydus.app.core.AlertNotification
import com.librespydus.app.core.AlertNotifier
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.MorningCheckScheduler
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.Reservations

/** Records what [com.librespydus.app.core.Alerts] posts and removes. */
internal class RecordingNotifier : AlertNotifier {
    val posted = mutableListOf<AlertNotification>()
    val removed = mutableListOf<String>()

    override fun post(notification: AlertNotification) {
        posted += notification
    }

    override fun removeAll(originHash: String) {
        removed += originHash
    }
}

/** Records the morning-check scheduling calls. */
internal class RecordingScheduler : MorningCheckScheduler {
    val calls = mutableListOf<String>()

    override fun ensureScheduled() {
        calls += "ensure"
    }

    override fun scheduleNext() {
        calls += "next"
    }

    override fun runNow() {
        calls += "now"
    }

    override fun cancel() {
        calls += "cancel"
    }
}

internal fun item(irn: String?, title: String?, fields: Map<String, String> = emptyMap(), bibIrn: String? = null) = PatronItem(
    irn = irn, bibIrn = bibIrn, title = title, bibUrl = null, author = null, year = null,
    coverUrls = emptyList(), isbn = null, fields = fields, actions = emptyList(),
)

internal fun loan(irn: String, title: String?, due: String) = item(irn, title, mapOf("Due" to due))

internal fun snapshot(
    loans: List<PatronItem> = emptyList(),
    ready: List<PatronItem> = emptyList(),
    pending: List<PatronItem> = emptyList(),
) = AccountSnapshot(
    DemoBackend.SNAPSHOT_VERSION,
    "Test Borrower",
    PatronList(loans.size.toLong(), loans),
    Reservations(PatronList(ready.size.toLong(), ready), PatronList(pending.size.toLong(), pending)),
    1_000L,
)

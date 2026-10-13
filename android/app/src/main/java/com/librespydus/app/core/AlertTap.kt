package com.librespydus.app.core

import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.patronItemKey

/**
 * A tapped alert (docs/native-apps.md §5.8 "Tap"): the Your Books list it opens and what it names.
 * Each notification carries its [kind], the library [origin] and the `patronItemKey`s of the
 * reservation or loans it names ([keys]) as intent extras ([extras], one PendingIntent per
 * notification, [AlertIntents]). A notification posted by an older build, and the collapsed group
 * summary, carry just the list ([EXTRA_OPEN_BORROWED]).
 */
data class AlertTap(
    val segment: BorrowedSegment,
    val kind: AlertKind? = null,
    val origin: String? = null,
    val keys: List<String> = emptyList(),
) {
    /** The intent extras: the list (every build reads it), then the kind, origin and keys. */
    fun extras(): Map<String, Any> {
        val keys = keys // inside buildMap, `keys` would be the map's own
        return buildMap {
            put(EXTRA_OPEN_BORROWED, segment.name.lowercase())
            kind?.let { put(EXTRA_KIND, it.channelId) }
            origin?.let { put(EXTRA_ORIGIN, it) }
            if (keys.isNotEmpty()) put(EXTRA_KEYS, keys.toTypedArray())
        }
    }

    companion object {
        /** `loans` / `reservations`: the only extra notifications posted before §5.8 "Tap" carry. */
        const val EXTRA_OPEN_BORROWED = "ls_open_borrowed"
        const val EXTRA_KIND = "ls_alert_kind"
        const val EXTRA_ORIGIN = "ls_alert_origin"
        const val EXTRA_KEYS = "ls_alert_keys"

        val EXTRAS = listOf(EXTRA_OPEN_BORROWED, EXTRA_KIND, EXTRA_ORIGIN, EXTRA_KEYS)

        /** What tapping [notification] opens. */
        fun of(notification: AlertNotification) =
            AlertTap(notification.kind.opens, notification.kind, notification.origin, notification.keys)

        /** The collapsed group summary of [kind]'s alerts: Your Books on its list. */
        fun summary(kind: AlertKind) = AlertTap(kind.opens)

        /** From intent extras; null when they aren't a notification tap. */
        fun parse(extras: Map<String, Any?>): AlertTap? {
            val kind = (extras[EXTRA_KIND] as? String)?.let { value -> AlertKind.entries.firstOrNull { it.channelId == value } }
            val segment = kind?.opens
                ?: (extras[EXTRA_OPEN_BORROWED] as? String)?.let { value -> BorrowedSegment.entries.firstOrNull { it.name.equals(value, ignoreCase = true) } }
                ?: return null
            val keys = when (val raw = extras[EXTRA_KEYS]) {
                is Array<*> -> raw.filterIsInstance<String>()
                is List<*> -> raw.filterIsInstance<String>()
                else -> emptyList()
            }
            return AlertTap(segment, kind, (extras[EXTRA_ORIGIN] as? String)?.takeIf { it.isNotBlank() }, keys)
        }
    }
}

/** Where a tapped alert opens: Your Books on [segment], with [book]'s page pushed over the list when there is one. */
data class AlertDestination(val segment: BorrowedSegment, val book: BookRef? = null)

object AlertRouting {
    /**
     * The book page of the one item [tap] names, built from the saved account's own row exactly as
     * tapping that row in Your Books builds it ([BookRef.from]). Just the list when the tap names
     * no item (an older build's notification, the group summary) or several (a due reminder for
     * several loans), when it belongs to another library than [origin] (the selected one), or when
     * its item is no longer on [snapshot]'s list (collected, returned) or there is no snapshot.
     */
    fun destination(tap: AlertTap, origin: String?, snapshot: AccountSnapshot?): AlertDestination {
        val list = AlertDestination(tap.segment)
        val key = tap.keys.singleOrNull()?.takeIf { it.isNotEmpty() } ?: return list
        if (origin == null || tap.origin == null || !sameOrigin(tap.origin, origin)) return list
        val item = snapshot?.let { items(tap.segment, it) }?.firstOrNull { patronItemKey(it) == key } ?: return list
        return AlertDestination(tap.segment, BookRef.from(item))
    }

    private fun items(segment: BorrowedSegment, snapshot: AccountSnapshot): List<PatronItem> = when (segment) {
        BorrowedSegment.LOANS -> snapshot.loans.items
        BorrowedSegment.RESERVATIONS -> snapshot.reservations.ready.items + snapshot.reservations.pending.items
    }

    private fun sameOrigin(a: String, b: String) = a.trimEnd('/').equals(b.trimEnd('/'), ignoreCase = true)
}

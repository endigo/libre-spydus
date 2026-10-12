package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Formatting
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.Reservations
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import java.util.Locale

// What the Borrowed cards show, built from the core's `PatronItem`s (docs/native-apps.md §4 data
// notes). Pure (no I/O), so the sorting and wording are unit-tested in BorrowedRowsTest. Dates
// are java.time (ISO/Gregorian), whatever calendar the device uses.

/**
 * The href of the first action whose label contains [keyword] (case-insensitive): the labels
 * vary by library ("Cancel reservation", "Edit", "Edit reservation").
 */
fun PatronItem.actionHref(keyword: String): String? =
    actions.firstOrNull { it.label.contains(keyword, ignoreCase = true) }?.href?.trim()?.takeIf { it.isNotEmpty() }

/** `/cancel/i`: the reservation's cancel link (`cancelReservation(url)`). */
val PatronItem.cancelHref: String? get() = actionHref("cancel")

/** `/edit/i`: the reservation's edit link (`editOptions(url)` / `editReservation(url)`). */
val PatronItem.editHref: String? get() = actionHref("edit")

private fun String?.nonBlank(): String? = this?.trim()?.takeIf { it.isNotEmpty() }

/** Borrowed's date wording ("10 Oct"), as the library's own dates are English. */
internal object BorrowedDates {
    private val SHORT: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM", Locale.ENGLISH)

    fun short(date: LocalDate): String = SHORT.format(date)
}

/** A loan's due date relative to today. */
data class DueStatus(val dueDate: LocalDate, val daysLeft: Long) {
    val isOverdue: Boolean get() = daysLeft < 0

    /** Danger colour (§6): due within 3 days, or overdue. */
    val isUrgent: Boolean get() = daysLeft <= 3

    companion object {
        /** Null when there's no date or the core can't read it. */
        fun of(dueText: String?, today: LocalDate): DueStatus? {
            val due = Formatting.parseDueDate(dueText) ?: return null
            return DueStatus(due, ChronoUnit.DAYS.between(today, due))
        }
    }
}

/** One loan card: "Due 27 Oct · 3 days left", danger colour when due within 3 days or overdue. */
data class LoanRow(
    /** The loan's IRN (made unique within its list by [rows]). */
    val id: String,
    val item: PatronItem,
    val ref: BookRef,
    /** Null when there's no "Due" field or the core can't read it as a date. */
    val due: DueStatus?,
    /** `fields["Due"]` as the library wrote it. */
    val dueText: String?,
) {
    /** The meta line; an unreadable date is shown as the library wrote it. */
    val meta: String?
        get() {
            due?.let { return dueLine(it) }
            val text = dueText ?: return null
            return if (text.contains("due", ignoreCase = true)) text else "Due $text"
        }

    val isUrgent: Boolean get() = due?.isUrgent ?: false

    companion object {
        fun of(item: PatronItem, today: LocalDate = LocalDate.now()): LoanRow {
            val ref = BookRef.from(item)
            val dueText = item.fields["Due"].nonBlank()
            return LoanRow(
                id = item.irn.nonBlank() ?: ref.id,
                item = item,
                ref = ref,
                due = DueStatus.of(dueText, today),
                dueText = dueText,
            )
        }

        /** "Due 27 Oct · 3 days left", "Due tomorrow · 13 Oct", "Due today · 12 Oct", "Due 10 Oct · 2 days overdue". */
        fun dueLine(due: DueStatus): String {
            val date = BorrowedDates.short(due.dueDate)
            return when {
                due.daysLeft < -1 -> "Due $date · ${-due.daysLeft} days overdue"
                due.daysLeft == -1L -> "Due $date · 1 day overdue"
                due.daysLeft == 0L -> "Due today · $date"
                due.daysLeft == 1L -> "Due tomorrow · $date"
                else -> "Due $date · ${due.daysLeft} days left"
            }
        }

        /**
         * Loans soonest-due first (overdue at the top); loans without a readable date go last, in
         * the library's order.
         */
        fun rows(items: List<PatronItem>, today: LocalDate = LocalDate.now()): List<LoanRow> {
            val ids = UniqueIds()
            return items.map { item -> of(item, today).let { it.copy(id = ids.claim(it.id)) } }
                .withIndex()
                .sortedWith(
                    compareBy<IndexedValue<LoanRow>>({ it.value.due == null }, { it.value.due?.dueDate }, { it.index }),
                )
                .map { it.value }
        }
    }
}

/** One reservation card, ready for pickup or pending. */
data class ReservationRow(
    /** The reservation's IRN, else its cancel/edit link (unique within [ReservationSections]). */
    val id: String,
    val kind: Kind,
    val item: PatronItem,
    val ref: BookRef,
    /** `fields["Status"]`, e.g. "Pending", "In transit", "Awaiting pickup". */
    val status: String?,
    /** `fields["Rank"]`: place in the queue. */
    val rank: String?,
    /** `fields["Placed"]` as the library wrote it. */
    val placed: String?,
    /** `fields["Pickup location"]`, title-cased when the library shouts it ("RIVERTON LIBRARY"). */
    val pickup: String?,
    val cancelHref: String?,
    val editHref: String?,
) {
    enum class Kind { READY, PENDING }

    val isReady: Boolean get() = kind == Kind.READY

    /** The status chip: "Ready for pickup", or the library's own pending status. */
    val statusLabel: String get() = if (isReady) "Ready for pickup" else status ?: "Pending"

    val isInTransit: Boolean get() = !isReady && statusLabel.contains("transit", ignoreCase = true)

    /**
     * Ready: "Pick up at Riverton Library". Pending: "#4 in queue · Placed 2 Oct" and the pickup
     * branch on a second line.
     */
    val meta: String?
        get() {
            val pickupLine = pickup?.let { "Pick up at $it" }
            if (isReady) return pickupLine
            val first = buildList {
                rank?.let { add(if (it.toIntOrNull() != null) "#$it in queue" else "Rank $it") }
                placed?.let { add("Placed ${Formatting.parseDueDate(it)?.let(BorrowedDates::short) ?: it}") }
            }
            val lines = listOfNotNull(first.takeIf { it.isNotEmpty() }?.joinToString(" · "), pickupLine)
            return lines.takeIf { it.isNotEmpty() }?.joinToString("\n")
        }

    companion object {
        fun of(item: PatronItem, kind: Kind): ReservationRow {
            val ref = BookRef.from(item)
            return ReservationRow(
                id = item.irn.nonBlank() ?: item.cancelHref ?: item.editHref ?: ref.id,
                kind = kind,
                item = item,
                ref = ref,
                status = item.fields["Status"].nonBlank(),
                rank = item.fields["Rank"].nonBlank(),
                placed = item.fields["Placed"].nonBlank(),
                pickup = displayName(item.fields["Pickup location"]),
                cancelHref = item.cancelHref,
                editHref = item.editHref,
            )
        }

        /** "RIVERTON LIBRARY" → "Riverton Library"; mixed case is kept. */
        fun displayName(raw: String?): String? {
            val name = raw.nonBlank() ?: return null
            if (name != name.uppercase(Locale.ROOT) || name.none { it.isLetter() }) return name
            val lower = name.lowercase(Locale.ROOT)
            return buildString(lower.length) {
                lower.forEachIndexed { index, c ->
                    val startsWord = index == 0 || !lower[index - 1].isLetterOrDigit() && lower[index - 1] != '\''
                    append(if (startsWord) c.titlecaseChar() else c)
                }
            }
        }
    }
}

/** Reservations split for display: ready for pickup first, then pending. */
data class ReservationSections(
    val ready: List<ReservationRow> = emptyList(),
    val pending: List<ReservationRow> = emptyList(),
) {
    val count: Int get() = ready.size + pending.size
    val isEmpty: Boolean get() = count == 0

    fun row(id: String): ReservationRow? = (ready + pending).firstOrNull { it.id == id }

    /** Without the row [id] (a cancelled reservation, before the list is reloaded). */
    fun removing(id: String) = ReservationSections(ready.filter { it.id != id }, pending.filter { it.id != id })

    companion object {
        /** From the core's two lists. A row the library lists on both pages is shown once, as ready. */
        fun from(reservations: Reservations): ReservationSections {
            val ids = UniqueIds()
            fun row(item: PatronItem, kind: ReservationRow.Kind) =
                ReservationRow.of(item, kind).let { it.copy(id = ids.claim(it.id)) }
            val ready = reservations.ready.items.map { row(it, ReservationRow.Kind.READY) }
            val readyIrns = reservations.ready.items.mapNotNull { it.irn.nonBlank() }.toSet()
            val pending = reservations.pending.items
                .filter { item -> item.irn.nonBlank()?.let { it !in readyIrns } ?: true }
                .map { row(it, ReservationRow.Kind.PENDING) }
            return ReservationSections(ready, pending)
        }
    }
}

/** The text of the "Cancel this reservation?" confirmation. */
object CancelConfirm {
    fun message(row: ReservationRow): String {
        if (row.isReady) {
            val place = row.pickup?.let { " at $it" }.orEmpty()
            return "“${row.ref.title}” is waiting for you$place. Cancelling releases it to the next person."
        }
        return "“${row.ref.title}” will be removed from your reservations and you'll lose your place in the queue."
    }
}

/** Keeps row ids unique within one list: a repeated key gets `#2`, `#3`, …. */
private class UniqueIds {
    private val used = mutableSetOf<String>()

    fun claim(id: String): String {
        var unique = id
        var suffix = 1
        while (unique in used) {
            suffix++
            unique = "$id#$suffix"
        }
        used += unique
        return unique
    }
}

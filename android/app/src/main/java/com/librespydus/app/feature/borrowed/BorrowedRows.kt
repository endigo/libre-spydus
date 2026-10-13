package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Copy
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

/** `/renew/i`: the loan's renew link (`renewLoan(url)`, §5.12). */
val PatronItem.renewHref: String? get() = actionHref("renew")

private fun String?.nonBlank(): String? = this?.trim()?.takeIf { it.isNotEmpty() }

/** Borrowed's date wording ("10 Oct"), as the library's own dates are English. */
internal object BorrowedDates {
    private val SHORT: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM", Locale.ENGLISH)
    private val SHORT_YEAR: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM yyyy", Locale.ENGLISH)

    /** "10 Oct", with the year only when it isn't [today]'s ("10 Jan 2027", §7.1). */
    fun short(date: LocalDate, today: LocalDate): String = (if (date.year == today.year) SHORT else SHORT_YEAR).format(date)
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
    /** The "Renew loan" link; null when the library offers no renewal for this loan. */
    val renewHref: String? = item.renewHref,
) {
    /** The due date as `loans.dueOn` writes it ("27 Oct"); an unreadable date as the library wrote it. */
    val dueDate: String?
        get() = due?.let { BorrowedDates.short(it.dueDate, today = it.dueDate.minusDays(it.daysLeft)) } ?: dueText

    /** The meta line; an unreadable date is shown as the library wrote it. */
    val meta: String?
        get() {
            due?.let { return dueLine(it) }
            val text = dueText ?: return null
            return if (text.contains("due", ignoreCase = true)) text else Copy.Loans.dueOn(text)
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
            val date = BorrowedDates.short(due.dueDate, today = due.dueDate.minusDays(due.daysLeft))
            return when {
                due.daysLeft < 0 -> Copy.Loans.lineDays(date, Copy.Loans.daysOverdue((-due.daysLeft).toInt()))
                due.daysLeft == 0L -> Copy.Loans.lineToday(date)
                due.daysLeft == 1L -> Copy.Loans.lineTomorrow(date)
                else -> Copy.Loans.lineDays(date, Copy.Loans.daysLeft(due.daysLeft.toInt()))
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

/** One reservation card, ready to collect or pending. */
data class ReservationRow(
    /** The reservation's IRN, else its cancel/edit link (unique within [ReservationSections]). */
    val id: String,
    val kind: Kind,
    val item: PatronItem,
    val ref: BookRef,
    /** The library's status, e.g. "In transit"; null when it only names the pickup branch ([StatusText]). */
    val status: String?,
    /** `fields["Rank"]`: place in the queue. */
    val rank: String?,
    /** `fields["Placed"]` as the library wrote it. */
    val placed: String?,
    /** `fields["Pickup location"]`, or the branch named in the status text; title-cased when the library shouts it. */
    val pickup: String?,
    val cancelHref: String?,
    val editHref: String?,
    /** Ready reservations: the last day to collect, as the library wrote it ("17 Oct 2026"). */
    val pickupBy: String? = null,
    /** For the dates in [meta]: the year only when it isn't this one. */
    val today: LocalDate = LocalDate.now(),
) {
    enum class Kind { READY, PENDING }

    val isReady: Boolean get() = kind == Kind.READY

    /** The status chip: "Ready to collect", or the library's own pending status. */
    val statusLabel: String get() = if (isReady) Copy.YourBooks.readyToCollect else status ?: Copy.YourBooks.pending

    val isInTransit: Boolean get() = !isReady && statusLabel.contains("transit", ignoreCase = true)

    /**
     * Ready: "Collect from Riverton Library by 17 Oct". Pending: "#4 in queue · Placed 2 Oct" and
     * "Collect from Riverton Library" on a second line.
     */
    val meta: String? get() = metaLines(spoken = false)?.joinToString("\n")

    /** [meta] as a screen reader says it: the queue place as "number 4 in the queue" (§7.1). */
    val spokenMeta: String? get() = metaLines(spoken = true)?.joinToString(", ")

    private fun metaLines(spoken: Boolean): List<String>? {
        val pickupLine = pickup?.let(Copy.YourBooks::collectFrom)
        if (isReady) {
            val by = pickupBy?.let(::shortDate) ?: return pickupLine?.let(::listOf)
            return listOf(pickup?.let { Copy.YourBooks.collectFromBy(it, by) } ?: Copy.YourBooks.collectBy(by))
        }
        val first = buildList {
            rank?.let { add(queuePlace(it, spoken)) }
            placed?.let { add(Copy.YourBooks.placed(shortDate(it))) }
        }
        val joined = first.takeIf { it.isNotEmpty() }?.joinToString(if (spoken) ", " else Copy.Common.separator)
        return listOfNotNull(joined, pickupLine).takeIf { it.isNotEmpty() }
    }

    private fun queuePlace(rank: String, spoken: Boolean): String = when {
        rank.toIntOrNull() == null -> if (spoken) Copy.YourBooks.queuePlaceTextLabel(rank) else Copy.YourBooks.queuePlaceText(rank)
        spoken -> Copy.YourBooks.queuePlaceLabel(rank)
        else -> Copy.YourBooks.queuePlace(rank)
    }

    companion object {
        fun of(item: PatronItem, kind: Kind, today: LocalDate = LocalDate.now()): ReservationRow {
            val ref = BookRef.from(item)
            val statusText = StatusText.parse(item.fields["Status"])
            return ReservationRow(
                id = item.irn.nonBlank() ?: item.cancelHref ?: item.editHref ?: ref.id,
                kind = kind,
                item = item,
                ref = ref,
                status = statusText.status,
                rank = item.fields["Rank"].nonBlank(),
                placed = item.fields["Placed"].nonBlank(),
                pickup = displayName(item.fields["Pickup location"]) ?: displayName(statusText.pickup),
                cancelHref = item.cancelHref,
                editHref = item.editHref,
                pickupBy = statusText.pickupBy,
                today = today,
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

/**
 * A reservation's `Status` field. Some libraries (Canning) put the pickup branch there instead of a
 * status: "Requested pickup location: RIVERTON LIBRARY" while pending, "Please pickup from:
 * RIVERTON LIBRARY by 17 Oct 2026" once ready. Those become a branch (and collect-by date) with no
 * status; anything else is kept as the status.
 */
data class StatusText(val status: String? = null, val pickup: String? = null, val pickupBy: String? = null) {
    companion object {
        private val REQUESTED = Regex("""^requested\s+pick\s?-?up\s+location\s*:\s*(.+)$""", RegexOption.IGNORE_CASE)
        private val READY = Regex("""^please\s+pick\s?-?up\s+from\s*:\s*(.+?)(?:\s+by\s+(.+))?$""", RegexOption.IGNORE_CASE)

        fun parse(raw: String?): StatusText {
            val text = raw.nonBlank() ?: return StatusText()
            REQUESTED.find(text)?.let { return StatusText(pickup = it.groupValues[1].nonBlank()) }
            READY.find(text)?.let { return StatusText(pickup = it.groupValues[1].nonBlank(), pickupBy = it.groupValues[2].nonBlank()) }
            return StatusText(status = text)
        }
    }
}

/** Reservations split for display: ready to collect first, then pending. */
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

/** The texts of a renewal (§5.12): the "Renew this loan?" confirmation and the result toast. */
object RenewText {
    /** "“Title” is due 14 Oct. Renewing asks the library for a later due date." */
    fun confirm(row: LoanRow): String =
        row.dueDate?.let { Copy.YourBooks.renewMessage(row.ref.title, it) } ?: Copy.YourBooks.renewMessageNoDate(row.ref.title)

    /** "Renewed · Due 4 Nov" for the date the library now gives ([due], e.g. "4 Nov 2026"); "Renewed" without one. */
    fun renewed(due: String?, today: LocalDate = LocalDate.now()): String {
        val text = due.nonBlank() ?: return Copy.YourBooks.renewedNoDate
        return Copy.YourBooks.renewed(Formatting.parseDueDate(text)?.let { BorrowedDates.short(it, today) } ?: text)
    }
}

/** The text of the "Cancel this reservation?" confirmation. */
object CancelConfirm {
    fun message(row: ReservationRow): String {
        if (row.isReady) {
            return row.pickup?.let { Copy.YourBooks.cancelReadyMessage(row.ref.title, it) }
                ?: Copy.YourBooks.cancelReadyMessageNoBranch(row.ref.title)
        }
        return Copy.YourBooks.cancelPendingMessage(row.ref.title)
    }
}

/** A date the library wrote ("17 Oct 2026") as "17 Oct"; unreadable text as written. */
private fun ReservationRow.shortDate(text: String): String =
    Formatting.parseDueDate(text)?.let { BorrowedDates.short(it, today) } ?: text

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

package com.librespydus.app.feature.detail

import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.feature.borrowed.BorrowedDates
import com.librespydus.app.feature.borrowed.LoanRow
import com.librespydus.app.feature.borrowed.ReservationRow
import uniffi.librespydus_core.AccountHolding
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.HoldingKind
import uniffi.librespydus_core.PatronItem
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

/**
 * The status pill that takes the place of "Reserve" when the patron already has the format on
 * screen (docs/native-apps.md §6 "Already yours"): "On loan · Due 14 Oct", "Ready to collect ·
 * Riverton Library", "Reserved · Riverton Library". Tapping it opens Your Books on [segment]. The
 * due date and pickup branch are read the way Your Books reads them (Gregorian dates). Pure, so
 * the wording is unit-tested (HeldStatusTest).
 */
data class HeldStatus(
    val text: String,
    /** What TalkBack reads, e.g. "On loan, due 14 October". */
    val accessibilityLabel: String,
    val tone: Tone,
    val segment: BorrowedSegment,
) {
    enum class Tone { LOAN, OVERDUE, READY, RESERVED }

    companion object {
        /** The pill's click label: TalkBack reads "Double-tap to open Your Books". */
        const val ACTION = "Open Your Books"

        private val LONG_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMMM", Locale.ENGLISH)

        /**
         * The loan or reservation of [bibIrns] in the saved account, or null (signed out, no
         * snapshot yet, no IRNs known): the page keeps "Reserve".
         */
        fun holding(snapshot: AccountSnapshot?, bibIrns: List<String>): AccountHolding? {
            if (snapshot == null || bibIrns.isEmpty()) return null
            return CoreHelpers.accountHolding(snapshot, bibIrns)
        }

        fun of(holding: AccountHolding, today: LocalDate = LocalDate.now()): HeldStatus = when (holding.kind) {
            HoldingKind.LOAN -> loan(holding.item, today)
            HoldingKind.RESERVATION_READY -> ready(holding.item)
            HoldingKind.RESERVATION_PENDING -> pending(holding.item)
        }

        private fun loan(item: PatronItem, today: LocalDate): HeldStatus {
            val row = LoanRow.of(item, today)
            val due = row.due
            val raw = row.dueText
            return when {
                due != null && due.isOverdue -> HeldStatus(
                    "On loan · Overdue",
                    "On loan, overdue, was due ${LONG_DATE.format(due.dueDate)}",
                    Tone.OVERDUE,
                    BorrowedSegment.LOANS,
                )
                due != null -> HeldStatus(
                    "On loan · Due ${BorrowedDates.short(due.dueDate)}",
                    "On loan, due ${LONG_DATE.format(due.dueDate)}",
                    Tone.LOAN,
                    BorrowedSegment.LOANS,
                )
                // A date the core can't read is shown as the library wrote it.
                raw != null -> HeldStatus("On loan · Due $raw", "On loan, due $raw", Tone.LOAN, BorrowedSegment.LOANS)
                else -> HeldStatus("On loan", "On loan", Tone.LOAN, BorrowedSegment.LOANS)
            }
        }

        private fun ready(item: PatronItem): HeldStatus {
            val pickup = ReservationRow.of(item, ReservationRow.Kind.READY).pickup
                ?: return HeldStatus("Ready to collect", "Ready to collect", Tone.READY, BorrowedSegment.RESERVATIONS)
            return HeldStatus("Ready to collect · $pickup", "Ready to collect at $pickup", Tone.READY, BorrowedSegment.RESERVATIONS)
        }

        private fun pending(item: PatronItem): HeldStatus {
            val row = ReservationRow.of(item, ReservationRow.Kind.PENDING)
            val rank = row.rank
            val (text, spoken) = when {
                row.pickup != null -> "Reserved · ${row.pickup}" to "Reserved, pick up at ${row.pickup}"
                rank != null && rank.toIntOrNull() != null -> "Reserved · #$rank in queue" to "Reserved, number $rank in the queue"
                rank != null -> "Reserved · Rank $rank" to "Reserved, rank $rank"
                else -> "Reserved" to "Reserved"
            }
            return HeldStatus(text, spoken, Tone.RESERVED, BorrowedSegment.RESERVATIONS)
        }
    }
}

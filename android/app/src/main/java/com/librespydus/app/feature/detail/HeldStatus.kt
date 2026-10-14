package com.librespydus.app.feature.detail

import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.Copy
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
 * Riverton Library", "Reserved · Riverton Library". Tapping it asks to renew a loan the library
 * lets the patron renew ([renewableLoan], §5.12), and otherwise opens Your Books on [segment]. The
 * due date and pickup branch are read the way Your Books reads them (Gregorian dates). Pure, so
 * the wording is unit-tested (HeldStatusTest).
 */
data class HeldStatus(
    val text: String,
    /** What TalkBack reads, e.g. "On loan, due 14 October". */
    val accessibilityLabel: String,
    val tone: Tone,
    val segment: BorrowedSegment,
    /** The loan, when it has a "Renew loan" link: tapping the pill asks to renew it. */
    val renewableLoan: PatronItem? = null,
) {
    enum class Tone { LOAN, OVERDUE, READY, RESERVED }

    /** What TalkBack reads after [accessibilityLabel]: what tapping the pill does. */
    val hint: String get() = if (renewableLoan == null) HINT else Copy.YourBooks.renewHint

    companion object {
        /** The pill's hint when it opens Your Books ("On loan, due 14 October. Opens Your Books"). */
        const val HINT = Copy.Detail.heldHint

        private val LONG_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMMM", Locale.ENGLISH)
        private val LONG_DATE_YEAR: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMMM yyyy", Locale.ENGLISH)

        /** "14 October", with the year only when it isn't [today]'s. */
        private fun longDate(date: LocalDate, today: LocalDate): String =
            (if (date.year == today.year) LONG_DATE else LONG_DATE_YEAR).format(date)

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
            val status = when {
                due != null && due.isOverdue -> HeldStatus(
                    Copy.Detail.onLoanOverdue,
                    Copy.Detail.onLoanOverdueLabel(longDate(due.dueDate, today)),
                    Tone.OVERDUE,
                    BorrowedSegment.LOANS,
                )
                due != null -> HeldStatus(
                    Copy.Detail.onLoanDue(BorrowedDates.short(due.dueDate, today)),
                    Copy.Detail.onLoanDueLabel(longDate(due.dueDate, today)),
                    Tone.LOAN,
                    BorrowedSegment.LOANS,
                )
                // A date the core can't read is shown as the library wrote it.
                raw != null -> HeldStatus(Copy.Detail.onLoanDue(raw), Copy.Detail.onLoanDueLabel(raw), Tone.LOAN, BorrowedSegment.LOANS)
                else -> HeldStatus(Copy.Detail.onLoan, Copy.Detail.onLoan, Tone.LOAN, BorrowedSegment.LOANS)
            }
            return status.copy(renewableLoan = item.takeIf { row.renewHref != null })
        }

        private fun ready(item: PatronItem): HeldStatus {
            val pickup = ReservationRow.of(item, ReservationRow.Kind.READY).pickup
                ?: return HeldStatus(Copy.Detail.readyToCollect, Copy.Detail.readyToCollect, Tone.READY, BorrowedSegment.RESERVATIONS)
            return HeldStatus(Copy.Detail.readyToCollectAt(pickup), Copy.Detail.readyToCollectAtLabel(pickup), Tone.READY, BorrowedSegment.RESERVATIONS)
        }

        private fun pending(item: PatronItem): HeldStatus {
            val row = ReservationRow.of(item, ReservationRow.Kind.PENDING)
            val rank = row.rank
            val (text, spoken) = when {
                row.pickup != null -> Copy.Detail.reservedAt(row.pickup) to Copy.Detail.reservedAtLabel(row.pickup)
                rank != null && rank.toIntOrNull() != null -> Copy.Detail.reservedQueue(rank) to Copy.Detail.reservedQueueLabel(rank)
                rank != null -> Copy.Detail.reservedQueueText(rank) to Copy.Detail.reservedQueueTextLabel(rank)
                else -> Copy.Detail.reserved to Copy.Detail.reserved
            }
            return HeldStatus(text, spoken, Tone.RESERVED, BorrowedSegment.RESERVATIONS)
        }
    }
}

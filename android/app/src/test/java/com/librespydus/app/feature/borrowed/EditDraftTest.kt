package com.librespydus.app.feature.borrowed

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.EditOptions
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.PickupChoice
import java.time.LocalDate
import java.util.Locale

class EditDraftTest {
    private val today = LocalDate.of(2026, 10, 12)
    private val choices = listOf(
        PickupChoice("25353", "CANNINGTON LIBRARY"),
        PickupChoice("25363", "RIVERTON LIBRARY"),
        PickupChoice("25379", "WILLETTON & DISTRICT"),
    )

    private fun options(selected: String? = "25363", delay: String? = "2026-11-01", expiry: String? = null) =
        EditOptions(canEdit = true, pickupChoices = choices, pickupSelected = selected, delayUntil = delay, notRequiredAfter = expiry, message = null)

    private fun date(text: String) = LocalDate.parse(text)

    @Test
    fun untouchedFormSendsOnlyTheShownPickup() {
        val draft = EditDraft.from(options(), today = today)
        assertEquals("25363", draft.pickup)
        assertTrue(draft.delay.isOn)
        assertEquals(date("2026-11-01"), draft.delay.date)
        assertFalse(draft.expiry.isOn)
        assertEquals(today.plusMonths(3), draft.expiry.date)
        assertFalse(draft.hasChanges)
        assertEquals(EditOverrides("25363", null, null, clearDelay = false, clearExpiry = false), draft.overrides())
    }

    @Test
    fun changedPickupAndDatesAreSent() {
        var draft = EditDraft.from(options(), today = today)
        draft = draft.copy(
            pickup = "25353",
            delay = draft.delay.copy(date = date("2026-11-15")),
            expiry = draft.expiry.copy(isOn = true, date = date("2027-01-31")),
        )
        assertTrue(draft.hasChanges)
        assertEquals(EditOverrides("25353", "2026-11-15", "2027-01-31", clearDelay = false, clearExpiry = false), draft.overrides())
    }

    @Test
    fun switchingOffAnExistingDateClearsIt() {
        var draft = EditDraft.from(options(expiry = "2027-03-01"), today = today)
        draft = draft.copy(delay = draft.delay.copy(isOn = false), expiry = draft.expiry.copy(isOn = false))
        assertTrue(draft.delay.isCleared)
        assertTrue(draft.expiry.isCleared)
        assertEquals(EditOverrides("25363", null, null, clearDelay = true, clearExpiry = true), draft.overrides())

        // Switching off a date the form never had changes nothing.
        var fresh = EditDraft.from(options(delay = null), today = today)
        assertEquals(today.plusDays(7), fresh.delay.date)
        fresh = fresh.copy(delay = fresh.delay.copy(isOn = true))
        assertTrue(fresh.hasChanges)
        fresh = fresh.copy(delay = fresh.delay.copy(isOn = false))
        assertFalse(fresh.delay.isCleared)
        assertFalse(fresh.hasChanges)
    }

    @Test
    fun pickupFallsBackToTheListedBranchThenTheFirst() {
        assertEquals("25379", EditDraft.from(options(selected = null), currentPickup = "Willetton & District", today = today).pickup)
        assertEquals("25353", EditDraft.from(options(selected = "99999"), currentPickup = null, today = today).pickup)
        val digital = EditDraft.from(EditOptions(true, emptyList(), null, null, null, null), today = today)
        assertNull(digital.pickup)
        assertNull(digital.overrides().pickup)
    }

    @Test
    fun datesAreWrittenInTheFormsOwnFormat() {
        var draft = EditDraft.from(options(delay = "01/11/2026"), today = today)
        assertEquals(date("2026-11-01"), draft.delay.date)
        assertNull(draft.overrides().delayUntil)
        draft = draft.copy(delay = draft.delay.copy(date = date("2026-11-20")))
        assertEquals("20/11/2026", draft.overrides().delayUntil)
        assertEquals("2026-12-05", EditDraft.formValue(date("2026-12-05"), like = null))
    }

    @Test
    fun libraryDatesStayGregorianOnOtherDeviceCalendars() {
        val saved = Locale.getDefault()
        try {
            // A Thai device with the Buddhist calendar would write 2026 as 2569.
            Locale.setDefault(Locale.forLanguageTag("th-TH-u-ca-buddhist-nu-thai"))
            val draft = EditDraft.from(options(), today = today)
            assertEquals(2026, draft.delay.date.year)
            assertFalse(draft.hasChanges)
            assertNull(draft.overrides().delayUntil)
            assertEquals("2027-01-31", EditDraft.formValue(date("2027-01-31"), like = "2026-11-01"))
            assertEquals("31/01/2027", EditDraft.formValue(date("2027-01-31"), like = "01/11/2026"))
            assertEquals("Due tomorrow · 13 Oct", LoanRow.of(patronItem("Book", fields = mapOf("Due" to "13 Oct 2026")), today).meta)
            assertEquals("13 Oct", BorrowedDates.short(date("2026-10-13")))
        } finally {
            Locale.setDefault(saved)
        }
    }
}

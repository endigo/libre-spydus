package com.librespydus.app.core

import kotlinx.coroutines.test.currentTime
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.SpydusException
import uniffi.librespydus_core.decodeAccountSnapshot
import uniffi.librespydus_core.encodeAccountSnapshot
import java.io.File
import java.time.LocalDate

class DemoBackendTest {
    private val today = LocalDate.of(2026, 10, 12)
    private val origin = "https://hutt.spydus.co.nz"

    /** The real bundled assets (unit tests run with the module dir as working directory). */
    private fun demoData(origin: String = this.origin) =
        DemoData.load(read = { File("src/main/assets/demo/$it").readText() }, origin = origin, today = today)

    private fun backend(public: FakeBackend = FakeBackend(origin)) = DemoBackend(public, demoData(public.origin), today)

    private suspend fun signedIn(b: DemoBackend) = b.login("demo", "demo")

    @Test
    fun loginAcceptsOnlyDemoDemo() = runTest {
        val b = backend()
        assertEquals(DemoBackend.SESSION, b.login(" demo ", "demo"))
        for ((user, pin) in listOf("demo" to "DEMO", "someone" to "demo", "" to "")) {
            try {
                b.login(user, pin)
                fail("expected CredentialsRejected for $user/$pin")
            } catch (e: SpydusException.CredentialsRejected) {
                assertEquals("credentials rejected", e.message)
            }
        }
    }

    @Test
    fun accountCallsNeedTheDemoSession() = runTest {
        val b = backend()
        try {
            b.loans("some-real-session")
            fail("expected NotLoggedIn")
        } catch (_: SpydusException.NotLoggedIn) {
        }
        assertEquals("Test Borrower", b.account(signedIn(b)).name)
    }

    @Test
    fun fixturesAreRewrittenToTheSelectedOrigin() = runTest {
        val b = backend()
        val session = signedIn(b)
        val links = b.account(session)
        assertTrue(links.loansUrl!!.startsWith("$origin/"))
        val all = b.loans(session).items + b.reservations(session).ready.items + b.reservations(session).pending.items
        assertTrue(all.isNotEmpty())
        all.forEach { item ->
            item.bibUrl?.let { assertTrue(it, it.startsWith("$origin/")) }
            item.actions.forEach { assertTrue(it.href, it.href.startsWith("$origin/")) }
        }
    }

    @Test
    fun loansGetDueDatesRelativeToToday() = runTest {
        val b = backend()
        val loans = b.loans(signedIn(b)).items
        assertEquals(4, loans.size)
        assertEquals(listOf("14 Oct 2026", "21 Oct 2026", "28 Oct 2026", "11 Oct 2026"), loans.map { it.fields["Due"] })
        assertEquals("Due 14 Oct · 2 days left", Formatting.dueLine(loans[0].fields["Due"], today))
        assertTrue(Formatting.isDueSoon(loans[3].fields["Due"], today))
    }

    @Test
    fun cancelRemovesTheRowAfterADelay() = runTest {
        val b = backend()
        val session = signedIn(b)
        val before = b.reservations(session)
        val target = before.ready.items.first()
        val cancel = target.actions.first { it.label.contains("cancel", ignoreCase = true) }.href

        val start = currentTime
        val result = b.cancelReservation(cancel, session)
        assertTrue(currentTime - start >= 600)
        assertTrue(result.ok)
        assertEquals("Reservation cancelled.", result.message)
        val after = b.reservations(session)
        assertFalse(after.ready.items.any { it.irn == target.irn })
        assertEquals(before.pending.items, after.pending.items)

        val again = b.cancelReservation(cancel, session)
        assertFalse(again.ok)
    }

    @Test
    fun reserveAddsAPendingRowForTheSearchedTitle() = runTest {
        val public = FakeBackend(origin)
        val b = backend(public)
        val session = signedIn(b)
        // Not a demo title: the real library's results (FakeBackend's Project Hail Mary).
        val record = b.search("andy weir").records.single()
        val reserveUrl = record.formats.first { it.reserveUrl != null }.reserveUrl!!

        val result = b.placeReservation(reserveUrl, session, xdays = "30", pickup = "25353")
        assertTrue(result.ok)
        val row = b.reservations(session).pending.items.first()
        assertEquals("Project Hail Mary", row.title)
        assertEquals("CANNINGTON LIBRARY", row.fields["Pickup location"])
        assertEquals("12 Oct 2026", row.fields["Placed"])
        assertEquals(listOf("andy weir"), public.searches)
        // FakeBackend's availability names no bib record, so the row keeps the search link.
        assertNull(row.bibIrn)
        assertEquals(record.detailUrl, row.bibUrl)

        val cancel = row.actions.first { it.label.contains("cancel", ignoreCase = true) }.href
        assertTrue(b.cancelReservation(cancel, session).ok)
        assertFalse(b.reservations(session).pending.items.any { it.irn == row.irn })
    }

    @Test
    fun editChangesThePickupLocation() = runTest {
        val b = backend()
        val session = signedIn(b)
        val item = b.reservations(session).pending.items.first { it.actions.isNotEmpty() }
        val edit = item.actions.first { it.label.contains("edit", ignoreCase = true) }.href
        val options = b.editOptions(edit, session)
        assertTrue(options.canEdit)
        val result = b.editReservation(edit, session, EditOverrides("25379", null, null, false, false))
        assertTrue(result.ok)
        val updated = b.reservations(session).pending.items.first { it.irn == item.irn }
        assertEquals("WILLETTON & DISTRICT", updated.fields["Pickup location"])
    }

    @Test
    fun theSnapshotIsTheDemoAccountAsItIsNow() = runTest {
        val b = DemoBackend(FakeBackend(origin), demoData(), today, now = { 1_234L })
        val session = signedIn(b)
        val start = currentTime
        val snapshot = b.accountSnapshot(session)
        assertTrue(currentTime - start >= 1_500) // slow enough to see the refreshing state
        assertEquals("Test Borrower", snapshot.accountName)
        assertEquals(1_234L, snapshot.fetchedAtMs)
        assertEquals(b.loans(session), snapshot.loans)
        assertEquals(b.reservations(session), snapshot.reservations)

        val ready = snapshot.reservations.ready.items.first()
        b.cancelReservation(ready.actions.first { it.label.contains("cancel", ignoreCase = true) }.href, session)
        assertFalse(b.accountSnapshot(session).reservations.ready.items.any { it.irn == ready.irn })
    }

    @Test
    fun theSnapshotVersionMatchesTheCore() = runTest {
        val b = backend()
        val snapshot = b.accountSnapshot(signedIn(b))
        // decodeAccountSnapshot rejects any other ACCOUNT_SNAPSHOT_VERSION.
        assertEquals(snapshot, decodeAccountSnapshot(encodeAccountSnapshot(snapshot)))
    }

    @Test
    fun theDebugFailureFlagFailsLikeAnUnreachableLibrary() = runTest {
        val b = DemoBackend(FakeBackend(origin), demoData(), today, refreshFails = true)
        try {
            b.accountSnapshot(signedIn(b))
            fail("expected Network")
        } catch (_: SpydusException.Network) {
        }
        try {
            b.accountSnapshot("some-real-session")
            fail("expected NotLoggedIn")
        } catch (_: SpydusException.NotLoggedIn) {
        }
    }

    @Test
    fun reserveOptionsComeFromTheFixture() = runTest {
        val b = backend()
        val options = b.reserveOptions("$origin/any", signedIn(b))
        assertTrue(options.canReserve)
        assertEquals(listOf("0", "30", "730"), options.expiryChoices.map { it.days })
        assertTrue(b.isDemo)
        assertEquals(origin, b.origin)
    }
}

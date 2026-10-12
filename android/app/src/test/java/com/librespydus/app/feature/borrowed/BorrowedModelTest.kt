package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.feature.onboarding.AccountFixture
import com.librespydus.app.feature.onboarding.TODAY
import com.librespydus.app.feature.onboarding.accountFixture
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.SpydusException
import java.util.concurrent.atomic.AtomicInteger
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

/** BorrowedModel against DemoBackend fixtures (no network, nothing posted). */
class BorrowedModelTest {
    @get:Rule val tmp = TemporaryFolder()
    private val scopes = mutableListOf<CoroutineScope>()
    private var clock = 1_000_000L

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private fun TestScope.setUp(session: String? = DemoBackend.SESSION): Triple<AccountFixture, BorrowedModel, CountingBackend> {
        var counting: CountingBackend? = null
        val f = accountFixture(tmp.root, session = session, wrap = { CountingBackend(it).also { c -> counting = c } })
        scopes += f.scope
        return Triple(f, BorrowedModel(f.state, today = { TODAY }, now = { clock }), counting!!)
    }

    @Test
    fun loadsSortedLoansAndSplitReservations() = runTest {
        val (_, model, _) = setUp()
        model.load()
        assertEquals(listOf(-1L, 2L, 9L, 16L), model.loans.value.valueOrNull?.mapNotNull { it.due?.daysLeft })
        assertEquals(1, model.reservations.value.valueOrNull?.ready?.size)
        assertEquals(2, model.reservations.value.valueOrNull?.pending?.size)
    }

    @Test
    fun cancelIsSubmittedOnceAndRemovesTheRow() = runTest {
        val (_, model, backend) = setUp()
        model.load()
        val row = model.reservations.value.valueOrNull!!.ready.first()
        model.askToCancel(row)
        assertSame(row, model.confirmingCancel.value)

        listOf(async { model.cancel(row) }, async { model.cancel(row) }).awaitAll()

        assertEquals(1, backend.cancelCalls.get())
        assertNull(model.confirmingCancel.value)
        assertEquals("Reservation cancelled.", model.notice.value?.message)
        assertEquals(false, model.notice.value?.isError)
        assertTrue(model.reservations.value.valueOrNull!!.ready.isEmpty())
        assertFalse(model.isBusy(row))
    }

    @Test
    fun aRefusedCancelIsShownAsAnError() = runTest {
        val (_, model, _) = setUp()
        model.load()
        val row = model.reservations.value.valueOrNull!!.ready.first()
        // Not in the demo lists: DemoBackend answers ok = false.
        val stale = row.copy(cancelHref = "https://canning.spydus.com/gone")
        model.cancel(stale)
        assertEquals("This reservation can't be cancelled.", model.notice.value?.message)
        assertEquals(true, model.notice.value?.isError)
        assertNotNull(model.reservations.value.valueOrNull!!.row(row.id))
    }

    @Test
    fun aFailedCancelIsReported() = runTest {
        val (_, model, backend) = setUp()
        model.load()
        backend.failCancel = SpydusException.Network("network error: connect timed out after 10s")
        val row = model.reservations.value.valueOrNull!!.ready.first()
        model.cancel(row)
        assertEquals("Can't reach City of Canning. Check your connection.", model.notice.value?.message)
        assertEquals(true, model.notice.value?.isError)
        assertFalse(model.isBusy(row))
    }

    @Test
    fun editResultIsShownAndRefreshes() = runTest {
        val (_, model, backend) = setUp()
        model.load()
        val row = model.reservations.value.valueOrNull!!.pending.first { it.editHref != null }
        model.edit(row)
        assertEquals(row, model.editing.value)

        val result = backend.editReservation(row.editHref!!, DemoBackend.SESSION, EditOverrides("25379", null, null, false, false))
        model.finishEdit(EditOutcome.Saved(result), row)
        assertNull(model.editing.value)
        assertEquals("Reservation updated.", model.notice.value?.message)
        assertEquals("Willetton & District", model.reservations.value.valueOrNull?.row(row.id)?.pickup)
    }

    @Test
    fun aLostSessionSignsOutAndAsksForLogin() = runTest {
        val (f, model, backend) = setUp()
        assertTrue(f.state.isSignedIn.value)
        backend.failLoans = SpydusException.NotLoggedIn("not logged in") // expired at the library
        model.load()
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
        assertTrue(model.loans.value is BorrowedLoad.Failed)
    }

    @Test
    fun editSheetSignOutIsReportedAfterTheSheetCloses() = runTest {
        val (f, model, _) = setUp()
        model.load()
        val row = model.reservations.value.valueOrNull!!.pending.first { it.editHref != null }
        model.edit(row)
        model.finishEdit(EditOutcome.Failed(SpydusException.NotLoggedIn("not logged in")), row)
        assertNull(model.editing.value)
        assertTrue(f.state.isSignedIn.value) // nothing reported until the sheet has gone
        model.reportDeferredError()
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
    }

    @Test
    fun aSheetClosedWhileLoadingCantCloseAnother() = runTest {
        val (_, model, _) = setUp()
        model.load()
        val sections = model.reservations.value.valueOrNull!!
        val rows = (sections.ready + sections.pending).filter { it.editHref != null }
        assertEquals(2, rows.size)
        model.edit(rows[1])
        model.finishEdit(EditOutcome.Closed, rows[0])
        assertEquals(rows[1], model.editing.value)
    }

    @Test
    fun signInErrorsWaitForTheEditSheet() = runTest {
        val (f, model, backend) = setUp()
        model.load()
        val editable = model.reservations.value.valueOrNull!!.pending.first { it.editHref != null }
        model.edit(editable)
        backend.failReservations = SpydusException.NotLoggedIn("not logged in")
        model.reloadReservations()
        // Login can't show over the edit sheet.
        assertTrue(f.state.isSignedIn.value)
        assertFalse(f.state.loginNeeded.value)
        model.finishEdit(EditOutcome.Closed, editable)
        model.reportDeferredError()
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
    }

    @Test
    fun aRefreshFromBeforeACancelDoesntBringTheRowBack() = runTest {
        val (_, model, backend) = setUp()
        model.load()
        val row = model.reservations.value.valueOrNull!!.ready.first()

        backend.delayNextReservations = 400.milliseconds // fetched now, delivered late
        val refresh = launch { model.load(keepContent = true) }
        advanceTimeBy(50)
        model.cancel(row)
        refresh.join()

        assertEquals(1, backend.cancelCalls.get())
        assertNull(model.reservations.value.valueOrNull?.row(row.id))
    }

    @Test
    fun signedOutLoadsNothing() = runTest {
        val (f, model, backend) = setUp()
        f.state.signOut()
        model.load()
        assertEquals(0, backend.accountCalls.get())
        assertEquals(BorrowedLoad.Idle, model.loans.value)
        assertEquals(BorrowedLoad.Idle, model.reservations.value)
    }

    @Test
    fun theScreenReloadsOnNewInputsAndQuietlyAfterThirtySeconds() = runTest {
        val (f, model, backend) = setUp()
        assertEquals(false, model.pendingLoad()) // first appearance: full load
        model.load()
        assertNull(model.pendingLoad()) // back on the tab straight away: nothing
        clock += 31_000
        assertEquals(true, model.pendingLoad()) // a while later: quiet refresh
        model.load(keepContent = true)
        assertNull(model.pendingLoad())
        val calls = backend.accountCalls.get()

        f.state.signOut()
        assertEquals(false, model.pendingLoad()) // signed out: reload (to the signed-out state)
        model.load()
        assertEquals(calls, backend.accountCalls.get())
        clock += 31_000
        assertNull(model.pendingLoad()) // nothing to refresh while signed out
    }

    @Test
    fun noticesCanBeDismissedOnlyByTheirOwnId() = runTest {
        val (_, model, _) = setUp()
        model.load()
        model.cancel(model.reservations.value.valueOrNull!!.ready.first())
        val notice = model.notice.value!!
        model.dismissNotice(notice.id + 1)
        assertNotNull(model.notice.value)
        model.dismissNotice(notice.id)
        assertNull(model.notice.value)
    }
}

/** Forwards to a DemoBackend, counting account calls; can delay or fail reservations. */
internal class CountingBackend(private val inner: LibraryBackend) : LibraryBackend by inner {
    val cancelCalls = AtomicInteger()
    val accountCalls = AtomicInteger()

    /** The next `reservations` answers with what it fetched, but only after this delay. */
    @Volatile var delayNextReservations: Duration? = null
    @Volatile var failReservations: Exception? = null
    @Volatile var failCancel: Exception? = null
    @Volatile var failLoans: Exception? = null

    override suspend fun loans(session: String): uniffi.librespydus_core.PatronList {
        accountCalls.incrementAndGet()
        failLoans?.let { throw it }
        return inner.loans(session)
    }

    override suspend fun reservations(session: String): uniffi.librespydus_core.Reservations {
        accountCalls.incrementAndGet()
        failReservations?.let { throw it }
        val wait = delayNextReservations.also { delayNextReservations = null }
        val snapshot = inner.reservations(session)
        if (wait != null) delay(wait)
        return snapshot
    }

    override suspend fun cancelReservation(url: String, session: String): uniffi.librespydus_core.ReserveResult {
        cancelCalls.incrementAndGet()
        failCancel?.let { throw it }
        return inner.cancelReservation(url, session)
    }
}

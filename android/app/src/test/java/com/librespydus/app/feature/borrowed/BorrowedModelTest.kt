package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.onboarding.AccountFixture
import com.librespydus.app.feature.onboarding.TODAY
import com.librespydus.app.feature.onboarding.accountFixture
import com.librespydus.app.feature.onboarding.demoData
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
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
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.EditOverrides
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.SpydusException
import java.util.concurrent.atomic.AtomicInteger
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

/** BorrowedModel against DemoBackend fixtures (no network, nothing posted). */
class BorrowedModelTest {
    @get:Rule val tmp = TemporaryFolder()
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    /** Signed in (by default) with the first account snapshot loaded. */
    private fun TestScope.setUp(session: String? = DemoBackend.SESSION): Triple<AccountFixture, BorrowedModel, CountingBackend> {
        var counting: CountingBackend? = null
        val f = accountFixture(tmp.root, session = session, wrap = { CountingBackend(it).also { c -> counting = c } })
        scopes += f.scope
        return Triple(f, BorrowedModel(f.state), counting!!)
    }

    private fun content(f: AccountFixture): BorrowedContent {
        val account = f.state.account
        return BorrowedContent.of(account.snapshot.value, account.isRefreshing.value, account.lastRefreshError.value, TODAY)
    }

    @Test
    fun showsSortedLoansAndSplitReservationsFromTheSnapshot() = runTest {
        val (f, model, backend) = setUp()
        assertEquals(1, backend.snapshotCalls.get())
        val content = content(f)
        assertEquals(listOf(-1L, 2L, 9L, 16L), content.loans.valueOrNull?.mapNotNull { it.due?.daysLeft })
        assertEquals(1, content.reservations.valueOrNull?.ready?.size)
        assertEquals(2, content.reservations.valueOrNull?.pending?.size)
        assertEquals(content.reservations.valueOrNull, model.reservations())
        assertTrue(content.status is AccountStatus.Updated)
    }

    @Test
    fun cancelIsSubmittedOnceRemovesTheRowAndRefreshes() = runTest {
        val (f, model, backend) = setUp()
        val row = model.reservations()!!.ready.first()
        model.askToCancel(row)
        assertSame(row, model.confirmingCancel.value)

        listOf(async { model.cancel(row) }, async { model.cancel(row) }).awaitAll()

        assertEquals(1, backend.cancelCalls.get())
        assertNull(model.confirmingCancel.value)
        assertEquals("Reservation cancelled", model.notice.value?.message)
        assertEquals(false, model.notice.value?.isError)
        // Gone at once, before the refresh that confirms it.
        assertTrue(model.reservations()!!.ready.isEmpty())
        assertTrue(f.state.account.isRefreshing.value)
        assertFalse(model.isBusy(row))
        advanceUntilIdle()
        assertEquals(2, backend.snapshotCalls.get())
        assertTrue(model.reservations()!!.ready.isEmpty())
    }

    @Test
    fun aRefusedCancelIsShownAsAnError() = runTest {
        val (_, model, backend) = setUp()
        val row = model.reservations()!!.ready.first()
        // Not in the demo lists: DemoBackend answers ok = false.
        val stale = row.copy(cancelHref = "https://canning.spydus.com/gone")
        model.cancel(stale)
        assertEquals("This reservation can’t be cancelled right now.", model.notice.value?.message)
        assertEquals(true, model.notice.value?.isError)
        assertNotNull(model.reservations()!!.row(row.id))
        advanceUntilIdle()
        assertEquals(1, backend.snapshotCalls.get()) // nothing changed: no refresh
    }

    @Test
    fun aFailedCancelIsReported() = runTest {
        val (_, model, backend) = setUp()
        backend.failCancel = SpydusException.Network("network error: connect timed out after 10s")
        val row = model.reservations()!!.ready.first()
        model.cancel(row)
        assertEquals("Can’t reach City of Canning. Check your connection.", model.notice.value?.message)
        assertEquals(true, model.notice.value?.isError)
        assertFalse(model.isBusy(row))
        assertNotNull(model.reservations()!!.row(row.id))
    }

    @Test
    fun editResultIsShownAndRefreshes() = runTest {
        val (_, model, backend) = setUp()
        val row = model.reservations()!!.pending.first { it.editHref != null }
        model.edit(row)
        assertEquals(row, model.editing.value)

        val result = backend.editReservation(row.editHref!!, DemoBackend.SESSION, EditOverrides("25379", null, null, false, false))
        model.finishEdit(EditOutcome.Saved(result), row)
        assertNull(model.editing.value)
        assertEquals("Reservation updated", model.notice.value?.message)
        advanceUntilIdle()
        assertEquals(2, backend.snapshotCalls.get())
        assertEquals("Willetton & District", model.reservations()?.row(row.id)?.pickup)
    }

    @Test
    fun aLostSessionSignsOutAndAsksForLogin() = runTest {
        val (f, model, backend) = setUp()
        assertTrue(f.state.isSignedIn.value)
        backend.failSnapshot = SpydusException.NotLoggedIn("not logged in") // expired at the library
        f.state.account.refresh(force = true)
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
        assertNull(f.state.account.snapshot.value)
        assertNull(model.reservations())
    }

    @Test
    fun editSheetSignOutIsReportedAfterTheSheetCloses() = runTest {
        val (f, model, _) = setUp()
        val row = model.reservations()!!.pending.first { it.editHref != null }
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
        val sections = model.reservations()!!
        val rows = (sections.ready + sections.pending).filter { it.editHref != null }
        assertEquals(2, rows.size)
        model.edit(rows[1])
        model.finishEdit(EditOutcome.Closed, rows[0])
        assertEquals(rows[1], model.editing.value)
    }

    @Test
    fun aRefreshFromBeforeACancelDoesntBringTheRowBack() = runTest {
        val (f, model, backend) = setUp()
        val row = model.reservations()!!.ready.first()

        backend.delayNextSnapshot = 3_000.milliseconds // read now, delivered late
        val refresh = f.state.account.refresh(force = true)!!
        advanceTimeBy(1_600) // it has read the account (with the row)
        model.cancel(row)
        assertTrue(refresh.isCancelled)
        advanceUntilIdle()

        assertEquals(1, backend.cancelCalls.get())
        assertNull(model.reservations()?.row(row.id))
    }

    @Test
    fun signedOutShowsNothingAndFetchesNothing() = runTest {
        val (f, model, backend) = setUp()
        f.state.signOut()
        val calls = backend.snapshotCalls.get()
        assertNull(f.state.account.refresh(force = true))
        advanceUntilIdle()
        assertEquals(calls, backend.snapshotCalls.get())
        assertNull(model.reservations())
        assertNull(f.state.account.snapshot.value)
    }

    @Test
    fun noticesCanBeDismissedOnlyByTheirOwnId() = runTest {
        val (_, model, _) = setUp()
        model.cancel(model.reservations()!!.ready.first())
        val notice = model.notice.value!!
        model.dismissNotice(notice.id + 1)
        assertNotNull(model.notice.value)
        model.dismissNotice(notice.id)
        assertNull(model.notice.value)
    }
}

/** What Your Books shows for each account store state. */
class BorrowedContentTest {
    private val snapshot = AccountSnapshot(
        DemoBackend.SNAPSHOT_VERSION,
        "Test Borrower",
        demoData().let { PatronList(it.loans.size.toLong(), it.loans) },
        demoData().let { Reservations(PatronList(it.ready.size.toLong(), it.ready), PatronList(it.pending.size.toLong(), it.pending)) },
        fetchedAtMs = 1_000L,
    )
    private val offline = UserFacingError(UserFacingError.Kind.NETWORK, "Can't reach City of Canning. Check your connection.")

    @Test
    fun withoutASnapshotItIsLoadingOrTheError() {
        assertEquals(BorrowedLoad.Loading, BorrowedContent.of(null, refreshing = true, error = null, TODAY).loans)
        assertEquals(BorrowedLoad.Loading, BorrowedContent.of(null, refreshing = false, error = null, TODAY).reservations)
        // A retry is running: loading again, not the old error.
        assertEquals(BorrowedLoad.Loading, BorrowedContent.of(null, refreshing = true, error = offline, TODAY).loans)
        val failed = BorrowedContent.of(null, refreshing = false, error = offline, TODAY)
        assertEquals(BorrowedLoad.Failed(offline), failed.loans)
        assertEquals(BorrowedLoad.Failed(offline), failed.reservations)
        assertNull(failed.status)
    }

    @Test
    fun aFailedRefreshKeepsTheSavedLists() {
        val content = BorrowedContent.of(snapshot, refreshing = false, error = offline, TODAY)
        assertEquals(4, content.loans.valueOrNull?.size)
        assertEquals(3, content.reservations.valueOrNull?.count)
        assertEquals(AccountStatus.RefreshFailed(offline), content.status)
        assertEquals("Couldn’t refresh · Showing the saved list", AccountStatus.REFRESH_FAILED)
    }

    @Test
    fun theStatusLineShowsTheAgeAndTheSpinner() {
        assertEquals(AccountStatus.Updated(1_000L, refreshing = false), BorrowedContent.of(snapshot, false, null, TODAY).status)
        assertEquals(AccountStatus.Updated(1_000L, refreshing = true), BorrowedContent.of(snapshot, true, null, TODAY).status)
        // Retrying after a failure: the spinner, not the failure.
        assertEquals(AccountStatus.Updated(1_000L, refreshing = true), BorrowedContent.of(snapshot, true, offline, TODAY).status)
    }
}

/** Forwards to a DemoBackend, counting account calls; can delay or fail the snapshot and cancels. */
internal class CountingBackend(private val inner: LibraryBackend) : LibraryBackend by inner {
    val cancelCalls = AtomicInteger()
    val snapshotCalls = AtomicInteger()

    /** The next `accountSnapshot` answers with what it read, but only after this extra delay. */
    @Volatile var delayNextSnapshot: Duration? = null
    @Volatile var failSnapshot: Exception? = null
    @Volatile var failCancel: Exception? = null

    override suspend fun accountSnapshot(session: String): AccountSnapshot {
        snapshotCalls.incrementAndGet()
        failSnapshot?.let { throw it }
        val wait = delayNextSnapshot.also { delayNextSnapshot = null }
        val snapshot = inner.accountSnapshot(session)
        if (wait != null) delay(wait)
        return snapshot
    }

    override suspend fun cancelReservation(url: String, session: String): uniffi.librespydus_core.ReserveResult {
        cancelCalls.incrementAndGet()
        failCancel?.let { throw it }
        return inner.cancelReservation(url, session)
    }
}

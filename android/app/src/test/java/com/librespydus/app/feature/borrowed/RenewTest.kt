package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.toSimpleDate
import com.librespydus.app.core.withLoanDue
import com.librespydus.app.feature.home.HomeLoans
import com.librespydus.app.feature.onboarding.AccountFixture
import com.librespydus.app.feature.onboarding.TODAY
import com.librespydus.app.feature.onboarding.accountFixture
import com.librespydus.app.feature.search.valueOrNull
import com.librespydus.app.ui.components.ToastTone
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.PatronList
import uniffi.librespydus_core.Reservations
import uniffi.librespydus_core.SpydusException
import uniffi.librespydus_core.dueReminders
import java.time.LocalDate

private const val RENEW_HREF = "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78500000/0/N/3/LOANRENQ?SVL=1"

/** Renewing a loan (§5.12): the action, its texts and the swipe's wording. Pure. */
class RenewRowsTest {
    private fun loan(actions: List<Pair<String, String>>, due: String? = "14 Oct 2026", irn: String = "1") =
        LoanRow.of(patronItem("Project Hail Mary", irn, fields = listOfNotNull(due?.let { "Due" to it }).toMap(), actions = actions), TODAY)

    @Test
    fun theRenewActionIsFoundByItsLabel() {
        assertEquals(RENEW_HREF, loan(listOf("Renew loan" to RENEW_HREF)).renewHref)
        assertEquals(RENEW_HREF, loan(listOf("RENEW" to " $RENEW_HREF ")).renewHref)
        assertEquals(RENEW_HREF, loan(listOf("Details" to "https://x/details", "Renew" to RENEW_HREF)).renewHref)
        assertNull(loan(emptyList()).renewHref)
        assertNull(loan(listOf("Renew loan" to "  ")).renewHref)
        assertNull(loan(listOf("Details" to "https://x/details")).renewHref)
    }

    @Test
    fun onlyALoanWithARenewLinkOffersRenew() {
        val renewable = loan(listOf("Renew loan" to RENEW_HREF))
        val fixed = loan(emptyList(), irn = "2")
        assertEquals(listOf(LoanAction.RENEW), LoanAction.of(renewable))
        assertEquals(emptyList<LoanAction>(), LoanAction.of(fixed))
        // The "Swipe left on a loan…" footnote only when one can be renewed.
        assertTrue(listOf(fixed, renewable).canRenew)
        assertFalse(listOf(fixed).canRenew)
        assertFalse(emptyList<LoanRow>().canRenew)
    }

    @Test
    fun renewWording() {
        assertEquals("Renew", LoanAction.RENEW.title)
        assertEquals("Renew loan", LoanAction.RENEW.fullTitle)
        assertEquals("Asks the library for a later due date", LoanAction.RENEW.hint)
        assertFalse(LoanAction.RENEW.isDestructive)
        assertEquals("Swipe left on a loan to renew it.", com.librespydus.app.core.Copy.YourBooks.loanSwipeHint)
    }

    @Test
    fun theConfirmationNamesTheBookAndItsDueDate() {
        assertEquals(
            "“Project Hail Mary” is due 14 Oct. Renewing asks the library for a later due date.",
            RenewText.confirm(loan(listOf("Renew loan" to RENEW_HREF))),
        )
        assertEquals(
            "“Project Hail Mary” is due 10 Jan 2027. Renewing asks the library for a later due date.",
            RenewText.confirm(loan(listOf("Renew loan" to RENEW_HREF), due = "10 Jan 2027")),
        )
        // A date the core can't read, as the library wrote it.
        assertEquals(
            "“Project Hail Mary” is due end of term. Renewing asks the library for a later due date.",
            RenewText.confirm(loan(listOf("Renew loan" to RENEW_HREF), due = "end of term")),
        )
        assertEquals(
            "Renewing “Project Hail Mary” asks the library for a later due date.",
            RenewText.confirm(loan(listOf("Renew loan" to RENEW_HREF), due = null)),
        )
    }

    @Test
    fun theResultToastGivesTheNewDate() {
        assertEquals("Renewed · Due 4 Nov", RenewText.renewed("4 Nov 2026", TODAY))
        assertEquals("Renewed · Due 2 Jan 2027", RenewText.renewed("2 Jan 2027", TODAY))
        assertEquals("Renewed · Due end of term", RenewText.renewed("end of term", TODAY))
        assertEquals("Renewed", RenewText.renewed(null, TODAY))
        assertEquals("Renewed", RenewText.renewed("  ", TODAY))
    }

    @Test
    fun aRenewedLoanTakesItsNewDateInTheSnapshot() {
        val item = patronItem(
            "Project Hail Mary",
            "1",
            fields = mapOf("Due" to "14 Oct 2026", "Barcode" to "B1"),
            actions = listOf("Renew loan" to RENEW_HREF),
        )
        val other = patronItem("Lessons in chemistry", "2", fields = mapOf("Due" to "21 Oct 2026"))
        val none = PatronList(0, emptyList())
        val snapshot =
            AccountSnapshot(DemoBackend.SNAPSHOT_VERSION, "Test Borrower", PatronList(2, listOf(item, other)), Reservations(none, none), 1_000L)
        val renewed = snapshot.withLoanDue(RENEW_HREF, "4 Nov 2026")!!
        assertEquals(listOf("4 Nov 2026", "21 Oct 2026"), renewed.loans.items.map { it.fields["Due"] })
        assertEquals(listOf("Due", "Barcode"), renewed.loans.items[0].fields.keys.toList())
        assertEquals(2L, renewed.loans.total)
        // Nothing to change: no such loan, or it already has that date.
        assertNull(snapshot.withLoanDue("https://x/other", "4 Nov 2026"))
        assertNull(renewed.withLoanDue(RENEW_HREF, "4 Nov 2026"))
    }
}

/** BorrowedModel renewing DemoBackend's loans (no network, nothing posted). */
class RenewModelTest {
    @get:Rule val tmp = TemporaryFolder()
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private fun TestScope.setUp(): Triple<AccountFixture, BorrowedModel, CountingBackend> {
        var counting: CountingBackend? = null
        val f = accountFixture(tmp.root, session = DemoBackend.SESSION, wrap = { CountingBackend(it).also { c -> counting = c } })
        scopes += f.scope
        return Triple(f, BorrowedModel(f.state) { TODAY }, counting!!)
    }

    private fun BorrowedModel.loan(title: String): LoanRow = loans()!!.first { it.ref.title == title }

    @Test
    fun demoLoansCanBeRenewed() = runTest {
        val (_, model, _) = setUp()
        assertTrue(model.loans()!!.all { it.renewHref != null })
    }

    @Test
    fun aConfirmedRenewalShowsProgressThenTheNewDateAndRefreshes() = runTest {
        val (f, model, backend) = setUp()
        val row = model.loan("Project Hail Mary")
        assertEquals("Due 14 Oct · 2 days left", row.meta)
        model.askToRenew(row)
        assertSame(row, model.confirmingRenew.value)

        val renewal = async { model.renew(row) }
        runCurrent()
        assertNull(model.confirmingRenew.value)
        assertTrue(model.isRenewing(row))
        assertEquals(setOf(row.id), model.renewing.value)
        assertEquals("Renewing…", model.toast.value?.message)
        assertEquals(ToastTone.PROGRESS, model.toast.value?.tone)
        // While it renews the row takes no other renewal and no confirmation.
        model.askToRenew(row)
        assertNull(model.confirmingRenew.value)

        advanceTimeBy(2_600) // DemoBackend's renewal (2.5 s), not yet the refresh (1.5 s)
        renewal.await()
        assertFalse(model.isRenewing(row))
        assertEquals("Renewed · Due 4 Nov", model.toast.value?.message)
        assertEquals(ToastTone.SUCCESS, model.toast.value?.tone)
        // The row has its new date at once, before the refresh that confirms it.
        assertEquals("Due 4 Nov · 23 days left", model.loan("Project Hail Mary").meta)
        assertTrue(f.state.account.isRefreshing.value)

        advanceUntilIdle()
        assertEquals(1, backend.renewCalls.get())
        assertEquals(2, backend.snapshotCalls.get())
        assertEquals("4 Nov 2026", f.state.account.snapshot.value!!.loans.items.first { it.title == "Project Hail Mary" }.fields["Due"])
        assertEquals("Due 4 Nov · 23 days left", model.loan("Project Hail Mary").meta)
    }

    @Test
    fun aRowRenewsOnceAtATime() = runTest {
        val (_, model, backend) = setUp()
        val row = model.loan("Project Hail Mary")
        listOf(async { model.renew(row) }, async { model.renew(row) }).awaitAll()
        advanceUntilIdle()
        assertEquals(1, backend.renewCalls.get())
        // Once: 14 Oct + 21 days, not + 42.
        assertEquals("Due 4 Nov · 23 days left", model.loan("Project Hail Mary").meta)
    }

    @Test
    fun homeAndTheDueReminderFollowTheNewDate() = runTest {
        val (f, model, _) = setUp()
        fun reminderFor(snapshot: AccountSnapshot) =
            dueReminders(snapshot, TODAY.toSimpleDate(), 2u).first { r -> r.items.any { it.title == "Project Hail Mary" } }
        assertEquals(TODAY.toSimpleDate(), reminderFor(f.state.account.snapshot.value!!).remindOn)

        model.renew(model.loan("Project Hail Mary"))
        advanceUntilIdle()

        val after = f.state.account.snapshot.value!!
        val home = HomeLoans.state(after, refreshing = false, error = null, today = TODAY).valueOrNull!!
        assertEquals(LocalDate.of(2026, 11, 4), home.first { it.ref.title == "Project Hail Mary" }.dueDate)
        assertEquals(LocalDate.of(2026, 11, 2).toSimpleDate(), reminderFor(after).remindOn)
    }

    @Test
    fun aRefusalShowsTheLibrarysReasonAndKeepsTheDate() = runTest {
        val (_, model, backend) = setUp()
        val row = model.loan("Lessons in chemistry")
        model.renew(row)
        assertEquals("This item cannot be renewed as it is reserved by another borrower.", model.toast.value?.message)
        assertEquals(ToastTone.ERROR, model.toast.value?.tone)
        assertFalse(model.isRenewing(row))
        advanceUntilIdle()
        assertEquals(2, backend.snapshotCalls.get()) // refreshed either way
        assertEquals(row.meta, model.loan("Lessons in chemistry").meta)
    }

    @Test
    fun aFailedRenewalSaysSo() = runTest {
        val (f, model, backend) = setUp()
        backend.failRenew = SpydusException.Network("network error: connect timed out after 10s")
        val row = model.loan("Project Hail Mary")
        model.renew(row)
        assertEquals("Couldn’t renew this loan", model.toast.value?.message)
        assertEquals(ToastTone.ERROR, model.toast.value?.tone)
        assertFalse(model.isRenewing(row))
        assertTrue(f.state.isSignedIn.value)
        assertEquals(row.meta, model.loan("Project Hail Mary").meta)
    }

    @Test
    fun anExpiredSessionSignsOutInsteadOfAToast() = runTest {
        val (f, model, backend) = setUp()
        backend.failRenew = SpydusException.NotLoggedIn("not logged in")
        model.renew(model.loan("Project Hail Mary"))
        advanceUntilIdle()
        assertFalse(f.state.isSignedIn.value)
        assertTrue(f.state.loginNeeded.value)
        assertNull(f.state.account.snapshot.value)
        assertNull(model.toast.value) // the progress toast is gone; no "Couldn’t renew"
        assertTrue(model.renewing.value.isEmpty())
    }

    @Test
    fun aResultForAnEndedSessionIsDropped() = runTest {
        val (f, model, _) = setUp()
        val renewal = async { model.renew(model.loan("Project Hail Mary")) }
        runCurrent()
        f.state.signOut()
        renewal.await()
        advanceUntilIdle()
        assertNull(model.toast.value)
        assertNull(f.state.account.snapshot.value)
    }

    @Test
    fun aLoanWithoutARenewLinkIsNeverAsked() = runTest {
        val (_, model, backend) = setUp()
        val row = model.loan("Project Hail Mary").copy(renewHref = null)
        model.askToRenew(row)
        assertNull(model.confirmingRenew.value)
        model.renew(row)
        assertEquals(0, backend.renewCalls.get())
        assertNull(model.toast.value)
    }

    @Test
    fun toastsAreDismissedOnlyByTheirOwnId() = runTest {
        val (_, model, _) = setUp()
        model.renew(model.loan("Project Hail Mary"))
        val toast = model.toast.value!!
        model.dismissToast(toast.id + 1)
        assertSame(toast, model.toast.value)
        model.dismissToast(toast.id)
        assertNull(model.toast.value)
    }
}

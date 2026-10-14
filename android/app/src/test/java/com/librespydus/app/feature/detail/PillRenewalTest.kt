package com.librespydus.app.feature.detail

import com.librespydus.app.core.Copy
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.feature.borrowed.CountingBackend
import com.librespydus.app.feature.borrowed.LoanRow
import com.librespydus.app.feature.onboarding.TODAY
import com.librespydus.app.feature.onboarding.accountFixture
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.librespydus_core.AccountHolding
import uniffi.librespydus_core.HoldingKind
import uniffi.librespydus_core.PatronAction
import uniffi.librespydus_core.PatronItem

/** The book page's "On loan" pill asks to renew a renewable loan (§5.12). */
@OptIn(ExperimentalCoroutinesApi::class)
class PillRenewalTest {
    @get:Rule val tmp = TemporaryFolder()
    private val scopes = mutableListOf<CoroutineScope>()

    @After
    fun cancelScopes() = scopes.forEach { it.cancel() }

    private fun item(actions: List<PatronAction>) =
        PatronItem("18000001", "1", "The long night", null, null, null, emptyList(), null, mapOf("Due" to "14 Oct 2026"), actions)

    @Test
    fun aRenewableLoansPillAsksToRenewIt() {
        val renewable = item(listOf(PatronAction("Renew loan", "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/N/3/LOANRENQ?SVL=18000001")))
        val pill = HeldStatus.of(AccountHolding(HoldingKind.LOAN, renewable), TODAY)
        assertEquals(renewable, pill.renewableLoan)
        assertEquals("Asks the library for a later due date", pill.hint)

        // No renew link (the library doesn't offer one), or a reservation: Your Books, as before.
        val bare = HeldStatus.of(AccountHolding(HoldingKind.LOAN, item(emptyList())), TODAY)
        assertNull(bare.renewableLoan)
        assertEquals("Opens Your Books", bare.hint)
        assertNull(HeldStatus.of(AccountHolding(HoldingKind.RESERVATION_PENDING, renewable), TODAY).renewableLoan)
    }

    @Test
    fun thePillRenewsTheLoanLikeYourBooks() = runTest {
        var counting: CountingBackend? = null
        val f = accountFixture(tmp.root, session = DemoBackend.SESSION, wrap = { CountingBackend(it).also { c -> counting = c } })
        scopes += f.scope
        val snapshot = f.state.account.snapshot.value!!
        val bibIrn = snapshot.loans.items.first { it.title == "Project Hail Mary" }.bibIrn!!
        val pill = HeldStatus.of(HeldStatus.holding(snapshot, listOf(bibIrn))!!, TODAY)
        assertEquals("On loan · Due 14 Oct", pill.text)
        val loan = pill.renewableLoan!!
        assertEquals(Copy.YourBooks.renewHint, pill.hint)
        val renewal = PillRenewal { TODAY }

        renewal.ask(LoanRow.of(loan, TODAY))
        val row = renewal.confirming.value!!
        val renewing = async { renewal.renew(row, f.state) }
        runCurrent()
        assertNull(renewal.confirming.value)
        assertTrue(renewal.renewing.value)
        assertEquals("Renewing…", renewal.toast.value?.message)
        // While it renews the pill asks nothing more.
        renewal.ask(row)
        assertNull(renewal.confirming.value)

        advanceTimeBy(2_600) // DemoBackend's renewal (2.5 s)
        renewing.await()
        assertFalse(renewal.renewing.value)
        assertEquals("Renewed · Due 4 Nov", renewal.toast.value?.message)
        // The pill follows the account to the new date at once, and the account is fetched again.
        val after = HeldStatus.of(HeldStatus.holding(f.state.account.snapshot.value, listOf(bibIrn))!!, TODAY)
        assertEquals("On loan · Due 4 Nov", after.text)
        assertTrue(f.state.account.isRefreshing.value)
        advanceUntilIdle()
        assertEquals(1, counting!!.renewCalls.get())
    }
}

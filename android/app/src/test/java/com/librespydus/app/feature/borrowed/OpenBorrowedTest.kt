package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.PendingNavigation
import com.librespydus.app.core.Route
import com.librespydus.app.core.Tab
import com.librespydus.app.feature.onboarding.accountFixture
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/** The book page's status pill opens Your Books on the matching list. */
class OpenBorrowedTest {
    @get:Rule val tmp = TemporaryFolder()

    @Test
    fun openBorrowedSwitchesTabPopsToTheListAndPicksTheSegment() = runTest {
        val f = accountFixture(tmp.root, session = DemoBackend.SESSION)
        try {
            val model = BorrowedModel(f.state)
            assertEquals(BorrowedModel.Segment.LOANS, model.segment.value)

            f.state.openBorrowed(BorrowedSegment.RESERVATIONS)
            assertEquals(Tab.BORROWED, f.state.selectedTab.value)
            // Your Books' own root: its NavHost pops back to the list instead of pushing another.
            assertEquals(PendingNavigation(Tab.BORROWED, Route.Borrowed), f.state.pendingNavigation.value)
            assertEquals(Route.root(Tab.BORROWED), f.state.consumePendingNavigation(Tab.BORROWED))

            model.showPendingSegment()
            assertEquals(BorrowedModel.Segment.RESERVATIONS, model.segment.value)
            assertNull(f.state.pendingBorrowedSegment.value)
            // Consumed once: showing Your Books again keeps whatever the user picks next.
            model.select(BorrowedModel.Segment.LOANS)
            model.showPendingSegment()
            assertEquals(BorrowedModel.Segment.LOANS, model.segment.value)

            f.state.openBorrowed(BorrowedSegment.LOANS)
            model.select(BorrowedModel.Segment.RESERVATIONS)
            model.showPendingSegment()
            assertEquals(BorrowedModel.Segment.LOANS, model.segment.value)
        } finally {
            f.scope.cancel()
        }
    }
}

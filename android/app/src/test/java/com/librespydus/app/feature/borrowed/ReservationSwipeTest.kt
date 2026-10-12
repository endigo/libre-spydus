package com.librespydus.app.feature.borrowed

import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ReservationSwipeTest {
    private val cancel = "Cancel reservation" to "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/RSV/CANCEL?IRN=1"
    private val edit = "Edit" to "https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/RSV/EDIT?IRN=1"

    private fun row(
        actions: List<Pair<String, String>>,
        kind: ReservationRow.Kind = ReservationRow.Kind.PENDING,
        irn: String = "1",
    ) = ReservationRow.of(patronItem("Book $irn", irn, actions = actions), kind)

    @Test
    fun offersOnlyTheLinkedActionsEditFirst() {
        assertEquals(listOf(ReservationAction.EDIT, ReservationAction.CANCEL), ReservationAction.of(row(listOf(cancel, edit))))
        assertEquals(listOf(ReservationAction.CANCEL), ReservationAction.of(row(listOf(cancel))))
        assertEquals(listOf(ReservationAction.EDIT), ReservationAction.of(row(listOf(edit))))
        assertEquals(emptyList<ReservationAction>(), ReservationAction.of(row(emptyList())))
        // A blank link isn't an action, so that card has no swipe.
        assertEquals(emptyList<ReservationAction>(), ReservationAction.of(row(listOf("Cancel reservation" to "  "))))
    }

    @Test
    fun actionWording() {
        assertEquals("Edit", ReservationAction.EDIT.title)
        assertEquals("Edit reservation", ReservationAction.EDIT.fullTitle)
        assertEquals("Cancel", ReservationAction.CANCEL.title)
        assertEquals("Cancel reservation", ReservationAction.CANCEL.fullTitle)
        assertEquals("Swipe left on a reservation to change or cancel it.", SWIPE_HINT)
    }

    @Test
    fun swipeHintOnlyWhenAReservationHasAnAction() {
        val none = row(emptyList(), irn = "1")
        assertFalse(ReservationSections().hasActions)
        assertFalse(ReservationSections(pending = listOf(none)).hasActions)
        assertTrue(ReservationSections(ready = listOf(row(listOf(cancel), ReservationRow.Kind.READY, "2")), pending = listOf(none)).hasActions)
        assertTrue(ReservationSections(pending = listOf(none, row(listOf(edit), irn = "3"))).hasActions)
    }

    @Test
    fun revealWidthFitsOneTileAndGapPerAction() {
        assertEquals(0.dp, ReservationSwipe.revealWidth(0))
        assertEquals(ReservationSwipe.tileWidth + ReservationSwipe.gap, ReservationSwipe.revealWidth(1))
        assertEquals((ReservationSwipe.tileWidth + ReservationSwipe.gap) * 2, ReservationSwipe.revealWidth(2))
    }

    @Test
    fun oneCardOpenAtATime() {
        val swipe = ReservationSwipeController()
        assertNull(swipe.openId)
        swipe.open("a")
        swipe.open("b")
        assertEquals("b", swipe.openId)
        // A card that closes only releases itself, not the card opened after it.
        swipe.close("a")
        assertEquals("b", swipe.openId)
        swipe.close("b")
        assertNull(swipe.openId)
    }

    @Test
    fun aTapWhileACardIsOpenOnlyClosesIt() {
        val swipe = ReservationSwipeController()
        assertFalse(swipe.dismiss())
        swipe.open("a")
        assertTrue(swipe.dismiss())
        assertNull(swipe.openId)
        assertFalse(swipe.dismiss())
        swipe.open("a")
        swipe.close()
        assertNull(swipe.openId)
    }
}

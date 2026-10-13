package com.librespydus.app.alerts

import com.librespydus.app.core.AlertDestination
import com.librespydus.app.core.AlertKind
import com.librespydus.app.core.AlertRouting
import com.librespydus.app.core.AlertTap
import com.librespydus.app.core.Alerts
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.BorrowedSegment
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.InMemoryAlertStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.LocalDate

/** A tapped alert (§5.8 "Tap"): what each notification carries, and where its tap opens. */
class AlertTapTest {
    private val canning = "https://canning.spydus.com"
    private val hutt = "https://hutt.spydus.co.nz"
    private val today = LocalDate.of(2026, 10, 13)

    private val notifier = RecordingNotifier()
    private val alerts = Alerts(InMemoryAlertStore(), notifier, RecordingScheduler(), today = { today }, io = Dispatchers.Unconfined)

    private val dune = item("19000001", "Dune", mapOf("Pickup location" to "RIVERTON LIBRARY"), bibIrn = "1700001")
        .copy(bibUrl = "$canning/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78577835?SETLVL=&QRY=IRN(1700001)&NAVLVL=SET")
    private val emma = item("19000002", "Emma", mapOf("Status" to "Waiting"), bibIrn = "1700002")
    private val hailMary = item("29000001", "Project Hail Mary", mapOf("Due" to "15 Oct 2026"), bibIrn = "17665994")
    private val chemistry = item("29000002", "Lessons in chemistry", mapOf("Due" to "15 Oct 2026"), bibIrn = "10958382")
    private val crawdads = item("29000003", "Where the crawdads sing", mapOf("Due" to "22 Oct 2026"), bibIrn = "12130675")
    private val account = snapshot(loans = listOf(hailMary, chemistry, crawdads), ready = listOf(dune), pending = listOf(emma))

    // region Payload

    @Test
    fun aReadyAlertCarriesItsKindLibraryAndReservationKey() = runTest {
        alerts.setReadyEnabled(true, canning, snapshot())
        alerts.process(canning, snapshot(ready = listOf(dune)))
        val posted = notifier.posted.single()
        assertEquals(AlertKind.READY, posted.kind)
        assertEquals(canning, posted.origin)
        assertEquals(listOf("19000001"), posted.keys)

        val extras = AlertTap.of(posted).extras()
        assertEquals("reservations", extras[AlertTap.EXTRA_OPEN_BORROWED])
        assertEquals("ready", extras[AlertTap.EXTRA_KIND])
        assertEquals(canning, extras[AlertTap.EXTRA_ORIGIN])
        assertArrayEquals(arrayOf("19000001"), extras[AlertTap.EXTRA_KEYS] as Array<*>)
    }

    @Test
    fun aDueReminderCarriesTheKeysOfItsLoans() = runTest {
        alerts.setDueEnabled(true)
        // One loan due on 15 Oct (reminded today), then two on 22 Oct (reminded on the 20th).
        alerts.postDueReminders(canning, snapshot(loans = listOf(hailMary, crawdads.copy(fields = mapOf("Due" to "22 Oct 2026")), chemistry.copy(fields = mapOf("Due" to "22 Oct 2026")))))
        alerts.postDueReminders(canning, snapshot(loans = listOf(crawdads.copy(fields = mapOf("Due" to "22 Oct 2026")), chemistry.copy(fields = mapOf("Due" to "22 Oct 2026")))), today.plusDays(7))
        val (one, several) = notifier.posted
        assertEquals(AlertKind.DUE, one.kind)
        assertEquals(canning, one.origin)
        assertEquals(listOf("29000001"), one.keys)
        assertEquals(listOf("29000003", "29000002"), several.keys)
        assertEquals("loans", AlertTap.of(several).extras()[AlertTap.EXTRA_OPEN_BORROWED])
        assertEquals("due", AlertTap.of(several).extras()[AlertTap.EXTRA_KIND])
        assertArrayEquals(arrayOf("29000003", "29000002"), AlertTap.of(several).extras()[AlertTap.EXTRA_KEYS] as Array<*>)
    }

    @Test
    fun demoAlertsCarryThePlainOrigin() = runTest {
        val scope = Alerts.scope(canning, demo = true)
        assertEquals("demo:$canning", scope)
        alerts.setReadyEnabled(true, scope, snapshot())
        alerts.process(scope, snapshot(ready = listOf(dune)))
        val posted = notifier.posted.single()
        assertEquals("ready:${Alerts.originHash(scope)}:19000001", posted.id)
        assertEquals(canning, posted.origin)
    }

    @Test
    fun theExtrasReadBackAsTheSameTap() {
        val tap = AlertTap(BorrowedSegment.LOANS, AlertKind.DUE, canning, listOf("29000001", "29000002"))
        assertEquals(tap, AlertTap.parse(tap.extras()))
        // Keys may come back as a list too.
        assertEquals(tap, AlertTap.parse(tap.extras() + (AlertTap.EXTRA_KEYS to listOf("29000001", "29000002"))))
    }

    @Test
    fun anOlderBuildsNotificationOrTheGroupSummaryCarriesJustTheList() {
        assertEquals(AlertTap(BorrowedSegment.RESERVATIONS), AlertTap.parse(mapOf("ls_open_borrowed" to "reservations")))
        assertEquals(AlertTap(BorrowedSegment.LOANS), AlertTap.parse(AlertTap.summary(AlertKind.DUE).extras()))
        assertEquals(mapOf("ls_open_borrowed" to "reservations"), AlertTap.summary(AlertKind.READY).extras())
    }

    @Test
    fun otherIntentsAreNotTaps() {
        assertNull(AlertTap.parse(emptyMap()))
        assertNull(AlertTap.parse(mapOf("ls_origin" to canning, "ls_demo" to true)))
        assertNull(AlertTap.parse(mapOf("ls_open_borrowed" to "favourites")))
    }

    // endregion
    // region Routing

    private fun tap(kind: AlertKind, vararg keys: String, origin: String? = canning) = AlertTap(kind.opens, kind, origin, keys.toList())

    @Test
    fun aReadyAlertOpensThatReservationsBookPageOverReservations() {
        val destination = AlertRouting.destination(tap(AlertKind.READY, "19000001"), canning, account)
        assertEquals(AlertDestination(BorrowedSegment.RESERVATIONS, BookRef.from(dune)), destination)
        // The session-free record URL, as tapping the row in Your Books opens.
        assertEquals(CoreHelpers.patronItemRecordUrl(dune), destination.book?.url)
        assertNotEquals(dune.bibUrl, destination.book?.url)
    }

    @Test
    fun aOneLoanReminderOpensThatLoansBookPageOverLoans() {
        assertEquals(
            AlertDestination(BorrowedSegment.LOANS, BookRef.from(hailMary)),
            AlertRouting.destination(tap(AlertKind.DUE, "29000001"), canning, account),
        )
    }

    @Test
    fun aPendingReservationStillOpensItsBookPage() {
        assertEquals(BookRef.from(emma), AlertRouting.destination(tap(AlertKind.READY, "19000002"), canning, account).book)
    }

    @Test
    fun anItemNoLongerOnTheSavedAccountOpensTheList() {
        // Collected (now a loan, not a reservation) or returned.
        assertEquals(AlertDestination(BorrowedSegment.RESERVATIONS), AlertRouting.destination(tap(AlertKind.READY, "29000001"), canning, account))
        assertEquals(AlertDestination(BorrowedSegment.LOANS), AlertRouting.destination(tap(AlertKind.DUE, "29000009"), canning, account))
        assertEquals(AlertDestination(BorrowedSegment.LOANS), AlertRouting.destination(tap(AlertKind.DUE, "29000001"), canning, null))
    }

    @Test
    fun aReminderForSeveralLoansOpensTheLoansList() {
        assertEquals(
            AlertDestination(BorrowedSegment.LOANS),
            AlertRouting.destination(tap(AlertKind.DUE, "29000001", "29000002"), canning, account),
        )
    }

    @Test
    fun anotherLibrarysAlertOpensTheList() {
        assertEquals(AlertDestination(BorrowedSegment.RESERVATIONS), AlertRouting.destination(tap(AlertKind.READY, "19000001", origin = hutt), canning, account))
        assertEquals(AlertDestination(BorrowedSegment.RESERVATIONS), AlertRouting.destination(tap(AlertKind.READY, "19000001"), null, account))
        // The same library written another way still matches.
        assertEquals(BookRef.from(dune), AlertRouting.destination(tap(AlertKind.READY, "19000001", origin = "https://Canning.spydus.com/"), canning, account).book)
    }

    @Test
    fun aTapWithoutPayloadOpensTheList() {
        val legacy = AlertTap.parse(mapOf("ls_open_borrowed" to "loans"))!!
        assertEquals(AlertDestination(BorrowedSegment.LOANS), AlertRouting.destination(legacy, canning, account))
        assertEquals(AlertDestination(BorrowedSegment.RESERVATIONS), AlertRouting.destination(tap(AlertKind.READY, "19000001", origin = null), canning, account))
        assertEquals(AlertDestination(BorrowedSegment.RESERVATIONS), AlertRouting.destination(tap(AlertKind.READY, ""), canning, account))
        assertEquals(AlertDestination(BorrowedSegment.RESERVATIONS), AlertRouting.destination(AlertTap.summary(AlertKind.READY), canning, account))
    }

    // endregion
}

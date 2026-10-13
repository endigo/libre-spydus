package com.librespydus.app.alerts

import com.librespydus.app.core.Alerts
import com.librespydus.app.core.InMemoryAlertStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate

/** The alert processor (§5.8): what it posts, remembers, forgets and schedules. */
class AlertsTest {
    private val canning = "https://canning.spydus.com"
    private val hutt = "https://hutt.spydus.co.nz"
    private val canningHash = Alerts.originHash(canning)
    private val today = LocalDate.of(2026, 10, 13)

    private val store = InMemoryAlertStore()
    private val notifier = RecordingNotifier()
    private val scheduler = RecordingScheduler()
    private val alerts = Alerts(store, notifier, scheduler, today = { today }, io = Dispatchers.Unconfined)

    private val dune = item("19000001", "Dune", mapOf("Pickup location" to "RIVERTON LIBRARY"))
    private val emma = item("19000002", "Emma", mapOf("Status" to "Please pickup from: CANNINGTON LIBRARY by 20 Oct 2026"))
    private val ulysses = item("19000003", "Ulysses")

    @Test
    fun originHashIsTheAccountCacheSha256() {
        assertEquals(64, canningHash.length)
        assertEquals(canningHash, Alerts.originHash(canning))
        assertFalse(canningHash == Alerts.originHash("demo:$canning"))
    }

    @Test
    fun postsOneNotificationPerNewReadyItemAndSavesTheKeys() = runTest {
        alerts.setReadyEnabled(true, canning, snapshot(ready = listOf(dune)))
        assertTrue(notifier.posted.isEmpty())

        alerts.process(canning, snapshot(ready = listOf(dune, emma, ulysses)))
        assertEquals(
            listOf("ready:$canningHash:19000002", "ready:$canningHash:19000003"),
            notifier.posted.map { it.id },
        )
        assertEquals(listOf("Ready to collect", "Ready to collect"), notifier.posted.map { it.title })
        assertEquals(listOf("Emma · Cannington Library", "Ulysses"), notifier.posted.map { it.body })
        assertEquals(setOf("19000001", "19000002", "19000003"), store.readyKeys(canningHash).toSet())

        // The same snapshot again: nothing new.
        alerts.process(canning, snapshot(ready = listOf(dune, emma, ulysses)))
        assertEquals(2, notifier.posted.size)
    }

    @Test
    fun aReservationReadyAgainAfterBeingCollectedIsAlertedAgain() = runTest {
        alerts.setReadyEnabled(true, canning, snapshot(ready = listOf(dune)))
        alerts.process(canning, snapshot()) // collected: the saved keys are replaced, not merged
        assertEquals(emptyList<String>(), store.readyKeys(canningHash))
        alerts.process(canning, snapshot(ready = listOf(dune)))
        assertEquals(listOf("ready:$canningHash:19000001"), notifier.posted.map { it.id })
    }

    @Test
    fun turningReadyOnRemembersWhatIsWaitingAndPostsNothing() = runTest {
        alerts.setReadyEnabled(true, canning, snapshot(ready = listOf(dune, emma)))
        assertTrue(alerts.readyEnabled.value)
        assertTrue(store.readyEnabled)
        assertEquals(setOf("19000001", "19000002"), store.readyKeys(canningHash).toSet())
        assertTrue(notifier.posted.isEmpty())

        alerts.process(canning, snapshot(ready = listOf(dune, emma)))
        assertTrue(notifier.posted.isEmpty())
    }

    @Test
    fun turningReadyOnWithoutASavedAccountRemembersNothing() = runTest {
        alerts.setReadyEnabled(true, canning, null)
        assertEquals(emptyList<String>(), store.readyKeys(canningHash))
        alerts.process(canning, snapshot(ready = listOf(dune)))
        assertEquals(1, notifier.posted.size)
    }

    @Test
    fun switchesOffPostNothingAndScheduleNothing() = runTest {
        assertFalse(alerts.readyEnabled.value)
        assertFalse(alerts.dueEnabled.value)
        alerts.process(canning, snapshot(ready = listOf(dune), loans = listOf(loan("1", "Dune", "15 Oct 2026"))))
        alerts.postDueReminders(canning, snapshot(loans = listOf(loan("1", "Dune", "15 Oct 2026"))))
        assertTrue(notifier.posted.isEmpty())
        assertTrue(store.readyKeys.isEmpty())
        assertTrue(store.duePosted.isEmpty())

        alerts.updateSchedule(signedIn = true)
        alerts.scheduleNext(signedIn = true)
        assertEquals(listOf("cancel", "cancel"), scheduler.calls)
    }

    @Test
    fun turningASwitchOffForgetsItsState() = runTest {
        alerts.setReadyEnabled(true, canning, snapshot(ready = listOf(dune)))
        alerts.setReadyEnabled(true, hutt, snapshot(ready = listOf(emma)))
        alerts.setDueEnabled(true)
        alerts.postDueReminders(canning, snapshot(loans = listOf(loan("1", "Dune", "15 Oct 2026"))))
        assertEquals(1, store.duePosted.size)

        alerts.setReadyEnabled(false, canning, null)
        assertFalse(alerts.readyEnabled.value)
        assertTrue(store.readyKeys.isEmpty()) // every library's: the switch is app-wide
        alerts.process(canning, snapshot(ready = listOf(ulysses)))
        assertEquals(1, notifier.posted.size) // just the due reminder

        alerts.setDueEnabled(false)
        assertFalse(store.dueEnabled)
        assertTrue(store.duePosted.isEmpty())
    }

    @Test
    fun theMorningCheckIsScheduledWhileSignedInWithASwitchOn() = runTest {
        alerts.setDueEnabled(true)
        alerts.updateSchedule(signedIn = true)
        alerts.updateSchedule(signedIn = false)
        alerts.scheduleNext(signedIn = true)
        alerts.runMorningCheckNow()
        assertEquals(listOf("ensure", "cancel", "next", "now"), scheduler.calls)
    }

    @Test
    fun dueRemindersPostOnTheirDayOnceADay() = runTest {
        alerts.setDueEnabled(true)
        val account = snapshot(
            loans = listOf(
                loan("1", "Project Hail Mary", "15 Oct 2026"), // remind today, 2 days ahead
                loan("2", "Lessons in chemistry", "15 Oct 2026"),
                loan("3", "Where the crawdads sing", "22 Oct 2026"), // remind on 20 Oct
                loan("4", "The midnight library", "12 Oct 2026"), // overdue: no reminder
            ),
        )
        alerts.postDueReminders(canning, account)
        assertEquals(listOf("due:$canningHash:2026-10-15"), notifier.posted.map { it.id })
        assertEquals("Due Thursday 15 Oct", notifier.posted.single().title)
        assertEquals("Project Hail Mary and Lessons in chemistry", notifier.posted.single().body)

        // A second check the same day posts nothing new.
        alerts.postDueReminders(canning, account)
        assertEquals(1, notifier.posted.size)

        // The next day the reminder (due tomorrow now) comes once more, as "Due tomorrow".
        alerts.postDueReminders(canning, account, today.plusDays(1))
        assertEquals(listOf("Due Thursday 15 Oct", "Due tomorrow"), notifier.posted.map { it.title })
        assertEquals(setOf(today.plusDays(1)), store.duePosted.values.toSet())
    }

    @Test
    fun aLoanFoundDueTodayIsRemindedAsDueToday() = runTest {
        alerts.setDueEnabled(true)
        alerts.postDueReminders(canning, snapshot(loans = listOf(loan("1", "Dune", "13 Oct 2026"))))
        assertEquals("Due today", notifier.posted.single().title)
        assertEquals("Dune", notifier.posted.single().body)
    }

    @Test
    fun forgettingALibraryKeepsTheOthers() = runTest {
        alerts.setReadyEnabled(true, canning, snapshot(ready = listOf(dune)))
        alerts.setReadyEnabled(true, hutt, snapshot(ready = listOf(emma)))
        alerts.setDueEnabled(true)
        alerts.postDueReminders(canning, snapshot(loans = listOf(loan("1", "Dune", "15 Oct 2026"))))

        alerts.forget(canning)
        assertEquals(emptyList<String>(), store.readyKeys(canningHash))
        assertEquals(listOf("19000002"), store.readyKeys(Alerts.originHash(hutt)))
        assertTrue(store.duePosted.isEmpty())
        assertEquals(listOf(canningHash), notifier.removed)
        assertTrue(alerts.readyEnabled.value) // the switches are app-wide
    }

    @Test
    fun resetTurnsEverythingOff() = runTest {
        alerts.setReadyEnabled(true, canning, snapshot(ready = listOf(dune)))
        alerts.setDueEnabled(true)
        alerts.reset()
        assertFalse(alerts.readyEnabled.value)
        assertFalse(alerts.dueEnabled.value)
        assertFalse(store.readyEnabled)
        assertTrue(store.readyKeys.isEmpty())
        assertEquals(listOf("cancel"), scheduler.calls)
    }

    @Test
    fun switchesComeBackFromTheStore() {
        val saved = InMemoryAlertStore().apply { dueEnabled = true }
        val restored = Alerts(saved, notifier, scheduler)
        assertFalse(restored.readyEnabled.value)
        assertTrue(restored.dueEnabled.value)
        assertTrue(restored.anyEnabled)
    }
}

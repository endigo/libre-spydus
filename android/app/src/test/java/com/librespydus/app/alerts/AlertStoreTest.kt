package com.librespydus.app.alerts

import android.content.SharedPreferences
import com.librespydus.app.core.LaunchOptions
import com.librespydus.app.core.NotificationAccess
import com.librespydus.app.core.SharedPreferencesAlertStore
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate

/** The `alerts` SharedPreferences layout, the permission decision and the DEBUG hooks. */
class AlertStoreTest {
    private val prefs = FakePreferences()
    private val store = SharedPreferencesAlertStore(prefs)
    private val day = LocalDate.of(2026, 10, 13)

    @Test
    fun switchesAreOffByDefaultAndSaved() {
        assertFalse(store.readyEnabled)
        assertFalse(store.dueEnabled)
        store.readyEnabled = true
        assertTrue(SharedPreferencesAlertStore(prefs).readyEnabled)
        assertEquals(true, prefs.all["readyEnabled"])
    }

    @Test
    fun readyKeysArePerLibrary() {
        store.setReadyKeys("aaa", listOf("1", "2"))
        store.setReadyKeys("bbb", listOf("3"))
        assertEquals(setOf("1", "2"), prefs.all["readyKeys.aaa"])
        assertEquals(setOf("1", "2"), store.readyKeys("aaa").toSet())
        assertEquals(emptyList<String>(), store.readyKeys("ccc"))

        store.setReadyKeys("aaa", listOf("2")) // replaced, not merged
        assertEquals(listOf("2"), store.readyKeys("aaa"))

        store.clearReadyKeys("aaa")
        assertEquals(emptyList<String>(), store.readyKeys("aaa"))
        assertEquals(listOf("3"), store.readyKeys("bbb"))
        store.clearReadyKeys(null)
        assertEquals(emptyList<String>(), store.readyKeys("bbb"))
    }

    @Test
    fun postedDueRemindersArePerLibraryAndPruned() {
        store.setDuePosted("aaa", "due:2026-10-15", day)
        store.setDuePosted("aaa", "due:2026-10-14", day.minusDays(1))
        store.setDuePosted("bbb", "due:2026-10-15", day.minusDays(1))
        assertEquals("2026-10-13", prefs.all["dueLastPosted.aaa.due:2026-10-15"])
        assertEquals(day, store.duePostedOn("aaa", "due:2026-10-15"))
        assertNull(store.duePostedOn("ccc", "due:2026-10-15"))

        store.pruneDuePosted("aaa", before = day)
        assertNull(store.duePostedOn("aaa", "due:2026-10-14"))
        assertEquals(day, store.duePostedOn("aaa", "due:2026-10-15"))
        assertEquals(day.minusDays(1), store.duePostedOn("bbb", "due:2026-10-15")) // another library's

        store.clearDuePosted("aaa")
        assertNull(store.duePostedOn("aaa", "due:2026-10-15"))
        assertEquals(day.minusDays(1), store.duePostedOn("bbb", "due:2026-10-15"))
        store.clearDuePosted(null)
        assertNull(store.duePostedOn("bbb", "due:2026-10-15"))
    }

    @Test
    fun clearAllForgetsEverything() {
        store.readyEnabled = true
        store.setReadyKeys("aaa", listOf("1"))
        store.clearAll()
        assertFalse(store.readyEnabled)
        assertTrue(prefs.all.isEmpty())
    }

    @Test
    fun turningASwitchOnAsksForPermissionOnlyOnAndroid13AndLater() {
        // Allowed.
        assertEquals(NotificationAccess.Step.ALLOWED, NotificationAccess.step(sdk = 36, permissionGranted = true, notificationsEnabled = true))
        assertEquals(NotificationAccess.Step.ALLOWED, NotificationAccess.step(sdk = 30, permissionGranted = true, notificationsEnabled = true))
        // Android 13+: ask (again: Android answers at once once the user has refused twice).
        assertEquals(NotificationAccess.Step.REQUEST, NotificationAccess.step(sdk = 33, permissionGranted = false, notificationsEnabled = false))
        // Granted but blocked in Settings, or below Android 13 (no runtime prompt): Settings.
        assertEquals(NotificationAccess.Step.BLOCKED, NotificationAccess.step(sdk = 36, permissionGranted = true, notificationsEnabled = false))
        assertEquals(NotificationAccess.Step.BLOCKED, NotificationAccess.step(sdk = 30, permissionGranted = true, notificationsEnabled = false))
    }

    @Test
    fun debugHooksParse() {
        val options = LaunchOptions.parse(mapOf("ls_morning_check_now" to true, "ls_demo" to true, "ls_demo_new_ready" to "yes"))
        assertTrue(options.morningCheckNow)
        assertTrue(options.demoNewReady)
        // The morning check alone is a launch option too.
        assertFalse(LaunchOptions.parse(mapOf("ls_morning_check_now" to true)).isEmpty)
        assertFalse(LaunchOptions.parse(emptyMap()).morningCheckNow)
        assertFalse(LaunchOptions.parse(emptyMap()).demoNewReady)
    }
}

/** A minimal in-memory [SharedPreferences] (edits apply at once). */
internal class FakePreferences : SharedPreferences {
    private val values = linkedMapOf<String, Any?>()

    override fun getAll(): MutableMap<String, *> = LinkedHashMap(values)
    override fun getString(key: String, defValue: String?) = values[key] as? String ?: defValue
    @Suppress("UNCHECKED_CAST")
    override fun getStringSet(key: String, defValues: MutableSet<String>?) = (values[key] as? Set<String>)?.toMutableSet() ?: defValues
    override fun getInt(key: String, defValue: Int) = values[key] as? Int ?: defValue
    override fun getLong(key: String, defValue: Long) = values[key] as? Long ?: defValue
    override fun getFloat(key: String, defValue: Float) = values[key] as? Float ?: defValue
    override fun getBoolean(key: String, defValue: Boolean) = values[key] as? Boolean ?: defValue
    override fun contains(key: String) = key in values
    override fun edit(): SharedPreferences.Editor = Editor()
    override fun registerOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) = Unit
    override fun unregisterOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) = Unit

    private inner class Editor : SharedPreferences.Editor {
        private val changes = linkedMapOf<String, Any?>()
        private val removals = mutableSetOf<String>()
        private var clear = false

        override fun putString(key: String, value: String?) = apply { changes[key] = value }
        override fun putStringSet(key: String, values: MutableSet<String>?) = apply { changes[key] = values?.toSet() }
        override fun putInt(key: String, value: Int) = apply { changes[key] = value }
        override fun putLong(key: String, value: Long) = apply { changes[key] = value }
        override fun putFloat(key: String, value: Float) = apply { changes[key] = value }
        override fun putBoolean(key: String, value: Boolean) = apply { changes[key] = value }
        override fun remove(key: String) = apply { removals += key }
        override fun clear() = apply { clear = true }
        override fun commit(): Boolean {
            apply()
            return true
        }
        override fun apply() {
            if (clear) values.clear()
            removals.forEach { values.remove(it) }
            changes.forEach { (key, value) -> if (value == null) values.remove(key) else values[key] = value }
        }
    }
}

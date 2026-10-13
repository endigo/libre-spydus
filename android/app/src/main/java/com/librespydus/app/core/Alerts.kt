package com.librespydus.app.core

import android.content.SharedPreferences
import android.util.Log
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.dueReminders
import uniffi.librespydus_core.patronItemKey
import uniffi.librespydus_core.readyAlerts
import java.time.LocalDate

/**
 * "Ready to collect" alerts and due-date reminders (docs/native-apps.md §5.8): local notifications
 * raised from the account snapshots the app already loads. The one place that decides what to
 * post: [AppState] calls [process] after every successful `accountSnapshot` and
 * [postDueReminders] from the morning check ([MorningCheckWorker]).
 *
 * A *scope* is the library the alerts belong to: its origin, or `demo:<origin>` for demo data so
 * the demo account's keys never mix with the real one's. Its [originHash] (sha256, like the
 * account cache) names the saved keys and the notification ids.
 *
 * The switches are app-wide and both off by default. Turning "Ready to collect" on remembers what
 * is already waiting ([setReadyEnabled]); turning a switch off forgets its state.
 */
class Alerts(
    private val store: AlertStore,
    private val notifier: AlertNotifier,
    private val scheduler: MorningCheckScheduler,
    private val today: () -> LocalDate = LocalDate::now,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) {
    private val mutex = Mutex()
    private val _readyEnabled = MutableStateFlow(store.readyEnabled)
    private val _dueEnabled = MutableStateFlow(store.dueEnabled)

    /** "Ready to collect": checked each morning and when the app opens. */
    val readyEnabled: StateFlow<Boolean> = _readyEnabled.asStateFlow()

    /** "Due-date reminders": two days before books are due, at 7:45. */
    val dueEnabled: StateFlow<Boolean> = _dueEnabled.asStateFlow()

    val anyEnabled: Boolean get() = _readyEnabled.value || _dueEnabled.value

    /**
     * Turns "Ready to collect" on or off. On: the reservations ready in [snapshot] (the saved
     * account of [scope], the selected library) count as already alerted, so only ones that become
     * ready later are announced; without a saved account the first fresh one is remembered the same
     * way ([process]). Off: every library's alerted keys are forgotten.
     */
    suspend fun setReadyEnabled(on: Boolean, scope: String?, snapshot: AccountSnapshot?) = locked {
        if (on) {
            if (scope != null) {
                val hash = originHash(scope)
                if (snapshot != null) store.setReadyKeys(hash, readyAlerts(snapshot, emptyList()).alertedKeys) else store.clearReadyKeys(hash)
            }
        } else {
            store.clearReadyKeys(null)
        }
        store.readyEnabled = on
        _readyEnabled.value = on
    }

    /** Turns due-date reminders on or off; off forgets which reminders were posted. */
    suspend fun setDueEnabled(on: Boolean) = locked {
        if (!on) store.clearDuePosted(null)
        store.dueEnabled = on
        _dueEnabled.value = on
    }

    /**
     * After a successful `accountSnapshot` of [scope] (foreground refresh or morning check alike):
     * one "Ready to collect" notification per reservation that wasn't ready last time, then the
     * keys of everything ready now are saved for next time. A library with no saved keys (signed in
     * again after a sign-out or an expired session, or switched on before any account was saved)
     * only has its keys saved: what is already waiting isn't announced. Nothing while the switch
     * is off.
     */
    suspend fun process(scope: String, snapshot: AccountSnapshot) {
        if (!_readyEnabled.value) return
        locked { processReady(scope, snapshot) }
    }

    private fun processReady(scope: String, snapshot: AccountSnapshot) {
        if (!store.readyEnabled) return
        val hash = originHash(scope)
        val saved = store.readyKeys(hash)
        val result = readyAlerts(snapshot, saved.orEmpty())
        val announced = if (saved == null) emptyList() else result.newItems
        announced.forEach { item -> notifier.post(AlertText.ready(hash, patronItemKey(item), item)) }
        store.setReadyKeys(hash, result.alertedKeys)
        if (announced.isNotEmpty()) Log.i(TAG, "posted ${announced.size} ready-to-collect alert(s)")
    }

    /**
     * The morning check (Android only posts due reminders then): the reminders whose `remindOn` is
     * today, each at most once a day. Nothing while the switch is off.
     */
    suspend fun postDueReminders(scope: String, snapshot: AccountSnapshot, day: LocalDate = today()) {
        if (!_dueEnabled.value) return
        locked { postDue(scope, snapshot, day) }
    }

    private fun postDue(scope: String, snapshot: AccountSnapshot, day: LocalDate) {
        if (!store.dueEnabled) return
        val hash = originHash(scope)
        var posted = 0
        for (reminder in dueReminders(snapshot, day.toSimpleDate(), DAYS_BEFORE)) {
            if (reminder.remindOn.toLocalDate() != day || store.duePostedOn(hash, reminder.key) == day) continue
            notifier.post(AlertText.due(hash, reminder, day))
            store.setDuePosted(hash, reminder.key, day)
            posted++
        }
        store.pruneDuePosted(hash, before = day)
        Log.i(TAG, "morning check: posted $posted due reminder(s)")
    }

    /**
     * Signed out of [scope], or its session expired: its alerted keys and posted reminders are
     * forgotten and its notifications removed. Other libraries keep theirs.
     */
    suspend fun forget(scope: String) = locked {
        val hash = originHash(scope)
        store.clearReadyKeys(hash)
        store.clearDuePosted(hash)
        notifier.removeAll(hash)
    }

    /** DEBUG `ls_reset`: both switches off, nothing remembered, no check scheduled. */
    suspend fun reset() = locked {
        store.clearAll()
        _readyEnabled.value = false
        _dueEnabled.value = false
        scheduler.cancel()
    }

    /**
     * Keeps the morning check scheduled at the next 07:45 while [signedIn] with a switch on
     * (a check already scheduled or running is kept), and cancels it otherwise.
     */
    fun updateSchedule(signedIn: Boolean) {
        if (signedIn && anyEnabled) scheduler.ensureScheduled() else scheduler.cancel()
    }

    /** The end of a morning check: the next one at the next 07:45, if still wanted. */
    fun scheduleNext(signedIn: Boolean) {
        if (signedIn && anyEnabled) scheduler.scheduleNext() else scheduler.cancel()
    }

    /** DEBUG `ls_morning_check_now`: runs the morning check now. */
    fun runMorningCheckNow() = scheduler.runNow()

    /** Off the main thread, one at a time, and finished even if the caller is cancelled (keys saved after posting). */
    private suspend fun locked(block: suspend () -> Unit) =
        withContext(io + NonCancellable) { mutex.withLock { block() } }

    companion object {
        private const val TAG = "Alerts"

        /** Due-date reminders come two days ahead. */
        const val DAYS_BEFORE: UInt = 2u

        fun originHash(scope: String): String = FileAccountSnapshotStorage.sha256(scope)

        /** Alerts that remember nothing on disk and post nowhere (JVM tests, previews). */
        fun inMemory(io: CoroutineDispatcher = Dispatchers.Unconfined) =
            Alerts(InMemoryAlertStore(), AlertNotifier.None, MorningCheckScheduler.None, io = io)
    }
}

/** Posts and removes the alert notifications (Android: [AndroidAlertNotifier]). */
interface AlertNotifier {
    fun post(notification: AlertNotification)

    /** Removes the delivered notifications of the library [originHash]. */
    fun removeAll(originHash: String)

    object None : AlertNotifier {
        override fun post(notification: AlertNotification) = Unit
        override fun removeAll(originHash: String) = Unit
    }
}

/** Schedules the 07:45 morning check (Android: [WorkManagerMorningCheck]). */
interface MorningCheckScheduler {
    /** At the next 07:45, keeping a check that is already scheduled or running. */
    fun ensureScheduled()

    /** At the next 07:45, replacing the one scheduled (the end of a run). */
    fun scheduleNext()

    /** Now (DEBUG `ls_morning_check_now`). */
    fun runNow()

    fun cancel()

    object None : MorningCheckScheduler {
        override fun ensureScheduled() = Unit
        override fun scheduleNext() = Unit
        override fun runNow() = Unit
        override fun cancel() = Unit
    }
}

/**
 * The switches and per-library state behind [Alerts]. Keys are IRNs (not secret); per library
 * they are filed under `originHash`. Blocking: [Alerts] calls it off the main thread.
 */
interface AlertStore {
    var readyEnabled: Boolean
    var dueEnabled: Boolean

    /** The library's alerted keys, or null when none are saved for it (not even an empty list). */
    fun readyKeys(originHash: String): List<String>?

    /** Replaces the saved keys (what `readyAlerts` returned). */
    fun setReadyKeys(originHash: String, keys: List<String>)

    /** Deletes one library's keys, or every library's when [originHash] is null. */
    fun clearReadyKeys(originHash: String?)

    /** The day the due reminder [key] (`due:YYYY-MM-DD`) was last posted, or null. */
    fun duePostedOn(originHash: String, key: String): LocalDate?

    fun setDuePosted(originHash: String, key: String, day: LocalDate)

    /** Forgets reminders posted before [before] (they can't be posted twice on the same day any more). */
    fun pruneDuePosted(originHash: String, before: LocalDate)

    /** Deletes one library's posted reminders, or every library's when [originHash] is null. */
    fun clearDuePosted(originHash: String?)

    fun clearAll()
}

/**
 * [AlertStore] in the `alerts` SharedPreferences: `readyEnabled`, `dueEnabled`,
 * `readyKeys.<originHash>` (a string set) and `dueLastPosted.<originHash>.<key>` (`YYYY-MM-DD`).
 */
class SharedPreferencesAlertStore(private val prefs: SharedPreferences) : AlertStore {
    override var readyEnabled: Boolean
        get() = prefs.getBoolean(READY_ENABLED, false)
        set(value) = prefs.edit().putBoolean(READY_ENABLED, value).apply()

    override var dueEnabled: Boolean
        get() = prefs.getBoolean(DUE_ENABLED, false)
        set(value) = prefs.edit().putBoolean(DUE_ENABLED, value).apply()

    override fun readyKeys(originHash: String): List<String>? =
        prefs.getStringSet("$READY_KEYS$originHash", null)?.toList()

    override fun setReadyKeys(originHash: String, keys: List<String>) {
        prefs.edit().putStringSet("$READY_KEYS$originHash", keys.toSet()).apply()
    }

    override fun clearReadyKeys(originHash: String?) = removeAll(READY_KEYS + originHash.orEmpty())

    override fun duePostedOn(originHash: String, key: String): LocalDate? =
        prefs.getString(dueKey(originHash, key), null)?.let { runCatching { LocalDate.parse(it) }.getOrNull() }

    override fun setDuePosted(originHash: String, key: String, day: LocalDate) {
        prefs.edit().putString(dueKey(originHash, key), day.toString()).apply()
    }

    override fun pruneDuePosted(originHash: String, before: LocalDate) {
        val prefix = "$DUE_POSTED$originHash."
        val stale = prefs.all.filter { (name, value) ->
            name.startsWith(prefix) && (value as? String)?.let { runCatching { LocalDate.parse(it) }.getOrNull() }?.isBefore(before) != false
        }.keys
        if (stale.isNotEmpty()) prefs.edit().apply { stale.forEach(::remove) }.apply()
    }

    override fun clearDuePosted(originHash: String?) = removeAll(DUE_POSTED + originHash?.let { "$it." }.orEmpty())

    override fun clearAll() {
        prefs.edit().clear().apply()
    }

    private fun dueKey(originHash: String, key: String) = "$DUE_POSTED$originHash.$key"

    private fun removeAll(prefix: String) {
        val names = prefs.all.keys.filter { it.startsWith(prefix) }
        if (names.isNotEmpty()) prefs.edit().apply { names.forEach(::remove) }.apply()
    }

    companion object {
        const val PREFS_NAME = "alerts"
        private const val READY_ENABLED = "readyEnabled"
        private const val DUE_ENABLED = "dueEnabled"
        private const val READY_KEYS = "readyKeys."
        private const val DUE_POSTED = "dueLastPosted."
    }
}

/** In-memory [AlertStore] for tests. */
class InMemoryAlertStore : AlertStore {
    override var readyEnabled = false
    override var dueEnabled = false
    val readyKeys = mutableMapOf<String, List<String>>()
    val duePosted = mutableMapOf<Pair<String, String>, LocalDate>()

    override fun readyKeys(originHash: String) = readyKeys[originHash]
    override fun setReadyKeys(originHash: String, keys: List<String>) {
        readyKeys[originHash] = keys
    }
    override fun clearReadyKeys(originHash: String?) {
        if (originHash == null) readyKeys.clear() else readyKeys.remove(originHash)
    }
    override fun duePostedOn(originHash: String, key: String) = duePosted[originHash to key]
    override fun setDuePosted(originHash: String, key: String, day: LocalDate) {
        duePosted[originHash to key] = day
    }
    override fun pruneDuePosted(originHash: String, before: LocalDate) {
        duePosted.entries.removeAll { (key, day) -> key.first == originHash && day.isBefore(before) }
    }
    override fun clearDuePosted(originHash: String?) {
        duePosted.keys.removeAll { originHash == null || it.first == originHash }
    }
    override fun clearAll() {
        readyEnabled = false
        dueEnabled = false
        readyKeys.clear()
        duePosted.clear()
    }
}

package com.librespydus.app.core

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.librespydus.app.MainActivity
import com.librespydus.app.R

/**
 * Android's side of [Alerts] (§5.8): the `ready` / `due` channels, posting with a tap that opens the
 * book page the alert names over Your Books ([AlertIntents], [AlertTap]), and the notification
 * permission. Each kind's alerts form a group whose collapsed summary opens Your Books on that
 * kind's list (the system shows the summary only while the group has two or more alerts).
 */
class AndroidAlertNotifier(context: Context) : AlertNotifier {
    private val context = context.applicationContext
    private val manager = NotificationManagerCompat.from(this.context)

    override fun post(notification: AlertNotification) {
        if (!NotificationAccess.canPost(context)) {
            Log.i(TAG, "notifications are off: not posting ${notification.kind}")
            return
        }
        val kind = notification.kind
        val built = NotificationCompat.Builder(context, kind.channelId)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(notification.title)
            .setContentText(notification.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(notification.body))
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
            .setGroup(group(kind))
            .setContentIntent(AlertIntents.open(context, notification.id, AlertTap.of(notification)))
            .build()
        val summary = NotificationCompat.Builder(context, kind.channelId)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(kind.channelName)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
            .setGroup(group(kind))
            .setGroupSummary(true)
            // The alerts themselves sound; the summary only gathers them.
            .setGroupAlertBehavior(NotificationCompat.GROUP_ALERT_CHILDREN)
            .setContentIntent(AlertIntents.open(context, summaryTag(kind), AlertTap.summary(kind)))
            .build()
        try {
            manager.notify(notification.id, NOTIFICATION_ID, built)
            manager.notify(summaryTag(kind), NOTIFICATION_ID, summary)
        } catch (e: SecurityException) {
            Log.w(TAG, "notification permission revoked", e)
        }
    }

    override fun removeAll(originHash: String) {
        val system = context.getSystemService(NotificationManager::class.java) ?: return
        val active = system.activeNotifications.filter { it.id == NOTIFICATION_ID && it.tag != null }
        val (removed, kept) = active.partition { it.tag.startsWith("ready:$originHash:") || it.tag.startsWith("due:$originHash:") }
        removed.forEach { system.cancel(it.tag, it.id) }
        // A group summary goes with the last of its alerts (another library's may remain).
        for (kind in AlertKind.entries) {
            if (kept.none { it.tag.startsWith("${kind.channelId}:") }) system.cancel(summaryTag(kind), NOTIFICATION_ID)
        }
    }

    companion object {
        private const val TAG = "Alerts"

        /** Every alert is told apart by its tag (the §5.8 id); the numeric id is fixed. */
        const val NOTIFICATION_ID = 1

        /** One group per kind: "Ready to collect" alerts, due-date reminders. */
        fun group(kind: AlertKind) = "com.librespydus.app.alerts.${kind.channelId}"

        /** The tag of [kind]'s group summary (no alert id starts with `summary:`). */
        fun summaryTag(kind: AlertKind) = "summary:${kind.channelId}"

        /** The `ready` "Ready to collect" and `due` "Due-date reminders" channels, default importance. */
        fun createChannels(context: Context) {
            val system = context.getSystemService(NotificationManager::class.java) ?: return
            system.createNotificationChannels(
                AlertKind.entries.map { NotificationChannel(it.channelId, it.channelName, NotificationManager.IMPORTANCE_DEFAULT) },
            )
        }
    }
}

/** A notification tap: MainActivity with the [AlertTap] extras, which [AppState.openFromAlert] opens. */
object AlertIntents {
    /**
     * The tap of the notification tagged [id]. One PendingIntent per notification: extras alone
     * don't tell PendingIntents apart, so the data URI (and request code) come from the id.
     */
    fun open(context: Context, id: String, tap: AlertTap): PendingIntent {
        val intent = Intent(context, MainActivity::class.java)
            .setAction(ACTION_OPEN)
            .setData(Uri.Builder().scheme(SCHEME).authority(AUTHORITY).appendPath(id).build())
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        for ((name, value) in tap.extras()) {
            when (value) {
                is String -> intent.putExtra(name, value)
                is Array<*> -> intent.putExtra(name, value.filterIsInstance<String>().toTypedArray())
            }
        }
        return PendingIntent.getActivity(context, id.hashCode(), intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
    }

    /** What a tapped notification asks for, or null when [intent] isn't one. */
    fun tap(intent: Intent?): AlertTap? {
        val extras = intent?.extras ?: return null
        @Suppress("DEPRECATION")
        return AlertTap.parse(AlertTap.EXTRAS.associateWith { extras.get(it) })
    }

    /** Removes the tap's extras, so the same intent isn't opened twice. */
    fun consume(intent: Intent?) {
        AlertTap.EXTRAS.forEach { intent?.removeExtra(it) }
    }

    private const val ACTION_OPEN = "com.librespydus.app.action.OPEN_YOUR_BOOKS"
    private const val SCHEME = "librespydus"
    private const val AUTHORITY = "alert"
}

/** Whether LibreSpydus may post notifications, and what turning a switch on has to ask for. */
object NotificationAccess {
    enum class Step {
        /** Notifications are allowed: turn the switch on. */
        ALLOWED,

        /** Android 13+ and not asked yet (or asked again): request `POST_NOTIFICATIONS`. */
        REQUEST,

        /** Blocked (below Android 13 there is no prompt): keep the switch off, offer Settings. */
        BLOCKED,
    }

    /**
     * What a switch turned on needs: [Step.ALLOWED] when notifications are on, else a permission
     * request on Android 13+ until it has been granted, else [Step.BLOCKED] (they're off in
     * Settings).
     */
    fun step(sdk: Int, permissionGranted: Boolean, notificationsEnabled: Boolean): Step = when {
        notificationsEnabled && (sdk < 33 || permissionGranted) -> Step.ALLOWED
        sdk >= 33 && !permissionGranted -> Step.REQUEST
        else -> Step.BLOCKED
    }

    fun step(context: Context): Step = step(Build.VERSION.SDK_INT, permissionGranted(context), enabled(context))

    fun permissionGranted(context: Context): Boolean =
        Build.VERSION.SDK_INT < 33 ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED

    fun enabled(context: Context): Boolean = NotificationManagerCompat.from(context).areNotificationsEnabled()

    fun canPost(context: Context): Boolean = permissionGranted(context) && enabled(context)

    /** The system's notification settings for LibreSpydus ("Open Settings"). */
    fun settingsIntent(context: Context): Intent =
        Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
            .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
}

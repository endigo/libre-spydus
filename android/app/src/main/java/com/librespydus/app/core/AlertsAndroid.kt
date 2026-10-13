package com.librespydus.app.core

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Settings
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.librespydus.app.MainActivity
import com.librespydus.app.R

/**
 * Android's side of [Alerts] (§5.8): the `ready` / `due` channels, posting with a tap that opens
 * Your Books on the right segment ([AlertIntents]), and the notification permission.
 */
class AndroidAlertNotifier(context: Context) : AlertNotifier {
    private val context = context.applicationContext
    private val manager = NotificationManagerCompat.from(this.context)

    override fun post(notification: AlertNotification) {
        if (!NotificationAccess.canPost(context)) {
            Log.i(TAG, "notifications are off: not posting ${notification.kind}")
            return
        }
        val built = NotificationCompat.Builder(context, notification.kind.channelId)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(notification.title)
            .setContentText(notification.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(notification.body))
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
            .setContentIntent(AlertIntents.open(context, notification.kind.opens))
            .build()
        try {
            manager.notify(notification.id, NOTIFICATION_ID, built)
        } catch (e: SecurityException) {
            Log.w(TAG, "notification permission revoked", e)
        }
    }

    override fun removeAll(originHash: String) {
        val system = context.getSystemService(NotificationManager::class.java) ?: return
        system.activeNotifications
            .filter { it.id == NOTIFICATION_ID && it.tag?.let { tag -> tag.startsWith("ready:$originHash:") || tag.startsWith("due:$originHash:") } == true }
            .forEach { system.cancel(it.tag, it.id) }
    }

    companion object {
        private const val TAG = "Alerts"

        /** Every alert is told apart by its tag (the §5.8 id); the numeric id is fixed. */
        const val NOTIFICATION_ID = 1

        /** The `ready` "Ready to collect" and `due` "Due-date reminders" channels, default importance. */
        fun createChannels(context: Context) {
            val system = context.getSystemService(NotificationManager::class.java) ?: return
            system.createNotificationChannels(
                AlertKind.entries.map { NotificationChannel(it.channelId, it.channelName, NotificationManager.IMPORTANCE_DEFAULT) },
            )
        }
    }
}

/** A notification tap: MainActivity with an extra that [AppState.openFromAlert] turns into Your Books on a segment. */
object AlertIntents {
    const val EXTRA_OPEN_BORROWED = "ls_open_borrowed"

    fun open(context: Context, segment: BorrowedSegment): PendingIntent {
        val intent = Intent(context, MainActivity::class.java)
            .setAction(ACTION_OPEN)
            .putExtra(EXTRA_OPEN_BORROWED, segment.name.lowercase())
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        // One PendingIntent per segment (extras alone don't tell PendingIntents apart).
        return PendingIntent.getActivity(context, segment.ordinal, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
    }

    /** The segment a notification tap asks for, or null. */
    fun segment(intent: Intent?): BorrowedSegment? =
        intent?.getStringExtra(EXTRA_OPEN_BORROWED)?.let { value -> BorrowedSegment.entries.firstOrNull { it.name.equals(value, ignoreCase = true) } }

    private const val ACTION_OPEN = "com.librespydus.app.action.OPEN_YOUR_BOOKS"
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

package com.librespydus.app.core

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.librespydus.app.LibreSpydusApplication
import kotlinx.coroutines.CancellationException
import java.time.Duration
import java.time.ZonedDateTime
import java.util.concurrent.TimeUnit

/**
 * The 07:45 morning check (§5.8): fetches the account with the saved session and saves it like a
 * foreground refresh, which raises "Ready to collect" alerts, then posts today's due reminders
 * (from the saved account if the fetch failed) and schedules the next run. All of it is
 * [AppState.morningCheck], the app's own account code. Silent on failure.
 */
class MorningCheckWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val appState = (applicationContext as LibreSpydusApplication).appState
        Log.i(TAG, "morning check started")
        try {
            appState.morningCheck()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.w(TAG, "morning check failed", e)
        }
        // Replaces this run's own (finished) work with the next 07:45.
        appState.scheduleNextMorningCheck()
        return Result.success()
    }

    companion object {
        const val TAG = "MorningCheck"
    }
}

/**
 * [MorningCheckScheduler] on WorkManager: unique one-time work `morning-check` with the delay to
 * the next 07:45 local time, no constraints (without a network the check falls back to the saved
 * account). The app keeps one scheduled ([ensureScheduled], KEEP, so it never cancels a check that
 * is running in this process); each run replaces itself with the next one ([scheduleNext], REPLACE).
 */
class WorkManagerMorningCheck(
    context: Context,
    private val now: () -> ZonedDateTime = ZonedDateTime::now,
) : MorningCheckScheduler {
    private val context = context.applicationContext

    override fun ensureScheduled() = enqueue(ExistingWorkPolicy.KEEP, MorningCheckTime.delayUntilNext(now()))

    override fun scheduleNext() = enqueue(ExistingWorkPolicy.REPLACE, MorningCheckTime.delayUntilNext(now()))

    override fun runNow() = enqueue(ExistingWorkPolicy.REPLACE, Duration.ZERO)

    override fun cancel() {
        WorkManager.getInstance(context).cancelUniqueWork(WORK_NAME)
    }

    private fun enqueue(policy: ExistingWorkPolicy, delay: Duration) {
        val request = OneTimeWorkRequestBuilder<MorningCheckWorker>()
            .setInitialDelay(delay.toMillis(), TimeUnit.MILLISECONDS)
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork(WORK_NAME, policy, request)
        Log.i(MorningCheckWorker.TAG, "morning check scheduled ($policy) in ${delay.toMinutes()} min")
    }

    companion object {
        const val WORK_NAME = "morning-check"
    }
}

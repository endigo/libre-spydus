package com.librespydus.app

import android.app.Application
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.request.crossfade
import com.librespydus.app.core.AndroidAlertNotifier
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BiometricPrompter
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/**
 * Holds the process-wide [AppState] (§5.2), creates the alert notification channels (§5.8) and
 * configures Coil for covers and logos.
 */
class LibreSpydusApplication : Application(), SingletonImageLoader.Factory {
    /** Lives as long as the process; AppState's serial operations run here. */
    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    lateinit var appState: AppState
        private set

    /** Shows the fingerprint check (§5.14) on MainActivity, which attaches itself. */
    lateinit var biometricPrompter: BiometricPrompter
        private set

    override fun onCreate() {
        super.onCreate()
        AndroidAlertNotifier.createChannels(this)
        biometricPrompter = BiometricPrompter(this)
        appState = AppState.create(this, appScope, biometricPrompter)
        appState.start()
    }

    override fun newImageLoader(context: PlatformContext): ImageLoader =
        ImageLoader.Builder(context).crossfade(true).build()
}

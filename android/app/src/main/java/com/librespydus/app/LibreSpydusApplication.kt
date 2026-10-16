package com.librespydus.app

import android.app.Application
import android.content.pm.PackageManager
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.request.crossfade
import com.librespydus.app.core.AndroidAlertNotifier
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BiometricPrompter
import com.librespydus.app.core.CopyDevice
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
        // The device's words (docs/native-apps.md §6.1): "tablet" where a phone says "phone", from
        // the device itself, before anything reads the copy.
        CopyDevice.isTablet = isTabletDevice(
            smallestWidthDp = resources.configuration.smallestScreenWidthDp,
            hasHinge = packageManager.hasSystemFeature(PackageManager.FEATURE_SENSOR_HINGE_ANGLE),
        )
        AndroidAlertNotifier.createChannels(this)
        biometricPrompter = BiometricPrompter(this)
        appState = AppState.create(this, appScope, biometricPrompter)
        appState.start()
    }

    override fun newImageLoader(context: PlatformContext): ImageLoader =
        ImageLoader.Builder(context).crossfade(true).build()
}

/**
 * Whether the device is a tablet: its screen is at least 600 dp both ways (the Android `sw600dp`
 * qualifier) and it doesn't fold. A foldable is a phone, whichever screen the app opens on: open,
 * its inner screen is tablet-sized.
 */
internal fun isTabletDevice(smallestWidthDp: Int, hasHinge: Boolean): Boolean =
    smallestWidthDp >= TABLET_SMALLEST_WIDTH_DP && !hasHinge

/** A tablet's smallest width (the Android `sw600dp` qualifier). */
private const val TABLET_SMALLEST_WIDTH_DP = 600

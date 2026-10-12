package com.librespydus.app

import android.app.Application
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.request.crossfade
import com.librespydus.app.core.AppState
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/** Holds the process-wide [AppState] (§5.2) and configures Coil for covers and logos. */
class LibreSpydusApplication : Application(), SingletonImageLoader.Factory {
    /** Lives as long as the process; AppState's serial operations run here. */
    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    lateinit var appState: AppState
        private set

    override fun onCreate() {
        super.onCreate()
        appState = AppState.create(this, appScope)
        appState.start()
    }

    override fun newImageLoader(context: PlatformContext): ImageLoader =
        ImageLoader.Builder(context).crossfade(true).build()
}

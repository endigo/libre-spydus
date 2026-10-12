package com.librespydus.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.librespydus.app.core.LaunchOptions
import com.librespydus.app.ui.nav.LibreSpydusRoot
import com.librespydus.app.ui.theme.LibreSpydusTheme

class MainActivity : ComponentActivity() {
    private val appState get() = (application as LibreSpydusApplication).appState

    override fun onCreate(savedInstanceState: Bundle?) {
        // Navy screens: light (white) status bar icons; the white tab bar sits above the
        // navigation bar, so its buttons get dark icons.
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.light(Color.TRANSPARENT, Color.TRANSPARENT),
        )
        super.onCreate(savedInstanceState)
        // Launch hooks only on a fresh start, not when the activity is recreated.
        if (savedInstanceState == null) applyLaunchOptions(intent)
        setContent {
            LibreSpydusTheme {
                LibreSpydusRoot(appState)
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        applyLaunchOptions(intent)
    }

    private fun applyLaunchOptions(intent: Intent?) {
        appState.applyLaunchOptions(LaunchOptions.fromIntent(intent, BuildConfig.DEBUG))
    }
}

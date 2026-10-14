package com.librespydus.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.fragment.app.FragmentActivity
import com.librespydus.app.core.AlertIntents
import com.librespydus.app.core.LaunchOptions
import com.librespydus.app.ui.nav.LibreSpydusRoot
import com.librespydus.app.ui.theme.LibreSpydusTheme

/** A FragmentActivity because the fingerprint check (BiometricPrompt, §5.14) needs one. */
class MainActivity : FragmentActivity() {
    private val app get() = application as LibreSpydusApplication
    private val appState get() = app.appState

    override fun onCreate(savedInstanceState: Bundle?) {
        // Navy screens: light (white) status bar icons; the white tab bar sits above the
        // navigation bar, so its buttons get dark icons.
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.light(Color.TRANSPARENT, Color.TRANSPARENT),
        )
        super.onCreate(savedInstanceState)
        app.biometricPrompter.attach(this)
        // Launch hooks and notification taps only on a fresh start, not when the activity is recreated.
        if (savedInstanceState == null) {
            applyLaunchOptions(intent)
            openAlert(intent)
        }
        setContent {
            LibreSpydusTheme {
                LibreSpydusRoot(appState)
            }
        }
    }

    override fun onStart() {
        super.onStart()
        // Back in the foreground: refresh the saved account if it's over two minutes old.
        appState.onForeground()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        // A check started on its own waits while a sheet or the system's password prompt is over the app (§5.14).
        app.biometricPrompter.onWindowFocusChanged(this, hasFocus)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        applyLaunchOptions(intent)
        openAlert(intent)
    }

    private fun applyLaunchOptions(intent: Intent?) {
        appState.applyLaunchOptions(LaunchOptions.fromIntent(intent, BuildConfig.DEBUG))
    }

    /** A tapped alert (§5.8 "Tap") opens its book page over Your Books (once: its extras are then removed). */
    private fun openAlert(intent: Intent?) {
        val tap = AlertIntents.tap(intent) ?: return
        AlertIntents.consume(intent)
        appState.openFromAlert(tap)
    }
}

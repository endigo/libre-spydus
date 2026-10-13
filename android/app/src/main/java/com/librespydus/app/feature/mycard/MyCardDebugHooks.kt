package com.librespydus.app.feature.mycard

import android.app.Activity
import com.librespydus.app.BuildConfig
import com.librespydus.app.core.AppState

/**
 * Screenshot hooks for My card (DEBUG builds, **demo mode only**), read from the launching intent
 * next to the shared `ls_*` launch options (android/README.md):
 *
 * - `--ez ls_open_card true`: Home opens the card sheet once.
 * - `--es ls_demo_card <number>`: the demo card number instead of `DEMO0012345`; `none` → no number
 *   (the sheet asks for one). In memory, like every demo number.
 */
internal object MyCardDebugHooks {
    const val OPEN_CARD = "ls_open_card"
    const val DEMO_CARD = "ls_demo_card"

    private val consumed = mutableSetOf<String>()

    /** Applies the hooks once per process (launch with `am start -S`); true → open the sheet. */
    suspend fun run(activity: Activity?, appState: AppState): Boolean {
        if (!BuildConfig.DEBUG || appState.backend.value?.isDemo != true || !consumed.add("my-card")) return false
        val extras = activity?.intent?.extras ?: return false
        extras.getString(DEMO_CARD)?.let { appState.setCardNumber(it.takeUnless { n -> n.trim().equals("none", ignoreCase = true) }) }
        @Suppress("DEPRECATION")
        return when (val open = extras.get(OPEN_CARD)) {
            is Boolean -> open
            is String -> open.trim().lowercase() in setOf("true", "1", "yes")
            else -> false
        }
    }
}

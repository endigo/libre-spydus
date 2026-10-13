package com.librespydus.app.feature.onboarding

import android.app.Activity
import com.librespydus.app.BuildConfig

/**
 * Screenshot hooks for onboarding (DEBUG builds only), read from the launching intent next to
 * the shared `ls_*` launch options (android/README.md):
 *
 * - `--es ls_picker_query <text>`: the library picker starts with this search text.
 * - `--es ls_login_card <card> --es ls_login_pin <pin>`: the login sheet starts filled in, and
 *   `--ez ls_login_submit true` signs in once. **Demo mode only**, never against a real library.
 */
internal object OnboardingDebugHooks {
    const val PICKER_QUERY = "ls_picker_query"
    const val LOGIN_CARD = "ls_login_card"
    const val LOGIN_PIN = "ls_login_pin"
    const val LOGIN_SUBMIT = "ls_login_submit"

    fun string(activity: Activity?, key: String): String? =
        if (BuildConfig.DEBUG) activity?.intent?.getStringExtra(key)?.takeIf { it.isNotBlank() } else null

    fun flag(activity: Activity?, key: String): Boolean {
        if (!BuildConfig.DEBUG) return false
        val extras = activity?.intent?.extras ?: return false
        @Suppress("DEPRECATION")
        return when (val value = extras.get(key)) {
            is Boolean -> value
            is String -> value.trim().lowercase() in setOf("true", "1", "yes")
            else -> false
        }
    }

    /** One-shot hooks run once per process (launch with `am start -S`). */
    private val consumed = mutableSetOf<String>()

    fun consumeOnce(key: String): Boolean = consumed.add(key)
}

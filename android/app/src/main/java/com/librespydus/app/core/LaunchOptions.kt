package com.librespydus.app.core

import android.content.Intent

/**
 * Debug launch hooks (§5.6), read from intent extras in DEBUG builds only:
 * `--es ls_origin <origin>`, `--es ls_route <route>`, `--ez ls_demo true`, `--ez ls_reset true`.
 * For screenshots of the saved account (§5.2a), with `ls_demo` only:
 * `--ez ls_demo_persist true` saves the demo account on the device (normally demo data stays in
 * memory), so a cold relaunch shows it at once; `--ez ls_demo_refresh_fails true` makes every
 * demo account refresh fail like an unreachable library.
 * Alerts (§5.8): `--ez ls_morning_check_now true` runs the 07:45 morning check once the launch's
 * account refresh has settled; with `ls_demo`, `--ez ls_demo_new_ready true` makes one more
 * reservation ready to collect on the demo account's next refresh (after the launch's own).
 *
 * ```
 * adb -s emulator-5554 shell am start -n com.librespydus.app/.MainActivity \
 *   --es ls_origin https://canning.spydus.com --ez ls_demo true --es ls_route borrowed
 * ```
 */
data class LaunchOptions(
    val origin: String? = null,
    val route: LaunchRoute? = null,
    val demo: Boolean = false,
    val reset: Boolean = false,
    val demoPersist: Boolean = false,
    val demoRefreshFails: Boolean = false,
    val demoNewReady: Boolean = false,
    val morningCheckNow: Boolean = false,
) {
    val isEmpty: Boolean get() = origin == null && route == null && !demo && !reset && !morningCheckNow

    companion object {
        const val EXTRA_ORIGIN = "ls_origin"
        const val EXTRA_ROUTE = "ls_route"
        const val EXTRA_DEMO = "ls_demo"
        const val EXTRA_RESET = "ls_reset"
        const val EXTRA_DEMO_PERSIST = "ls_demo_persist"
        const val EXTRA_DEMO_REFRESH_FAILS = "ls_demo_refresh_fails"
        const val EXTRA_DEMO_NEW_READY = "ls_demo_new_ready"
        const val EXTRA_MORNING_CHECK_NOW = "ls_morning_check_now"

        /** From an activity intent; always empty unless [debug]. */
        fun fromIntent(intent: Intent?, debug: Boolean): LaunchOptions {
            if (!debug || intent == null) return LaunchOptions()
            val extras = intent.extras ?: return LaunchOptions()
            return parse(extras.keySet().associateWith { key -> @Suppress("DEPRECATION") extras.get(key) })
        }

        /**
         * From raw extras. Booleans also accept the strings "true"/"1"/"yes" (`--es ls_demo true`
         * is an easy mistake to make on the command line).
         */
        fun parse(extras: Map<String, Any?>): LaunchOptions = LaunchOptions(
            origin = (extras[EXTRA_ORIGIN] as? String)?.trim()?.trimEnd('/')?.takeIf { it.isNotEmpty() },
            route = (extras[EXTRA_ROUTE] as? String)?.let(LaunchRoute::parse),
            demo = flag(extras[EXTRA_DEMO]),
            reset = flag(extras[EXTRA_RESET]),
            demoPersist = flag(extras[EXTRA_DEMO_PERSIST]),
            demoRefreshFails = flag(extras[EXTRA_DEMO_REFRESH_FAILS]),
            demoNewReady = flag(extras[EXTRA_DEMO_NEW_READY]),
            morningCheckNow = flag(extras[EXTRA_MORNING_CHECK_NOW]),
        )

        private fun flag(value: Any?): Boolean = when (value) {
            is Boolean -> value
            is String -> value.trim().lowercase() in setOf("true", "1", "yes")
            is Number -> value.toInt() != 0
            else -> false
        }
    }
}

/** The `ls_route` values (§5.6), plus `gallery` (DEBUG components gallery). */
sealed interface LaunchRoute {
    data object Home : LaunchRoute
    data class Search(val query: String) : LaunchRoute
    data class Results(val query: String) : LaunchRoute
    data class Detail(val url: String) : LaunchRoute
    data object Borrowed : LaunchRoute
    data object Favourites : LaunchRoute
    data object Profile : LaunchRoute
    data object Libraries : LaunchRoute
    data object Login : LaunchRoute
    data object Verify : LaunchRoute
    data object Gallery : LaunchRoute

    companion object {
        /** `home`, `search:<query>`, `results:<query>`, `detail:<url>`, …; null if unknown. */
        fun parse(raw: String): LaunchRoute? {
            val value = raw.trim()
            val name = value.substringBefore(':').lowercase()
            val arg = if (':' in value) value.substringAfter(':').trim() else ""
            return when (name) {
                "home" -> Home
                "search" -> Search(arg)
                "results" -> arg.takeIf { it.isNotEmpty() }?.let(::Results)
                "detail" -> arg.takeIf { it.isNotEmpty() }?.let(::Detail)
                "borrowed" -> Borrowed
                "favourites", "favorites" -> Favourites
                "profile" -> Profile
                "libraries" -> Libraries
                "login" -> Login
                "verify" -> Verify
                "gallery" -> Gallery
                else -> null
            }
        }
    }
}

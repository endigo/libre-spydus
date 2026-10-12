package com.librespydus.app.core

import android.content.Intent

/**
 * Debug launch hooks (§5.6), read from intent extras in DEBUG builds only:
 * `--es ls_origin <origin>`, `--es ls_route <route>`, `--ez ls_demo true`, `--ez ls_reset true`.
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
) {
    val isEmpty: Boolean get() = origin == null && route == null && !demo && !reset

    companion object {
        const val EXTRA_ORIGIN = "ls_origin"
        const val EXTRA_ROUTE = "ls_route"
        const val EXTRA_DEMO = "ls_demo"
        const val EXTRA_RESET = "ls_reset"

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

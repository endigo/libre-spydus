package com.librespydus.app.ui.nav

import android.net.Uri
import android.os.Bundle
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.navigation.NavController
import androidx.navigation.NavType
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.Route
import com.librespydus.app.core.Tab
import com.librespydus.app.feature.featured.InAppBrowser
import kotlinx.serialization.KSerializer
import kotlinx.serialization.json.Json
import kotlinx.serialization.serializer
import kotlin.reflect.typeOf

/** The process-wide [AppState], provided at the root (screens may also take it as a parameter). */
val LocalAppState = staticCompositionLocalOf<AppState> { error("LocalAppState not provided") }

/**
 * What a screen may do to navigation. Every feature screen receives one; pushes go onto the
 * current tab's own stack.
 */
interface AppNavigator {
    /** Book detail (§5.5 `detail(BookRef)`). */
    fun openBook(book: BookRef)

    /** A results list (§5.5 `results(query, sort, title)`): Home "View All", genre tiles. */
    fun openResults(query: String, sort: ResultsSort = ResultsSort.RELEVANCE, title: String? = null)

    /** Pops the current tab's stack (no-op at its root). */
    fun back()

    fun switchTab(tab: Tab)

    /** Switches to the Search tab with [query] prefilled (Home's search field). */
    fun openSearch(query: String = "")

    /** A library's featured collection (§5.10 `collection(FeaturedLink)`): one shelf per section. */
    fun openCollection(title: String, url: String)

    /** A results list that starts from [url] (§5.10: a section's "View All", a Featured saved search). */
    fun openList(url: String, title: String)

    /** A web page in the in-app browser (Custom Tabs). */
    fun openWeb(url: String)
}

internal class TabNavigator(
    private val navController: NavController,
    private val appState: AppState,
) : AppNavigator {
    override fun openBook(book: BookRef) = navController.navigate(Route.Detail(book))

    override fun openResults(query: String, sort: ResultsSort, title: String?) =
        navController.navigate(Route.Results(query, sort, title))

    override fun back() {
        // A second Back tap during the pop animation must not pop the tab's root.
        if (navController.previousBackStackEntry != null) navController.popBackStack()
    }

    override fun switchTab(tab: Tab) = appState.selectTab(tab)

    override fun openSearch(query: String) = appState.openSearch(query)

    override fun openCollection(title: String, url: String) = navController.navigate(Route.Collection(title, url))

    override fun openList(url: String, title: String) = navController.navigate(Route.ListResults(url, title))

    override fun openWeb(url: String) = InAppBrowser.open(navController.context, url)
}

/** A [NavType] carrying any @Serializable value as JSON (BookRef, ResultsSort in routes). */
internal class JsonNavType<T : Any>(private val serializer: KSerializer<T>) : NavType<T>(isNullableAllowed = false) {
    override fun get(bundle: Bundle, key: String): T? = bundle.getString(key)?.let { json.decodeFromString(serializer, it) }

    override fun put(bundle: Bundle, key: String, value: T) = bundle.putString(key, json.encodeToString(serializer, value))

    // Navigation has already decoded the route argument; decoding again would turn `%5C` inside
    // a JSON string (Spydus availability URLs) into a bare `\` and break the JSON.
    override fun parseValue(value: String): T = json.decodeFromString(serializer, value)

    override fun serializeAsValue(value: T): String = Uri.encode(json.encodeToString(serializer, value))

    private companion object {
        val json = Json { ignoreUnknownKeys = true }
    }
}

/** typeMap entries for the route classes' custom argument types. */
internal val routeTypeMap = mapOf(
    typeOf<BookRef>() to JsonNavType(serializer<BookRef>()),
    typeOf<ResultsSort>() to JsonNavType(serializer<ResultsSort>()),
)

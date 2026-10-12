package com.librespydus.app.core

import kotlinx.serialization.Serializable

/** The five tabs (§5.5); [SEARCH] is the raised centre button. Each tab keeps its own stack. */
enum class Tab(val label: String) {
    HOME("Home"),
    BORROWED("Borrowed"),
    SEARCH("Search"),
    FAVOURITES("Favourites"),
    PROFILE("Profile"),
}

/**
 * Search result ordering. App-side mirror of the core's `SearchSort` (P, §4.1), so routes stay
 * serializable and independent of the generated bindings.
 * TODO(P): map to `uniffi.librespydus_core.SearchSort` in [CoreBackend.searchSorted].
 */
@Serializable
enum class ResultsSort(val label: String) {
    RELEVANCE("Relevance"),
    NEWEST("Newest"),
}

/**
 * Type-safe navigation destinations inside a tab's stack (ui/nav). Each tab's root is its own
 * route; [Detail] and [Results] can be pushed onto any tab.
 */
@Serializable
sealed interface Route {
    @Serializable data object Home : Route
    @Serializable data object Borrowed : Route
    @Serializable data object Search : Route
    @Serializable data object Favourites : Route
    @Serializable data object Profile : Route

    /** Book detail (§6, A3). */
    @Serializable data class Detail(val book: BookRef) : Route

    /** A results list for a fixed query (Home "View All", genre tiles; §6, A3). */
    @Serializable
    data class Results(
        val query: String,
        val sort: ResultsSort = ResultsSort.RELEVANCE,
        val title: String? = null,
    ) : Route

    companion object {
        fun root(tab: Tab): Route = when (tab) {
            Tab.HOME -> Home
            Tab.BORROWED -> Borrowed
            Tab.SEARCH -> Search
            Tab.FAVOURITES -> Favourites
            Tab.PROFILE -> Profile
        }
    }
}

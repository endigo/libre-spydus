package com.librespydus.app.core

import kotlinx.serialization.Serializable
import uniffi.librespydus_core.SearchSort

/** The five tabs (§5.5); [SEARCH] is the raised centre button. Each tab keeps its own stack. */
enum class Tab(val label: String) {
    HOME(Copy.Tabs.home),
    BORROWED(Copy.Tabs.yourBooks),
    SEARCH(Copy.Tabs.search),
    FAVOURITES(Copy.Tabs.myPicks),
    PROFILE(Copy.Tabs.profile),
}

/** Your Books' two lists, for opening it on one (the book page's "On loan" / "Reserved" pill). */
enum class BorrowedSegment { LOANS, RESERVATIONS }

/**
 * Search result ordering: an app-side mirror of the core's `SearchSort` so routes stay
 * serializable; `NEWEST` = most recently added to the catalogue.
 */
@Serializable
enum class ResultsSort(val label: String) {
    RELEVANCE(Copy.Search.relevance),
    NEWEST(Copy.Search.newest),
    ;

    fun toCore(): SearchSort = when (this) {
        RELEVANCE -> SearchSort.RELEVANCE
        NEWEST -> SearchSort.NEWEST
    }

    companion object {
        fun from(sort: SearchSort): ResultsSort = when (sort) {
            SearchSort.RELEVANCE -> RELEVANCE
            SearchSort.NEWEST -> NEWEST
        }
    }
}

/**
 * Type-safe navigation destinations inside a tab's stack (ui/nav). Each tab's root is its own
 * route; [Detail], [Results], [Collection] and [ListResults] can be pushed onto any tab.
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

    /**
     * A library's featured collection (§5.10, A9): one shelf per section of the page at [url],
     * titled with the Featured link's [title].
     */
    @Serializable data class Collection(val title: String, val url: String) : Route

    /**
     * A results list that starts from a URL (`page(url)`, then `nextPageUrl`) instead of a query:
     * a collection section's "View All", a Featured saved search (§5.10). No sort toggle.
     */
    @Serializable data class ListResults(val url: String, val title: String) : Route

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

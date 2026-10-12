package com.librespydus.app.core

import kotlinx.serialization.Serializable
import uniffi.librespydus_core.SearchSort

/** The five tabs (§5.5); [SEARCH] is the raised centre button. Each tab keeps its own stack. */
enum class Tab(val label: String) {
    HOME("Home"),
    BORROWED("Borrowed"),
    SEARCH("Search"),
    FAVOURITES("Favourites"),
    PROFILE("Profile"),
}

/**
 * Search result ordering: an app-side mirror of the core's `SearchSort` so routes stay
 * serializable; `NEWEST` = most recently added to the catalogue.
 */
@Serializable
enum class ResultsSort(val label: String) {
    RELEVANCE("Relevance"),
    NEWEST("Newest"),
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

package com.librespydus.app.feature.featured

import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.Route
import com.librespydus.app.feature.detail.ReserveModel
import com.librespydus.app.feature.search.trimmedOrNull
import com.librespydus.app.ui.nav.AppNavigator
import uniffi.librespydus_core.FeaturedKind
import uniffi.librespydus_core.FeaturedLink
import uniffi.librespydus_core.Holding

// Pure rules behind the library's Featured collections (docs/native-apps.md §5.10), kept apart
// from the UI so they're unit-tested (FeaturedLogicTest).

/** A Featured tile's icon, from the library's Font Awesome name (an id the UI maps to a vector). */
enum class FeaturedIcon {
    BOOK, CELEBRATION, COMPUTER, CIRCLE, GLOBE, PUZZLE, MAP, PERSON_CIRCLE,

    /** Any other name, or none. */
    SPARKLES,
    ;

    companion object {
        /** `book`, `balloons`, `computer`, … as the core reports them (no `fa-` prefix). */
        fun forName(name: String?): FeaturedIcon = when (name?.trim()?.lowercase()?.removePrefix("fa-")) {
            "book" -> BOOK
            "balloons" -> CELEBRATION
            "computer" -> COMPUTER
            "circle" -> CIRCLE
            "globe" -> GLOBE
            "game-board" -> PUZZLE
            "map-location-dot" -> MAP
            "user-circle" -> PERSON_CIRCLE
            else -> SPARKLES
        }
    }
}

/** Where tapping a Featured link goes. */
sealed interface FeaturedDestination {
    /** The Collection screen (`collection(url)` → one shelf per section). */
    data class Collection(val route: Route.Collection) : FeaturedDestination

    /** A saved search: the results list for its URL. */
    data class Results(val route: Route.ListResults) : FeaturedDestination

    /** Anything else: the in-app browser. */
    data class Web(val url: String) : FeaturedDestination
}

object FeaturedRouting {
    fun destination(link: FeaturedLink): FeaturedDestination = when (link.kind) {
        FeaturedKind.COLLECTION -> FeaturedDestination.Collection(Route.Collection(link.title, link.url))
        FeaturedKind.SEARCH -> FeaturedDestination.Results(Route.ListResults(link.url, link.title))
        FeaturedKind.WEB -> FeaturedDestination.Web(link.url)
    }
}

/**
 * Edition-level items (every item in a saved-search list): one bib, no format tabs. The core gives
 * such a record exactly one format with an empty code and no label — the card's own reserve link,
 * and an availability URL whose counts come from its holdings. The app shows no format chip or
 * picker for it, never re-finds its formats, and adds a "Copies" list on its book page.
 */
object EditionLevel {
    fun isEditionLevel(format: FormatRef): Boolean = format.code.isBlank() && format.label.trimmedOrNull() == null

    /** Exactly one format, and that one has an empty code and no label. */
    fun isEditionLevel(formats: List<FormatRef>): Boolean = formats.size == 1 && isEditionLevel(formats[0])
}

/** One row of the book page's "Copies" list. */
data class CopyRow(
    /** The branch, title-cased ("RIVERTON LIBRARY" → "Riverton Library"). */
    val location: String?,
    /** "Available", "On loan - Due: 29 Oct 2026". */
    val status: String?,
    /** Drawn in the success colour. */
    val available: Boolean,
    /** A note on the copy ("Pink body, teal handle"), secondary text. */
    val description: String?,
    val callNumber: String?,
) {
    /** What a screen reader reads for the row. */
    val accessibilityLabel: String
        get() = listOfNotNull(location, status, description, callNumber?.let { "Call number $it" }).joinToString(", ")

    companion object {
        fun from(holding: Holding) = CopyRow(
            location = holding.location.trimmedOrNull()?.let(ReserveModel::titleCased),
            status = holding.status.trimmedOrNull(),
            available = holding.available,
            description = holding.description.trimmedOrNull(),
            callNumber = holding.callNumber.trimmedOrNull(),
        )

        /** Rows that say something (a holding without location, status or call number is dropped). */
        fun rows(holdings: List<Holding>): List<CopyRow> =
            holdings.map(::from).filter { it.location != null || it.status != null || it.callNumber != null }
    }
}

/** Opens a Featured link where §5.10 says: its collection, its results list or the in-app browser. */
fun AppNavigator.openFeatured(link: FeaturedLink) {
    when (val destination = FeaturedRouting.destination(link)) {
        is FeaturedDestination.Collection -> openCollection(destination.route.title, destination.route.url)
        is FeaturedDestination.Results -> openList(destination.route.url, destination.route.title)
        is FeaturedDestination.Web -> openWeb(destination.url)
    }
}

package com.librespydus.app.feature.favourites

import com.librespydus.app.core.Copy
import com.librespydus.app.core.PickIrn
import com.librespydus.app.core.PicksList
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.SearchResultItem
import com.librespydus.app.feature.search.SearchResultsModel

/** Which list the My Picks tab shows (§5.11): My Picks while signed in, else this phone's own. */
enum class FavouritesMode {
    MY_PICKS,
    LOCAL,
    ;

    /** The screen's title: "My Picks" either way (§7.1). */
    val title: String get() = Copy.Picks.title

    /** Where the list is kept: "Saved on your library account" / "Saved on this phone". */
    val subtitle: String get() = if (this == MY_PICKS) Copy.Picks.subtitleAccount else Copy.Picks.subtitlePhone

    /** The empty list's message, with [Copy.Picks.emptyTitle]. */
    val emptyMessage: String get() = if (this == MY_PICKS) Copy.Picks.emptyAccountMessage else Copy.Picks.emptyPhoneMessage

    companion object {
        fun of(signedIn: Boolean): FavouritesMode = if (signedIn) MY_PICKS else LOCAL
    }
}

/** A row of My Picks: the search results row, plus the IRN its bookmark removes (null: no remove). */
data class PickRow(val item: SearchResultItem, val irn: String?)

/** What the My Picks list draws for a [PicksList] (unit-tested). */
sealed interface MyPicksContent {
    data object Loading : MyPicksContent
    data class Failed(val error: UserFacingError) : MyPicksContent

    /** "No picks yet". */
    data object Empty : MyPicksContent

    data class Rows(
        val rows: List<PickRow>,
        val footer: Footer,
    ) : MyPicksContent

    sealed interface Footer {
        data object None : Footer

        /** A page is loading, or the next one should be requested when the footer shows. */
        data class More(val nextPageUrl: String?) : Footer
        data class Failed(val error: UserFacingError) : Footer
    }

    companion object {
        fun of(list: PicksList): MyPicksContent = when (val phase = list.phase) {
            PicksList.Phase.Idle, PicksList.Phase.Loading -> Loading
            is PicksList.Phase.Failed -> Failed(phase.error)
            PicksList.Phase.Loaded -> {
                val rows = rows(list)
                if (rows.isEmpty()) {
                    Empty
                } else {
                    val footer = when {
                        list.loadMoreError != null -> Footer.Failed(list.loadMoreError)
                        list.loadingMore || list.nextPageUrl != null -> Footer.More(list.nextPageUrl)
                        else -> Footer.None
                    }
                    Rows(rows, footer)
                }
            }
        }

        /** Rows exactly like search results: same card, same chips (none for an edition-level item). */
        fun rows(list: PicksList): List<PickRow> {
            val keys = mutableSetOf<String>()
            return list.records.mapIndexed { index, record ->
                // Lazy list keys must be unique: a repeated key falls back to the row's position.
                val key = SearchResultsModel.dedupeKey(record)?.takeIf(keys::add)
                PickRow(SearchResultItem.from(record, key, index), PickIrn.of(record))
            }
        }
    }
}

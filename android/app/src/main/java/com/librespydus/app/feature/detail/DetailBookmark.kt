package com.librespydus.app.feature.detail

import com.librespydus.app.core.PicksText

/**
 * The book page's blue bookmark: signed in it saves to My Picks on the library account (§5.11),
 * signed out to this phone's favourites (§5.3).
 */
data class DetailBookmark(
    /** Filled. */
    val saved: Boolean,
    /** False while the pick IRN isn't known yet (the record is still loading). */
    val enabled: Boolean,
    /** What a screen reader says. */
    val label: String,
) {
    companion object {
        /** Signed in: picked when [irn] is on the list; waits (disabled) while [irn] is null. */
        fun myPicks(irn: String?, picked: Set<String>): DetailBookmark {
            val saved = irn != null && irn in picked
            return DetailBookmark(saved = saved, enabled = irn != null, label = if (saved) PicksText.REMOVE_LABEL else PicksText.SAVE_LABEL)
        }

        /** Signed out: the local favourites toggle, unchanged. */
        fun local(isFavourite: Boolean) =
            DetailBookmark(saved = isFavourite, enabled = true, label = if (isFavourite) "Remove from favourites" else "Add to favourites")
    }
}

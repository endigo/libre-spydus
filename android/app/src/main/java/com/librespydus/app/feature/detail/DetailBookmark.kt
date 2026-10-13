package com.librespydus.app.feature.detail

import com.librespydus.app.core.Copy

/**
 * The book page's blue bookmark: signed in it saves to My Picks on the library account (§5.11),
 * signed out to My Picks on this phone (§5.3). Either way it says "Save to My Picks" / "Remove from
 * My Picks" (§7.1).
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
            return DetailBookmark(saved = saved, enabled = irn != null, label = label(saved))
        }

        /** Signed out: the phone's own list. */
        fun local(isFavourite: Boolean) = DetailBookmark(saved = isFavourite, enabled = true, label = label(isFavourite))

        private fun label(saved: Boolean): String = if (saved) Copy.Picks.remove else Copy.Picks.save
    }
}

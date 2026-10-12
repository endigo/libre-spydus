package com.librespydus.app.feature.borrowed

import android.app.Activity
import com.librespydus.app.BuildConfig

/**
 * Screenshot hooks for "Your Books" and Favourites (DEBUG builds only), read from the launching
 * intent next to the shared `ls_*` launch options (android/README.md):
 *
 * - `--ez ls_scroll_bottom true` opens Borrowed and Favourites scrolled to the end, to check the
 *   last card clears the tab bar.
 * - `--es ls_borrowed_segment reservations` opens on Reservations.
 * - `--es ls_borrowed_script <step>` (demo mode only) runs one step once the lists have loaded:
 *   `cancel` (the confirmation for the first ready reservation), `cancel-confirmed` (cancels it
 *   and shows the result), `edit` (the edit sheet for the first editable pending reservation),
 *   `edit-saved` (that sheet picks another branch and saves).
 * - `--ez ls_favourites_editing true` opens Favourites in Edit mode.
 * - `--ez ls_seed_favourites true` (demo mode only) saves the demo loans as favourites, so the
 *   grid can be shown before Book detail exists. Local only: nothing is fetched.
 *
 * Mutating steps only ever reach `DemoBackend`, never a real library.
 */
internal object BorrowedDebugHooks {
    const val SCROLL_BOTTOM = "ls_scroll_bottom"
    const val SEGMENT = "ls_borrowed_segment"
    const val SCRIPT = "ls_borrowed_script"
    const val FAVOURITES_EDITING = "ls_favourites_editing"
    const val SEED_FAVOURITES = "ls_seed_favourites"

    enum class Script(val value: String) {
        CANCEL("cancel"),
        CANCEL_CONFIRMED("cancel-confirmed"),
        EDIT("edit"),
        EDIT_SAVED("edit-saved"),
    }

    fun string(activity: Activity?, key: String): String? =
        if (BuildConfig.DEBUG) activity?.intent?.getStringExtra(key)?.trim()?.takeIf { it.isNotEmpty() } else null

    fun flag(activity: Activity?, key: String): Boolean {
        if (!BuildConfig.DEBUG) return false
        val extras = activity?.intent?.extras ?: return false
        @Suppress("DEPRECATION")
        return when (val value = extras.get(key)) {
            is Boolean -> value
            is String -> value.trim().lowercase() in setOf("true", "1", "yes")
            else -> false
        }
    }

    fun script(activity: Activity?): Script? =
        string(activity, SCRIPT)?.let { raw -> Script.entries.firstOrNull { it.value == raw.lowercase() } }

    /** The edit sheet saves by itself (`edit-saved`). */
    var autoSavesEdit = false
        private set

    private val consumed = mutableSetOf<String>()

    /** One-shot hooks run once per process (launch with `am start -S`). */
    fun consumeOnce(key: String): Boolean = consumed.add(key)

    /** Runs the launch script once, after the first load in demo mode. */
    suspend fun run(activity: Activity?, model: BorrowedModel, isDemo: Boolean) {
        if (!BuildConfig.DEBUG || !isDemo || !consumeOnce("borrowed-script")) return
        if (string(activity, SEGMENT)?.lowercase() == "reservations") model.select(BorrowedModel.Segment.RESERVATIONS)
        val script = script(activity) ?: return
        model.select(BorrowedModel.Segment.RESERVATIONS)
        val sections = model.reservations.value.valueOrNull ?: return
        val ready = sections.ready.firstOrNull { it.cancelHref != null }
        val editable = sections.pending.firstOrNull { it.editHref != null }
        when (script) {
            Script.CANCEL -> model.askToCancel(ready)
            Script.CANCEL_CONFIRMED -> ready?.let { model.cancel(it) }
            Script.EDIT, Script.EDIT_SAVED -> editable?.let {
                autoSavesEdit = script == Script.EDIT_SAVED
                model.edit(it)
            }
        }
    }
}

package com.librespydus.app.feature.search

import androidx.compose.ui.graphics.vector.ImageVector
import com.librespydus.app.core.AppState
import com.librespydus.app.core.FormatRef
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.ui.theme.LsIcons
import kotlinx.coroutines.CancellationException

// Small pieces the Home, Search and Book detail screens (A3) share.

/** The state of one load: a Home shelf, a results page, a detail record or panel. */
sealed interface BrowseLoadable<out T> {
    data object Idle : BrowseLoadable<Nothing>
    data object Loading : BrowseLoadable<Nothing>
    data class Loaded<T>(val value: T) : BrowseLoadable<T>
    data class Failed(val error: UserFacingError) : BrowseLoadable<Nothing>
}

val <T> BrowseLoadable<T>.valueOrNull: T? get() = (this as? BrowseLoadable.Loaded<T>)?.value
val BrowseLoadable<*>.isLoading: Boolean get() = this is BrowseLoadable.Loading
val BrowseLoadable<*>.errorOrNull: UserFacingError? get() = (this as? BrowseLoadable.Failed)?.error

/**
 * Turns a failed call into what the screen shows, with AppState's side effects (verification
 * sheet, sign-in). Null means "show nothing": the call was cancelled.
 */
typealias BrowseReporter = (Throwable) -> UserFacingError?

/** [AppState.report] as a [BrowseReporter]. */
fun AppState.browseReporter(): BrowseReporter = { error -> if (error is CancellationException) null else report(error) }

/** The trimmed text, or null when it's null or blank. */
internal fun String?.trimmedOrNull(): String? = this?.trim()?.takeIf { it.isNotEmpty() }

/** A format's chip text: its label, else its code. */
internal val FormatRef.displayLabel: String get() = label.trimmedOrNull() ?: code

/** Short heading for an error card ("Can't connect"). */
val UserFacingError.browseTitle: String
    get() = when (kind) {
        UserFacingError.Kind.NETWORK -> "Can't connect"
        UserFacingError.Kind.VERIFICATION -> "Verification needed"
        UserFacingError.Kind.SIGN_IN_AGAIN -> "Signed out"
        UserFacingError.Kind.CREDENTIALS -> "Sign-in failed"
        UserFacingError.Kind.UNEXPECTED -> "Something went wrong"
    }

val UserFacingError.browseIcon: ImageVector
    get() = when (kind) {
        UserFacingError.Kind.NETWORK -> BrowseIcons.WifiOff
        UserFacingError.Kind.VERIFICATION -> LsIcons.Shield
        UserFacingError.Kind.SIGN_IN_AGAIN, UserFacingError.Kind.CREDENTIALS -> LsIcons.Person
        UserFacingError.Kind.UNEXPECTED -> BrowseIcons.Warning
    }

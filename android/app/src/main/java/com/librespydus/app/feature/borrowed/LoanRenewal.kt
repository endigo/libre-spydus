package com.librespydus.app.feature.borrowed

import com.librespydus.app.core.AppState
import uniffi.librespydus_core.RenewResult

/**
 * Renews [row] at the library after the reader confirmed (Your Books, or the book page's "On loan"
 * pill, docs/native-apps.md §5.12). A renewed loan shows its new due date at once, and the account
 * is fetched again either way, so Your Books, Home's loans and the due-date reminders follow. Null
 * when there's nothing to renew (no link, signed out) or the library or session changed meanwhile;
 * throws what `renewLoan` throws.
 */
suspend fun AppState.renew(row: LoanRow): RenewResult? {
    val href = row.renewHref ?: return null
    val library = backend.value ?: return null
    val signedIn = session.value ?: return null
    val result = library.renewLoan(href, signedIn)
    if (library !== backend.value || signedIn != session.value) return null
    if (result.ok) result.due?.trim()?.takeIf { it.isNotEmpty() }?.let { account.renewLoan(href, it) }
    accountDidChange()
    return result
}

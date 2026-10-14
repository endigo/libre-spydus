package com.librespydus.app.feature.detail

import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.feature.borrowed.BorrowedModel
import com.librespydus.app.feature.borrowed.LoanRow
import com.librespydus.app.feature.borrowed.RenewText
import com.librespydus.app.feature.borrowed.renew
import com.librespydus.app.ui.components.ToastTone
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import uniffi.librespydus_core.SpydusException
import java.time.LocalDate

/**
 * Renewing from the book page's "On loan" pill (docs/native-apps.md §5.12), as Your Books does
 * ([BorrowedModel]): the confirmation, "Renewing…" until the library answers, then the result as a
 * toast. One renewal at a time.
 */
class PillRenewal(private val today: () -> LocalDate = LocalDate::now) {
    private val _confirming = MutableStateFlow<LoanRow?>(null)
    private val _renewing = MutableStateFlow(false)
    private val _toast = MutableStateFlow<BorrowedModel.Toast?>(null)

    /** The loan awaiting the renew confirmation. */
    val confirming: StateFlow<LoanRow?> = _confirming.asStateFlow()
    val renewing: StateFlow<Boolean> = _renewing.asStateFlow()

    /** "Renewing…", then the renewal's result. */
    val toast: StateFlow<BorrowedModel.Toast?> = _toast.asStateFlow()

    /** Asks to confirm renewing [loan] (null closes the dialog). Not while a renewal runs. */
    fun ask(loan: LoanRow?) {
        if (loan != null && (loan.renewHref == null || _renewing.value)) return
        _confirming.value = loan
    }

    /**
     * Renews [loan] after the reader confirmed; the pill follows the account to the new due date.
     * A failed call says "Couldn’t renew this loan" (`NotLoggedIn` asks to sign in again instead).
     */
    suspend fun renew(loan: LoanRow, appState: AppState) {
        _confirming.value = null
        if (_renewing.value) return
        _renewing.value = true
        val progress = show(Copy.YourBooks.renewing, ToastTone.PROGRESS)
        try {
            val result = appState.renew(loan) ?: return
            show(RenewText.message(result, today()), if (result.ok) ToastTone.SUCCESS else ToastTone.ERROR)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            appState.report(e)
            if (e !is SpydusException.NotLoggedIn) show(Copy.YourBooks.renewFailed, ToastTone.ERROR)
        } finally {
            _renewing.value = false
            dismiss(progress)
        }
    }

    /** Hides the toast [id] (or whichever is shown when null); a newer one stays. */
    fun dismiss(id: Long? = null) {
        if (id == null || _toast.value?.id == id) _toast.value = null
    }

    private fun show(message: String, tone: ToastTone): Long = BorrowedModel.Toast(message, tone).also { _toast.value = it }.id
}

//! Your Books' own account change (iOS `BorrowedViewModel.cancel`): cancel a reservation through
//! `perform_account_change`, then take its row out of the saved account until the next fetch.
//! Renew is `AppState::renew` (shared with the book page); edit runs from its dialog.

use gpui_kit::{Context, Task};
use librespydus_core::ReserveResult;

use crate::state::{AppState, UserFacingError};

use super::rows::remove_reservation;

impl AppState {
    /// Cancels the reservation whose cancel link is `href`, after the reader confirmed. A fetch
    /// that overlaps it is dropped, and an accepted cancel fetches the account again. The row
    /// leaves the saved account at once when the library accepts. `ok: false` is the library's
    /// refusal, shown as data.
    pub fn cancel_reservation(
        &mut self,
        href: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<ReserveResult, UserFacingError>> {
        let Some(session) = self.session.clone() else {
            return Task::ready(Ok(ReserveResult { ok: false, message: None }));
        };
        let key = self.reload_key();
        let call = self.backend.cancel_reservation(href.clone(), session);
        let change = self.perform_account_change(call, |result| result.ok, cx);
        cx.spawn(async move |this, cx| {
            let result = change.await;
            this.update(cx, |this, cx| match result {
                Ok(result) => {
                    if result.ok && this.reload_key() == key {
                        this.update_account_snapshot(|snapshot| remove_reservation(snapshot, &href), cx);
                    }
                    Ok(result)
                }
                Err(error) => Err(this.report(&error, cx)),
            })
            .map_err(|_| UserFacingError::unexpected(None))?
        })
    }
}

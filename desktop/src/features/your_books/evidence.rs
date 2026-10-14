//! Screenshot steps for Your Books (cargo feature `evidence` only; docs/desktop-app.md §9): with
//! `LIBRESPYDUS_EVIDENCE=your_books:<step>`, once the account has loaded, the screen opens a menu or
//! dialog in the real window so `--screenshot` can capture it. Ignored without the prefix.
//!
//! Steps: `loan-menu`, `loan-context`, `renew-confirm`, `renew-ok`, `renew-refused`,
//! `reservation-menu`, `reservation-context`, `edit`, `cancel`, `cancel-done`, `refresh-fails`
//! (the saved lists, then a refresh that fails), `empty-loans`, `empty-reservations`.

use std::time::Duration;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{AsyncWindowContext, Context, WeakEntity, Window};

use crate::state::YourBooksSegment;
use crate::state::formatting::today;

use super::YourBooksScreen;
use super::rows::Lists;

const PREFIX: &str = "your_books:";
/// The demo loans the library renews and refuses (`DemoBackend`).
const RENEWABLE: &str = "18000003";
const REFUSED: &str = "18000002";
/// A demo reservation with Edit and Cancel.
const RESERVATION: &str = "19000003";

pub(super) fn schedule(window: &mut Window, cx: &mut Context<YourBooksScreen>) {
    let Some(step) = std::env::var("LIBRESPYDUS_EVIDENCE").ok().and_then(|v| v.strip_prefix(PREFIX).map(str::to_owned))
    else {
        return;
    };
    cx.spawn_in(window, async move |this, cx| {
        if wait_for_account(&this, cx).await {
            run(&step, &this, cx).await;
        }
    })
    .detach();
}

async fn pause(cx: &mut AsyncWindowContext, millis: u64) {
    cx.background_executor().timer(Duration::from_millis(millis)).await;
}

async fn wait_for_account(this: &WeakEntity<YourBooksScreen>, cx: &mut AsyncWindowContext) -> bool {
    for _ in 0..100 {
        let loaded = this.update(cx, |this, cx| this.state.read(cx).account().snapshot().is_some());
        match loaded {
            Ok(true) => {
                pause(cx, 400).await;
                return true;
            }
            Ok(false) => pause(cx, 150).await,
            Err(_) => return false,
        }
    }
    false
}

fn lists(this: &YourBooksScreen, cx: &gpui_kit::App) -> Option<Lists> {
    this.state.read(cx).account().snapshot().map(|snapshot| Lists::new(snapshot, today()))
}

fn click(cx: &mut AsyncWindowContext, id: &str, right: bool) {
    let id = id.to_owned();
    let _ = cx.update(move |window, cx| {
        if right {
            window.right_click(id, cx);
        } else {
            window.click(id, cx);
        }
    });
}

async fn run(step: &str, this: &WeakEntity<YourBooksScreen>, cx: &mut AsyncWindowContext) {
    let reservations = step.starts_with("reservation")
        || step == "empty-reservations"
        || step.starts_with("edit")
        || step.starts_with("cancel");
    if reservations {
        let _ = this.update(cx, |this, cx| this.select_segment(YourBooksSegment::Reservations, cx));
        pause(cx, 300).await;
    }
    match step {
        "loan-menu" => click(cx, &format!("loan-{RENEWABLE}-more"), false),
        "loan-context" => click(cx, &format!("loan-{RENEWABLE}-row"), true),
        "reservation-menu" => click(cx, &format!("reservation-{RESERVATION}-more"), false),
        "reservation-context" => click(cx, &format!("reservation-{RESERVATION}-row"), true),
        "renew-confirm" => {
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(row) = lists(this, cx).and_then(|l| l.loans.into_iter().find(|r| r.id == RENEWABLE)) {
                    this.request_renew(row, window, cx);
                }
            });
        }
        "renew-ok" | "renew-refused" => {
            let irn = if step == "renew-ok" { RENEWABLE } else { REFUSED };
            let _ = this.update(cx, |this, cx| {
                if let Some(row) = lists(this, cx).and_then(|l| l.loans.into_iter().find(|r| r.id == irn)) {
                    this.renew(row, cx);
                }
            });
        }
        "edit" => {
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(row) = lists(this, cx).and_then(|l| l.reservations.row(RESERVATION).cloned()) {
                    this.request_edit(row, window, cx);
                }
            });
        }
        "cancel" | "cancel-done" => {
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(row) = lists(this, cx).and_then(|l| l.reservations.row(RESERVATION).cloned()) {
                    if step == "cancel" {
                        this.request_cancel(row, window, cx);
                    } else {
                        this.cancel(row, cx);
                    }
                }
            });
        }
        "refresh-fails" => {
            // The same account on a demo backend whose refreshes fail: the lists stay on screen.
            let _ = this.update(cx, |this, cx| {
                this.state.update(cx, |state, cx| {
                    let origin = state.backend().origin();
                    let session = state.session().map(str::to_owned);
                    let failing = crate::backend::demo::DemoBackend::live(Some(&origin), None, true);
                    state.account.use_account(Some(&origin), session.as_deref(), std::sync::Arc::new(failing), true);
                    state.refresh_account(true, cx).detach();
                })
            });
        }
        "empty-loans" | "empty-reservations" => {
            let _ = this.update(cx, |this, cx| {
                this.state.update(cx, |state, cx| {
                    state.update_account_snapshot(
                        |snapshot| {
                            snapshot.loans.items.clear();
                            snapshot.reservations.ready.items.clear();
                            snapshot.reservations.pending.items.clear();
                        },
                        cx,
                    )
                })
            });
        }
        _ => eprintln!("librespydus: unknown evidence step {PREFIX}{step}"),
    }
}

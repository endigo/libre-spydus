//! Screenshot hooks for the book page (debug builds with `--features evidence` only; docs/desktop-
//! app.md §9), like iOS `DetailDebugActions`. With `LIBRESPYDUS_EVIDENCE=detail:<step>[,<step>…]`,
//! the first book page opened runs the steps once its record has loaded, by clicking the real
//! window's elements (gpui-kit's `TestWindowExt`). Values with another prefix are ignored.
//!
//! Steps: `stall` (never load: the loading state), `format:<code>` (click that format's chip),
//! `bookmark`, `reserve` (signed out: asks to sign in), `expiry:<n>`, `confirm` (demo only),
//! `refuse` (demo only: the demo library has this reservation already, so Confirm is refused),
//! `close` (the Reserve dialog's Close / Done), `held` (click the "On loan" / "Reserved" pill), `renew` (demo only: confirm the renewal),
//! `more` (expand the synopsis), `scroll:<px>`, `wait:<ms>`.

#![cfg(debug_assertions)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{AsyncWindowContext, Context, ScrollDelta, WeakEntity, Window, point, px};

use crate::state::{AppState, Loadable};

use super::{DetailScreen, FormatLookup, HeldSearch};

const ENV: &str = "LIBRESPYDUS_EVIDENCE";
const PREFIX: &str = "detail:";

static TAKEN: AtomicBool = AtomicBool::new(false);

fn steps() -> Vec<String> {
    let Ok(value) = std::env::var(ENV) else { return Vec::new() };
    let Some(steps) = value.strip_prefix(PREFIX) else { return Vec::new() };
    steps.split(',').map(|step| step.trim().to_owned()).filter(|step| !step.is_empty()).collect()
}

/// `stall`: the first page never loads, to show its loading state.
pub fn stalls() -> bool {
    !TAKEN.load(Ordering::SeqCst) && steps().iter().any(|step| step == "stall")
}

/// Runs the steps on the first book page, once.
pub fn schedule(window: &mut Window, cx: &mut Context<DetailScreen>) {
    let steps = steps();
    if steps.is_empty() || TAKEN.swap(true, Ordering::SeqCst) {
        return;
    }
    cx.spawn_in(window, async move |this, cx| {
        wait_until(&this, cx, 60_000, |screen, _, _| {
            matches!(screen.record, Loadable::Loaded(_) | Loadable::Failed(_))
                && !matches!(screen.lookup, FormatLookup::Pending | FormatLookup::Searching)
                && screen.held_search != HeldSearch::Running
        })
        .await;
        pause(cx, 800).await;
        for step in steps {
            run(&this, cx, &step).await;
            pause(cx, 500).await;
        }
    })
    .detach();
}

async fn pause(cx: &mut AsyncWindowContext, ms: u64) {
    cx.background_executor().timer(Duration::from_millis(ms)).await;
}

/// Waits (up to `limit_ms`) until `ready` holds.
async fn wait_until(
    this: &WeakEntity<DetailScreen>,
    cx: &mut AsyncWindowContext,
    limit_ms: u64,
    ready: impl Fn(&DetailScreen, &mut Window, &mut gpui_kit::App) -> bool,
) -> bool {
    for _ in 0..limit_ms / 200 {
        let done = this.update_in(cx, |screen, window, cx| ready(screen, window, cx)).unwrap_or(true);
        if done {
            return true;
        }
        pause(cx, 200).await;
    }
    false
}

fn click(window: &mut Window, cx: &mut gpui_kit::App, id: &str) {
    window.render_frame(cx);
    if window.try_find(id.to_owned()).is_some() {
        window.click(id.to_owned(), cx);
    } else {
        log::warn!("evidence: no element {id}");
    }
}

/// Clicks `id` through the window (outside any entity update: the click updates the screen).
fn click_now(cx: &mut AsyncWindowContext, id: &str) {
    let _ = cx.update(|window, cx| click(window, cx, id));
}

async fn wait_for_element(cx: &mut AsyncWindowContext, id: &'static str) -> bool {
    for _ in 0..100 {
        let found = cx
            .update(|window, cx| {
                window.render_frame(cx);
                window.try_find(id).is_some()
            })
            .unwrap_or(false);
        if found {
            return true;
        }
        pause(cx, 200).await;
    }
    false
}

async fn run(this: &WeakEntity<DetailScreen>, cx: &mut AsyncWindowContext, step: &str) {
    let (demo, signed_in) = cx
        .update(|_, cx| {
            let state = AppState::global(cx).read(cx);
            (state.is_demo(), state.is_signed_in())
        })
        .unwrap_or((false, true));
    let demo_or_signed_out = demo || !signed_in;
    let (name, argument) = step.split_once(':').map_or((step, None), |(name, argument)| (name, Some(argument)));
    match name {
        "stall" => {}
        "format" => {
            let code = argument.unwrap_or_default().to_owned();
            let index = this
                .update(cx, |screen, _| {
                    screen.formats.iter().position(|format| format.code.eq_ignore_ascii_case(&code))
                })
                .ok()
                .flatten();
            if let Some(index) = index {
                click_now(cx, &format!("detail-format-{index}"));
            }
        }
        // Demo mode or signed out only: never saves to a real account's My Picks.
        "bookmark" if demo_or_signed_out => {
            click_now(cx, "detail-bookmark");
        }
        // Demo mode, or signed out (which only asks to sign in): never a real reservation.
        "reserve" if demo_or_signed_out => {
            if wait_for_element(cx, "detail-reserve").await {
                click_now(cx, "detail-reserve");
            }
        }
        "expiry" => {
            let id = format!("reserve-expiry-{}", argument.unwrap_or("0"));
            if wait_for_element(cx, "reserve-expiry").await {
                click_now(cx, &id);
            }
        }
        "confirm" if demo => {
            if wait_for_element(cx, "reserve-confirm").await {
                click_now(cx, "reserve-confirm");
            }
        }
        // The demo library takes this reservation first, so the one confirmed on screen is refused.
        "refuse" if demo => {
            let place = this.update(cx, |screen, cx| {
                let state = AppState::global(cx).read(cx);
                let url = screen.reservable_format()?.reserve_url.clone()?;
                Some(state.backend().place_reservation(url, state.session()?.to_owned(), None, None))
            });
            if let Ok(Some(place)) = place {
                let _ = place.await;
            }
        }
        "held" => {
            if wait_for_element(cx, "detail-held").await {
                click_now(cx, "detail-held");
            }
        }
        // Demo mode only: never a real renewal.
        "renew" if demo => {
            if wait_for_element(cx, "confirm-ok").await {
                click_now(cx, "confirm-ok");
            }
        }
        "close" => click_now(cx, "reserve-close"),
        "more" => {
            click_now(cx, "detail-synopsis-more");
        }
        "scroll" => {
            let pixels: f32 = argument.and_then(|value| value.parse().ok()).unwrap_or(400.);
            let _ = cx.update(|window, cx| {
                window.render_frame(cx);
                window.scroll("detail-screen", ScrollDelta::Pixels(point(px(0.), px(-pixels))), cx);
            });
        }
        "wait" => pause(cx, argument.and_then(|value| value.parse().ok()).unwrap_or(1000)).await,
        _ => log::warn!("evidence: step {step} skipped"),
    }
}

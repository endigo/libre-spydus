//! Screenshot steps for My Picks (cargo feature `evidence` only; docs/desktop-app.md §9): with
//! `LIBRESPYDUS_EVIDENCE=my_picks:<step>` the screen sets up a state the launch flags can't reach.
//! Ignored without the prefix.
//!
//! Steps: `seed-local` (two books saved on this computer), `copy-offer` (the offer dialog, as after
//! a sign-in with local picks), `menu` (the first cell's "…" menu).

use std::time::Duration;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{AsyncWindowContext, Context, Window};

use crate::state::BookRef;
use crate::state::picks::PicksCopyOffer;
use crate::state::picks::PicksPhase;

use super::MyPicksScreen;

const PREFIX: &str = "my_picks:";

pub(super) fn schedule(window: &mut Window, cx: &mut Context<MyPicksScreen>) {
    let Some(step) = std::env::var("LIBRESPYDUS_EVIDENCE").ok().and_then(|v| v.strip_prefix(PREFIX).map(str::to_owned))
    else {
        return;
    };
    cx.spawn_in(window, async move |this, cx| {
        pause(cx, 800).await;
        let state = match this.update(cx, |this, _| this.state.clone()) {
            Ok(state) => state,
            Err(_) => return,
        };
        match step.as_str() {
            "seed-local" => {
                let _ = cx.update(|_, cx| {
                    state.update(cx, |state, cx| {
                        let now = chrono::Utc::now();
                        for (id, title, author, cover) in SAMPLES {
                            let mut book = BookRef::new(*id, *title);
                            book.author = Some((*author).to_owned());
                            book.cover_url = Some((*cover).to_owned());
                            state.local_picks.add(&book, now);
                        }
                        cx.notify();
                    })
                });
            }
            "copy-offer" => {
                let _ = cx.update(|_, cx| {
                    state.update(cx, |state, cx| {
                        let origin = state.backend().origin();
                        state.picks_copy_offer = Some(PicksCopyOffer { origin, count: 2 });
                        cx.notify();
                    })
                });
            }
            "menu" => {
                for _ in 0..60 {
                    let loaded = cx.update(|_, cx| {
                        let state = state.read(cx);
                        !state.is_signed_in() || *state.picks().phase() == PicksPhase::Loaded
                    });
                    if loaded.unwrap_or(true) {
                        break;
                    }
                    pause(cx, 150).await;
                }
                pause(cx, 400).await;
                let first = cx.update(|_, cx| {
                    let state = state.read(cx);
                    if state.is_signed_in() {
                        state.picks().items().first().map(|item| item.id.clone())
                    } else {
                        state.local_picks().items().first().map(|book| book.id.clone())
                    }
                });
                if let Ok(Some(id)) = first {
                    let _ = cx.update(|window, cx| window.click(format!("pick-{id}-more"), cx));
                }
            }
            _ => eprintln!("librespydus: unknown evidence step {PREFIX}{step}"),
        }
    })
    .detach();
}

async fn pause(cx: &mut AsyncWindowContext, millis: u64) {
    cx.background_executor().timer(Duration::from_millis(millis)).await;
}

/// Public Syndetics covers on Canning's account.
const SAMPLES: &[(&str, &str, &str, &str)] = &[
    (
        "work:15642035",
        "The midnight library", // copy-exempt: evidence-only sample data (a book title)
        "Haig, Matt, author",
        "https://secure.syndetics.com/index.aspx?isbn=9781786892737/lc.gif&client=citycanning",
    ),
    (
        "bib:18000001",
        "The long night", // copy-exempt: evidence-only sample data (a book title)
        "White, Christian, author",
        "https://secure.syndetics.com/index.aspx?isbn=9781399197809/lc.gif&client=citycanning",
    ),
];

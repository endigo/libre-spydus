//! My Picks (D5; docs/desktop-app.md §6, native-apps.md §5.3, §5.11; a port of iOS
//! `FavouritesView.swift` / `MyPicksView.swift`): a cover grid. Signed in, the Pick list on the
//! library account ("Saved on your library account"), paged; signed out, this library's books saved
//! on this computer ("Saved on this computer"), newest first. A click opens the book page; the
//! "…" button or a right-click opens or removes a book (a toast either way).
//!
//! `offer` shows the copy-once offer after a sign-in (§5.11); the shell watches for it.

#[cfg(feature = "evidence")]
mod evidence;
mod offer;

pub use offer::{present_if_needed, watch_picks_copy_offer};

use gpui_kit::{
    AnyElement, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Role, SharedString,
    StatefulInteractiveElement, Styled, Subscription, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::formatting::author_name;
use crate::state::picks::PicksPhase;
use crate::state::results::SearchResultItem;
use crate::state::{AppState, BookRef, ReloadKey, Route, Section};
use crate::theme::{self, Font, colors, spacing};
use crate::ui::{
    Banner, BannerTone, CardAction, EmptyStateView, ErrorStateView, FontExt, LoadingView, Lucide, ShelfCover,
    card_menu_button, content_column, on_click, with_context_menu,
};

/// A grid cell's cover width.
const CELL_WIDTH: f32 = 148.;
const STATE_HEIGHT: f32 = 420.;

/// What the grid shows: the account's list or this computer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PicksContent {
    Account,
    Local,
}

impl PicksContent {
    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Account => Copy::Picks::subtitleAccount,
            Self::Local => Copy::Picks::subtitlePhone,
        }
    }
}

/// One cell: the book, and how to remove it.
#[derive(Clone)]
enum Pick {
    Account(SearchResultItem),
    Local(BookRef),
}

impl Pick {
    fn book(&self) -> &BookRef {
        match self {
            Self::Account(item) => &item.book,
            Self::Local(book) => book,
        }
    }

    fn id(&self) -> &str {
        match self {
            Self::Account(item) => &item.id,
            Self::Local(book) => &book.id,
        }
    }
}

pub struct MyPicksScreen {
    state: Entity<AppState>,
    shown: bool,
    reload_key: ReloadKey,
    _observe: Subscription,
}

impl MyPicksScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let observe = cx.observe_in(&state, window, |this, _, window, cx| this.state_changed(window, cx));
        let (shown, reload_key) = {
            let state = state.read(cx);
            (state.section() == Section::MyPicks, state.reload_key())
        };
        let screen = Self { state, shown, reload_key, _observe: observe };
        screen.load_if_needed(cx);
        offer::present_if_needed(window, cx);
        #[cfg(feature = "evidence")]
        evidence::schedule(window, cx);
        screen
    }

    pub fn content(&self, cx: &gpui_kit::App) -> PicksContent {
        if self.state.read(cx).is_signed_in() { PicksContent::Account } else { PicksContent::Local }
    }

    fn state_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (section, key, stale) = {
            let state = self.state.read(cx);
            (state.section(), state.reload_key(), state.picks().is_stale())
        };
        let shown = section == Section::MyPicks;
        let appeared = shown && !self.shown;
        self.shown = shown;
        if key != self.reload_key || appeared || (shown && stale) {
            self.reload_key = key;
            self.load_if_needed(cx);
        }
        // The offer may have waited for this screen (or for a dialog to close).
        offer::present_if_needed(window, cx);
        cx.notify();
    }

    /// Signed in: the list when it hasn't loaded, failed, or misses a pick saved since.
    fn load_if_needed(&self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.load_picks_if_needed(cx).detach());
    }

    fn open(&self, book: BookRef, cx: &mut gpui_kit::App) {
        self.state.update(cx, |state, cx| state.push_on(Section::MyPicks, Route::Detail(book), cx));
    }

    /// Removes `pick`: from the account (the cell leaves at once, back if the library refuses) or
    /// from this computer. "Removed from My Picks" either way.
    fn remove(&self, pick: &Pick, cx: &mut gpui_kit::App) {
        self.state.update(cx, |state, cx| match pick {
            Pick::Account(item) => state.remove_pick_item(item, cx).detach(),
            Pick::Local(book) => state.set_picked(book.clone(), false, cx).detach(),
        });
    }

    fn header(&self, content: PicksContent) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::XS))
            .child(
                div()
                    .id("my-picks-title")
                    .test_support()
                    .role(Role::Heading)
                    .aria_label(SharedString::from(Copy::Picks::title))
                    .typography(Font::Greeting)
                    .text_color(theme::color(colors::TEXT_PRIMARY))
                    .child(Copy::Picks::title),
            )
            .child(
                div()
                    .id("my-picks-subtitle")
                    .test_support()
                    .aria_label(SharedString::from(content.subtitle()))
                    .typography(Font::Caption)
                    .text_color(theme::color(colors::TEXT_SECONDARY))
                    .child(content.subtitle()),
            )
    }

    fn cell(&self, pick: Pick, cx: &mut Context<Self>) -> AnyElement {
        let id = format!("pick-{}", pick.id());
        let book = pick.book().clone();
        let view = cx.entity().downgrade();
        let actions = vec![
            CardAction::new(Copy::Common::open, {
                let (view, book) = (view.clone(), book.clone());
                move |_, cx| {
                    let _ = view.update(cx, |this, cx| this.open(book.clone(), cx));
                }
            }),
            CardAction::new(Copy::Picks::remove, {
                let (view, pick) = (view.clone(), pick.clone());
                move |_, cx| {
                    let _ = view.update(cx, |this, cx| this.remove(&pick, cx));
                }
            }),
        ];
        let cover = ShelfCover::new(SharedString::from(id.clone()), &book)
            .width(CELL_WIDTH)
            .shows_title(true)
            .on_click(on_click({
                let book = book.clone();
                move |_, cx| {
                    let _ = view.update(cx, |this, cx| this.open(book.clone(), cx));
                }
            }));
        let cover = match author_name(book.author.as_deref()) {
            Some(author) => cover.caption(author, None),
            None => cover,
        };
        let cell = div()
            .id(SharedString::from(format!("{id}-cell")))
            .test_support()
            .relative()
            .flex_none()
            .w(px(CELL_WIDTH))
            .child(cover)
            .child(
                div()
                    .id(SharedString::from(format!("{id}-more")))
                    .test_support()
                    .absolute()
                    .top(px(spacing::XS))
                    .right(px(spacing::XS))
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(theme::color_alpha(colors::BACKGROUND, 0.9))
                    .child(card_menu_button(SharedString::from(format!("{id}-more-button")), actions.clone())),
            );
        with_context_menu(cell, actions).into_any_element()
    }

    fn grid(&self, picks: Vec<Pick>, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("my-picks-grid")
            .test_support()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_x(px(spacing::XL))
            .gap_y(px(spacing::XL))
            .children(picks.into_iter().map(|pick| self.cell(pick, cx)))
    }

    fn local(&self, cx: &mut Context<Self>) -> AnyElement {
        let (items, error) = {
            let store = self.state.read(cx).local_picks();
            (store.items().to_vec(), store.last_error())
        };
        let body = div()
            .flex()
            .flex_col()
            .gap(px(spacing::L))
            .when_some(error, |body, error| body.child(Banner::new(error, BannerTone::Error)));
        if items.is_empty() {
            let state = self.state.clone();
            return body
                .child(
                    div().id("my-picks-empty").test_support().w_full().h(px(STATE_HEIGHT)).child(
                        EmptyStateView::new(Lucide::Bookmark, Copy::Picks::emptyTitle)
                            .message(Copy::Picks::emptyPhoneMessage)
                            .action(
                                "my-picks-find",
                                Copy::Picks::findABook,
                                on_click(move |_, cx| state.update(cx, |state, cx| state.open_search(None, cx))),
                            ),
                    ),
                )
                .into_any_element();
        }
        body.child(self.grid(items.into_iter().map(Pick::Local).collect(), cx)).into_any_element()
    }

    fn account(&self, cx: &mut Context<Self>) -> AnyElement {
        let (phase, items, next_page, loading_more, more_error) = {
            let picks = self.state.read(cx).picks();
            (
                picks.phase().clone(),
                picks.items().to_vec(),
                picks.next_page_url().is_some(),
                picks.is_loading_more(),
                picks.load_more_error().cloned(),
            )
        };
        let state = self.state.clone();
        match phase {
            PicksPhase::Idle | PicksPhase::Loading => {
                div().w_full().h(px(STATE_HEIGHT)).child(LoadingView::new()).into_any_element()
            }
            PicksPhase::Empty => div()
                .id("my-picks-empty")
                .test_support()
                .w_full()
                .h(px(STATE_HEIGHT))
                .child(
                    EmptyStateView::new(Lucide::Bookmark, Copy::Picks::emptyTitle)
                        .message(Copy::Picks::emptyAccountMessage),
                )
                .into_any_element(),
            PicksPhase::Failed(error) => div()
                .w_full()
                .h(px(STATE_HEIGHT))
                .child(
                    ErrorStateView::new("my-picks-error", &error)
                        .retry(on_click(move |_, cx| state.update(cx, |state, cx| state.refresh_picks(cx).detach()))),
                )
                .into_any_element(),
            PicksPhase::Loaded => {
                if next_page && !loading_more && more_error.is_none() {
                    // The grid's end is on screen: the next page.
                    let state = self.state.clone();
                    cx.defer(move |cx| state.update(cx, |state, cx| state.load_more_picks(cx).detach()));
                }
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::L))
                    .child(self.grid(items.into_iter().map(Pick::Account).collect(), cx))
                    .when_some(more_error, |column, error| {
                        let state = self.state.clone();
                        column.child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(spacing::S))
                                .py(px(spacing::L))
                                .child(
                                    div()
                                        .typography(Font::Caption)
                                        .text_color(theme::color(colors::TEXT_SECONDARY))
                                        .child(error.message.clone()),
                                )
                                .when(error.can_retry(), |footer| {
                                    footer.child(
                                        div()
                                            .id("my-picks-more-retry")
                                            .test_support()
                                            .role(Role::Button)
                                            .aria_label(SharedString::from(Copy::Common::retry))
                                            .aria_description(SharedString::from(Copy::Picks::retryLoadingMoreHint))
                                            .cursor_pointer()
                                            .typography(Font::BodyStrong)
                                            .text_color(theme::color(colors::PRIMARY))
                                            .on_click(move |_, _, cx| {
                                                state.update(cx, |state, cx| state.retry_load_more_picks(cx).detach())
                                            })
                                            .child(Copy::Common::retry),
                                    )
                                }),
                        )
                    })
                    .when(next_page && loading_more, |column| {
                        column.child(div().h(px(80.)).child(LoadingView::new().message(Copy::Picks::loadingMore)))
                    })
                    .into_any_element()
            }
        }
    }
}

impl Render for MyPicksScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.content(cx);
        let body = match content {
            PicksContent::Account => self.account(cx),
            PicksContent::Local => self.local(cx),
        };
        div().id("my-picks-screen").test_support().size_full().overflow_y_scroll().restrict_scroll_to_axis().child(
            content_column()
                .py(px(spacing::XL))
                .flex()
                .flex_col()
                .gap(px(spacing::L))
                .child(self.header(content))
                .child(body),
        )
    }
}

//! Home (D3; docs/desktop-app.md §6, native-apps.md §6 "Home"; iOS `HomeView`, `HomeViewModel`):
//! the library logo (→ Profile), the greeting with My card while signed in, the search field
//! (→ Search), "New arrivals", "Popular genres", the library's "Featured" collections (§5.10, only
//! when it has some) and "Your loans".
//!
//! Each shelf loads on its own: New arrivals with its own skeleton and error, Featured without
//! either, and the loans from the saved account (§5.2a). New arrivals and Featured show what was
//! saved on this computer at once while fresh copies load (§5.13). A new library, verification or
//! sign-in/out reloads New arrivals; Featured is public, so only a new library or verification
//! reloads it.

pub mod components;
pub mod loans;

use gpui_kit::{
    Context, CursorStyle, InteractiveElement, IntoElement, ObjectFit, ParentElement, Render, Role, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, StyledImage, Subscription, Task, TestSupportExt, Window, div,
    img, prelude::FluentBuilder, px,
};
use librespydus_core::app::{Genre, Shelf, home_genres, home_shelves};
use librespydus_core::{FeaturedLink, SearchResults, SearchSort};

use crate::copy::Copy;
use crate::features::featured::featured_row;
use crate::state::formatting::{greeting, today};
use crate::state::results::{merge, unrecognised_page};
use crate::state::{AppState, BookRef, Loadable, Route, Section};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{
    FontExt, GenreTile, Lucide, SearchFieldButton, SecondaryPillButton, SectionHeader, content_column, icon, on_click,
};

use self::components::{SectionErrorCard, ShelfEntry, cover_shelf, section_message_card, shelf_skeleton, sign_in_card};
use self::loans::{LoanShelfItem, LoanTone};

/// How many covers the "New arrivals" shelf shows.
pub const SHELF_LIMIT: usize = 10;

/// What Featured was loaded for: public data, so only a new library or verification reloads it.
type FeaturedKey = (String, u64);

pub struct HomeScreen {
    arrivals_shelf: Option<Shelf>,
    genres: Vec<Genre>,
    arrivals: Loadable<Vec<BookRef>>,
    /// The `reload_key` New arrivals was last loaded for.
    loaded_key: Option<crate::state::ReloadKey>,
    /// Bumped by each load: a late answer for an older load is dropped.
    arrivals_generation: u64,
    featured: Vec<FeaturedLink>,
    /// The origin `featured` belongs to.
    featured_origin: Option<String>,
    featured_key: Option<FeaturedKey>,
    featured_generation: u64,
    scroll: ScrollHandle,
    _arrivals_task: Option<Task<()>>,
    _featured_task: Option<Task<()>>,
    _observe: Subscription,
    #[cfg(debug_assertions)]
    debug_hooks_applied: bool,
}

impl HomeScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let observe = cx.observe_in(&state, window, |this, _, _, cx| this.load_if_needed(cx));
        let mut screen = Self {
            arrivals_shelf: home_shelves().into_iter().next(),
            genres: home_genres(),
            arrivals: Loadable::Idle,
            loaded_key: None,
            arrivals_generation: 0,
            featured: Vec::new(),
            featured_origin: None,
            featured_key: None,
            featured_generation: 0,
            scroll: ScrollHandle::new(),
            _arrivals_task: None,
            _featured_task: None,
            _observe: observe,
            #[cfg(debug_assertions)]
            debug_hooks_applied: false,
        };
        // Back on Home with a stale account: fetch it in the background.
        state.update(cx, |state, cx| {
            if state.is_signed_in() {
                state.refresh_account(false, cx).detach();
            }
        });
        screen.load_if_needed(cx);
        screen
    }

    pub fn arrivals(&self) -> &Loadable<Vec<BookRef>> {
        &self.arrivals
    }

    pub fn featured(&self) -> &[FeaturedLink] {
        &self.featured
    }

    /// Loads each shelf once per key: coming back from a pushed screen doesn't reload.
    fn load_if_needed(&mut self, cx: &mut Context<Self>) {
        let state = AppState::global(cx).read(cx);
        let key = state.reload_key();
        let featured_key = (key.origin.clone(), key.verification);
        if self.loaded_key.as_ref() != Some(&key) {
            self.loaded_key = Some(key);
            self.load_arrivals(cx);
        }
        if self.featured_key.as_ref() != Some(&featured_key) {
            self.featured_key = Some(featured_key);
            self.load_featured(cx);
        }
    }

    /// The shelf's covers: those saved on this computer at once, then fresh ones (§5.13).
    fn load_arrivals(&mut self, cx: &mut Context<Self>) {
        let Some(shelf) = self.arrivals_shelf.clone() else {
            self.arrivals = Loadable::Loaded(Vec::new());
            return;
        };
        self.arrivals_generation += 1;
        let generation = self.arrivals_generation;
        let state = AppState::global(cx);
        let backend = state.read(cx).backend();
        let shows_saved = match backend.saved_shelf(&shelf.key) {
            Some(saved) if !saved.records.is_empty() || saved.no_results => {
                self.arrivals = Loadable::Loaded(shelf_items(&saved));
                true
            }
            _ => {
                self.arrivals = Loadable::Loading;
                false
            }
        };
        let fetch = backend.search_sorted(shelf.query.clone(), shelf.sort);
        cx.notify();
        self._arrivals_task = Some(cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let save = this
                .update(cx, |this, cx| {
                    if generation != this.arrivals_generation {
                        return None;
                    }
                    cx.notify();
                    match result {
                        Ok(results) if results.records.is_empty() && !results.no_results => {
                            // An unrecognised page never replaces saved covers, and isn't saved.
                            if !shows_saved {
                                this.arrivals = Loadable::Failed(unrecognised_page());
                            }
                            None
                        }
                        Ok(results) => {
                            this.arrivals = Loadable::Loaded(shelf_items(&results));
                            Some(backend.save_shelf(shelf.key.clone(), results))
                        }
                        Err(error) => {
                            let shown = state.update(cx, |state, cx| state.report(&error, cx));
                            if !shows_saved {
                                this.arrivals = Loadable::Failed(shown);
                            }
                            None
                        }
                    }
                })
                .ok()
                .flatten();
            if let Some(save) = save {
                save.await;
            }
        }));
    }

    fn retry_arrivals(&mut self, cx: &mut Context<Self>) {
        self.load_arrivals(cx);
    }

    /// `featured()`: the tiles saved on this computer show while it loads; nothing shows while
    /// it loads otherwise, when the library has no such list, or when it fails (no error on Home).
    fn load_featured(&mut self, cx: &mut Context<Self>) {
        let state = AppState::global(cx);
        let backend = state.read(cx).backend();
        let origin = backend.origin();
        // Another library's tiles never show while this one's load.
        if self.featured_origin.as_deref() != Some(origin.as_str()) {
            self.featured = backend.saved_featured().unwrap_or_default();
            self.featured_origin = Some(origin.clone());
        }
        self.featured_generation += 1;
        let generation = self.featured_generation;
        let fetch = backend.featured();
        cx.notify();
        self._featured_task = Some(cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let save = this
                .update(cx, |this, cx| {
                    if generation != this.featured_generation {
                        return None;
                    }
                    match result {
                        Ok(links) => {
                            this.featured = links.clone();
                            this.featured_origin = Some(origin);
                            cx.notify();
                            Some(backend.save_featured(links))
                        }
                        Err(_) => {
                            // Hidden, no error: Home's other shelves report verification and network
                            // problems. It tries again with the next key.
                            this.featured_key = None;
                            None
                        }
                    }
                })
                .ok()
                .flatten();
            if let Some(save) = save {
                save.await;
            }
        }));
    }

    // ---- Sections ----

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::global(cx).read(cx);
        let library_name = state.library_name();
        let logo = state.library().and_then(|library| library.logo_url.clone());
        let signed_in = state.is_signed_in();
        let title = greeting(state.account_name().as_deref());
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::L))
            .child(
                div()
                    .id("home-logo")
                    .test_support()
                    .role(Role::Button)
                    .aria_label(SharedString::from(Copy::Home::profileButton(&library_name)))
                    .aria_description(SharedString::from(Copy::Home::profileButtonHint))
                    .cursor(CursorStyle::PointingHand)
                    .flex_none()
                    .size(px(52.))
                    .rounded(px(radius::FIELD))
                    .overflow_hidden()
                    .bg(theme::color(0xFFFFFF))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click(|_, _, cx| {
                        AppState::global(cx).update(cx, |state, cx| state.select(Section::Profile, cx))
                    })
                    .child(icon(Lucide::LibraryBig, px(24.), theme::color(colors::TAB_ACTIVE)))
                    .when_some(logo, |frame, url| {
                        frame.child(
                            img(SharedString::from(url))
                                .id("home-logo-img")
                                .absolute()
                                .size(px(52.))
                                .p(px(4.))
                                .object_fit(ObjectFit::Contain)
                                .bg(theme::color(0xFFFFFF)),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .id("home-greeting")
                            .test_support()
                            .role(Role::Heading)
                            .aria_label(SharedString::from(title.clone()))
                            .typography(Font::Greeting)
                            .text_color(theme::color(colors::TEXT_PRIMARY))
                            .truncate()
                            .child(title),
                    )
                    .child(
                        div()
                            .typography(Font::Body)
                            .text_color(theme::color(colors::TEXT_SECONDARY))
                            .truncate()
                            .child(library_name),
                    ),
            )
            .when(signed_in, |row| {
                row.child(
                    SecondaryPillButton::new("home-my-card", Copy::MyCard::button)
                        .icon(Lucide::ScanBarcode)
                        .on_click(on_click(crate::features::my_card::open_my_card)),
                )
            })
    }

    fn arrivals_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self
            .arrivals_shelf
            .as_ref()
            .map_or_else(|| Copy::Home::newArrivals.to_owned(), |shelf| shelf.title.clone());
        let mut header = SectionHeader::new("home-new-arrivals-view-all", title);
        if let Some(shelf) = self.arrivals_shelf.clone() {
            header = header.action(
                None,
                on_click(move |_, cx| {
                    let route =
                        Route::Results { query: shelf.query.clone(), sort: shelf.sort, title: shelf.title.clone() };
                    AppState::global(cx).update(cx, |state, cx| state.push_on(Section::Home, route, cx));
                }),
            );
        }
        let entity = cx.entity();
        let content = match &self.arrivals {
            Loadable::Idle | Loadable::Loading => shelf_skeleton("home-new-arrivals-loading").into_any_element(),
            Loadable::Failed(error) => SectionErrorCard::new(
                "home-new-arrivals-error",
                error,
                on_click(move |_, cx| entity.update(cx, |home, cx| home.retry_arrivals(cx))),
            )
            .into_any_element(),
            Loadable::Loaded(books) if books.is_empty() => section_message_card(
                "home-new-arrivals-empty",
                Lucide::Sparkles,
                Copy::Home::nothingNewTitle,
                Some(Copy::Home::nothingNewMessage.into()),
            )
            .into_any_element(),
            Loadable::Loaded(books) => {
                cover_shelf("home-new-arrival", books.iter().cloned().map(ShelfEntry::titled).collect())
                    .into_any_element()
            }
        };
        section(header, content)
    }

    fn genres_section(&self) -> impl IntoElement {
        section(
            SectionHeader::new("home-genres-header", Copy::Home::popularGenres),
            div().id("home-genres").w_full().flex().flex_row().flex_wrap().gap(px(spacing::M)).children(
                self.genres.iter().enumerate().map(|(index, genre)| {
                    let route = Route::Results {
                        query: genre.query.clone(),
                        sort: SearchSort::Relevance,
                        title: genre.label.clone(),
                    };
                    GenreTile::new(
                        SharedString::from(format!("home-genre-{}", genre.key)),
                        genre.label.clone(),
                        genre_icon(&genre.key),
                    )
                    .color(theme::color(colors::GENRE[index % colors::GENRE.len()]))
                    .size(84.)
                    .hint(Copy::Home::genreHint(&genre.label))
                    .on_click(on_click(move |_, cx| {
                        let route = route.clone();
                        AppState::global(cx).update(cx, |state, cx| state.push_on(Section::Home, route, cx));
                    }))
                }),
            ),
        )
    }

    fn loans_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::global(cx).read(cx);
        let signed_in = state.is_signed_in();
        let mut header = SectionHeader::new("home-loans-view-all", Copy::Home::yourLoans);
        if signed_in {
            header = header.action(
                None,
                on_click(|_, cx| AppState::global(cx).update(cx, |state, cx| state.select(Section::YourBooks, cx))),
            );
        }
        let content = if !signed_in {
            sign_in_card(&state.library_name()).into_any_element()
        } else {
            match home_loans(state) {
                Loadable::Idle | Loadable::Loading => shelf_skeleton("home-loans-loading").into_any_element(),
                Loadable::Failed(error) => SectionErrorCard::new(
                    "home-loans-error",
                    &error,
                    on_click(|_, cx| {
                        AppState::global(cx).update(cx, |state, cx| state.refresh_account(true, cx).detach())
                    }),
                )
                .into_any_element(),
                Loadable::Loaded(loans) if loans.is_empty() => section_message_card(
                    "home-loans-empty",
                    Lucide::LibraryBig,
                    Copy::Home::noLoansTitle,
                    Some(Copy::Home::noLoansMessage.into()),
                )
                .into_any_element(),
                Loadable::Loaded(loans) => cover_shelf(
                    "home-loan",
                    loans
                        .into_iter()
                        .map(|loan| ShelfEntry {
                            caption_color: Some(theme::color(match loan.tone {
                                LoanTone::Normal => colors::TEXT_SECONDARY,
                                LoanTone::Soon => colors::WARNING,
                                LoanTone::Overdue => colors::DANGER,
                            })),
                            caption: loan.caption,
                            book: loan.book,
                        })
                        .collect(),
                )
                .into_any_element(),
            }
        };
        section(header, content)
    }

    #[cfg(debug_assertions)]
    fn apply_debug_hooks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Evidence hooks: `LS_HOME_OPEN=arrivals|genre:<key>|featured:<title>` opens that list or
        // collection as if clicked; `LS_HOME_SCROLL_TO=bottom` scrolls to "Your loans".
        if self.debug_hooks_applied {
            return;
        }
        let open = std::env::var("LS_HOME_OPEN").ok();
        let scroll = std::env::var("LS_HOME_SCROLL_TO").ok();
        if open.is_none() && scroll.is_none() {
            self.debug_hooks_applied = true;
            return;
        }
        if let Some(title) = open.as_deref().and_then(|hook| hook.strip_prefix("featured:")) {
            let Some(link) = self.featured.iter().find(|link| same_title(&link.title, title)).cloned() else {
                return; // until the tiles arrive
            };
            self.debug_hooks_applied = true;
            // After `--route home` has selected Home (which empties its stack).
            cx.spawn_in(window, async move |_, cx| {
                cx.background_executor().timer(std::time::Duration::from_millis(500)).await;
                let _ = cx.update(|_, cx| crate::features::featured::open_link(&link, Section::Home, cx));
            })
            .detach();
            return;
        }
        self.debug_hooks_applied = true;
        if let Some(hook) = open {
            let route = if hook == "arrivals" {
                self.arrivals_shelf.as_ref().map(|shelf| Route::Results {
                    query: shelf.query.clone(),
                    sort: shelf.sort,
                    title: shelf.title.clone(),
                })
            } else {
                hook.strip_prefix("genre:").and_then(|key| self.genres.iter().find(|genre| genre.key == key)).map(
                    |genre| Route::Results {
                        query: genre.query.clone(),
                        sort: SearchSort::Relevance,
                        title: genre.label.clone(),
                    },
                )
            };
            if let Some(route) = route {
                cx.defer_in(window, move |_, _, cx| {
                    AppState::global(cx).update(cx, |state, cx| state.push_on(Section::Home, route, cx))
                });
            }
        }
        if scroll.as_deref() == Some("bottom") {
            let handle = self.scroll.clone();
            cx.spawn_in(window, async move |_, cx| {
                cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
                handle.scroll_to_bottom();
                let _ = cx.update(|window, _| window.refresh());
            })
            .detach();
        }
    }
}

/// Debug hooks match titles loosely: case and apostrophes ("What’s New").
#[cfg(debug_assertions)]
fn same_title(a: &str, b: &str) -> bool {
    let plain = |text: &str| text.replace('’', "'").to_lowercase();
    plain(a) == plain(b)
}

/// A section: its header over its content.
fn section(header: SectionHeader, content: impl IntoElement) -> impl IntoElement {
    div().w_full().flex().flex_col().gap(px(spacing::M)).child(header).child(content)
}

/// One cover per book, at most `SHELF_LIMIT`.
fn shelf_items(results: &SearchResults) -> Vec<BookRef> {
    let mut seen = std::collections::HashSet::new();
    merge(&[], &results.records)
        .into_iter()
        .map(|item| item.book)
        .filter(|book| seen.insert(book.id.clone()))
        .take(SHELF_LIMIT)
        .collect()
}

/// "Your loans" from the saved account: `Idle` signed out, `Loading` until the first snapshot (or
/// while retrying one that failed), else its loans in due order.
pub fn home_loans(state: &AppState) -> Loadable<Vec<LoanShelfItem>> {
    if !state.is_signed_in() {
        return Loadable::Idle;
    }
    let account = state.account();
    if let Some(snapshot) = account.snapshot() {
        return Loadable::Loaded(LoanShelfItem::make(&snapshot.loans.items, today()));
    }
    match account.last_refresh_error() {
        Some(error) if !account.is_refreshing() => Loadable::Failed(error.clone()),
        _ => Loadable::Loading,
    }
}

/// The icon for a `home_genres()` key; unknown keys get a generic books icon.
pub fn genre_icon(key: &str) -> Lucide {
    match key {
        "biography" => Lucide::User,
        "fiction" => Lucide::BookOpenText,
        "science" => Lucide::Atom,
        "history" => Lucide::Landmark,
        "fantasy" => Lucide::WandSparkles,
        "crime" => Lucide::VenetianMask,
        "cooking" => Lucide::ChefHat,
        "children" => Lucide::Baby,
        _ => Lucide::LibraryBig,
    }
}

impl Render for HomeScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(debug_assertions)]
        self.apply_debug_hooks(window, cx);
        #[cfg(not(debug_assertions))]
        let _ = window;
        let featured = (!self.featured.is_empty()).then(|| featured_row(&self.featured));
        div()
            .id("home-screen")
            .test_support()
            .size_full()
            .overflow_y_scroll()
            .restrict_scroll_to_axis()
            .track_scroll(&self.scroll)
            .child(
                content_column()
                    .pt(px(spacing::S))
                    .pb(px(spacing::XXL))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XL))
                    .child(self.header(cx))
                    .child(SearchFieldButton::new(
                        "home-search",
                        on_click(|_, cx| AppState::global(cx).update(cx, |state, cx| state.open_search(None, cx))),
                    ))
                    .child(self.arrivals_section(cx))
                    .child(self.genres_section())
                    .children(featured)
                    .child(self.loans_section(cx)),
            )
    }
}

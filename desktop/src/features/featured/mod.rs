//! The library's own "Featured" collections (D3; docs/native-apps.md §5.10; iOS `FeaturedModels`,
//! `FeaturedRow`, `CollectionView`): Home's row of tiles, where a tile leads, and the Collection
//! screen's shelves. `web` links open in the default browser (docs/desktop-app.md §5.5).

use std::collections::HashMap;

use gpui_kit::{
    App, Context, CursorStyle, InteractiveElement, IntoElement, ParentElement, Render, Role, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt, Window, div, px,
};
use librespydus_core::{FeaturedKind, FeaturedLink, FeaturedSection, SearchRecord, SearchResults};

use crate::copy::Copy;
use crate::features::home::components::{SectionErrorCard, ShelfEntry, SkeletonBlock, cover_shelf, shelf_skeleton};
use crate::features::search::sentinel::sentinel;
use crate::state::results::{merge, unrecognised_page};
use crate::state::{AppState, BookRef, Route, Section, UserFacingError};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{EmptyStateView, ErrorStateView, FontExt, Lucide, SectionHeader, content_column, icon, on_click};

// ---- Icons ----

/// The icon for the library's Font Awesome name ("game-board", "fa-globe"); anything else gets
/// sparkles.
pub fn featured_icon(name: Option<&str>) -> Lucide {
    let name = name.unwrap_or_default().trim().to_lowercase();
    match name.strip_prefix("fa-").unwrap_or(&name) {
        "book" => Lucide::BookOpenText,
        "balloons" => Lucide::PartyPopper,
        "computer" => Lucide::Monitor,
        "circle" => Lucide::Circle,
        "globe" => Lucide::Globe,
        "game-board" => Lucide::Puzzle,
        "map-location-dot" => Lucide::MapPinned,
        "user-circle" => Lucide::CircleUserRound,
        _ => Lucide::Sparkles,
    }
}

// ---- Where a tile leads ----

#[derive(Debug, Clone, PartialEq)]
pub enum FeaturedDestination {
    /// Pushed on the current section: a collection's shelves, or a saved search's results.
    Route(Route),
    /// The default browser.
    Web(String),
}

impl FeaturedDestination {
    /// `Collection` → the Collection screen; `Search` → its results list, titled with the link;
    /// `Web` → the browser (`None` for a URL that isn't http(s)).
    pub fn new(link: &FeaturedLink) -> Option<Self> {
        match link.kind {
            FeaturedKind::Collection => Some(Self::Route(Route::Collection(link.clone()))),
            FeaturedKind::Search => {
                Some(Self::Route(Route::ResultsPage { url: link.url.clone(), title: link.title.clone() }))
            }
            FeaturedKind::Web => web_url(&link.url).map(Self::Web),
        }
    }
}

/// `url` when the browser should open it (http or https).
pub fn web_url(url: &str) -> Option<String> {
    let url = url.trim();
    let lower = url.to_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://")).then(|| url.to_owned())
}

/// A Featured tile clicked: its collection or results list on `section`'s stack, or its web page.
pub fn open_link(link: &FeaturedLink, section: Section, cx: &mut App) {
    match FeaturedDestination::new(link) {
        Some(FeaturedDestination::Route(route)) => {
            AppState::global(cx).update(cx, |state, cx| state.push_on(section, route, cx))
        }
        Some(FeaturedDestination::Web(url)) => cx.open_url(&url),
        None => {}
    }
}

// ---- Home's row ----

pub const TILE_WIDTH: f32 = 128.;
const ICON_DIAMETER: f32 = 44.;

/// Home's "Featured" section: the collections as a horizontal row of tiles (rounded `surface`
/// cards, the icon on a tinted circle, the title under it, two lines at most). Ids
/// `home-featured-{index}`.
pub fn featured_row(links: &[FeaturedLink]) -> impl IntoElement {
    div()
        .id("home-featured")
        .test_support()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::M))
        .child(SectionHeader::new("home-featured-header", Copy::Featured::title))
        .child(
            div().id("home-featured-row").w_full().overflow_x_scroll().restrict_scroll_to_axis().child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(spacing::M))
                    .pb(px(spacing::S))
                    .children(links.iter().enumerate().map(|(index, link)| featured_tile(index, link))),
            ),
        )
}

fn featured_tile(index: usize, link: &FeaturedLink) -> impl IntoElement {
    let tint = theme::color(colors::GENRE[index % colors::GENRE.len()]);
    let hint = link.description.clone().filter(|text| !text.trim().is_empty()).unwrap_or_else(|| {
        match link.kind {
            FeaturedKind::Collection | FeaturedKind::Search => Copy::Featured::collectionHint,
            FeaturedKind::Web => Copy::Featured::websiteHint,
        }
        .to_owned()
    });
    let opened = link.clone();
    div()
        .id(SharedString::from(format!("home-featured-{index}")))
        .test_support()
        .role(Role::Button)
        .aria_label(SharedString::from(link.title.clone()))
        .aria_description(SharedString::from(hint))
        .cursor(CursorStyle::PointingHand)
        .flex_none()
        .w(px(TILE_WIDTH))
        .flex()
        .flex_col()
        .gap(px(spacing::M))
        .p(px(spacing::M))
        .rounded(px(radius::CARD))
        .bg(theme::color(colors::SURFACE))
        .hover(|style| style.bg(theme::color(colors::SURFACE_ELEVATED)))
        .on_click(move |_, _, cx| open_link(&opened, Section::Home, cx))
        .child(
            div()
                .size(px(ICON_DIAMETER))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(tint.opacity(0.18))
                .child(icon(featured_icon(link.icon.as_deref()), px(ICON_DIAMETER * 0.45), tint)),
        )
        .child(
            div()
                // Two lines' height, so the tiles line up.
                .h(px(Font::Caption.size_value() * 1.3 * 2.))
                .overflow_hidden()
                .typography(Font::Caption)
                .line_height(px(Font::Caption.size_value() * 1.3))
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_color(theme::color(colors::TEXT_PRIMARY))
                .line_clamp(2)
                .child(link.title.clone()),
        )
}

// ---- Collection screen ----

/// How many covers a shelf shows (one results page).
pub const SHELF_LIMIT: usize = 20;
/// Shelves loading at once (What's New has 11 sections).
pub const MAX_CONCURRENT_SHELVES: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub enum CollectionPhase {
    Loading,
    /// One shelf per section.
    Shelves,
    /// A page of links to outside services (eLibrary): "Open website".
    NoSections,
    Failed(UserFacingError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShelfState {
    /// Not in view yet.
    Idle,
    /// In view, waiting for a free slot.
    Queued,
    Loading,
    Loaded(Vec<BookRef>),
    Failed(UserFacingError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollectionShelf {
    pub section: FeaturedSection,
    pub state: ShelfState,
}

impl CollectionShelf {
    /// "View All" → the section's results list.
    pub fn route(&self) -> Route {
        Route::ResultsPage { url: self.section.url.clone(), title: self.section.title.clone() }
    }
}

/// A Featured collection (`Route::Collection`), titled with the link: one shelf per section, each
/// the first `page(section.url)`. Shelves load as they come into view, three at a time; one
/// scrolled past before its turn waits until it's back. A section that comes back empty is
/// hidden, one that fails offers Retry in its row. A collection without sections offers its web
/// page.
pub struct CollectionScreen {
    link: FeaturedLink,
    phase: CollectionPhase,
    shelves: Vec<CollectionShelf>,
    /// Bumped by every load: shelf answers for an older load are dropped.
    generation: u64,
    queue: Vec<usize>,
    running: HashMap<usize, Task<()>>,
    peak_concurrent: usize,
    loaded_key: (String, u64),
    scroll: ScrollHandle,
    _load: Option<Task<()>>,
    _observe: Subscription,
}

impl CollectionScreen {
    pub fn new(link: FeaturedLink, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let observe = cx.observe_in(&state, window, |this, _, _, cx| {
            let key = public_key(cx);
            if key != this.loaded_key {
                this.loaded_key = key;
                this.load(cx);
            }
        });
        let mut screen = Self {
            link,
            phase: CollectionPhase::Loading,
            shelves: Vec::new(),
            generation: 0,
            queue: Vec::new(),
            running: HashMap::new(),
            peak_concurrent: 0,
            loaded_key: public_key(cx),
            scroll: ScrollHandle::new(),
            _load: None,
            _observe: observe,
        };
        screen.load(cx);
        screen
    }

    pub fn link(&self) -> &FeaturedLink {
        &self.link
    }

    pub fn phase(&self) -> &CollectionPhase {
        &self.phase
    }

    pub fn shelves(&self) -> &[CollectionShelf] {
        &self.shelves
    }

    /// The most shelves that were loading at once (tests).
    pub fn peak_concurrent_shelves(&self) -> usize {
        self.peak_concurrent
    }

    /// Loads the collection page afresh.
    pub fn load(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        let generation = self.generation;
        self.running.clear();
        self.queue.clear();
        self.phase = CollectionPhase::Loading;
        self.shelves.clear();
        let state = AppState::global(cx);
        let fetch = state.read(cx).backend().collection(self.link.url.clone());
        cx.notify();
        self._load = Some(cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let _ = this.update(cx, |this, cx| {
                if generation != this.generation {
                    return;
                }
                match result {
                    Ok(collection) => {
                        this.shelves = collection
                            .sections
                            .into_iter()
                            .map(|section| CollectionShelf { section, state: ShelfState::Idle })
                            .collect();
                        this.phase = if this.shelves.is_empty() {
                            CollectionPhase::NoSections
                        } else {
                            CollectionPhase::Shelves
                        };
                    }
                    Err(error) => {
                        this.phase = CollectionPhase::Failed(state.update(cx, |state, cx| state.report(&error, cx)))
                    }
                }
                cx.notify();
            });
        }));
    }

    /// Shelves to draw: every section except those that came back empty.
    fn visible_shelves(&self) -> impl Iterator<Item = (usize, &CollectionShelf)> {
        self.shelves
            .iter()
            .enumerate()
            .filter(|(_, shelf)| !matches!(&shelf.state, ShelfState::Loaded(books) if books.is_empty()))
    }

    /// Every section came back empty.
    pub fn is_empty(&self) -> bool {
        self.phase == CollectionPhase::Shelves && self.visible_shelves().next().is_none()
    }

    /// A shelf's row came into view: queue its first page (once; a failed one waits for Retry).
    pub fn shelf_appeared(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.shelves.get(index).is_some_and(|shelf| shelf.state == ShelfState::Idle) {
            self.enqueue(index, cx);
        }
    }

    /// A shelf's row left the window before its turn: it waits until it's back, so the shelves on
    /// screen go first.
    pub fn shelf_disappeared(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(shelf) = self.shelves.get_mut(index)
            && shelf.state == ShelfState::Queued
        {
            shelf.state = ShelfState::Idle;
            self.queue.retain(|queued| *queued != index);
            cx.notify();
        }
    }

    pub fn retry_shelf(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.shelves.get(index).is_some_and(|shelf| matches!(shelf.state, ShelfState::Failed(_))) {
            self.enqueue(index, cx);
        }
    }

    fn enqueue(&mut self, index: usize, cx: &mut Context<Self>) {
        self.shelves[index].state = ShelfState::Queued;
        self.queue.push(index);
        self.start_queued(cx);
        cx.notify();
    }

    fn start_queued(&mut self, cx: &mut Context<Self>) {
        let state = AppState::global(cx);
        let backend = state.read(cx).backend();
        while self.running.len() < MAX_CONCURRENT_SHELVES && !self.queue.is_empty() {
            let index = self.queue.remove(0);
            if self.shelves.get(index).is_none_or(|shelf| shelf.state != ShelfState::Queued) {
                continue;
            }
            self.shelves[index].state = ShelfState::Loading;
            let generation = self.generation;
            let fetch = backend.page(self.shelves[index].section.url.clone());
            let state = state.clone();
            let task = cx.spawn(async move |this, cx| {
                let result = fetch.await;
                let _ = this.update(cx, |this, cx| {
                    if generation != this.generation {
                        return;
                    }
                    this.running.remove(&index);
                    let shown = match result {
                        Ok(page) => shelf_state(page),
                        Err(error) => ShelfState::Failed(state.update(cx, |state, cx| state.report(&error, cx))),
                    };
                    if let Some(shelf) = this.shelves.get_mut(index) {
                        shelf.state = shown;
                    }
                    this.start_queued(cx);
                    cx.notify();
                });
            });
            self.running.insert(index, task);
            self.peak_concurrent = self.peak_concurrent.max(self.running.len());
        }
    }

    fn shelf_row(&self, index: usize, shelf: &CollectionShelf, cx: &mut Context<Self>) -> impl IntoElement {
        let route = shelf.route();
        let weak = cx.entity().downgrade();
        let content = match &shelf.state {
            ShelfState::Idle | ShelfState::Queued | ShelfState::Loading => {
                let idle = shelf.state == ShelfState::Idle;
                let queued = shelf.state == ShelfState::Queued;
                let weak = weak.clone();
                div()
                    .w_full()
                    .child(shelf_skeleton(SharedString::from(format!("collection-shelf-{index}-loading"))))
                    .child(sentinel(move |visible, cx| {
                        let _ = weak.update(cx, |screen, cx| {
                            if visible && idle {
                                screen.shelf_appeared(index, cx);
                            } else if !visible && queued {
                                screen.shelf_disappeared(index, cx);
                            }
                        });
                    }))
                    .into_any_element()
            }
            ShelfState::Failed(error) => SectionErrorCard::new(
                SharedString::from(format!("collection-shelf-{index}-error")),
                error,
                on_click(move |_, cx| {
                    let _ = weak.update(cx, |screen, cx| screen.retry_shelf(index, cx));
                }),
            )
            .into_any_element(),
            ShelfState::Loaded(books) => cover_shelf(
                &format!("collection-shelf-{index}-item"),
                books.iter().cloned().map(ShelfEntry::titled).collect(),
            )
            .into_any_element(),
        };
        div()
            .id(SharedString::from(format!("collection-shelf-{index}")))
            .test_support()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(spacing::M))
            .child(
                SectionHeader::new(
                    SharedString::from(format!("collection-shelf-{index}-view-all")),
                    shelf.section.title.clone(),
                )
                .action(
                    None,
                    on_click(move |_, cx| {
                        let route = route.clone();
                        AppState::global(cx).update(cx, |state, cx| state.push(route, cx));
                    }),
                ),
            )
            .child(content)
    }

    fn website_offer(
        &self,
        glyph: Lucide,
        title: String,
        message: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let _ = cx;
        let url = web_url(&self.link.url);
        div().id("collection-website").test_support().size_full().child(
            EmptyStateView::new(glyph, title).message(message).action(
                "collection-open-website",
                Copy::Common::openWebsite,
                on_click(move |_, cx| {
                    if let Some(url) = &url {
                        cx.open_url(url);
                    }
                }),
            ),
        )
    }
}

fn public_key(cx: &App) -> (String, u64) {
    let state = AppState::global(cx).read(cx);
    (state.backend().origin(), state.verification_generation())
}

/// A shelf's first page: its covers, or an unrecognised page's error.
fn shelf_state(page: SearchResults) -> ShelfState {
    if page.records.is_empty() && !page.no_results {
        ShelfState::Failed(unrecognised_page())
    } else {
        ShelfState::Loaded(covers(&page.records))
    }
}

/// One cover per item, at most `SHELF_LIMIT`.
pub fn covers(records: &[SearchRecord]) -> Vec<BookRef> {
    let mut seen = std::collections::HashSet::new();
    merge(&[], records)
        .into_iter()
        .map(|item| item.book)
        .filter(|book| seen.insert(book.id.clone()))
        .take(SHELF_LIMIT)
        .collect()
}

/// The collection page loading: a few section headers over cover skeletons.
fn collection_skeleton() -> impl IntoElement {
    div()
        .id("collection-loading")
        .test_support()
        .role(Role::ProgressIndicator)
        .aria_label(SharedString::from(Copy::Common::loading))
        .size_full()
        .overflow_hidden()
        .child(content_column().pt(px(spacing::L)).flex().flex_col().gap(px(spacing::XL)).children((0..3).map(
            |index| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::M))
                    .child(SkeletonBlock::new(px(160.), px(16.)))
                    .child(shelf_skeleton(SharedString::from(format!("collection-skeleton-{index}"))))
            },
        )))
}

impl Render for CollectionScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self.link.title.clone();
        let content = match self.phase.clone() {
            CollectionPhase::Loading => collection_skeleton().into_any_element(),
            CollectionPhase::Failed(error) => {
                let entity = cx.entity();
                div()
                    .id("collection-error-state")
                    .test_support()
                    .size_full()
                    .child(
                        ErrorStateView::new("collection-error", &error)
                            .retry(on_click(move |_, cx| entity.update(cx, |screen, cx| screen.load(cx)))),
                    )
                    .into_any_element()
            }
            CollectionPhase::NoSections => {
                self.website_offer(Lucide::Globe, title.clone(), Copy::Featured::websiteOnly, cx).into_any_element()
            }
            CollectionPhase::Shelves if self.is_empty() => self
                .website_offer(
                    featured_icon(self.link.icon.as_deref()),
                    Copy::Featured::emptyTitle.to_owned(),
                    Copy::Featured::emptyMessage,
                    cx,
                )
                .into_any_element(),
            CollectionPhase::Shelves => {
                let rows: Vec<_> = self
                    .visible_shelves()
                    .map(|(index, shelf)| (index, shelf.clone()))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|(index, shelf)| self.shelf_row(index, &shelf, cx).into_any_element())
                    .collect();
                div()
                    .id("collection-shelves")
                    .test_support()
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        content_column()
                            .pt(px(spacing::S))
                            .pb(px(spacing::XXL))
                            .flex()
                            .flex_col()
                            .gap(px(spacing::XL))
                            .children(rows),
                    )
                    .into_any_element()
            }
        };
        div()
            .id("collection-screen")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .child(
                content_column().flex_none().pt(px(spacing::S)).pb(px(spacing::M)).child(
                    div()
                        .id("collection-title")
                        .test_support()
                        .role(Role::Heading)
                        .aria_label(SharedString::from(title.clone()))
                        .typography(Font::Title)
                        .text_color(theme::color(colors::TEXT_PRIMARY))
                        .truncate()
                        .child(title),
                ),
            )
            .child(div().flex_1().min_h_0().w_full().child(content))
    }
}

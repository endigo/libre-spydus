//! One search, paged, and the list that shows it (iOS `SearchResultsModel` + `SearchResultsList`):
//! the first page from `search_sorted` (or, for a list given by URL, `page(url)`), later pages from
//! `next_page_url` → `page` when the footer comes into view. Records are merged without duplicates,
//! each page URL is fetched at most once, and an answer for an older request is dropped. Shared by
//! the Search section and the Results routes.

use std::collections::HashSet;

use gpui_kit::{
    Context, CursorStyle, InteractiveElement, IntoElement, ParentElement, Render, Role, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Task, TestSupportExt, Window, div, point, prelude::FluentBuilder, px,
};
use librespydus_core::{SearchResults, SearchSort};

use crate::copy::Copy;
use crate::features::home::components::SkeletonBlock;
use crate::features::search::sentinel::sentinel;
use crate::state::formatting::{non_empty, result_count};
use crate::state::results::{SearchResultItem, merge, unrecognised_page};
use crate::state::{AppState, Route, UserFacingError};
use crate::theme::{self, Font, colors, cover, radius, spacing};
use crate::ui::{BookRowCard, Chip, EmptyStateView, ErrorStateView, FontExt, Lucide, content_column, icon, on_click};

/// How many format chips a row shows before "+N".
pub const MAX_CHIPS: usize = 4;

/// Where the first page comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultsQuery {
    Search {
        query: String,
        sort: SearchSort,
    },
    /// A Featured saved search or a collection section (§5.10): `page(url)`, no sort.
    Page {
        url: String,
        title: String,
    },
}

/// What a load is for: any change (query, sort, library, verification, a retry) starts afresh; the
/// same request again keeps the pages already loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultsRequest {
    pub query: ResultsQuery,
    pub origin: String,
    pub verification: u64,
    pub attempt: u64,
}

impl ResultsRequest {
    /// `query` for the current library.
    pub fn new(query: ResultsQuery, cx: &gpui_kit::App) -> Self {
        let state = AppState::global(cx).read(cx);
        Self { query, origin: state.backend().origin(), verification: state.verification_generation(), attempt: 0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Idle,
    Loading,
    Loaded,
    /// The OPAC's own "did not find any records" page.
    NoResults,
    Failed(UserFacingError),
}

pub struct ResultsList {
    phase: Phase,
    items: Vec<SearchResultItem>,
    /// `total_results` from the first page.
    total: Option<i64>,
    next_page_url: Option<String>,
    loading_more: bool,
    /// The last next-page failure: the footer offers Retry and automatic paging stops.
    load_more_error: Option<UserFacingError>,
    request: Option<ResultsRequest>,
    /// The request whose first page settled (results, no results or a failure).
    settled: Option<ResultsRequest>,
    /// Bumped by every new request: late answers for an older one are dropped.
    generation: u64,
    requested_pages: HashSet<String>,
    scroll: ScrollHandle,
    _load: Option<Task<()>>,
    _more: Option<Task<()>>,
    #[cfg(debug_assertions)]
    scrolled_to_end: bool,
}

impl ResultsList {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            phase: Phase::Idle,
            items: Vec::new(),
            total: None,
            next_page_url: None,
            loading_more: false,
            load_more_error: None,
            request: None,
            settled: None,
            generation: 0,
            requested_pages: HashSet::new(),
            scroll: ScrollHandle::new(),
            _load: None,
            _more: None,
            #[cfg(debug_assertions)]
            scrolled_to_end: false,
        }
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn items(&self) -> &[SearchResultItem] {
        &self.items
    }

    pub fn request(&self) -> Option<&ResultsRequest> {
        self.request.as_ref()
    }

    pub fn has_more(&self) -> bool {
        self.next_page_url.is_some()
    }

    /// Runs `request`'s first page, unless it's the request already shown.
    pub fn load(&mut self, request: ResultsRequest, cx: &mut Context<Self>) {
        if self.settled.as_ref() == Some(&request) && self.phase != Phase::Idle {
            return;
        }
        self.generation += 1;
        let generation = self.generation;
        self.request = Some(request.clone());
        self.settled = None;
        self.phase = Phase::Loading;
        self.items.clear();
        self.total = None;
        self.next_page_url = None;
        self.loading_more = false;
        self.load_more_error = None;
        self.requested_pages.clear();
        self._more = None;
        self.scroll.set_offset(point(px(0.), px(0.)));
        let state = AppState::global(cx);
        let backend = state.read(cx).backend();
        let fetch = match &request.query {
            ResultsQuery::Search { query, sort } => backend.search_sorted(query.clone(), *sort),
            ResultsQuery::Page { url, .. } => {
                self.requested_pages.insert(url.clone());
                backend.page(url.clone())
            }
        };
        cx.notify();
        self._load = Some(cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let _ = this.update(cx, |this, cx| {
                if generation != this.generation {
                    return;
                }
                this.settled = Some(request);
                match result {
                    Ok(page) => this.show_first_page(page),
                    Err(error) => this.phase = Phase::Failed(state.update(cx, |state, cx| state.report(&error, cx))),
                }
                cx.notify();
            });
        }));
    }

    /// The same request again, afresh (Retry).
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        if let Some(mut request) = self.request.clone() {
            request.attempt += 1;
            self.load(request, cx);
        }
    }

    fn show_first_page(&mut self, page: SearchResults) {
        if page.records.is_empty() {
            self.phase = if page.no_results { Phase::NoResults } else { Phase::Failed(unrecognised_page()) };
            return;
        }
        self.items = merge(&[], &page.records);
        self.total = page.total_results;
        self.next_page_url = non_empty(page.next_page_url.as_deref());
        self.phase = Phase::Loaded;
    }

    /// Fetches the next page, once. Ignored while a page is loading, after a page failed (use
    /// `retry_load_more`), and at the end of the results.
    pub fn load_more(&mut self, cx: &mut Context<Self>) {
        if self.load_more_error.is_none() {
            self.fetch_next_page(cx);
        }
    }

    pub fn retry_load_more(&mut self, cx: &mut Context<Self>) {
        self.load_more_error = None;
        self.fetch_next_page(cx);
    }

    fn fetch_next_page(&mut self, cx: &mut Context<Self>) {
        if self.phase != Phase::Loaded || self.loading_more {
            return;
        }
        let Some(url) = self.next_page_url.clone().filter(|url| !self.requested_pages.contains(url)) else { return };
        let generation = self.generation;
        self.loading_more = true;
        self.requested_pages.insert(url.clone());
        let state = AppState::global(cx);
        let fetch = state.read(cx).backend().page(url.clone());
        cx.notify();
        self._more = Some(cx.spawn(async move |this, cx| {
            let result = fetch.await;
            let _ = this.update(cx, |this, cx| {
                if generation != this.generation {
                    return;
                }
                this.loading_more = false;
                match result {
                    Ok(page) if page.records.is_empty() => {
                        if page.no_results {
                            this.next_page_url = None;
                        } else {
                            this.requested_pages.remove(&url);
                            this.load_more_error = Some(unrecognised_page());
                        }
                    }
                    Ok(page) => {
                        this.items = merge(&this.items, &page.records);
                        // A next link back to a page already fetched would loop forever.
                        this.next_page_url = non_empty(page.next_page_url.as_deref())
                            .filter(|next| !this.requested_pages.contains(next));
                    }
                    Err(error) => {
                        this.requested_pages.remove(&url);
                        this.load_more_error = Some(state.update(cx, |state, cx| state.report(&error, cx)));
                    }
                }
                cx.notify();
            });
        }));
    }

    fn no_results(&self) -> impl IntoElement {
        let view = match self.request.as_ref().map(|request| &request.query) {
            Some(ResultsQuery::Page { title, .. }) => {
                EmptyStateView::new(Lucide::Inbox, Copy::Search::listEmptyTitle(title))
                    .message(Copy::Search::listEmptyMessage)
            }
            Some(ResultsQuery::Search { query, .. }) => {
                EmptyStateView::new(Lucide::Search, Copy::Search::noResultsTitle(query))
                    .message(Copy::Search::noResultsMessage)
            }
            None => EmptyStateView::new(Lucide::Search, Copy::Search::noResultsMessage),
        };
        div().id("results-no-results").test_support().size_full().child(view)
    }

    fn row(&self, index: usize, item: &SearchResultItem) -> impl IntoElement {
        let mut chips: Vec<Chip> =
            item.formats.iter().take(MAX_CHIPS).map(|format| Chip::format(format, false)).collect();
        if item.formats.len() > MAX_CHIPS {
            chips.push(Chip::new(Copy::Search::moreFormats((item.formats.len() - MAX_CHIPS) as i64)));
        }
        let book = item.book.clone();
        let card = BookRowCard::new(SharedString::from(format!("result-{index}")), &item.book).chips(chips).on_click(
            on_click(move |_, cx| {
                let route = Route::Detail(book.clone());
                AppState::global(cx).update(cx, |state, cx| state.push(route, cx));
            }),
        );
        match item.book.year.clone() {
            Some(year) => card.meta(year, None),
            None => card,
        }
    }

    fn footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let footer = div().w_full().flex().flex_col().items_center().gap(px(spacing::S)).py(px(spacing::L));
        if let Some(error) = &self.load_more_error {
            let entity = cx.entity();
            return footer
                .child(
                    div()
                        .typography(Font::Caption)
                        .text_color(theme::color(colors::TEXT_SECONDARY))
                        .text_center()
                        .child(error.message.clone()),
                )
                .when(error.can_retry(), |footer| {
                    footer.child(
                        div()
                            .id("results-load-more-retry")
                            .test_support()
                            .role(Role::Button)
                            .aria_label(SharedString::from(Copy::Common::retry))
                            .aria_description(SharedString::from(Copy::Search::retryLoadingMoreHint))
                            .cursor(CursorStyle::PointingHand)
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(spacing::XS))
                            .typography(Font::BodyStrong)
                            .text_color(theme::color(colors::PRIMARY))
                            .on_click(move |_, _, cx| entity.update(cx, |list, cx| list.retry_load_more(cx)))
                            .child(icon(Lucide::RefreshCw, px(14.), theme::color(colors::PRIMARY)))
                            .child(Copy::Common::retry),
                    )
                })
                .into_any_element();
        }
        if self.has_more() {
            let weak = cx.entity().downgrade();
            return footer
                .child(
                    div()
                        .id("results-loading-more")
                        .test_support()
                        .role(Role::ProgressIndicator)
                        .aria_label(SharedString::from(Copy::Search::loadingMore))
                        .child(
                            gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::TEXT_SECONDARY)),
                        ),
                )
                .child(sentinel(move |visible, cx| {
                    if visible {
                        let _ = weak.update(cx, |list, cx| list.load_more(cx));
                    }
                }))
                .into_any_element();
        }
        footer
            .when(!self.items.is_empty(), |footer| {
                footer.child(
                    div()
                        .id("results-end")
                        .test_support()
                        .typography(Font::Caption)
                        .text_color(theme::color(colors::TEXT_MUTED))
                        .child(Copy::Search::endOfResults),
                )
            })
            .into_any_element()
    }

    #[cfg(debug_assertions)]
    fn apply_debug_hooks(&mut self) {
        // Evidence hook: `LS_RESULTS_SCROLL_TO_END=1` scrolls to the paging footer once.
        if !self.scrolled_to_end
            && self.phase == Phase::Loaded
            && std::env::var_os("LS_RESULTS_SCROLL_TO_END").is_some()
        {
            self.scrolled_to_end = true;
            self.scroll.scroll_to_bottom();
        }
    }
}

/// Placeholder rows while the first page loads.
pub fn skeleton_rows(rows: usize) -> impl IntoElement {
    let cover_height = cover::height(cover::SMALL_WIDTH);
    div()
        .id("results-loading")
        .test_support()
        .role(Role::ProgressIndicator)
        .aria_label(SharedString::from(Copy::Search::loadingResults))
        .size_full()
        .overflow_hidden()
        .child(content_column().pt(px(spacing::S)).flex().flex_col().gap(px(spacing::M)).children((0..rows).map(
            |_| {
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(spacing::L))
                    .p(px(spacing::L))
                    .rounded(px(radius::CARD))
                    .bg(theme::color(colors::SURFACE))
                    .child(SkeletonBlock::new(px(cover::SMALL_WIDTH), px(cover_height)).radius(radius::COVER))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(spacing::S))
                            .child(SkeletonBlock::fill(px(14.)))
                            .child(SkeletonBlock::new(px(140.), px(12.)))
                            .child(div().pt(px(spacing::XS)).child(SkeletonBlock::new(px(90.), px(18.)).radius(9.))),
                    )
            },
        )))
}

impl Render for ResultsList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(debug_assertions)]
        self.apply_debug_hooks();
        match &self.phase {
            Phase::Idle | Phase::Loading => skeleton_rows(5).into_any_element(),
            Phase::NoResults => self.no_results().into_any_element(),
            Phase::Failed(error) => {
                let entity = cx.entity();
                div()
                    .id("results-error-state")
                    .test_support()
                    .size_full()
                    .child(
                        ErrorStateView::new("results-error", error)
                            .retry(on_click(move |_, cx| entity.update(cx, |list, cx| list.retry(cx)))),
                    )
                    .into_any_element()
            }
            Phase::Loaded => div()
                .id("results-list")
                .test_support()
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(
                    content_column()
                        .pb(px(spacing::XXL))
                        .flex()
                        .flex_col()
                        .gap(px(spacing::M))
                        .children(result_count(self.total).map(|count| {
                            div()
                                .id("results-count")
                                .test_support()
                                .role(Role::Heading)
                                .aria_label(SharedString::from(count.clone()))
                                .typography(Font::Caption)
                                .text_color(theme::color(colors::TEXT_SECONDARY))
                                .child(count)
                        }))
                        .children(self.items.iter().enumerate().map(|(index, item)| self.row(index, item)))
                        .child(self.footer(cx)),
                )
                .into_any_element(),
        }
    }
}

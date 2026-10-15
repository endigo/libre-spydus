//! Search and results lists (D3; docs/desktop-app.md §6, native-apps.md §6 "Search"; iOS
//! `SearchView`, `SearchResultsView`, `ResultsPageView`).
//!
//! - `SearchScreen`, the Search section's root: the field (focused when Home or ⌘F hands over),
//!   Relevance / Newest, and the results inline; recent searches while the field is empty.
//!   Searches run on Return only.
//! - `ResultsScreen`, the `Results` and `ResultsPage` routes: the same list titled with the route,
//!   with the sort control for a query and none for a list given by URL.

pub mod results;
pub(crate) mod sentinel;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{
    AppContext, Context, CursorStyle, Entity, InteractiveElement, IntoElement, ParentElement, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, Subscription, TestSupportExt, Window, div,
    prelude::FluentBuilder, px,
};
use librespydus_core::SearchSort;

use crate::copy::Copy;
use crate::state::route::sort_label;
use crate::state::{AppState, ResultsSource};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{
    CardAction, EmptyStateView, FontExt, Lucide, PAGE_GAP, SearchField, SectionHeader, icon, on_click, page_body,
    page_header, page_title, with_context_menu,
};

use self::results::{ResultsList, ResultsQuery, ResultsRequest};

/// (origin, verification): what a public list depends on. Signing in doesn't reload it.
fn public_key(cx: &gpui_kit::App) -> (String, u64) {
    let state = AppState::global(cx).read(cx);
    (state.backend().origin(), state.verification_generation())
}

/// The Relevance / Newest control. Ids `{prefix}-relevance`, `{prefix}-newest`.
fn sort_control(
    prefix: &'static str,
    selected: SearchSort,
    on_change: impl Fn(SearchSort, &mut Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    let on_change = std::rc::Rc::new(on_change);
    div()
        .id(prefix)
        .test_support()
        .role(Role::RadioGroup)
        .aria_label(SharedString::from(Copy::Search::sortResults))
        .flex_none()
        .flex()
        .flex_row()
        .p(px(3.))
        .gap(px(2.))
        .rounded(px(radius::PILL))
        .bg(theme::color(colors::SURFACE))
        .children([(SearchSort::Relevance, "relevance"), (SearchSort::Newest, "newest")].into_iter().map(
            |(sort, suffix)| {
                let chosen = sort == selected;
                let on_change = on_change.clone();
                div()
                    .id(SharedString::from(format!("{prefix}-{suffix}")))
                    .test_support()
                    .role(Role::RadioButton)
                    .aria_label(SharedString::from(sort_label(sort)))
                    .aria_selected(chosen)
                    .cursor(CursorStyle::PointingHand)
                    .px(px(spacing::L))
                    .py(px(spacing::XS + 2.))
                    .rounded(px(radius::PILL))
                    .typography(if chosen { Font::BodyStrong } else { Font::Body })
                    .text_color(theme::color(if chosen { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }))
                    .when(chosen, |segment| segment.bg(theme::color(colors::SURFACE_ELEVATED)))
                    .when(!chosen, |segment| {
                        segment
                            .hover(|style| style.text_color(theme::color(colors::TEXT_PRIMARY)))
                            .on_click(move |_, window, cx| on_change(sort, window, cx))
                    })
                    .child(sort_label(sort))
            },
        ))
}

// ---- Search ----

/// The Search section's root.
pub struct SearchScreen {
    field: Entity<InputState>,
    /// The query whose results are shown (set on Return); `None`: recent searches.
    submitted: Option<String>,
    sort: SearchSort,
    results: Entity<ResultsList>,
    loaded_key: (String, u64),
    _subscriptions: Vec<Subscription>,
}

impl SearchScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = cx.new(|cx| InputState::new(window, cx).placeholder(Copy::Search::placeholder));
        let results = cx.new(ResultsList::new);
        let state = AppState::global(cx);
        let subscriptions = vec![
            cx.subscribe_in(&field, window, |this, field, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.submit(window, cx),
                // Clearing the field ends the search: recent searches show until the next Return.
                InputEvent::Change if field.read(cx).value().trim().is_empty() && this.submitted.take().is_some() => {
                    cx.notify();
                }
                _ => {}
            }),
            cx.observe_in(&state, window, |this, _, window, cx| this.state_changed(window, cx)),
        ];
        let mut screen = Self {
            field,
            submitted: None,
            sort: SearchSort::Relevance,
            results,
            loaded_key: public_key(cx),
            _subscriptions: subscriptions,
        };
        // Opened from Home or ⌘F: run its query, or just focus the field.
        if !screen.take_pending_search(window, cx) {
            screen.focus_field(window, cx);
        }
        screen
    }

    /// The field's current text (tests).
    pub fn text(&self, cx: &gpui_kit::App) -> String {
        self.field.read(cx).value().to_string()
    }

    pub fn results(&self) -> &Entity<ResultsList> {
        &self.results
    }

    fn state_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.take_pending_search(window, cx);
        let key = public_key(cx);
        if key != self.loaded_key {
            // Another library or a verification: search again (recent searches belong to the app).
            self.loaded_key = key;
            if self.submitted.is_some() {
                self.run(cx);
            }
        }
    }

    /// Home's search field and ⌘F hand over a query (or `""`: focus the field).
    fn take_pending_search(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(pending) = AppState::global(cx).update(cx, |state, _| state.take_pending_search()) else {
            return false;
        };
        let query = pending.trim().to_owned();
        if query.is_empty() {
            self.focus_field(window, cx);
        } else {
            self.field.update(cx, |field, cx| field.set_value(query, window, cx));
            self.submit(window, cx);
        }
        true
    }

    /// Focuses the field once the screen is laid out (it may be new in this frame).
    fn focus_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.defer_in(window, |this, window, cx| this.field.update(cx, |field, cx| field.focus(window, cx)));
    }

    /// Return: searches the field's words, remembered as a recent search.
    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.field.read(cx).value().to_string();
        let query = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if query.is_empty() {
            return;
        }
        if query != text {
            self.field.update(cx, |field, cx| field.set_value(query.clone(), window, cx));
        }
        AppState::global(cx).update(cx, |state, cx| state.add_recent_search(&query, cx));
        self.submitted = Some(query);
        self.run_fresh(cx);
    }

    /// A recent search clicked.
    fn search_again(&mut self, query: String, window: &mut Window, cx: &mut Context<Self>) {
        self.field.update(cx, |field, cx| field.set_value(query, window, cx));
        self.submit(window, cx);
    }

    fn set_sort(&mut self, sort: SearchSort, cx: &mut Context<Self>) {
        if sort != self.sort {
            self.sort = sort;
            self.run(cx);
        }
    }

    fn request(&self, cx: &gpui_kit::App) -> Option<ResultsRequest> {
        let query = self.submitted.clone()?;
        Some(ResultsRequest::new(ResultsQuery::Search { query, sort: self.sort }, cx))
    }

    /// Shows the current request's results (kept when they're already shown).
    fn run(&mut self, cx: &mut Context<Self>) {
        if let Some(request) = self.request(cx) {
            self.results.update(cx, |results, cx| results.load(request, cx));
        }
        cx.notify();
    }

    /// Return always searches again, even for the query on screen.
    fn run_fresh(&mut self, cx: &mut Context<Self>) {
        if let Some(mut request) = self.request(cx) {
            let attempt = self.results.read(cx).request().map_or(0, |shown| shown.attempt + 1);
            request.attempt = attempt;
            self.results.update(cx, |results, cx| results.load(request, cx));
        }
        cx.notify();
    }

    fn recent_searches(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::global(cx).read(cx);
        let recents = state.recent_searches().to_vec();
        let library = state.library_name();
        if recents.is_empty() {
            return div()
                .id("search-intro")
                .test_support()
                .size_full()
                .child(
                    EmptyStateView::new(Lucide::BookOpenText, Copy::Search::introTitle(&library))
                        .message(Copy::Search::introMessage),
                )
                .into_any_element();
        }
        let entity = cx.entity();
        div()
            .id("recent-searches")
            .test_support()
            .size_full()
            .overflow_y_scroll()
            .restrict_scroll_to_axis()
            .child(
                page_body()
                    .flex()
                    .flex_col()
                    .child(div().pb(px(spacing::S)).child(
                        SectionHeader::new("recent-searches-clear", Copy::Search::recentSearches).action(
                            Some(Copy::Common::clear.into()),
                            on_click(|_, cx| {
                                AppState::global(cx).update(cx, |state, cx| state.clear_recent_searches(cx))
                            }),
                        ),
                    ))
                    .children(recents.into_iter().enumerate().map(|(index, query)| {
                        let entity = entity.clone();
                        let run = query.clone();
                        let removed = query.clone();
                        let row = div()
                            .id(SharedString::from(format!("recent-search-{index}")))
                            .test_support()
                            .role(Role::Button)
                            .aria_label(SharedString::from(query.clone()))
                            .aria_description(SharedString::from(Copy::Search::recentSearchHint))
                            .cursor(CursorStyle::PointingHand)
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(spacing::M))
                            .px(px(spacing::S))
                            .py(px(spacing::M))
                            .rounded(px(radius::FIELD))
                            .border_b_1()
                            .border_color(theme::color(colors::DIVIDER))
                            .hover(|style| style.bg(theme::color(colors::SURFACE)))
                            .on_click(move |_, window, cx| {
                                let query = run.clone();
                                entity.update(cx, |screen, cx| screen.search_again(query, window, cx));
                            })
                            .child(icon(Lucide::Search, px(16.), theme::color(colors::TEXT_MUTED)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .typography(Font::Body)
                                    .text_color(theme::color(colors::TEXT_PRIMARY))
                                    .line_clamp(2)
                                    .text_ellipsis()
                                    .child(query),
                            )
                            .child(icon(Lucide::ChevronRight, px(14.), theme::color(colors::TEXT_MUTED)));
                        with_context_menu(
                            row,
                            vec![CardAction::new(Copy::Common::remove, move |_, cx| {
                                let query = removed.clone();
                                AppState::global(cx).update(cx, |state, cx| state.remove_recent_search(&query, cx));
                            })],
                        )
                    })),
            )
            .into_any_element()
    }
}

impl Render for SearchScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let showing_results = self.submitted.is_some();
        let header = page_header()
            .flex()
            .flex_col()
            .gap(px(PAGE_GAP))
            .child(page_title("search-title", Copy::Search::title))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::M))
                    .child(div().flex_1().child(SearchField::new(&self.field).id("search-field")))
                    .when(showing_results, |row| {
                        row.child(sort_control("search-sort", self.sort, move |sort, _, cx| {
                            entity.update(cx, |screen, cx| screen.set_sort(sort, cx))
                        }))
                    }),
            );
        div().id("search-screen").test_support().size_full().flex().flex_col().child(header).child(
            div().flex_1().min_h_0().w_full().map(|body| {
                if showing_results { body.child(self.results.clone()) } else { body.child(self.recent_searches(cx)) }
            }),
        )
    }
}

// ---- Results routes ----

/// A results list from a query (`Route::Results`) or a URL (`Route::ResultsPage`).
pub struct ResultsScreen {
    source: ResultsSource,
    sort: SearchSort,
    results: Entity<ResultsList>,
    loaded_key: (String, u64),
    _observe: Subscription,
}

impl ResultsScreen {
    pub fn new(source: ResultsSource, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sort = match &source {
            ResultsSource::Query { sort, .. } => *sort,
            ResultsSource::Page { .. } => SearchSort::Relevance,
        };
        let state = AppState::global(cx);
        let observe = cx.observe_in(&state, window, |this, _, _, cx| {
            let key = public_key(cx);
            if key != this.loaded_key {
                this.loaded_key = key;
                this.run(cx);
            }
        });
        let mut screen =
            Self { source, sort, results: cx.new(ResultsList::new), loaded_key: public_key(cx), _observe: observe };
        screen.run(cx);
        screen
    }

    pub fn source(&self) -> &ResultsSource {
        &self.source
    }

    pub fn results(&self) -> &Entity<ResultsList> {
        &self.results
    }

    fn run(&mut self, cx: &mut Context<Self>) {
        let query = match &self.source {
            ResultsSource::Query { query, .. } => ResultsQuery::Search { query: query.clone(), sort: self.sort },
            ResultsSource::Page { url, title } => ResultsQuery::Page { url: url.clone(), title: title.clone() },
        };
        let request = ResultsRequest::new(query, cx);
        self.results.update(cx, |results, cx| results.load(request, cx));
        cx.notify();
    }
}

impl Render for ResultsScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let title = self.source.title().to_owned();
        let sortable = matches!(self.source, ResultsSource::Query { .. });
        div()
            .id("results-screen")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .child(
                page_header()
                    .id("results-header")
                    .test_support()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::L))
                    .child(
                        div()
                            .id("results-title")
                            .test_support()
                            .role(Role::Heading)
                            .aria_label(SharedString::from(title.clone()))
                            .flex_1()
                            .min_w_0()
                            .typography(Font::Title)
                            .text_color(theme::color(colors::TEXT_PRIMARY))
                            .truncate()
                            .child(title),
                    )
                    .when(sortable, |row| {
                        row.child(sort_control("results-sort", self.sort, move |sort, _, cx| {
                            entity.update(cx, |screen, cx| {
                                screen.sort = sort;
                                screen.run(cx);
                            })
                        }))
                    }),
            )
            .child(div().flex_1().min_h_0().w_full().child(self.results.clone()))
    }
}

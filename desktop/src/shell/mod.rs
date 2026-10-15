//! The root view (docs/desktop-app.md §5.5): onboarding while no library is selected; otherwise
//! the sidebar, a toolbar with Back, and the current section's screen. Each section keeps its own
//! stack, and the shell keeps one screen entity per stack entry, so going back doesn't reload.
//! Dialogs AppState asks for (sign in, the library picker) open through D2's entry points, and
//! AppState's notices show as toasts.

pub mod actions;
pub mod sidebar;
pub mod toolbar;

use std::collections::HashMap;

use gpui_kit::{
    AnyView, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, Window, div,
};

use crate::features::{
    detail::DetailScreen,
    featured::CollectionScreen,
    home::HomeScreen,
    my_picks::MyPicksScreen,
    onboarding::{self, LibraryPickerScreen},
    profile::ProfileScreen,
    search::{ResultsScreen, SearchScreen},
    your_books::YourBooksScreen,
};
use crate::state::{AppState, ResultsSource, Route, Section};
use crate::theme::{self, colors};
use crate::ui::toast;

/// One section's screens: its root and one per pushed route.
#[derive(Default)]
struct SectionScreens {
    root: Option<AnyView>,
    pushed: Vec<(Route, AnyView)>,
}

pub struct RootView {
    state: Entity<AppState>,
    focus: FocusHandle,
    onboarding: Option<Entity<LibraryPickerScreen>>,
    screens: HashMap<Section, SectionScreens>,
    /// The origin the screens were made for: another library starts them afresh.
    origin: String,
    last_notice: u64,
    /// A debug screen shown instead of the section's (`--route gallery`).
    overlay: Option<AnyView>,
    _observe: Subscription,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let observe = cx.observe_in(&state, window, |this, _, window, cx| this.state_changed(window, cx));
        let origin = state.read(cx).backend().origin();
        let last_notice = state.read(cx).notice().map_or(0, |notice| notice.id);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        // My Picks' copy-once offer shows after a sign-in, wherever the reader is (§5.11).
        crate::features::my_picks::watch_picks_copy_offer(window, cx);
        // Wave 3 (docs/desktop-app.md §11): the human check, notifications, Touch ID.
        crate::features::verification::watch(window, cx);
        crate::features::alerts::start(window, cx);
        crate::features::biometrics::watch(window, cx);
        Self {
            state,
            focus,
            onboarding: None,
            screens: HashMap::new(),
            origin,
            last_notice,
            overlay: None,
            _observe: observe,
        }
    }

    /// Shows `view` over the content area until a section is clicked (debug routes).
    pub fn show_overlay(&mut self, view: AnyView, cx: &mut Context<Self>) {
        self.overlay = Some(view);
        cx.notify();
    }

    fn state_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (sign_in, picker, notice, origin) = self.state.update(cx, |state, _| {
            (
                state.take_sign_in_request(),
                state.take_library_picker_request(),
                state.notice().cloned(),
                state.backend().origin(),
            )
        });
        if origin != self.origin {
            self.origin = origin;
            self.screens.clear();
        }
        if let Some(notice) = notice
            && notice.id != self.last_notice
        {
            self.last_notice = notice.id;
            toast::show(window, cx, notice.message, notice.tone);
        }
        if picker {
            onboarding::open_library_picker(window, cx);
        }
        if sign_in {
            onboarding::open_sign_in(window, cx);
        }
        cx.notify();
    }

    /// The screen on top of `section`'s stack, making the entities the stack needs and dropping
    /// those for popped routes.
    fn current_screen(&mut self, section: Section, window: &mut Window, cx: &mut Context<Self>) -> AnyView {
        let routes: Vec<Route> = self.state.read(cx).stack(section).to_vec();
        let screens = self.screens.entry(section).or_default();
        let kept = screens.pushed.iter().zip(&routes).take_while(|((made, _), wanted)| made == *wanted).count();
        screens.pushed.truncate(kept);
        for route in &routes[kept..] {
            let view = route_screen(route, window, cx);
            screens.pushed.push((route.clone(), view));
        }
        if let Some((_, view)) = screens.pushed.last() {
            return view.clone();
        }
        screens.root.get_or_insert_with(|| root_screen(section, window, cx)).clone()
    }
}

fn root_screen(section: Section, window: &mut Window, cx: &mut gpui_kit::App) -> AnyView {
    match section {
        Section::Home => cx.new(|cx| HomeScreen::new(window, cx)).into(),
        Section::YourBooks => cx.new(|cx| YourBooksScreen::new(window, cx)).into(),
        Section::Search => cx.new(|cx| SearchScreen::new(window, cx)).into(),
        Section::MyPicks => cx.new(|cx| MyPicksScreen::new(window, cx)).into(),
        Section::Profile => cx.new(|cx| ProfileScreen::new(window, cx)).into(),
    }
}

fn route_screen(route: &Route, window: &mut Window, cx: &mut gpui_kit::App) -> AnyView {
    match route.clone() {
        Route::Detail(book) => cx.new(|cx| DetailScreen::new(book, window, cx)).into(),
        Route::Results { query, sort, title } => {
            cx.new(|cx| ResultsScreen::new(ResultsSource::Query { query, sort, title }, window, cx)).into()
        }
        Route::Collection(link) => cx.new(|cx| CollectionScreen::new(link, window, cx)).into(),
        Route::ResultsPage { url, title } => {
            cx.new(|cx| ResultsScreen::new(ResultsSource::Page { url, title }, window, cx)).into()
        }
    }
}

impl Focusable for RootView {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .id("root")
            .key_context(actions::CONTEXT)
            .track_focus(&self.focus)
            .on_action(|_: &actions::Minimize, window, _| window.minimize_window())
            .on_action(|_: &actions::Zoom, window, _| window.zoom_window())
            .on_action(|_: &actions::CloseWindow, window, _| window.remove_window())
            .size_full()
            .flex()
            .flex_row()
            .bg(theme::color(colors::BACKGROUND))
            .text_color(theme::color(colors::TEXT_PRIMARY));

        if self.state.read(cx).needs_onboarding() {
            let picker =
                self.onboarding.get_or_insert_with(|| cx.new(|cx| LibraryPickerScreen::new(window, cx))).clone();
            return root.child(div().size_full().child(picker));
        }
        self.onboarding = None;

        let section = self.state.read(cx).section();
        let content = match &self.overlay {
            Some(overlay) => overlay.clone(),
            None => self.current_screen(section, window, cx),
        };
        let view = cx.entity();
        root.child(
            div()
                .on_mouse_down(gpui_kit::MouseButton::Left, move |_, _, cx| {
                    // A section click leaves a debug overlay.
                    view.update(cx, |root, cx| {
                        if root.overlay.is_some() {
                            root.overlay = None;
                            cx.notify();
                        }
                    })
                })
                .h_full()
                .child(sidebar::Sidebar),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .child(toolbar::Toolbar)
                .child(div().id("content").flex_1().min_h_0().w_full().child(content)),
        )
    }
}

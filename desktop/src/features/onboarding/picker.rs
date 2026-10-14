//! The library picker (docs/native-apps.md §6; iOS `LibraryPickerView.swift`): a search field and
//! the directory grouped by region. The full window on first launch (on navy); the Change library
//! dialog afterwards (on white). Rows are drawn only while visible (a virtual list), so only the
//! logos on screen load.

use std::rc::Rc;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{VirtualListScrollHandle, WindowExt, v_virtual_list};
use gpui_kit::{
    AnyElement, App, AppContext, Context, CursorStyle, ElementId, Entity, Hsla, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, Role, ScrollStrategy, SharedString, Size, StatefulInteractiveElement, Styled,
    Subscription, TestSupportExt, Window, div, prelude::FluentBuilder, px, size,
};
use librespydus_core::LibraryInfo;

use super::directory::{self, Region};
use super::logo::LibraryLogo;
use super::sheet;
use super::store::DirectoryStore;
use crate::copy::Copy;
use crate::state::{AppState, Section};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{
    Banner, BannerTone, EmptyStateView, ErrorStateView, FontExt, LoadingView, Lucide, SearchField, icon, on_click,
};

const HEADER_HEIGHT: f32 = 44.;
const ROW_HEIGHT: f32 = 76.;
const NOTE_HEIGHT: f32 = 20.;
const ROW_GAP: f32 = 8.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    /// The whole window, on navy, before a library is chosen.
    Onboarding,
    /// The Change library dialog, on white.
    Dialog,
}

#[derive(Clone)]
enum Item {
    Header { region: Region, count: usize },
    Row { library: LibraryInfo, region: Region },
}

impl Item {
    fn height(&self) -> f32 {
        match self {
            Self::Header { .. } => HEADER_HEIGHT,
            Self::Row { region, .. } => ROW_HEIGHT + ROW_GAP + if region.caveat().is_some() { NOTE_HEIGHT } else { 0. },
        }
    }
}

/// The colours for navy (onboarding) or white (dialog).
struct Palette {
    text: Hsla,
    secondary: Hsla,
    muted: Hsla,
    card: Hsla,
    card_hover: Hsla,
    current: Hsla,
    check: Hsla,
    note: Hsla,
    header_bg: Hsla,
}

impl Palette {
    fn of(mode: PickerMode) -> Self {
        match mode {
            PickerMode::Onboarding => Self {
                text: theme::color(colors::TEXT_PRIMARY),
                secondary: theme::color(colors::TEXT_SECONDARY),
                muted: theme::color(colors::TEXT_MUTED),
                card: theme::color(colors::SURFACE),
                card_hover: theme::color(colors::SURFACE_ELEVATED),
                current: theme::color(colors::SURFACE_ELEVATED),
                check: theme::color(colors::PRIMARY),
                note: theme::color(colors::WARNING),
                header_bg: theme::color(colors::BACKGROUND),
            },
            PickerMode::Dialog => Self {
                text: theme::color(colors::ON_SHEET),
                secondary: theme::color(colors::ON_SHEET_SECONDARY),
                muted: theme::color(colors::ON_SHEET_SECONDARY),
                card: theme::color(colors::SHEET),
                card_hover: theme::color(colors::CHIP_ON_SHEET),
                current: theme::color(colors::CHIP_ON_SHEET),
                check: theme::color(colors::ACCENT),
                note: theme::color(colors::ON_SHEET_SECONDARY),
                header_bg: theme::color(colors::SHEET),
            },
        }
    }
}

pub struct LibraryPicker {
    mode: PickerMode,
    query: Entity<InputState>,
    directory: Entity<DirectoryStore>,
    scroll: VirtualListScrollHandle,
    items: Rc<Vec<Item>>,
    sizes: Rc<Vec<Size<Pixels>>>,
    /// The query and directory generation `items` were built for.
    built_for: Option<(String, u64)>,
    /// The dialog starts at the current library, once.
    scrolled_to_current: bool,
    _subscriptions: Vec<Subscription>,
}

impl LibraryPicker {
    pub fn new(mode: PickerMode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| {
            let input = InputState::new(window, cx).placeholder(Copy::Onboarding::searchPlaceholder);
            #[cfg(feature = "evidence")]
            let input = match super::evidence_step("onboarding").as_deref() {
                // copy-exempt: a screenshot's search text
                Some("filtered") => input.default_value("zealand"),
                // copy-exempt: a screenshot's search text
                Some("filtered-uk") => input.default_value("kingdom"),
                _ => input,
            };
            input
        });
        let directory = DirectoryStore::global(cx);
        let state = AppState::global(cx);
        let subscriptions = vec![
            cx.subscribe_in(&query, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.scroll.scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify();
                }
            }),
            cx.observe(&directory, |_, _, cx| cx.notify()),
            cx.observe(&state, |_, _, cx| cx.notify()),
        ];
        directory.update(cx, |directory, cx| directory.load(cx));
        Self {
            mode,
            query,
            directory,
            scroll: VirtualListScrollHandle::new(),
            items: Rc::new(Vec::new()),
            sizes: Rc::new(Vec::new()),
            built_for: None,
            scrolled_to_current: false,
            _subscriptions: subscriptions,
        }
    }

    pub fn query(&self) -> &Entity<InputState> {
        &self.query
    }

    fn rebuild(&mut self, cx: &App) {
        let query = self.query.read(cx).value().to_string();
        let directory = self.directory.read(cx);
        let key = (query.clone(), directory.generation());
        if self.built_for.as_ref() == Some(&key) {
            return;
        }
        let mut items = Vec::new();
        for section in directory::sections(directory.libraries(), &query) {
            items.push(Item::Header { region: section.region, count: section.libraries.len() });
            items.extend(section.libraries.into_iter().map(|library| Item::Row { library, region: section.region }));
        }
        self.sizes = Rc::new(items.iter().map(|item| size(px(0.), px(item.height()))).collect());
        self.items = Rc::new(items);
        self.built_for = Some(key);
    }

    fn choose(&mut self, library: LibraryInfo, window: &mut Window, cx: &mut Context<Self>) {
        AppState::global(cx).update(cx, |state, cx| {
            let changed = state.library().map(|current| current.origin.as_str()) != Some(library.origin.as_str());
            state.select_library(library, cx);
            if changed {
                state.select(Section::Home, cx);
            }
        });
        if self.mode == PickerMode::Dialog {
            window.close_dialog(cx);
        }
    }

    fn render_item(&self, item: &Item, current: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let palette = Palette::of(self.mode);
        match item {
            Item::Header { region, count } => div()
                .h(px(HEADER_HEIGHT))
                .w_full()
                .flex()
                .flex_row()
                .items_end()
                .justify_between()
                .pb(px(spacing::S))
                .bg(palette.header_bg)
                .child(
                    div()
                        .id(SharedString::from(format!("region-{}", region.code())))
                        .test_support()
                        .role(Role::Heading)
                        .aria_label(SharedString::from(region.title()))
                        .typography(Font::Section)
                        .text_color(palette.text)
                        .child(region.title()),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("region-{}-count", region.code())))
                        .test_support()
                        .role(Role::Label)
                        .aria_label(SharedString::from(Copy::Onboarding::libraries(*count as i64)))
                        .typography(Font::Caption)
                        .text_color(palette.muted)
                        .child(count.to_string()),
                )
                .into_any_element(),
            Item::Row { library, region } => {
                let is_current = current == Some(library.origin.as_str());
                let host = directory::host(&library.origin);
                let chosen = library.clone();
                let row_height = item.height() - ROW_GAP;
                let country = library.country.clone().filter(|country| !country.trim().is_empty());
                div()
                    .h(px(item.height()))
                    .w_full()
                    .pb(px(ROW_GAP))
                    .child(
                        div()
                            .id(SharedString::from(format!("library-{host}")))
                            .test_support()
                            .role(Role::Button)
                            .aria_label(SharedString::from(library.name.clone()))
                            .aria_description(SharedString::from(if is_current {
                                Copy::Onboarding::currentLibrary
                            } else {
                                Copy::Onboarding::selectHint
                            }))
                            .aria_selected(is_current)
                            .cursor(CursorStyle::PointingHand)
                            .h(px(row_height))
                            .w_full()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(spacing::M))
                            .px(px(spacing::M))
                            .rounded(px(radius::CARD))
                            .bg(if is_current { palette.current } else { palette.card })
                            .when(self.mode == PickerMode::Dialog && !is_current, |row| {
                                row.border_1().border_color(theme::color(colors::CHIP_ON_SHEET))
                            })
                            .when(is_current, |row| row.border_1().border_color(palette.check.opacity(0.7)))
                            .hover(|style| style.bg(palette.card_hover))
                            .on_click(cx.listener(move |this, _, window, cx| this.choose(chosen.clone(), window, cx)))
                            .child(LibraryLogo::library(SharedString::from(format!("logo-{host}")), library))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .typography(Font::BodyStrong)
                                            .text_color(palette.text)
                                            .truncate()
                                            .child(library.name.clone()),
                                    )
                                    .when_some(country, |column, country| {
                                        column.child(
                                            div()
                                                .typography(Font::Caption)
                                                .text_color(palette.secondary)
                                                .child(country),
                                        )
                                    })
                                    .when_some(region.caveat(), |column, caveat| {
                                        let glyph =
                                            if *region == Region::Uk { Lucide::ShieldCheck } else { Lucide::WifiOff };
                                        column.child(
                                            div()
                                                .pt(px(2.))
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(spacing::XS))
                                                .typography(Font::Caption)
                                                .text_color(palette.note)
                                                .child(icon(glyph, px(12.), theme::color(colors::WARNING)))
                                                .child(caveat),
                                        )
                                    }),
                            )
                            .when(is_current, |row| row.child(icon(Lucide::CircleCheck, px(22.), palette.check))),
                    )
                    .into_any_element()
            }
        }
    }

    fn render_list(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let palette = Palette::of(self.mode);
        let directory = self.directory.read(cx);
        let retry = {
            let directory = self.directory.clone();
            on_click(move |_, cx| directory.update(cx, |directory, cx| directory.refresh(cx)))
        };
        if directory.is_loading() {
            return match self.mode {
                PickerMode::Onboarding => LoadingView::new().message(Copy::Onboarding::loading).into_any_element(),
                PickerMode::Dialog => sheet::sheet_loading("picker-loading", Copy::Onboarding::loading),
            };
        }
        if let Some(error) = directory.load_error().cloned() {
            return match self.mode {
                PickerMode::Onboarding => ErrorStateView::new("picker-error", &error).retry(retry).into_any_element(),
                PickerMode::Dialog => sheet::sheet_error("picker-error", &error, retry),
            };
        }
        if directory.libraries().is_empty() {
            return match self.mode {
                PickerMode::Onboarding => EmptyStateView::new(Lucide::Library, Copy::Onboarding::emptyTitle)
                    .message(Copy::Onboarding::emptyMessage)
                    .action("picker-empty-retry", Copy::Common::retry, retry)
                    .into_any_element(),
                PickerMode::Dialog => sheet::sheet_state(
                    "picker-empty",
                    Lucide::Library,
                    Copy::Onboarding::emptyTitle,
                    Some(Copy::Onboarding::emptyMessage.into()),
                    Some(("picker-empty-retry".into(), Copy::Common::retry.into(), retry)),
                ),
            };
        }
        let refresh_error = directory.refresh_error().map(|error| Copy::Onboarding::savedList(&error.message));
        self.rebuild(cx);
        let current = AppState::global(cx).read(cx).library().map(|library| library.origin.clone());
        if self.mode == PickerMode::Dialog
            && !self.scrolled_to_current
            && let Some(origin) = &current
            && let Some(index) = self
                .items
                .iter()
                .position(|item| matches!(item, Item::Row { library, .. } if &library.origin == origin))
        {
            self.scrolled_to_current = true;
            self.scroll.scroll_to_item(index, ScrollStrategy::Center);
        }
        let list: AnyElement = if self.items.is_empty() {
            let query = self.query.read(cx).value().trim().to_owned();
            let title = Copy::Onboarding::noMatchesTitle(&query);
            match self.mode {
                PickerMode::Onboarding => EmptyStateView::new(Lucide::Search, title)
                    .message(Copy::Onboarding::noMatchesMessage)
                    .into_any_element(),
                PickerMode::Dialog => sheet::sheet_state(
                    "picker-no-matches",
                    Lucide::Search,
                    title,
                    Some(Copy::Onboarding::noMatchesMessage.into()),
                    None,
                ),
            }
        } else {
            let items = self.items.clone();
            v_virtual_list(cx.entity(), "library-list", self.sizes.clone(), move |this, range, _, cx| {
                range.map(|index| this.render_item(&items[index], current.as_deref(), cx)).collect::<Vec<_>>()
            })
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element()
        };
        div()
            .id("picker-results")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(spacing::S))
            .when_some(refresh_error, |column, message| {
                column.child(match self.mode {
                    PickerMode::Onboarding => Banner::new(message, BannerTone::Error).into_any_element(),
                    PickerMode::Dialog => sheet::error_note("picker-refresh-error", message).into_any_element(),
                })
            })
            .child(div().flex_1().min_h_0().w_full().text_color(palette.text).child(list))
            .into_any_element()
    }
}

impl Render for LibraryPicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let search: AnyElement =
            match self.mode {
                PickerMode::Onboarding => SearchField::new(&self.query).into_any_element(),
                PickerMode::Dialog => sheet::field_box()
                    .child(
                        sheet::sheet_input(&self.query, Copy::Onboarding::searchPlaceholder)
                            .cleanable(true)
                            .prefix(icon(Lucide::Search, px(16.), theme::color(colors::ON_SHEET_SECONDARY))),
                    )
                    .into_any_element(),
            };
        div()
            .id(ElementId::from(SharedString::from(match self.mode {
                PickerMode::Onboarding => "library-picker",
                PickerMode::Dialog => "library-picker-dialog",
            })))
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(spacing::M))
            .child(div().id("picker-search").test_support().w_full().child(search))
            .child(div().flex_1().min_h_0().w_full().child(self.render_list(cx)))
    }
}

//! Buttons, fields, tiles and chips (iOS `Controls.swift`). Every control has an `.id(..)` and a
//! role or accessibility label from `Copy` (docs/desktop-app.md §7.3).

use std::rc::Rc;

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::{
    App, ClickEvent, CursorStyle, ElementId, Entity, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    Role, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, linear_color_stop,
    linear_gradient, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::FormatRef;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::icons::{Lucide, icon};
use crate::ui::style::{FontExt, glow};

/// A click handler.
pub type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub fn on_click(handler: impl Fn(&mut Window, &mut App) + 'static) -> OnClick {
    Rc::new(handler)
}

fn click(handler: &Option<OnClick>) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let handler = handler.clone();
    move |_, window, cx| {
        if let Some(handler) = &handler {
            handler(window, cx);
        }
    }
}

/// "New arrivals ……… View All".
#[derive(IntoElement)]
pub struct SectionHeader {
    id: ElementId,
    title: SharedString,
    action_title: SharedString,
    on_sheet: bool,
    action: Option<OnClick>,
}

impl SectionHeader {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            action_title: Copy::Common::viewAll.into(),
            on_sheet: false,
            action: None,
        }
    }

    pub fn on_sheet(mut self, on_sheet: bool) -> Self {
        self.on_sheet = on_sheet;
        self
    }

    /// The View All button (or another action, labelled `title`).
    pub fn action(mut self, title: Option<SharedString>, action: OnClick) -> Self {
        if let Some(title) = title {
            self.action_title = title;
        }
        self.action = Some(action);
        self
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (title_color, action_color) = if self.on_sheet {
            (theme::color(colors::ON_SHEET), theme::color(colors::ON_SHEET_SECONDARY))
        } else {
            (theme::color(colors::TEXT_PRIMARY), theme::color(colors::TEXT_SECONDARY))
        };
        // "View All New arrivals"; any other action reads as shown.
        let label: SharedString = if self.action_title.as_ref() == Copy::Common::viewAll {
            Copy::A11y::viewAllSection(&self.title).into()
        } else {
            self.action_title.clone()
        };
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .w_full()
            .child(
                div()
                    .id(SharedString::from(format!("{}-title", self.id)))
                    .test_support()
                    .role(Role::Heading)
                    .aria_label(self.title.clone())
                    .typography(Font::Section)
                    .text_color(title_color)
                    .child(self.title.clone()),
            )
            .when_some(self.action.clone(), |row, _| {
                row.child(
                    div()
                        .id(self.id.clone())
                        .test_support()
                        .role(Role::Button)
                        .aria_label(label)
                        .cursor(CursorStyle::PointingHand)
                        .typography(Font::Caption)
                        .text_color(action_color)
                        .hover(|style| style.text_color(title_color))
                        .on_click(click(&self.action))
                        .child(self.action_title.clone()),
                )
            })
    }
}

/// The rounded search field on navy, around a gpui-kit `Input` (its `InputState` holds the text).
#[derive(IntoElement)]
pub struct SearchField {
    state: Entity<InputState>,
    id: Option<ElementId>,
}

impl SearchField {
    /// `state` from `cx.new(|cx| InputState::new(window, cx).placeholder(Copy::Search::placeholder))`.
    pub fn new(state: &Entity<InputState>) -> Self {
        Self { state: state.clone(), id: None }
    }

    /// The input's element id (tests, accessibility).
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl RenderOnce for SearchField {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .px(px(spacing::L))
            .min_h(px(52.))
            .flex()
            .items_center()
            .rounded(px(radius::FIELD))
            .bg(theme::color(colors::FIELD))
            .child(
                Input::new(&self.state)
                    .when_some(self.id, |input, id| input.id(id))
                    .appearance(false)
                    .cleanable(true)
                    .prefix(icon(Lucide::Search, px(16.), theme::color(colors::TEXT_MUTED)))
                    .text_size(Font::Body.size())
                    .text_color(theme::color(colors::TEXT_PRIMARY)),
            )
    }
}

/// Looks like `SearchField` but is a button (Home: click → Search).
#[derive(IntoElement)]
pub struct SearchFieldButton {
    id: ElementId,
    placeholder: SharedString,
    action: Option<OnClick>,
}

impl SearchFieldButton {
    pub fn new(id: impl Into<ElementId>, action: OnClick) -> Self {
        Self { id: id.into(), placeholder: Copy::Search::placeholder.into(), action: Some(action) }
    }
}

impl RenderOnce for SearchFieldButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .test_support()
            .role(Role::Button)
            .aria_label(SharedString::from(Copy::Home::searchField))
            .aria_description(SharedString::from(Copy::Home::searchFieldHint))
            .cursor(CursorStyle::PointingHand)
            .w_full()
            .min_h(px(52.))
            .px(px(spacing::L))
            .flex()
            .items_center()
            .gap(px(spacing::S))
            .rounded(px(radius::FIELD))
            .bg(theme::color(colors::FIELD))
            .text_color(theme::color(colors::TEXT_MUTED))
            .on_click(click(&self.action))
            .child(icon(Lucide::Search, px(16.), theme::color(colors::TEXT_MUTED)))
            .child(div().typography(Font::Body).child(self.placeholder))
    }
}

/// The coral gradient pill ("Reserve", "Sign in").
#[derive(IntoElement)]
pub struct PrimaryPillButton {
    id: ElementId,
    title: SharedString,
    icon: Option<Lucide>,
    loading: bool,
    disabled: bool,
    fills_width: bool,
    hint: Option<SharedString>,
    action: Option<OnClick>,
}

impl PrimaryPillButton {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            loading: false,
            disabled: false,
            fills_width: false,
            hint: None,
            action: None,
        }
    }

    pub fn icon(mut self, icon: Lucide) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn fills_width(mut self, fills: bool) -> Self {
        self.fills_width = fills;
        self
    }

    /// A screen-reader hint ("Reserves the book").
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn on_click(mut self, action: OnClick) -> Self {
        self.action = Some(action);
        self
    }
}

impl RenderOnce for PrimaryPillButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let enabled = !self.disabled && !self.loading;
        pill(self.id, self.title.clone(), self.hint.clone())
            .test_support()
            .when(self.fills_width, |pill| pill.w_full())
            .bg(linear_gradient(
                90.,
                linear_color_stop(theme::color(colors::PRIMARY_GRADIENT_START), 0.),
                linear_color_stop(theme::color(colors::PRIMARY_GRADIENT_END), 1.),
            ))
            .when(enabled, |pill| pill.shadow(glow(theme::color(colors::PRIMARY))))
            .when(!enabled, |pill| pill.opacity(0.5))
            .text_color(theme::color(colors::ON_PRIMARY))
            .when(self.loading, |pill| {
                pill.child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::ON_PRIMARY)))
            })
            .when_some(self.icon.filter(|_| !self.loading), |pill, name| {
                pill.child(icon(name, px(16.), theme::color(colors::ON_PRIMARY)))
            })
            .child(div().typography(Font::BodyStrong).child(self.title))
            .when(enabled, |pill| pill.cursor(CursorStyle::PointingHand).on_click(click(&self.action)))
    }
}

/// The quieter pill beside a `PrimaryPillButton`.
#[derive(IntoElement)]
pub struct SecondaryPillButton {
    id: ElementId,
    title: SharedString,
    icon: Option<Lucide>,
    disabled: bool,
    fills_width: bool,
    action: Option<OnClick>,
}

impl SecondaryPillButton {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self { id: id.into(), title: title.into(), icon: None, disabled: false, fills_width: false, action: None }
    }

    pub fn icon(mut self, icon: Lucide) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn fills_width(mut self, fills: bool) -> Self {
        self.fills_width = fills;
        self
    }

    pub fn on_click(mut self, action: OnClick) -> Self {
        self.action = Some(action);
        self
    }
}

impl RenderOnce for SecondaryPillButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        pill(self.id, self.title.clone(), None)
            .test_support()
            .when(self.fills_width, |pill| pill.w_full())
            .bg(theme::color(colors::SURFACE_ELEVATED))
            .text_color(theme::color(colors::TEXT_PRIMARY))
            .when(self.disabled, |pill| pill.opacity(0.5))
            .when_some(self.icon, |pill, name| pill.child(icon(name, px(16.), theme::color(colors::TEXT_PRIMARY))))
            .child(div().typography(Font::BodyStrong).child(self.title))
            .when(!self.disabled, |pill| {
                pill.cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(theme::color(colors::SURFACE)))
                    .on_click(click(&self.action))
            })
    }
}

fn pill(id: ElementId, label: SharedString, hint: Option<SharedString>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .when_some(hint, |pill, hint| pill.aria_description(hint))
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(spacing::S))
        .min_h(px(44.))
        .px(px(spacing::XL + spacing::S))
        .rounded(px(radius::PILL))
}

/// The blue circular icon button (the My Picks bookmark).
#[derive(IntoElement)]
pub struct CircleIconButton {
    id: ElementId,
    icon: Lucide,
    label: SharedString,
    diameter: f32,
    tint: Hsla,
    foreground: Hsla,
    disabled: bool,
    action: Option<OnClick>,
}

impl CircleIconButton {
    pub fn new(id: impl Into<ElementId>, icon: Lucide, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            diameter: 50.,
            tint: theme::color(colors::ACCENT),
            foreground: theme::color(colors::ON_ACCENT),
            disabled: false,
            action: None,
        }
    }

    pub fn diameter(mut self, diameter: f32) -> Self {
        self.diameter = diameter;
        self
    }

    pub fn tint(mut self, tint: Hsla, foreground: Hsla) -> Self {
        self.tint = tint;
        self.foreground = foreground;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, action: OnClick) -> Self {
        self.action = Some(action);
        self
    }
}

impl RenderOnce for CircleIconButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .test_support()
            .role(Role::Button)
            .aria_label(self.label)
            .flex_none()
            .size(px(self.diameter))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(self.tint)
            .shadow(glow(self.tint))
            .when(self.disabled, |button| button.opacity(0.5))
            .when(!self.disabled, |button| button.cursor(CursorStyle::PointingHand).on_click(click(&self.action)))
            .child(icon(self.icon, px(self.diameter * 0.4), self.foreground))
    }
}

/// A rounded colour tile with an icon and a caption under it (Home "Popular genres").
#[derive(IntoElement)]
pub struct GenreTile {
    id: ElementId,
    title: SharedString,
    icon: Lucide,
    color: Option<Hsla>,
    size: f32,
    hint: Option<SharedString>,
    action: Option<OnClick>,
}

impl GenreTile {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, icon: Lucide) -> Self {
        Self { id: id.into(), title: title.into(), icon, color: None, size: 76., hint: None, action: None }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn on_click(mut self, action: OnClick) -> Self {
        self.action = Some(action);
        self
    }
}

impl RenderOnce for GenreTile {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.color.unwrap_or_else(|| theme::genre_color(&self.title));
        div()
            .id(self.id)
            .test_support()
            .role(Role::Button)
            .aria_label(self.title.clone())
            .when_some(self.hint, |tile, hint| tile.aria_description(hint))
            .cursor(CursorStyle::PointingHand)
            .w(px(self.size + spacing::M))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(spacing::S))
            .on_click(click(&self.action))
            .child(
                div()
                    .size(px(self.size))
                    .rounded(px(radius::TILE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(linear_gradient(135., linear_color_stop(fill.opacity(0.8), 0.), linear_color_stop(fill, 1.)))
                    .child(icon(self.icon, px(self.size * 0.36), theme::color(0xFFFFFF))),
            )
            .child(
                div()
                    .typography(Font::Caption)
                    .text_color(theme::color(colors::TEXT_SECONDARY))
                    .truncate()
                    .child(self.title),
            )
    }
}

/// A chip's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChipTone {
    /// On navy.
    #[default]
    Neutral,
    /// Digital formats ("Available online").
    Digital,
    /// On the white sheet.
    OnSheet,
    /// The selected format.
    Selected,
    Success,
    Warning,
    Danger,
}

/// A small rounded label: format chips, subject chips, status.
#[derive(IntoElement)]
pub struct Chip {
    text: SharedString,
    tone: ChipTone,
    icon: Option<Lucide>,
}

impl Chip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into(), tone: ChipTone::Neutral, icon: None }
    }

    pub fn tone(mut self, tone: ChipTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn icon(mut self, icon: Lucide) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A format chip; digital formats get the `digital` tint and a cloud.
    pub fn format(format: &FormatRef, selected: bool) -> Self {
        let digital = format.is_digital();
        let tone = if selected {
            ChipTone::Selected
        } else if digital {
            ChipTone::Digital
        } else {
            ChipTone::Neutral
        };
        let chip = Self::new(format.display_label()).tone(tone);
        if digital { chip.icon(Lucide::Cloud) } else { chip }
    }

    fn colors(tone: ChipTone) -> (Hsla, Hsla) {
        match tone {
            ChipTone::Neutral => (theme::color(colors::TEXT_PRIMARY), theme::color(colors::SURFACE_ELEVATED)),
            ChipTone::Digital => (theme::color(colors::DIGITAL), theme::color_alpha(colors::DIGITAL, 0.16)),
            ChipTone::OnSheet => (theme::color(colors::ON_SHEET_SECONDARY), theme::color(colors::CHIP_ON_SHEET)),
            ChipTone::Selected => (theme::color(colors::ON_PRIMARY), theme::color(colors::PRIMARY)),
            ChipTone::Success => (theme::color(colors::SUCCESS), theme::color_alpha(colors::SUCCESS, 0.16)),
            ChipTone::Warning => (theme::color(colors::WARNING), theme::color_alpha(colors::WARNING, 0.16)),
            ChipTone::Danger => (theme::color(colors::DANGER), theme::color_alpha(colors::DANGER, 0.16)),
        }
    }
}

impl RenderOnce for Chip {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (foreground, background) = Self::colors(self.tone);
        div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(spacing::XS))
            .px(px(spacing::S + 2.))
            .py(px(spacing::XS + 1.))
            .rounded(px(radius::PILL))
            .bg(background)
            .text_color(foreground)
            .typography(Font::Caption)
            .when_some(self.icon, |chip, name| chip.child(icon(name, px(12.), foreground)))
            .child(div().truncate().child(self.text))
    }
}

/// An inline result banner (reservation placed / refused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerTone {
    Success,
    Info,
    Error,
}

#[derive(IntoElement)]
pub struct Banner {
    message: SharedString,
    tone: BannerTone,
}

impl Banner {
    pub fn new(message: impl Into<SharedString>, tone: BannerTone) -> Self {
        Self { message: message.into(), tone }
    }
}

impl RenderOnce for Banner {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (hex, name) = match self.tone {
            BannerTone::Success => (colors::SUCCESS, Lucide::CircleCheck),
            BannerTone::Info => (colors::ACCENT, Lucide::Info),
            BannerTone::Error => (colors::DANGER, Lucide::TriangleAlert),
        };
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_start()
            .gap(px(spacing::S))
            .p(px(spacing::M))
            .rounded(px(radius::FIELD))
            .bg(theme::color_alpha(hex, 0.22))
            .border_1()
            .border_color(theme::color_alpha(hex, 0.6))
            .text_color(theme::color(colors::TEXT_PRIMARY))
            .typography(Font::Body)
            .child(icon(name, px(16.), theme::color(colors::TEXT_PRIMARY)))
            .child(div().flex_1().child(self.message))
    }
}

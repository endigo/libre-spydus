//! Pieces for D2's white dialogs (sign in, Change library, My card): the shared components are
//! drawn for navy screens, and a dialog is white (docs/desktop-app.md §7.1) with neutral buttons,
//! never coral (native-apps.md §7 "Dialogs").

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::{
    AnyElement, App, CursorStyle, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::UserFacingError;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{FontExt, Lucide, OnClick, icon};

/// A caption over a rounded light field, on white.
pub fn labelled_field(label: &'static str, field: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S))
        .child(div().typography(Font::Caption).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(label))
        .child(field)
}

/// A text field on white: `onSheet` text on the light chip colour.
pub fn sheet_input(state: &Entity<InputState>, label: &'static str) -> Input {
    Input::new(state)
        .appearance(false)
        .aria_label(label)
        .text_size(Font::Body.size())
        .text_color(theme::color(colors::ON_SHEET))
}

/// The light rounded box around a `sheet_input`.
pub fn field_box() -> gpui_kit::Div {
    div()
        .w_full()
        .min_h(px(48.))
        .px(px(spacing::L))
        .flex()
        .items_center()
        .rounded(px(radius::FIELD))
        .bg(theme::color(colors::CHIP_ON_SHEET))
        .text_color(theme::color(colors::ON_SHEET))
}

/// A dialog's main action: a navy pill, bold, full width (neutral, never coral).
#[derive(IntoElement)]
pub struct SheetButton {
    id: ElementId,
    title: SharedString,
    loading: bool,
    disabled: bool,
    filled: bool,
    action: Option<OnClick>,
}

impl SheetButton {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self { id: id.into(), title: title.into(), loading: false, disabled: false, filled: true, action: None }
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// `false`: the quieter light pill (Edit card number, Cancel).
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    pub fn on_click(mut self, action: OnClick) -> Self {
        self.action = Some(action);
        self
    }
}

impl RenderOnce for SheetButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let enabled = !self.disabled && !self.loading;
        let (background, foreground) = if self.filled {
            (theme::color(colors::TAB_ACTIVE), theme::color(colors::SHEET))
        } else {
            (theme::color(colors::CHIP_ON_SHEET), theme::color(colors::ON_SHEET))
        };
        let action = self.action.clone();
        div()
            .id(self.id)
            .test_support()
            .role(Role::Button)
            .aria_label(self.title.clone())
            .w_full()
            .min_h(px(44.))
            .px(px(spacing::XL))
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(spacing::S))
            .rounded(px(radius::PILL))
            .bg(background)
            .text_color(foreground)
            .typography(if self.filled { Font::BodyStrong } else { Font::Body })
            .when(!enabled, |button| button.opacity(0.45))
            .when(self.loading, |button| button.child(gpui_kit::component::spinner::Spinner::new().color(foreground)))
            .child(self.title)
            .when(enabled, |button| {
                button.cursor(CursorStyle::PointingHand).hover(|style| style.opacity(0.88)).on_click(
                    move |_, window, cx| {
                        if let Some(action) = &action {
                            action(window, cx);
                        }
                    },
                )
            })
    }
}

/// An inline error under a form, on white.
pub fn error_note(id: impl Into<ElementId>, message: impl Into<SharedString>) -> impl IntoElement {
    let message = message.into();
    div()
        .id(id.into())
        .test_support()
        .role(Role::Alert)
        .aria_label(message.clone())
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(spacing::S))
        .p(px(spacing::M))
        .rounded(px(radius::FIELD))
        .bg(theme::color_alpha(colors::DANGER, 0.12))
        .border_1()
        .border_color(theme::color_alpha(colors::DANGER, 0.5))
        .text_color(theme::color(colors::ON_SHEET))
        .typography(Font::Body)
        .child(icon(Lucide::TriangleAlert, px(16.), theme::color(colors::DANGER)))
        .child(div().flex_1().child(message))
}

/// An informational note on white (the debug demo hint).
pub fn info_note(message: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(spacing::S))
        .p(px(spacing::M))
        .rounded(px(radius::FIELD))
        .bg(theme::color_alpha(colors::ACCENT, 0.1))
        .text_color(theme::color(colors::ON_SHEET))
        .typography(Font::Caption)
        .child(icon(Lucide::Info, px(14.), theme::color(colors::ACCENT)))
        .child(div().flex_1().child(message.into()))
}

/// A dialog's loading / empty / error state, centred, on white.
pub fn sheet_state(
    id: impl Into<ElementId>,
    glyph: Lucide,
    title: impl Into<SharedString>,
    message: Option<SharedString>,
    action: Option<(ElementId, SharedString, OnClick)>,
) -> AnyElement {
    div()
        .id(id.into())
        .test_support()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(spacing::M))
        .p(px(spacing::XL))
        .child(icon(glyph, px(36.), theme::color(colors::ON_SHEET_SECONDARY)))
        .child(
            div()
                .typography(Font::Section)
                .text_color(theme::color(colors::ON_SHEET))
                .text_center()
                .child(title.into()),
        )
        .when_some(message, |view, message| {
            view.child(
                div()
                    .max_w(px(380.))
                    .typography(Font::Body)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .text_center()
                    .child(message),
            )
        })
        .when_some(action, |view, (id, title, action)| {
            view.child(div().pt(px(spacing::S)).w(px(200.)).child(SheetButton::new(id, title).on_click(action)))
        })
        .into_any_element()
}

/// The spinner state on white.
pub fn sheet_loading(id: impl Into<ElementId>, message: &'static str) -> AnyElement {
    div()
        .id(id.into())
        .test_support()
        .role(Role::ProgressIndicator)
        .aria_label(SharedString::from(message))
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(spacing::M))
        .child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::ON_SHEET_SECONDARY)))
        .child(div().typography(Font::Body).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(message))
        .into_any_element()
}

/// An error with Retry (when it makes sense), on white.
pub fn sheet_error(id: &'static str, error: &UserFacingError, retry: OnClick) -> AnyElement {
    let action = error
        .can_retry()
        .then(|| (ElementId::from(SharedString::from(format!("{id}-retry"))), Copy::Common::retry.into(), retry));
    sheet_state(id, Lucide::TriangleAlert, error.title(), Some(error.message.clone().into()), action)
}

/// A dialog's title row with a close button on the right (gpui-kit's own close button is drawn
/// for a dark dialog and vanishes on white). Use with `.close_button(false)`.
pub fn dialog_header(title: impl IntoElement, close_id: &'static str) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .justify_between()
        .gap(px(spacing::M))
        .child(div().flex_1().min_w_0().child(title))
        .child(close_button(close_id))
}

/// The round × that closes the top dialog.
pub fn close_button(id: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .role(Role::Button)
        .aria_label(SharedString::from(Copy::Common::close))
        .cursor(CursorStyle::PointingHand)
        .flex_none()
        .size(px(30.))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme::color(colors::CHIP_ON_SHEET))
        .hover(|style| style.bg(theme::color_alpha(colors::ON_SHEET_SECONDARY, 0.2)))
        .on_click(|_, window, cx| {
            use gpui_kit::component::WindowExt;
            window.close_dialog(cx)
        })
        .child(icon(Lucide::Close, px(14.), theme::color(colors::ON_SHEET_SECONDARY)))
}

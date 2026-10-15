//! Loading, error and result states drawn on white (the book page's sheet and the Reserve dialog),
//! as iOS `SheetLoadingView` / `SheetErrorView`: the shared states are drawn for navy.

use gpui_kit::{
    App, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::UserFacingError;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::controls::OnClick;
use crate::ui::{FontExt, Lucide, icon};

/// A spinner with a message, in sheet colours.
#[derive(IntoElement)]
pub struct SheetLoading {
    id: ElementId,
    message: SharedString,
}

impl SheetLoading {
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self { id: id.into(), message: message.into() }
    }
}

impl RenderOnce for SheetLoading {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .test_support()
            .role(Role::ProgressIndicator)
            .aria_label(self.message.clone())
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(spacing::M))
            .py(px(spacing::XL))
            .child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::ON_SHEET_SECONDARY)))
            .child(
                div().typography(Font::Body).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(self.message),
            )
    }
}

/// Grey bars where text will be, pulsing while the record loads.
pub fn skeleton_lines(widths: &[f32]) -> impl IntoElement {
    crate::ui::pulse(div().w_full().flex().flex_col().gap(px(spacing::S)).children(widths.iter().map(|width| {
        div().h(px(12.)).w(gpui_kit::relative(*width)).rounded(px(radius::PILL)).bg(theme::color(colors::CHIP_ON_SHEET))
    })))
}

/// What went wrong, Retry, and the detail behind Details, in sheet colours.
#[derive(IntoElement)]
pub struct SheetError {
    id: ElementId,
    error: UserFacingError,
    retry: Option<OnClick>,
}

impl SheetError {
    pub fn new(id: impl Into<ElementId>, error: &UserFacingError) -> Self {
        Self { id: id.into(), error: error.clone(), retry: None }
    }

    pub fn retry(mut self, retry: OnClick) -> Self {
        self.retry = Some(retry);
        self
    }
}

impl RenderOnce for SheetError {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state =
            window.use_keyed_state(SharedString::from(format!("{}-details-open", self.id)), cx, |_, _| false);
        let open = *open_state.read(cx);
        let detail = self.error.detail.clone();
        let retry = self.retry.filter(|_| self.error.can_retry());
        div()
            .id(self.id.clone())
            .test_support()
            .role(Role::Alert)
            .aria_label(SharedString::from(self.error.message.clone()))
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(spacing::M))
            .py(px(spacing::XL))
            .child(icon(Lucide::TriangleAlert, px(36.), theme::color(colors::WARNING)))
            .child(div().typography(Font::Section).text_color(theme::color(colors::ON_SHEET)).child(self.error.title()))
            .child(
                div()
                    .max_w(px(420.))
                    .typography(Font::Body)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .text_center()
                    .child(self.error.message.clone()),
            )
            .when_some(retry, |view, retry| {
                view.child(
                    div()
                        .id(SharedString::from(format!("{}-retry", self.id)))
                        .test_support()
                        .role(Role::Button)
                        .aria_label(SharedString::from(Copy::Common::retry))
                        .cursor(CursorStyle::PointingHand)
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(spacing::S))
                        .px(px(spacing::XL))
                        .min_h(px(40.))
                        .rounded(px(radius::PILL))
                        .bg(theme::color(colors::CHIP_ON_SHEET))
                        .hover(|style| style.bg(theme::color_alpha(colors::ON_SHEET_SECONDARY, 0.18)))
                        .text_color(theme::color(colors::ON_SHEET))
                        .on_click(move |_, window, cx| retry(window, cx))
                        .child(icon(Lucide::RefreshCw, px(14.), theme::color(colors::ON_SHEET)))
                        .child(div().typography(Font::BodyStrong).child(Copy::Common::retry)),
                )
            })
            .when_some(detail, |view, detail| {
                view.child(
                    div()
                        .max_w(px(360.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(spacing::XS))
                        .child(
                            div()
                                .id(SharedString::from(format!("{}-details", self.id)))
                                .test_support()
                                .role(Role::Button)
                                .aria_label(SharedString::from(Copy::Errors::details))
                                .cursor(CursorStyle::PointingHand)
                                .typography(Font::Caption)
                                .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                                .on_click(move |_, _, cx| {
                                    open_state.update(cx, |open, cx| {
                                        *open = !*open;
                                        cx.notify();
                                    })
                                })
                                .child(Copy::Errors::details),
                        )
                        .when(open, |column| {
                            column.child(
                                div()
                                    .typography(Font::Caption)
                                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                                    .child(detail),
                            )
                        }),
                )
            })
    }
}

/// A result banner on white: placed, refused, information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetBannerTone {
    Success,
    Info,
    Error,
}

#[derive(IntoElement)]
pub struct SheetBanner {
    id: ElementId,
    message: SharedString,
    tone: SheetBannerTone,
}

impl SheetBanner {
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>, tone: SheetBannerTone) -> Self {
        Self { id: id.into(), message: message.into(), tone }
    }
}

impl RenderOnce for SheetBanner {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (hex, name) = match self.tone {
            SheetBannerTone::Success => (colors::SUCCESS, Lucide::CircleCheck),
            SheetBannerTone::Info => (colors::ACCENT, Lucide::Info),
            SheetBannerTone::Error => (colors::DANGER, Lucide::TriangleAlert),
        };
        div()
            .id(self.id)
            .test_support()
            .role(if self.tone == SheetBannerTone::Error { Role::Alert } else { Role::Status })
            .aria_label(self.message.clone())
            .w_full()
            .flex()
            .flex_row()
            .items_start()
            .gap(px(spacing::S))
            .p(px(spacing::M))
            .rounded(px(radius::FIELD))
            .bg(theme::color_alpha(hex, 0.12))
            .border_1()
            .border_color(theme::color_alpha(hex, 0.5))
            .text_color(theme::color(colors::ON_SHEET))
            .typography(Font::Body)
            .child(icon(name, px(16.), theme::color(hex)))
            .child(div().flex_1().child(self.message))
    }
}

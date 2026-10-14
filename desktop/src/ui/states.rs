//! Loading, empty and error states for a screen or section (iOS `StateViews.swift`), and the
//! wave-1 placeholder screen.

use gpui_kit::{
    App, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::{ErrorKind, UserFacingError};
use crate::theme::{self, Font, colors, spacing};
use crate::ui::controls::{OnClick, PrimaryPillButton};
use crate::ui::icons::{Lucide, icon};
use crate::ui::style::FontExt;

fn centred() -> gpui_kit::Div {
    div().size_full().flex().flex_col().items_center().justify_center().gap(px(spacing::M)).p(px(spacing::XL))
}

/// A spinner, with an optional line under it.
#[derive(IntoElement)]
pub struct LoadingView {
    message: Option<SharedString>,
}

impl LoadingView {
    pub fn new() -> Self {
        Self { message: None }
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Default for LoadingView {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for LoadingView {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let label: SharedString = self.message.clone().unwrap_or_else(|| Copy::Common::loading.into());
        centred()
            .id("loading")
            .test_support()
            .role(Role::ProgressIndicator)
            .aria_label(label)
            .child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::TEXT_SECONDARY)))
            .when_some(self.message, |view, message| {
                view.child(div().typography(Font::Body).text_color(theme::color(colors::TEXT_SECONDARY)).child(message))
            })
    }
}

/// An empty state: icon, title, message, an optional action ("No results for …").
#[derive(IntoElement)]
pub struct EmptyStateView {
    icon: Lucide,
    title: SharedString,
    message: Option<SharedString>,
    action: Option<(ElementId, SharedString, OnClick)>,
}

impl EmptyStateView {
    pub fn new(icon: Lucide, title: impl Into<SharedString>) -> Self {
        Self { icon, title: title.into(), message: None, action: None }
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn action(mut self, id: impl Into<ElementId>, title: impl Into<SharedString>, action: OnClick) -> Self {
        self.action = Some((id.into(), title.into(), action));
        self
    }
}

impl RenderOnce for EmptyStateView {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        centred()
            .child(icon(self.icon, px(44.), theme::color(colors::TEXT_MUTED)))
            .child(
                div()
                    .typography(Font::Section)
                    .text_color(theme::color(colors::TEXT_PRIMARY))
                    .text_center()
                    .child(self.title),
            )
            .when_some(self.message, |view, message| {
                view.child(
                    div()
                        .max_w(px(420.))
                        .typography(Font::Body)
                        .text_color(theme::color(colors::TEXT_SECONDARY))
                        .text_center()
                        .child(message),
                )
            })
            .when_some(self.action, |view, (id, title, action)| {
                view.child(div().pt(px(spacing::S)).child(PrimaryPillButton::new(id, title).on_click(action)))
            })
    }
}

/// An error state with Retry (when it makes sense) and the core's technical text under Details.
#[derive(IntoElement)]
pub struct ErrorStateView {
    id: ElementId,
    error: UserFacingError,
    retry: Option<OnClick>,
}

impl ErrorStateView {
    pub fn new(id: impl Into<ElementId>, error: &UserFacingError) -> Self {
        Self { id: id.into(), error: error.clone(), retry: None }
    }

    pub fn retry(mut self, retry: OnClick) -> Self {
        self.retry = Some(retry);
        self
    }
}

fn error_icon(kind: ErrorKind) -> Lucide {
    match kind {
        ErrorKind::Network => Lucide::WifiOff,
        ErrorKind::VerificationRequired | ErrorKind::VerificationUnavailable => Lucide::ShieldCheck,
        ErrorKind::NotLoggedIn | ErrorKind::CredentialsRejected => Lucide::UserRoundX,
        ErrorKind::Unexpected => Lucide::TriangleAlert,
    }
}

impl RenderOnce for ErrorStateView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let shows_detail =
            window.use_keyed_state(SharedString::from(format!("{}-details-open", self.id)), cx, |_, _| false);
        let open = *shows_detail.read(cx);
        let detail = self.error.detail.clone();
        centred()
            .child(icon(error_icon(self.error.kind), px(40.), theme::color(colors::WARNING)))
            .child(
                div()
                    .typography(Font::Section)
                    .text_color(theme::color(colors::TEXT_PRIMARY))
                    .child(self.error.title()),
            )
            .child(
                div()
                    .max_w(px(420.))
                    .typography(Font::Body)
                    .text_color(theme::color(colors::TEXT_SECONDARY))
                    .text_center()
                    .child(self.error.message.clone()),
            )
            .when_some(self.retry.filter(|_| self.error.can_retry()), |view, retry| {
                view.child(
                    div().pt(px(spacing::S)).child(
                        PrimaryPillButton::new(SharedString::from(format!("{}-retry", self.id)), Copy::Common::retry)
                            .icon(Lucide::RefreshCw)
                            .on_click(retry),
                    ),
                )
            })
            .when_some(detail, |view, detail| {
                view.child(
                    div()
                        .pt(px(spacing::S))
                        .max_w(px(320.))
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
                                .text_color(theme::color(colors::TEXT_SECONDARY))
                                .on_click(move |_, _, cx| {
                                    shows_detail.update(cx, |open, cx| {
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
                                    .text_color(theme::color(colors::TEXT_MUTED))
                                    .child(detail),
                            )
                        }),
                )
            })
    }
}

/// What every wave-1 feature screen shows until its worker builds it: its title and a debug
/// "coming soon" label.
#[derive(IntoElement)]
pub struct PlaceholderScreen {
    id: ElementId,
    title: SharedString,
    icon: Lucide,
    detail: Option<SharedString>,
}

impl PlaceholderScreen {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, icon: Lucide) -> Self {
        Self { id: id.into(), title: title.into(), icon, detail: None }
    }

    /// A debug line under the label (the route's argument, say).
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl RenderOnce for PlaceholderScreen {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        centred()
            .id(self.id)
            .test_support()
            .gap(px(spacing::L))
            .child(
                div()
                    .size(px(104.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme::color(colors::SURFACE))
                    .child(icon(self.icon, px(52.), theme::color(colors::PRIMARY))),
            )
            .child(
                div()
                    .typography(Font::Title)
                    .text_color(theme::color(colors::TEXT_PRIMARY))
                    .text_center()
                    .child(self.title),
            )
            .child(
                div().typography(Font::Caption).text_color(theme::color(colors::TEXT_MUTED)).child("coming soon"), // copy-exempt: debug placeholder until the wave-2 screen replaces it
            )
            .when_some(self.detail, |view, detail| {
                view.child(
                    div()
                        .max_w(px(560.))
                        .typography(Font::Caption)
                        .text_color(theme::color(colors::TEXT_MUTED))
                        .text_center()
                        .child(detail),
                )
            })
    }
}

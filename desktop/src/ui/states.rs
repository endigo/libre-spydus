//! Loading, empty and error states for a screen or section (iOS `StateViews.swift`), the pulsing
//! skeleton blocks (iOS `SkeletonBlock`), and the wave-1 placeholder screen.

use std::time::Duration;

use gpui_kit::{
    Animation, AnimationExt, AnyElement, App, CursorStyle, Div, ElementId, InteractiveElement, IntoElement,
    ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    div, ease_in_out, prelude::FluentBuilder, px,
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

// ---- Skeletons ----

/// One pulse, dim and back: iOS fades a skeleton to 45 % over 0.9 s and back, forever.
pub const PULSE_PERIOD: Duration = Duration::from_millis(1800);
/// How far a skeleton dims at the middle of a pulse.
pub const PULSE_DIMMED_OPACITY: f32 = 0.45;

/// A skeleton's opacity `progress` (0…1) through a pulse: 1 → 0.45 → 1, eased in and out.
pub fn pulse_opacity(progress: f32) -> f32 {
    let there_and_back = if progress <= 0.5 { progress * 2. } else { (1. - progress) * 2. };
    1. - (1. - PULSE_DIMMED_OPACITY) * ease_in_out(there_and_back.clamp(0., 1.))
}

/// `element` pulsing gently while content loads. Every skeleton pulses in step (one clock for the
/// app). Still when the reader asked macOS to reduce motion (Accessibility › Display), or GPUI's
/// own `reduce_motion` is set.
pub fn pulse(element: Div) -> AnyElement {
    if system_reduces_motion() {
        return element.into_any_element();
    }
    element
        .with_animation("skeleton-pulse", Animation::new(PULSE_PERIOD).repeat_synced(), |element, progress| {
            element.opacity(pulse_opacity(progress))
        })
        .into_any_element()
}

/// macOS's "Reduce motion" (`NSWorkspace.accessibilityDisplayShouldReduceMotion`).
#[cfg(target_os = "macos")]
pub fn system_reduces_motion() -> bool {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    let Some(class) = AnyClass::get(c"NSWorkspace") else { return false };
    // SAFETY: `+[NSWorkspace sharedWorkspace]` returns the shared instance (never released by
    // us), and `accessibilityDisplayShouldReduceMotion` is a BOOL property (macOS 10.12+).
    unsafe {
        let workspace: *mut AnyObject = msg_send![class, sharedWorkspace];
        !workspace.is_null() && msg_send![workspace, accessibilityDisplayShouldReduceMotion]
    }
}

#[cfg(not(target_os = "macos"))]
pub fn system_reduces_motion() -> bool {
    false
}

/// A pulsing placeholder block while content loads (Home shelves, search rows, collections).
#[derive(IntoElement)]
pub struct SkeletonBlock {
    width: Option<Pixels>,
    height: Pixels,
    radius: f32,
}

impl SkeletonBlock {
    pub fn new(width: Pixels, height: Pixels) -> Self {
        Self { width: Some(width), height, radius: 6. }
    }

    /// As wide as its container.
    pub fn fill(height: Pixels) -> Self {
        Self { width: None, height, radius: 6. }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

impl RenderOnce for SkeletonBlock {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        pulse(
            div()
                .flex_none()
                .map(|block| match self.width {
                    Some(width) => block.w(width),
                    None => block.w_full(),
                })
                .h(self.height)
                .rounded(px(self.radius))
                .bg(theme::color(colors::SURFACE_ELEVATED)),
        )
    }
}

// ---- Loading, empty, error ----

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pulse_dims_to_45_percent_and_back() {
        assert_eq!(pulse_opacity(0.), 1.);
        assert!((pulse_opacity(0.5) - PULSE_DIMMED_OPACITY).abs() < 1e-6);
        assert_eq!(pulse_opacity(1.), 1.);
        // Eased: symmetric, and always between the two ends.
        assert!((pulse_opacity(0.2) - pulse_opacity(0.8)).abs() < 1e-6);
        for step in 0..=20 {
            let opacity = pulse_opacity(step as f32 / 20.);
            assert!((PULSE_DIMMED_OPACITY..=1.).contains(&opacity), "{opacity}");
        }
    }
}

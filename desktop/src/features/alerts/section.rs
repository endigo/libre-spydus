//! Profile › Notifications (docs/native-apps.md §5.8 "Settings"; iOS `NotificationsSection.swift`),
//! shown while signed in: "Ready to collect" and "Due-date reminders", app-wide and off until
//! turned on. Turning one on asks for permission; refused, it stays off and a line under the
//! section says so, with Open System Settings.

use gpui_kit::component::Disableable;
use gpui_kit::component::switch::Switch;
use gpui_kit::{
    AnyElement, App, CursorStyle, InteractiveElement, IntoElement, ParentElement, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use super::content::AlertKind;
use super::system::SETTINGS_URL;
use crate::copy::Copy;
use crate::state::AppState;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{FontExt, Lucide, icon};

pub(super) fn render(_window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    {
        let state = AppState::global(cx).read(cx);
        if !state.is_signed_in() || state.library().is_none() {
            return None;
        }
    }
    let center = super::center(cx);
    let center = center.read(cx);
    let settings = center.settings();
    let asking = center.asking();
    let ready_on = settings.ready_enabled || asking == Some(AlertKind::Ready);
    let due_on = settings.due_enabled || asking == Some(AlertKind::Due);
    let blocked = center.permission_denied();
    Some(
        div()
            .id("alerts-section")
            .test_support()
            .flex()
            .flex_col()
            .gap(px(spacing::M))
            .child(
                div()
                    .id("alerts-header")
                    .test_support()
                    .role(Role::Heading)
                    .aria_label(SharedString::from(Copy::Notifications::section))
                    .typography(Font::Section)
                    .text_color(theme::color(colors::TEXT_PRIMARY))
                    .child(Copy::Notifications::section),
            )
            .child(
                div()
                    .id("alerts-card")
                    .test_support()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::M))
                    .p(px(spacing::L))
                    .rounded(px(radius::CARD))
                    .bg(theme::color(colors::SURFACE))
                    .child(switch_row(
                        AlertKind::Ready,
                        Copy::Notifications::readyTitle,
                        Copy::Notifications::readySubtitle,
                        Lucide::Inbox,
                        ready_on,
                        asking.is_some(),
                    ))
                    .child(div().w_full().h(px(1.)).bg(theme::color(colors::DIVIDER)))
                    .child(switch_row(
                        AlertKind::Due,
                        Copy::Notifications::dueTitle,
                        Copy::Notifications::dueSubtitle,
                        Lucide::Calendar,
                        due_on,
                        asking.is_some(),
                    )),
            )
            .when(blocked, |section| section.child(blocked_line()))
            .into_any_element(),
    )
}

/// One switch: icon, title and a line saying what it does.
fn switch_row(
    kind: AlertKind,
    title: &'static str,
    subtitle: &'static str,
    glyph: Lucide,
    on: bool,
    disabled: bool,
) -> impl IntoElement {
    let (row_id, switch_id) = match kind {
        AlertKind::Ready => ("alerts-ready-row", "alerts-ready-switch"),
        AlertKind::Due => ("alerts-due-row", "alerts-due-switch"),
    };
    div()
        .id(row_id)
        .test_support()
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(spacing::M))
        .child(div().w(px(24.)).pt(px(1.)).flex().justify_center().child(icon(
            glyph,
            px(18.),
            theme::color(colors::PRIMARY),
        )))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(spacing::XS))
                .child(div().typography(Font::BodyStrong).text_color(theme::color(colors::TEXT_PRIMARY)).child(title))
                .child(
                    div().typography(Font::Caption).text_color(theme::color(colors::TEXT_SECONDARY)).child(subtitle),
                ),
        )
        .child(
            Switch::new(switch_id)
                .checked(on)
                .disabled(disabled)
                .color(theme::color(colors::SUCCESS))
                .accessibility_label(SharedString::from(title))
                .on_change(move |on, _, cx| super::set_switch(kind, *on, cx)),
        )
}

/// "Notifications are turned off for LibreSpydus." — Open System Settings.
fn blocked_line() -> impl IntoElement {
    div()
        .id("alerts-blocked")
        .test_support()
        .aria_label(SharedString::from(Copy::Notifications::blocked))
        .w_full()
        .px(px(spacing::XS))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S))
        .child(icon(Lucide::TriangleAlert, px(14.), theme::color(colors::WARNING)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .typography(Font::Caption)
                .text_color(theme::color(colors::WARNING))
                .child(Copy::Notifications::blocked),
        )
        .child(
            div()
                .id("alerts-open-settings")
                .test_support()
                .role(Role::Button)
                .aria_label(SharedString::from(Copy::Notifications::openSettings))
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.opacity(0.8))
                .typography(Font::BodyStrong)
                .text_color(theme::color(colors::PRIMARY))
                .on_click(|_, _, cx| cx.open_url(SETTINGS_URL))
                .child(Copy::Notifications::openSettings),
        )
}

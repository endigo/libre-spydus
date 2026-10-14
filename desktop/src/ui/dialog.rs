//! Dialogs (docs/desktop-app.md §7.1, native-apps.md §7 "Dialogs"): white like the phones' system
//! dialogs; a destructive action is red, every other button neutral, the confirming one bold, and
//! coral never appears in a dialog.

use std::rc::Rc;

use gpui_kit::component::WindowExt;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::{
    App, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::controls::OnClick;
use crate::ui::style::FontExt;

/// The app's look for a gpui-kit dialog: white, `onSheet` text. Use it on every dialog a feature
/// opens (`window.open_dialog(cx, |dialog, _, _| sheet_dialog(dialog).title(…))`).
pub fn sheet_dialog(dialog: Dialog) -> Dialog {
    dialog
        .bg(theme::color(colors::SHEET))
        .text_color(theme::color(colors::ON_SHEET))
        .rounded(px(radius::CARD))
        .border_color(theme::color(colors::CHIP_ON_SHEET))
}

/// A dialog's title in the app's type.
pub fn dialog_title(title: impl Into<SharedString>) -> impl IntoElement {
    div().typography(Font::Section).text_color(theme::color(colors::ON_SHEET)).child(title.into())
}

/// A dialog button: neutral, or red when `destructive`; bold when it confirms.
#[derive(IntoElement)]
pub struct DialogButton {
    id: ElementId,
    label: SharedString,
    confirming: bool,
    destructive: bool,
    action: OnClick,
}

impl DialogButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, action: OnClick) -> Self {
        Self { id: id.into(), label: label.into(), confirming: false, destructive: false, action }
    }

    pub fn confirming(mut self, confirming: bool) -> Self {
        self.confirming = confirming;
        self
    }

    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }
}

impl RenderOnce for DialogButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let color = theme::color(if self.destructive { colors::DANGER } else { colors::ON_SHEET });
        let action = self.action.clone();
        div()
            .id(self.id)
            .test_support()
            .role(Role::Button)
            .aria_label(self.label.clone())
            .cursor(CursorStyle::PointingHand)
            .px(px(spacing::L))
            .py(px(spacing::S))
            .rounded(px(radius::FIELD))
            .bg(theme::color(colors::CHIP_ON_SHEET))
            .hover(|style| style.bg(theme::color_alpha(colors::ON_SHEET_SECONDARY, 0.18)))
            .text_color(color)
            .typography(if self.confirming { Font::BodyStrong } else { Font::Body })
            .on_click(move |_, window, cx| action(window, cx))
            .child(self.label)
    }
}

/// "Cancel this reservation?" — Keep / Cancel reservation. The confirming action runs
/// `on_confirm` and closes the dialog; the other button (and Escape) just closes it.
pub struct ConfirmDialog {
    title: SharedString,
    message: Option<SharedString>,
    confirm: SharedString,
    cancel: SharedString,
    destructive: bool,
    on_confirm: OnClick,
}

impl ConfirmDialog {
    pub fn new(title: impl Into<SharedString>, confirm: impl Into<SharedString>, on_confirm: OnClick) -> Self {
        Self {
            title: title.into(),
            message: None,
            confirm: confirm.into(),
            cancel: Copy::Common::cancel.into(),
            destructive: false,
            on_confirm,
        }
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// The other button's label (`Copy::Common::notNow`, …).
    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel = label.into();
        self
    }

    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    pub fn open(self, window: &mut Window, cx: &mut App) {
        let ConfirmDialog { title, message, confirm, cancel, destructive, on_confirm } = self;
        window.open_dialog(cx, move |dialog, _, _| {
            let confirm_action = on_confirm.clone();
            let ok_action = on_confirm.clone();
            sheet_dialog(dialog)
                .w(px(420.))
                .close_button(false)
                .title(dialog_title(title.clone()))
                .on_ok(move |_, window, cx| {
                    ok_action(window, cx);
                    true
                })
                .when_some(message.clone(), |dialog, message| {
                    dialog.child(
                        div()
                            .typography(Font::Body)
                            .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                            .child(message),
                    )
                })
                .footer(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .justify_end()
                        .gap(px(spacing::S))
                        .child(DialogButton::new(
                            "confirm-cancel",
                            cancel.clone(),
                            Rc::new(|window: &mut Window, cx: &mut App| window.close_dialog(cx)),
                        ))
                        .child(
                            DialogButton::new(
                                "confirm-ok",
                                confirm.clone(),
                                Rc::new(move |window: &mut Window, cx: &mut App| {
                                    window.close_dialog(cx);
                                    confirm_action(window, cx);
                                }),
                            )
                            .confirming(true)
                            .destructive(destructive),
                        ),
                )
        });
    }
}

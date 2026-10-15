//! The toolbar above the content: Back (⌘[) while the section's stack isn't empty.

use gpui_kit::{
    App, CursorStyle, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::AppState;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{FontExt, Lucide, content_column, icon};

pub const HEIGHT: f32 = 52.;

#[derive(IntoElement)]
pub struct Toolbar;

impl RenderOnce for Toolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = AppState::global(cx).read(cx);
        let can_go_back = !state.stack(state.section()).is_empty();
        div()
            .id("toolbar")
            .test_support()
            .role(Role::Toolbar)
            .flex_none()
            .h(px(HEIGHT))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .bg(theme::color(colors::BACKGROUND))
            // Back lines up with the page's left edge (`content_column`): its chevron, past the
            // button's own padding, sits where the page's title starts.
            .child(content_column().when(can_go_back, |column| {
                column.child(
                    div()
                        .id("toolbar-back")
                        .test_support()
                        .role(Role::Button)
                        .aria_label(SharedString::from(Copy::Common::back))
                        .aria_keyshortcuts("Meta+BracketLeft")
                        .cursor(CursorStyle::PointingHand)
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(spacing::XS))
                        .ml(px(-spacing::S))
                        .pl(px(spacing::S))
                        .pr(px(spacing::M))
                        .h(px(32.))
                        .rounded(px(radius::PILL))
                        .text_color(theme::color(colors::TEXT_PRIMARY))
                        .hover(|style| style.bg(theme::color(colors::SURFACE)))
                        .on_click(|_, _, cx| AppState::global(cx).update(cx, |state, cx| state.pop(cx)))
                        .child(icon(Lucide::ChevronLeft, px(18.), theme::color(colors::TEXT_PRIMARY)))
                        .child(div().typography(Font::BodyStrong).child(Copy::Common::back)),
                )
            }))
    }
}

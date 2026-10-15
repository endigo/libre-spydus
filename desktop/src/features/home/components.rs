//! Home's building blocks (iOS `HomeComponents.swift`), also used by a Featured collection: cover
//! skeletons, a section's inline error and empty cards, the signed-out "Your loans" card, and the
//! horizontal cover shelf.

use gpui_kit::{
    AnyElement, App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::{AppState, BookRef, Route, UserFacingError};
use crate::theme::{self, Font, colors, cover, radius, spacing};
use crate::ui::{FontExt, Lucide, OnClick, PrimaryPillButton, ShelfCover, icon, on_click};

/// The pulsing placeholder block (shared, `ui::states`).
pub use crate::ui::SkeletonBlock;

/// A shelf of cover-shaped skeletons.
pub fn shelf_skeleton(id: impl Into<ElementId>) -> impl IntoElement {
    let width = cover::SHELF_WIDTH;
    div()
        .id(id.into())
        .test_support()
        .role(Role::ProgressIndicator)
        .aria_label(SharedString::from(Copy::Common::loading))
        .w_full()
        .overflow_hidden()
        .flex()
        .flex_row()
        .gap(px(spacing::M))
        .children((0..6).map(move |_| {
            div()
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(spacing::S))
                .child(SkeletonBlock::new(px(width), px(cover::height(width))).radius(radius::COVER))
                .child(SkeletonBlock::new(px(width * 0.7), px(10.)))
        }))
}

/// Home's section card on `surface`.
fn home_card() -> gpui_kit::Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::M))
        .p(px(spacing::L))
        .rounded(px(radius::CARD))
        .bg(theme::color(colors::SURFACE))
}

fn card_text(title: SharedString, message: Option<SharedString>) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::XS))
        .child(div().typography(Font::BodyStrong).text_color(theme::color(colors::TEXT_PRIMARY)).child(title))
        .when_some(message, |column, message| {
            column
                .child(div().typography(Font::Caption).text_color(theme::color(colors::TEXT_SECONDARY)).child(message))
        })
}

/// A section's inline failure: what went wrong and Retry, without taking over the screen.
#[derive(IntoElement)]
pub struct SectionErrorCard {
    id: ElementId,
    error: UserFacingError,
    retry: OnClick,
}

impl SectionErrorCard {
    pub fn new(id: impl Into<ElementId>, error: &UserFacingError, retry: OnClick) -> Self {
        Self { id: id.into(), error: error.clone(), retry }
    }
}

impl RenderOnce for SectionErrorCard {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let retry = self.retry.clone();
        home_card()
            .id(self.id.clone())
            .test_support()
            .role(Role::Group)
            .aria_label(SharedString::from(self.error.message.clone()))
            .child(icon(Lucide::TriangleAlert, px(20.), theme::color(colors::WARNING)))
            .child(card_text(self.error.title().into(), Some(self.error.message.clone().into())))
            .when(self.error.can_retry(), |card| {
                card.child(
                    div()
                        .id(SharedString::from(format!("{}-retry", self.id)))
                        .test_support()
                        .role(Role::Button)
                        .aria_label(SharedString::from(Copy::Common::retry))
                        .cursor(gpui_kit::CursorStyle::PointingHand)
                        .typography(Font::BodyStrong)
                        .text_color(theme::color(colors::PRIMARY))
                        .on_click(move |_, window, cx| retry(window, cx))
                        .child(Copy::Common::retry),
                )
            })
    }
}

/// A section's empty state: an icon and a line or two.
pub fn section_message_card(
    id: impl Into<ElementId>,
    glyph: Lucide,
    title: impl Into<SharedString>,
    message: Option<SharedString>,
) -> impl IntoElement {
    let title = title.into();
    home_card()
        .id(id.into())
        .test_support()
        .role(Role::Group)
        .aria_label(title.clone())
        .child(icon(glyph, px(20.), theme::color(colors::TEXT_MUTED)))
        .child(card_text(title, message))
}

/// Signed out, in place of "Your loans".
pub fn sign_in_card(library_name: &str) -> impl IntoElement {
    home_card()
        .id("home-sign-in-card")
        .test_support()
        .role(Role::Group)
        .aria_label(SharedString::from(Copy::Home::signInCardTitle))
        .child(
            div()
                .flex_none()
                .size(px(44.))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme::color(colors::ACCENT))
                .child(icon(Lucide::LibraryBig, px(20.), theme::color(colors::ON_ACCENT))),
        )
        .child(card_text(Copy::Home::signInCardTitle.into(), Some(Copy::Home::signInCardMessage(library_name).into())))
        .child(
            PrimaryPillButton::new("home-sign-in", Copy::Account::signIn)
                .on_click(on_click(|_, cx| AppState::global(cx).update(cx, |state, cx| state.request_sign_in(cx)))),
        )
}

/// One cover on a shelf.
#[derive(Clone)]
pub struct ShelfEntry {
    pub book: BookRef,
    pub caption: Option<String>,
    pub caption_color: Option<Hsla>,
}

impl ShelfEntry {
    /// Captioned with its title.
    pub fn titled(book: BookRef) -> Self {
        let caption = Some(book.title.clone());
        Self { book, caption, caption_color: None }
    }
}

/// A horizontal row of covers; a click opens the book on the current section. Ids
/// `{prefix}-{index}`.
pub fn cover_shelf(prefix: &str, entries: Vec<ShelfEntry>) -> impl IntoElement {
    div()
        .id(SharedString::from(prefix.to_owned()))
        .test_support()
        .w_full()
        // The covers lay out on the scroller itself: wrapped in an inner row, that row is only as
        // wide as the scroller, the covers spill past it, and GPUI finds nothing to scroll.
        .overflow_x_scroll()
        .restrict_scroll_to_axis()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(spacing::M))
        .pb(px(spacing::S))
        .children(entries.into_iter().enumerate().map(|(index, entry)| -> AnyElement {
            let book = entry.book.clone();
            let cover = ShelfCover::new(SharedString::from(format!("{prefix}-{index}")), &entry.book).on_click(
                on_click(move |_, cx| {
                    let route = Route::Detail(book.clone());
                    AppState::global(cx).update(cx, |state, cx| state.push(route, cx));
                }),
            );
            match entry.caption {
                Some(caption) => cover.caption(caption, entry.caption_color).into_any_element(),
                None => cover.into_any_element(),
            }
        }))
}

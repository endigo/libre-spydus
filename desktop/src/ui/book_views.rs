//! Book cards (iOS `BookViews.swift`): a shelf cover with a caption, and the Your Books-style row
//! card on `surface`.

use gpui_kit::{
    AnyElement, App, CursorStyle, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};

use crate::state::BookRef;
use crate::state::formatting::byline;
use crate::theme::{self, Font, colors, cover, radius, spacing};
use crate::ui::controls::{Chip, OnClick};
use crate::ui::cover::CoverImage;
use crate::ui::style::FontExt;

/// A shelf item: the cover with a one- or two-line caption under it (Home shelves, the My Picks
/// grid, the book page's carousels). `caption` replaces the title line, e.g. "3 days left".
#[derive(IntoElement)]
pub struct ShelfCover {
    id: ElementId,
    book: BookRef,
    caption: Option<SharedString>,
    caption_color: Hsla,
    width: f32,
    shows_title: bool,
    on_sheet: bool,
    action: Option<OnClick>,
}

impl ShelfCover {
    pub fn new(id: impl Into<ElementId>, book: &BookRef) -> Self {
        Self {
            id: id.into(),
            book: book.clone(),
            caption: None,
            caption_color: theme::color(colors::TEXT_SECONDARY),
            width: cover::SHELF_WIDTH,
            shows_title: false,
            on_sheet: false,
            action: None,
        }
    }

    pub fn caption(mut self, caption: impl Into<SharedString>, color: Option<Hsla>) -> Self {
        self.caption = Some(caption.into());
        if let Some(color) = color {
            self.caption_color = color;
        }
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn shows_title(mut self, shows: bool) -> Self {
        self.shows_title = shows;
        self
    }

    /// Text colours for the white sheet.
    pub fn on_sheet(mut self, on_sheet: bool) -> Self {
        self.on_sheet = on_sheet;
        self
    }

    pub fn on_click(mut self, action: OnClick) -> Self {
        self.action = Some(action);
        self
    }
}

impl RenderOnce for ShelfCover {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let title_color = theme::color(if self.on_sheet { colors::ON_SHEET } else { colors::TEXT_PRIMARY });
        let action = self.action.clone();
        div()
            .id(self.id.clone())
            .test_support()
            .role(Role::Button)
            .aria_label(SharedString::from(self.book.title.clone()))
            .flex_none()
            .w(px(self.width))
            .flex()
            .flex_col()
            .gap(px(spacing::S))
            .when_some(action, |card, action| {
                card.cursor(CursorStyle::PointingHand).on_click(move |_, window, cx| action(window, cx))
            })
            .child(
                CoverImage::new(
                    SharedString::from(format!("{}-cover", self.id)),
                    self.book.cover_url.clone(),
                    self.book.title.clone(),
                )
                .author(self.book.author.clone())
                .width(self.width),
            )
            .when(self.shows_title, |card| {
                card.child(
                    div()
                        .typography(Font::BodyStrong)
                        .text_color(title_color)
                        .line_clamp(2)
                        .text_ellipsis()
                        .child(self.book.title.clone()),
                )
            })
            .when_some(self.caption, |card, caption| {
                card.child(div().typography(Font::Caption).text_color(self.caption_color).truncate().child(caption))
            })
    }
}

/// A Your Books-style card on `surface`: cover, title, "By author", chips, a meta line, and a
/// trailing slot (the "…" menu, text actions). Also fits search results.
#[derive(IntoElement)]
pub struct BookRowCard {
    id: ElementId,
    book: BookRef,
    meta: Option<SharedString>,
    meta_color: Hsla,
    chips: Vec<Chip>,
    trailing: Option<AnyElement>,
    highlighted: bool,
    action: Option<OnClick>,
}

impl BookRowCard {
    pub fn new(id: impl Into<ElementId>, book: &BookRef) -> Self {
        Self {
            id: id.into(),
            book: book.clone(),
            meta: None,
            meta_color: theme::color(colors::TEXT_SECONDARY),
            chips: Vec::new(),
            trailing: None,
            highlighted: false,
            action: None,
        }
    }

    /// "Due 27 Oct · 3 days left", "Ready to collect · Riverton", "2021".
    pub fn meta(mut self, meta: impl Into<SharedString>, color: Option<Hsla>) -> Self {
        self.meta = Some(meta.into());
        if let Some(color) = color {
            self.meta_color = color;
        }
        self
    }

    pub fn chips(mut self, chips: Vec<Chip>) -> Self {
        self.chips = chips;
        self
    }

    pub fn trailing(mut self, trailing: impl IntoElement) -> Self {
        self.trailing = Some(trailing.into_any_element());
        self
    }

    /// Ready-to-collect reservations: an accent outline.
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    pub fn on_click(mut self, action: OnClick) -> Self {
        self.action = Some(action);
        self
    }
}

impl RenderOnce for BookRowCard {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let action = self.action.clone();
        div()
            .id(self.id.clone())
            .test_support()
            .role(Role::Group)
            .aria_label(SharedString::from(self.book.title.clone()))
            .w_full()
            // Fills its grid cell, so the cards in a row line up.
            .h_full()
            .flex()
            .flex_row()
            .items_start()
            .gap(px(spacing::L))
            .p(px(spacing::L))
            .rounded(px(radius::CARD))
            .bg(theme::color(if self.highlighted { colors::SURFACE_ELEVATED } else { colors::SURFACE }))
            .when(self.highlighted, |card| card.border_2().border_color(theme::color(colors::ACCENT)))
            .when_some(action, |card, action| {
                card.cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(theme::color(colors::SURFACE_ELEVATED)))
                    .on_click(move |_, window, cx| action(window, cx))
            })
            .child(
                CoverImage::new(
                    SharedString::from(format!("{}-cover", self.id)),
                    self.book.cover_url.clone(),
                    self.book.title.clone(),
                )
                .author(self.book.author.clone())
                .width(cover::SMALL_WIDTH),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .typography(Font::BodyStrong)
                            .text_color(theme::color(colors::TEXT_PRIMARY))
                            .line_clamp(2)
                            .text_ellipsis()
                            .child(self.book.title.clone()),
                    )
                    .children(byline(self.book.author.as_deref()).map(|line| {
                        div()
                            .typography(Font::Caption)
                            .text_color(theme::color(colors::TEXT_SECONDARY))
                            .truncate()
                            .child(line)
                    }))
                    .when(!self.chips.is_empty(), |column| {
                        column.child(
                            div()
                                .pt(px(spacing::XS))
                                .flex()
                                .flex_row()
                                .flex_wrap()
                                .gap(px(spacing::XS))
                                .children(self.chips),
                        )
                    })
                    .when_some(self.meta, |column, meta| {
                        column.child(
                            div()
                                .pt(px(spacing::XS))
                                .typography(Font::Caption)
                                .text_color(self.meta_color)
                                .line_clamp(2)
                                .text_ellipsis()
                                .child(meta),
                        )
                    }),
            )
            .children(self.trailing)
    }
}

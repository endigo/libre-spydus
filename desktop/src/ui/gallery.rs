//! Every shared component on one screen (debug `--route gallery`), like iOS `ComponentsGallery`.
//! The first cover loads a real Syndetics cover through the app's HTTP client.

use gpui_kit::component::input::InputState;
use gpui_kit::{
    AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, px,
};

use crate::state::error::ErrorKind;
use crate::state::{BookRef, FormatRef, UserFacingError};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::style::{FontExt, caption_label, content_column};
use crate::ui::*;

pub struct GalleryScreen {
    query: Entity<InputState>,
    bookmarked: bool,
}

/// A real cover on Canning's Syndetics account (public).
pub const SAMPLE_COVER: &str =
    "https://secure.syndetics.com/index.aspx?isbn=9781399197809/lc.gif&client=citycanning&upc=&oclc=&issn=";

impl GalleryScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(crate::copy::Copy::Search::placeholder)
                .default_value("harry potter")
        });
        Self { query, bookmarked: true }
    }

    fn sample() -> BookRef {
        let mut book = BookRef::new("brn:467086", "The long night");
        book.author = Some("White, Christian".into());
        book.cover_url = Some(SAMPLE_COVER.into());
        book.year = Some("2026".into());
        book.formats = vec![
            FormatRef { code: "BK".into(), label: Some("Book".into()), reserve_url: None, availability_url: None },
            FormatRef { code: "EBK".into(), label: Some("eBook".into()), reserve_url: None, availability_url: None },
        ];
        book
    }

    fn group(title: &str, content: impl IntoElement) -> impl IntoElement {
        div().flex().flex_col().gap(px(spacing::M)).child(caption_label(title.to_uppercase())).child(content)
    }
}

impl Render for GalleryScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sample = Self::sample();
        let no_cover = {
            let mut book = BookRef::new("title:gallery", "Thinking about water");
            book.author = Some("Bhatnagar, Nirat".into());
            book
        };
        let named = {
            let mut book = BookRef::new("title:words", "More than words");
            book.author = Some("Santopolo, Jill".into());
            book
        };
        let bookmarked = self.bookmarked;
        let entity = cx.entity();
        let error = UserFacingError::new(
            ErrorKind::Network,
            "Can’t reach City of Canning. Check your connection.",
            Some("network error: connect timed out after 10s".into()),
        );
        div().id("gallery").size_full().overflow_y_scroll().child(
            content_column().py(px(spacing::XL)).flex().flex_col().gap(px(spacing::XL)).children([
                Self::group(
                    "SearchField / SearchFieldButton",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::M))
                        .child(SearchField::new(&self.query))
                        .child(SearchFieldButton::new("gallery-search-button", on_click(|_, _| {}))),
                )
                .into_any_element(),
                Self::group(
                    "SectionHeader · ShelfCover · CoverImage",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::M))
                        .child(SectionHeader::new("gallery-view-all", "New arrivals").action(None, on_click(|_, _| {})))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(spacing::L))
                                .child(ShelfCover::new("gallery-shelf-1", &sample).caption("3 days left", None))
                                .child(ShelfCover::new("gallery-shelf-2", &no_cover).caption("Due today", Some(theme::color(colors::DANGER))))
                                .child(ShelfCover::new("gallery-shelf-3", &named).shows_title(true))
                                .child(CoverImage::new("gallery-detail-cover", Some(SAMPLE_COVER.into()), "The long night").width(theme::cover::DETAIL_WIDTH)),
                        ),
                )
                .into_any_element(),
                Self::group(
                    "GenreTile",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(spacing::S))
                        .child(GenreTile::new("gallery-genre-1", "Biography", Lucide::User).color(theme::color(colors::GENRE[0])))
                        .child(GenreTile::new("gallery-genre-2", "Fiction", Lucide::WandSparkles).color(theme::color(colors::GENRE[1])))
                        .child(GenreTile::new("gallery-genre-3", "Science", Lucide::Atom).color(theme::color(colors::GENRE[2])))
                        .child(GenreTile::new("gallery-genre-4", "History", Lucide::Landmark).color(theme::color(colors::GENRE[3]))),
                )
                .into_any_element(),
                Self::group(
                    "BookRowCard · CardMenu",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::M))
                        .child(BookRowCard::new("gallery-row-1", &sample).meta("Due 14 Oct · 2 days left", Some(theme::color(colors::DANGER))))
                        .child(with_context_menu(
                            div().id("gallery-row-2-menu").child(
                                BookRowCard::new("gallery-row-2", &no_cover)
                                    .meta("Ready to collect · Riverton", None)
                                    .chips(sample.formats.iter().map(|format| Chip::format(format, false)).collect())
                                    .highlighted(true)
                                    .trailing(card_menu_button(
                                        "gallery-row-2-more",
                                        vec![CardAction::new("Edit reservation", |_, _| {}), CardAction::new("Cancel reservation", |_, _| {})],
                                    )),
                            ),
                            vec![CardAction::new("Edit reservation", |_, _| {}), CardAction::new("Cancel reservation", |_, _| {})],
                        )),
                )
                .into_any_element(),
                Self::group(
                    "Chip · Banner",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::M))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_wrap()
                                .gap(px(spacing::S))
                                .child(Chip::new("Book"))
                                .child(Chip::format(&sample.formats[1], false))
                                .child(Chip::new("Selected").tone(ChipTone::Selected))
                                .child(Chip::new("Available").tone(ChipTone::Success))
                                .child(Chip::new("Reserved").tone(ChipTone::Warning))
                                .child(Chip::new("Overdue").tone(ChipTone::Danger)),
                        )
                        .child(Banner::new("Reservation placed", BannerTone::Success))
                        .child(Banner::new("You already have a reservation for this title.", BannerTone::Error)),
                )
                .into_any_element(),
                Self::group(
                    "PrimaryPillButton · SecondaryPillButton · CircleIconButton",
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .gap(px(spacing::L))
                        .child(PrimaryPillButton::new("gallery-reserve", "Reserve").on_click(on_click(|_, _| {})))
                        .child(PrimaryPillButton::new("gallery-loading", "Signing in").loading(true))
                        .child(PrimaryPillButton::new("gallery-disabled", "Disabled").disabled(true))
                        .child(SecondaryPillButton::new("gallery-secondary", "Not now").on_click(on_click(|_, _| {})))
                        .child(
                            CircleIconButton::new(
                                "gallery-bookmark",
                                if bookmarked { Lucide::BookmarkCheck } else { Lucide::Bookmark },
                                "Save to My Picks",
                            )
                            .on_click(on_click(move |_, cx| {
                                entity.update(cx, |gallery, cx| {
                                    gallery.bookmarked = !gallery.bookmarked;
                                    cx.notify();
                                })
                            })),
                        ),
                )
                .into_any_element(),
                Self::group(
                    "Detail sheet surface",
                    div()
                        .p(px(spacing::XL))
                        .rounded(px(radius::SHEET))
                        .bg(theme::color(colors::SHEET))
                        .flex()
                        .flex_col()
                        .gap(px(spacing::M))
                        .child(SectionHeader::new("gallery-sheet-header", "Similar books").on_sheet(true).action(None, on_click(|_, _| {})))
                        .child(div().typography(Font::Body).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child("Synopsis text on the white sheet."))
                        .child(div().flex().flex_row().gap(px(spacing::S)).child(Chip::new("Fiction").tone(ChipTone::OnSheet)).child(Chip::new("Thriller").tone(ChipTone::OnSheet))),
                )
                .into_any_element(),
                Self::group(
                    "LoadingView · EmptyStateView · ErrorStateView",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(spacing::L))
                        .child(div().w(px(220.)).h(px(260.)).rounded(px(radius::CARD)).bg(theme::color(colors::SURFACE)).child(LoadingView::new().message("Loading…")))
                        .child(
                            div().w(px(320.)).h(px(260.)).rounded(px(radius::CARD)).bg(theme::color(colors::SURFACE)).child(
                                EmptyStateView::new(Lucide::Search, "No results for “zzzz”").message("Try another title or author."),
                            ),
                        )
                        .child(
                            div().w(px(380.)).h(px(320.)).rounded(px(radius::CARD)).bg(theme::color(colors::SURFACE)).child(
                                ErrorStateView::new("gallery-error", &error).retry(on_click(|_, _| {})),
                            ),
                        ),
                )
                .into_any_element(),
                Self::group(
                    "Toast · ConfirmDialog",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(spacing::L))
                        .child(SecondaryPillButton::new("gallery-toast", "Show a toast").on_click(on_click(|window, cx| {
                            toast::success(window, cx, "Saved to My Picks")
                        })))
                        .child(SecondaryPillButton::new("gallery-confirm", "Show a dialog").on_click(on_click(|window, cx| {
                            ConfirmDialog::new("Cancel this reservation?", "Cancel reservation", on_click(|_, _| {}))
                                .message("“Whistler” will be removed from your reservations and you’ll lose your place in the queue.")
                                .cancel_label("Keep reservation")
                                .destructive(true)
                                .open(window, cx)
                        }))),
                )
                .into_any_element(),
                Self::group(
                    "Type",
                    div().flex().flex_col().gap(px(spacing::S)).children(
                        Font::ALL.map(|font| div().typography(font).text_color(theme::color(colors::TEXT_PRIMARY)).child(font.key())),
                    ),
                )
                .into_any_element(),
            ]),
        )
    }
}

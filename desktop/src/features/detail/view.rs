//! The book page's drawing: the navy panel (cover, title, availability, format chips, Reserve or
//! the held pill, the bookmark) and the white sheet (synopsis, subjects, branch holdings, details,
//! carousels). Side by side in a wide window, stacked below `WIDE_LAYOUT_MIN_WIDTH`.

use gpui_kit::{
    AnyElement, ClickEvent, Context, CursorStyle, InteractiveElement, IntoElement, ParentElement, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, prelude::FluentBuilder, px,
};
use librespydus_core::{RecordDetail, SearchSort};

use crate::copy::Copy;
use crate::state::formatting::{author_name, byline, non_empty};
use crate::state::{AppState, Loadable, Route};
use crate::theme::{self, Font, colors, cover, radius, spacing};
use crate::ui::{
    Chip, ChipTone, CircleIconButton, CoverImage, FontExt, Lucide, PrimaryPillButton, SectionHeader, ShelfCover, icon,
    on_click,
};

use super::logic::{self, AvailabilityTone, DetailFact, RelatedPanel, SubjectChip, display_title};
use super::sheet::{SheetError, SheetLoading, skeleton_lines};
use super::{DetailScreen, HeaderAvailability, HeldStatus, HeldTone, HoldingRow};

/// Narrower windows put the sheet under the panel (docs/desktop-app.md §6).
pub const WIDE_LAYOUT_MIN_WIDTH: f32 = 1100.;
/// The panel's width beside the sheet.
const PANEL_WIDTH: f32 = 400.;
/// The page's widest extent.
const PAGE_MAX_WIDTH: f32 = 1160.;
/// A synopsis longer than this is clamped, with More / Less.
const SYNOPSIS_CLAMP_CHARS: usize = 420;

impl Render for DetailScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let wide = window.viewport_size().width >= px(WIDE_LAYOUT_MIN_WIDTH);
        let panel = self.panel(cx).into_any_element();
        let sheet = self.sheet(wide, cx).into_any_element();
        let root = div()
            .id("detail-screen")
            .test_support()
            .role(Role::Main)
            .aria_label(SharedString::from(self.shown.title.clone()))
            .size_full()
            .overflow_y_scroll()
            .bg(theme::color(colors::BACKGROUND));
        if wide {
            root.child(
                div()
                    .w_full()
                    .max_w(px(PAGE_MAX_WIDTH))
                    .mx_auto()
                    .px(px(spacing::XL))
                    .pt(px(spacing::S))
                    .pb(px(spacing::XXL))
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(spacing::XL))
                    .child(div().w(px(PANEL_WIDTH)).flex_none().child(panel))
                    .child(div().flex_1().min_w_0().child(sheet)),
            )
        } else {
            root.child(
                div()
                    .w_full()
                    .min_h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(theme::CONTENT_MAX_WIDTH))
                            .mx_auto()
                            .px(px(spacing::XL))
                            .pt(px(spacing::S))
                            .pb(px(spacing::XL))
                            .child(panel),
                    )
                    .child(div().w_full().flex_1().flex().flex_col().child(sheet)),
            )
        }
    }
}

impl DetailScreen {
    // ---- The navy panel ----

    fn panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let book = self.shown.clone();
        let title_known = !book.title.trim().is_empty();
        let show_chips = self.formats.len() > 1 && !self.is_edition_level();
        div()
            .id("detail-panel")
            .flex()
            .flex_col()
            .gap(px(spacing::L))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(spacing::L))
                    .child(
                        CoverImage::new("detail-cover", book.cover_url.clone(), display_title(&book.title))
                            .author(book.author.clone())
                            .width(cover::DETAIL_WIDTH),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(spacing::S))
                            .when(title_known, |column| {
                                column.child(
                                    div()
                                        .id("detail-title")
                                        .test_support()
                                        .role(Role::Heading)
                                        .aria_label(SharedString::from(display_title(&book.title)))
                                        .typography(Font::Title)
                                        .text_color(theme::color(colors::TEXT_PRIMARY))
                                        .line_clamp(6)
                                        .child(display_title(&book.title)),
                                )
                            })
                            .when(!title_known && matches!(self.record, Loadable::Idle | Loadable::Loading), |column| {
                                column.child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .gap(px(spacing::S))
                                        .child(skeleton_bar(0.9, 20.))
                                        .child(skeleton_bar(0.6, 20.))
                                        .child(skeleton_bar(0.4, 12.)),
                                )
                            })
                            .children(byline(book.author.as_deref()).map(|line| {
                                div()
                                    .id("detail-byline")
                                    .test_support()
                                    .aria_label(SharedString::from(line.clone()))
                                    .typography(Font::Body)
                                    .text_color(theme::color(colors::TEXT_SECONDARY))
                                    .line_clamp(2)
                                    .child(line)
                            }))
                            .children(non_empty(book.year.as_deref()).map(|year| {
                                div().typography(Font::Caption).text_color(theme::color(colors::TEXT_MUTED)).child(year)
                            }))
                            .child(div().pt(px(spacing::XS)).child(self.availability_line(cx))),
                    ),
            )
            .when(show_chips, |panel| panel.child(self.format_chips(cx)))
            .child(self.actions(cx))
    }

    fn availability_line(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let base = |label: String| {
            div()
                .id("detail-availability")
                .test_support()
                .role(Role::Status)
                .aria_label(SharedString::from(label))
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(spacing::S))
        };
        match self.header_availability() {
            HeaderAvailability::Loading(text) => base(text.to_owned())
                .typography(Font::Caption)
                .text_color(theme::color(colors::TEXT_SECONDARY))
                .child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::TEXT_SECONDARY)))
                .child(text)
                .into_any_element(),
            HeaderAvailability::Line(line) => {
                let (name, hex) = match line.tone {
                    AvailabilityTone::Available => (Lucide::CircleCheck, colors::SUCCESS),
                    AvailabilityTone::Normal => (Lucide::LibraryBig, colors::TEXT_PRIMARY),
                    AvailabilityTone::Digital => (Lucide::Cloud, colors::DIGITAL),
                    AvailabilityTone::Muted => (Lucide::Info, colors::TEXT_SECONDARY),
                };
                base(Copy::Detail::availability(&line.text))
                    .typography(Font::BodyStrong)
                    .text_color(theme::color(hex))
                    .child(icon(name, px(16.), theme::color(hex)))
                    .child(line.text)
                    .into_any_element()
            }
            HeaderAvailability::AvailabilityFailed => self.retry_line(
                Copy::Detail::availabilityFailed,
                Copy::Detail::retryAvailabilityHint,
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    if let Some(url) =
                        this.selected_format().and_then(|format| non_empty(format.availability_url.as_deref()))
                    {
                        this.availability.remove(&url);
                    }
                    this.ensure_availability(cx);
                }),
            ),
            HeaderAvailability::LookupFailed => self.retry_line(
                Copy::Detail::lookupFailed,
                Copy::Detail::retryLookupHint,
                cx.listener(|this, _: &ClickEvent, _, cx| this.retry_lookup(cx)),
            ),
            HeaderAvailability::CheckWebsite => base(Copy::Detail::checkWebsite.to_owned())
                .typography(Font::Caption)
                .text_color(theme::color(colors::TEXT_SECONDARY))
                .child(icon(Lucide::Globe, px(14.), theme::color(colors::TEXT_SECONDARY)))
                .child(Copy::Detail::checkWebsite)
                .into_any_element(),
            HeaderAvailability::None => div().into_any_element(),
        }
    }

    fn retry_line(
        &self,
        text: &'static str,
        hint: &'static str,
        retry: impl Fn(&ClickEvent, &mut Window, &mut gpui_kit::App) + 'static,
    ) -> AnyElement {
        div()
            .id("detail-availability")
            .test_support()
            .role(Role::Status)
            .aria_label(SharedString::from(text))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S))
            .typography(Font::Caption)
            .child(div().text_color(theme::color(colors::TEXT_SECONDARY)).child(text))
            .child(
                div()
                    .id("detail-availability-retry")
                    .test_support()
                    .role(Role::Button)
                    .aria_label(SharedString::from(Copy::Common::retry))
                    .aria_description(SharedString::from(hint))
                    .cursor(CursorStyle::PointingHand)
                    .text_color(theme::color(colors::PRIMARY))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .on_click(retry)
                    .child(Copy::Common::retry),
            )
            .into_any_element()
    }

    fn format_chips(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("detail-formats")
            .test_support()
            .role(Role::TabList)
            .aria_label(SharedString::from(Copy::Detail::formats))
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(spacing::S))
            .children(self.formats.iter().enumerate().map(|(index, format)| {
                let selected = Some(index) == self.selected;
                div()
                    .id(SharedString::from(format!("detail-format-{index}")))
                    .test_support()
                    .role(Role::Tab)
                    .aria_label(SharedString::from(format.display_label()))
                    .aria_description(SharedString::from(Copy::Detail::formatHint))
                    .aria_selected(selected)
                    .cursor(CursorStyle::PointingHand)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select_format(index, cx)))
                    .child(Chip::format(format, selected))
            }))
    }

    fn actions(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let state = AppState::global(cx).read(cx);
        let signed_in = state.is_signed_in();
        let held = self.held(cx);
        let primary: AnyElement = if let Some(held) = held {
            self.held_pill(held, cx).into_any_element()
        } else if let Some(format) = self.reservable_format().cloned() {
            if self.awaiting_holding(cx) {
                // Held or not isn't known yet: keep the place, show nothing.
                div().flex_1().min_h(px(44.)).into_any_element()
            } else {
                let reserve = entity.clone();
                PrimaryPillButton::new("detail-reserve", Copy::Detail::reserve)
                    .fills_width(true)
                    .hint(reserve_hint(&format, signed_in))
                    .on_click(on_click(move |window, cx| reserve.update(cx, |this, cx| this.reserve(window, cx))))
                    .into_any_element()
            }
        } else {
            div().flex_1().into_any_element()
        };
        let picked = self.is_picked(cx);
        let waits = AppState::global(cx).read(cx).picks().is_active() && self.pick_irn().is_none();
        let toggle = entity.clone();
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::M))
            .child(div().flex_1().min_w_0().flex().child(primary))
            .child(
                CircleIconButton::new(
                    "detail-bookmark",
                    if picked { Lucide::BookmarkCheck } else { Lucide::Bookmark },
                    if picked { Copy::Picks::remove } else { Copy::Picks::save },
                )
                .disabled(waits)
                .on_click(on_click(move |_, cx| toggle.update(cx, |this, cx| this.toggle_pick(cx)))),
            )
    }

    /// Reserve's place when the patron already has the edition: a status, not a primary action. It
    /// asks to renew a renewable loan, else opens Your Books. A spinner while renewing.
    fn held_pill(&self, held: HeldStatus, cx: &mut Context<Self>) -> impl IntoElement {
        let tint = theme::color(match held.tone {
            HeldTone::Loan => colors::TEXT_PRIMARY,
            HeldTone::Overdue => colors::DANGER,
            HeldTone::Ready => colors::SUCCESS,
            HeldTone::Reserved => colors::WARNING,
        });
        let renewing = self.renewing;
        div()
            .id("detail-held")
            .test_support()
            .role(Role::Button)
            .aria_label(SharedString::from(held.label.clone()))
            .aria_description(SharedString::from(held.hint()))
            .flex_1()
            .min_w_0()
            .min_h(px(50.))
            .px(px(spacing::L))
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(spacing::S))
            .rounded(px(radius::PILL))
            .bg(theme::color(colors::SURFACE_ELEVATED))
            .border_1()
            .border_color(tint.opacity(0.35))
            .text_color(tint)
            .when(renewing, |pill| pill.opacity(0.8))
            .when(!renewing, |pill| {
                pill.cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(theme::color(colors::SURFACE)))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.tap_held(window, cx)))
            })
            .child(if renewing {
                gpui_kit::component::spinner::Spinner::new().color(tint).into_any_element()
            } else {
                icon(held.icon, px(16.), tint).into_any_element()
            })
            .child(div().typography(Font::BodyStrong).truncate().child(held.text))
    }

    // ---- The white sheet ----

    fn sheet(&mut self, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let content: Vec<AnyElement> = match self.record.clone() {
            Loadable::Idle | Loadable::Loading => vec![
                SheetLoading::new("detail-loading", Copy::Detail::loadingDetails).into_any_element(),
                skeleton_lines(&[0.35, 1.0, 0.95, 0.9, 0.6]).into_any_element(),
                skeleton_lines(&[0.25, 0.7, 0.5]).into_any_element(),
            ],
            Loadable::Failed(error) => vec![
                SheetError::new("detail-error", &error)
                    .retry(on_click({
                        let entity = cx.entity();
                        move |_, cx| entity.update(cx, |this, cx| this.load(cx))
                    }))
                    .into_any_element(),
            ],
            Loadable::Loaded(record) => self.loaded(&record, cx),
        };
        div()
            .id("detail-sheet")
            .w_full()
            .when(wide, |sheet| sheet.rounded(px(radius::SHEET)))
            .when(!wide, |sheet| sheet.flex_1().rounded_t(px(radius::SHEET)))
            .bg(theme::color(colors::SHEET))
            .text_color(theme::color(colors::ON_SHEET))
            .child(
                div()
                    .w_full()
                    .when(!wide, |inner| inner.max_w(px(theme::CONTENT_MAX_WIDTH)).mx_auto())
                    .p(px(spacing::XL))
                    .pb(px(spacing::XXL))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XL))
                    .children(content),
            )
    }

    fn loaded(&mut self, record: &RecordDetail, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut sections: Vec<AnyElement> = Vec::new();
        let summary = non_empty(record.summary.as_deref());
        let subjects = self.subjects();
        let facts = logic::facts(&record.details, logic::MAX_FACT_ROWS);
        let shows_holdings =
            self.holdings_source().is_some() && self.holdings.value().is_none_or(|rows| !rows.is_empty());
        if let Some(summary) = &summary {
            sections.push(self.synopsis(summary, cx).into_any_element());
        }
        if !subjects.is_empty() {
            sections.push(subject_chips(&subjects).into_any_element());
        }
        if shows_holdings {
            sections.push(self.holdings_list(cx).into_any_element());
        }
        if !facts.is_empty() {
            sections.push(facts_list(&facts).into_any_element());
        }
        let has_panels = self.has_related_panels();
        if has_panels {
            for panel in self.visible_panels() {
                sections.push(self.carousel(panel).into_any_element());
            }
        }
        if summary.is_none() && subjects.is_empty() && !shows_holdings && facts.is_empty() && !has_panels {
            sections.push(
                div()
                    .id("detail-no-more")
                    .test_support()
                    .aria_label(SharedString::from(Copy::Detail::noMoreDetails))
                    .typography(Font::Body)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .child(Copy::Detail::noMoreDetails)
                    .into_any_element(),
            );
        }
        sections
    }

    fn synopsis(&self, summary: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let long = summary.chars().count() > SYNOPSIS_CLAMP_CHARS;
        let expanded = self.synopsis_expanded;
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S))
            .child(SectionHeader::new("detail-synopsis", Copy::Detail::synopsis).on_sheet(true))
            .child(
                div()
                    .id("detail-summary")
                    .test_support()
                    .aria_label(SharedString::from(summary.to_owned()))
                    .typography(Font::Body)
                    .line_height(px(Font::Body.size_value() * 1.45))
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .child(if long && !expanded { clipped(summary, SYNOPSIS_CLAMP_CHARS) } else { summary.to_owned() }),
            )
            .when(long, |column| {
                column.child(
                    div()
                        .id("detail-synopsis-more")
                        .test_support()
                        .role(Role::Button)
                        .aria_label(SharedString::from(if expanded { Copy::Detail::less } else { Copy::Detail::more }))
                        .aria_description(SharedString::from(if expanded {
                            Copy::Detail::lessHint
                        } else {
                            Copy::Detail::moreHint
                        }))
                        .cursor(CursorStyle::PointingHand)
                        .typography(Font::BodyStrong)
                        .text_color(theme::color(colors::ACCENT))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_synopsis(cx)))
                        .child(if expanded { Copy::Detail::less } else { Copy::Detail::more }),
                )
            })
    }

    fn holdings_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body: AnyElement = match &self.holdings {
            Loadable::Idle | Loadable::Loading => div()
                .id("detail-copies-loading")
                .test_support()
                .role(Role::ProgressIndicator)
                .aria_label(SharedString::from(Copy::Detail::loadingCopies))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S))
                .py(px(spacing::S))
                .typography(Font::Caption)
                .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                .child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::ON_SHEET_SECONDARY)))
                .child(Copy::Detail::loadingCopies)
                .into_any_element(),
            Loadable::Failed(error) => div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S))
                .py(px(spacing::S))
                .typography(Font::Caption)
                .child(div().text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(Copy::Detail::copiesFailed))
                .when(error.can_retry(), |row| {
                    row.child(
                        div()
                            .id("detail-copies-retry")
                            .test_support()
                            .role(Role::Button)
                            .aria_label(SharedString::from(Copy::Common::retry))
                            .aria_description(SharedString::from(Copy::Detail::retryCopiesHint))
                            .cursor(CursorStyle::PointingHand)
                            .text_color(theme::color(colors::ACCENT))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.retry_holdings(cx)))
                            .child(Copy::Common::retry),
                    )
                })
                .into_any_element(),
            Loadable::Loaded(rows) => {
                divided(rows.iter().enumerate().map(|(index, row)| holding_row(index, row).into_any_element()))
                    .into_any_element()
            }
        };
        div()
            .id("detail-copies")
            .test_support()
            .role(Role::List)
            .aria_label(SharedString::from(Copy::Detail::copies))
            .flex()
            .flex_col()
            .gap(px(spacing::S))
            .child(SectionHeader::new("detail-copies-header", Copy::Detail::copies).on_sheet(true))
            .child(body)
    }

    fn carousel(&self, panel: RelatedPanel) -> impl IntoElement {
        let state = self.related.get(&panel);
        let loading = !matches!(state, Some(Loadable::Loaded(_)));
        let label =
            if loading { Copy::Detail::loadingPanel(&panel.title().to_lowercase()) } else { panel.title().to_owned() };
        let items: Vec<AnyElement> = match state {
            Some(Loadable::Loaded(books)) => books
                .iter()
                .enumerate()
                .map(|(index, book)| {
                    let mut shown = book.clone();
                    shown.title = display_title(&book.title);
                    let caption = author_name(book.author.as_deref()).or_else(|| non_empty(book.year.as_deref()));
                    let target = book.clone();
                    let card = ShelfCover::new(SharedString::from(format!("detail-{}-{index}", panel.key())), &shown)
                        .on_sheet(true)
                        .shows_title(true)
                        .width(cover::SHELF_WIDTH)
                        .on_click(on_click(move |_, cx| {
                            let route = Route::Detail(target.clone());
                            AppState::global(cx).update(cx, |state, cx| state.push(route, cx));
                        }));
                    match caption {
                        Some(caption) => card.caption(caption, Some(theme::color(colors::ON_SHEET_SECONDARY))),
                        None => card,
                    }
                    .into_any_element()
                })
                .collect(),
            _ => (0..5)
                .map(|_| {
                    div()
                        .flex_none()
                        .w(px(cover::SHELF_WIDTH))
                        .h(px(cover::height(cover::SHELF_WIDTH)))
                        .rounded(px(radius::COVER))
                        .bg(theme::color(colors::CHIP_ON_SHEET))
                        .into_any_element()
                })
                .collect(),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::M))
            .child(
                SectionHeader::new(SharedString::from(format!("detail-{}-header", panel.key())), panel.title())
                    .on_sheet(true),
            )
            .child(
                div()
                    .id(SharedString::from(format!("detail-carousel-{}", panel.key())))
                    .test_support()
                    .role(Role::List)
                    .aria_label(SharedString::from(label))
                    .w_full()
                    .overflow_x_scroll()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(spacing::L))
                    .pb(px(spacing::S))
                    .children(items),
            )
    }
}

/// "Reserves the book" / "Asks you to sign in, then reserves the book"; an edition-level item's
/// format has no name, so "this item".
fn reserve_hint(format: &crate::state::FormatRef, signed_in: bool) -> String {
    if format.is_edition_level() {
        return (if signed_in { Copy::Detail::reserveItemHint } else { Copy::Detail::signInToReserveItemHint })
            .to_owned();
    }
    let name = format.display_label().to_lowercase();
    if signed_in { Copy::Detail::reserveFormatHint(&name) } else { Copy::Detail::signInToReserveFormatHint(&name) }
}

fn skeleton_bar(width: f32, height: f32) -> impl IntoElement {
    div()
        .h(px(height))
        .w(gpui_kit::relative(width))
        .rounded(px(radius::PILL))
        .bg(theme::color(colors::SURFACE_ELEVATED))
}

fn subject_chips(subjects: &[SubjectChip]) -> impl IntoElement {
    div()
        .id("detail-subjects")
        .test_support()
        .role(Role::List)
        .aria_label(SharedString::from(Copy::Detail::subjects))
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(spacing::S))
        .children(subjects.iter().enumerate().map(|(index, subject)| {
            let route = Route::Results {
                query: subject.query.clone(),
                sort: SearchSort::Relevance,
                title: subject.label.clone(),
            };
            div()
                .id(SharedString::from(format!("detail-subject-{index}")))
                .test_support()
                .role(Role::Link)
                .aria_label(SharedString::from(subject.label.clone()))
                .aria_description(SharedString::from(Copy::Detail::subjectHint))
                .max_w(px(280.))
                .cursor(CursorStyle::PointingHand)
                .on_click(move |_, _, cx| {
                    let route = route.clone();
                    AppState::global(cx).update(cx, |state, cx| state.push(route, cx));
                })
                .child(Chip::new(subject.label.clone()).tone(ChipTone::OnSheet))
        }))
}

fn facts_list(facts: &[DetailFact]) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S))
        .child(SectionHeader::new("detail-details", Copy::Detail::details).on_sheet(true))
        .child(divided(facts.iter().map(|fact| {
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(spacing::M))
                .py(px(spacing::S))
                .child(
                    div()
                        .w(px(132.))
                        .flex_none()
                        .typography(Font::Caption)
                        .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                        .child(fact.label.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .typography(Font::Body)
                        .text_color(theme::color(colors::ON_SHEET))
                        .line_clamp(5)
                        .children(fact.value.lines().map(|line| div().child(line.to_owned()))),
                )
                .into_any_element()
        })))
}

fn holding_row(index: usize, row: &HoldingRow) -> impl IntoElement {
    let label = [Some(row.location.clone()), row.status.clone(), row.description.clone(), row.call_number.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(Copy::Common::separator);
    div()
        .id(SharedString::from(format!("detail-copy-{index}")))
        .test_support()
        .role(Role::ListItem)
        .aria_label(SharedString::from(label))
        .flex()
        .flex_row()
        .items_start()
        .justify_between()
        .gap(px(spacing::M))
        .py(px(spacing::S))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .typography(Font::BodyStrong)
                        .text_color(theme::color(colors::ON_SHEET))
                        .child(row.location.clone()),
                )
                .children(row.description.clone().map(|description| {
                    div().typography(Font::Body).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(description)
                }))
                .children(row.call_number.clone().map(|number| {
                    div().typography(Font::Caption).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(number)
                })),
        )
        .children(row.status.clone().map(|status| {
            div()
                .flex_none()
                .max_w(px(260.))
                .typography(Font::Body)
                .text_color(theme::color(if row.available { colors::SUCCESS } else { colors::ON_SHEET }))
                .child(status)
        }))
}

/// `text` cut at the last word break before `limit` characters, with an ellipsis.
fn clipped(text: &str, limit: usize) -> String {
    let cut: String = text.chars().take(limit).collect();
    let cut = cut.rfind(char::is_whitespace).map_or(cut.as_str(), |at| &cut[..at]);
    format!("{}…", cut.trim_end_matches(|c: char| c.is_whitespace() || ",;:".contains(c))) // copy-exempt: the ellipsis after a clipped synopsis
}

/// Rows with a hairline between them.
fn divided(rows: impl Iterator<Item = AnyElement>) -> impl IntoElement {
    let mut column = div().flex().flex_col();
    for (index, row) in rows.enumerate() {
        if index > 0 {
            column = column.child(div().h(px(1.)).w_full().bg(theme::color(colors::CHIP_ON_SHEET)));
        }
        column = column.child(row);
    }
    column
}

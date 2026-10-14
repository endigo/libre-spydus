//! "Edit reservation" (iOS `EditReservationSheet.swift`) as a dialog: loads the form
//! (`edit_options`), lets the reader change the pickup branch and the two dates, then sends
//! `edit_reservation`. A refusal (`ok: false`) stays in the dialog; success closes it and Your
//! Books shows the library's message.

use chrono::{Months, NaiveDate};
use gpui_kit::component::WindowExt;
use gpui_kit::component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use gpui_kit::component::switch::Switch;
use gpui_kit::{
    AppContext, Context, CursorStyle, Entity, InteractiveElement, IntoElement, ParentElement, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, Subscription, TestSupportExt, Toggled, WeakEntity, Window, div,
    prelude::FluentBuilder, px,
};
use librespydus_core::{EditOptions, EditOverrides, PickupChoice, ReserveResult, SpydusError};

use crate::copy::Copy;
use crate::state::formatting::{due_date, non_empty, today};
use crate::state::{AppState, Loadable};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{
    Banner, BannerTone, CoverImage, DialogButton, FontExt, Lucide, dialog_title, icon, on_click, sheet_dialog,
};

use super::YourBooksScreen;
use super::rows::{ReservationRow, display_name};

/// One of the form's two dates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateField {
    /// The form's value as the library wrote it ("2026-11-01"); `None` when unset.
    pub original: Option<String>,
    pub is_on: bool,
    pub date: NaiveDate,
}

impl DateField {
    pub fn new(original: Option<&str>, default_date: NaiveDate) -> Self {
        let original = non_empty(original);
        let parsed = original.as_deref().and_then(due_date::parse);
        Self { is_on: parsed.is_some(), date: parsed.unwrap_or(default_date), original }
    }

    /// The new value to send, or `None` when it's off or unchanged.
    pub fn changed_value(&self) -> Option<String> {
        if !self.is_on {
            return None;
        }
        let value = form_value(self.date, self.original.as_deref());
        (Some(&value) != self.original.as_ref()).then_some(value)
    }

    /// The form had a date and it was switched off.
    pub fn is_cleared(&self) -> bool {
        !self.is_on && self.original.is_some()
    }

    pub fn has_changes(&self) -> bool {
        self.is_cleared() || self.changed_value().is_some()
    }
}

/// `date` in the form's own format: "dd/mm/yyyy" when the form wrote dates that way, else the HTML
/// date input's "yyyy-mm-dd" (what Spydus's edit form uses).
pub fn form_value(date: NaiveDate, sample: Option<&str>) -> String {
    // copy-exempt: date formats sent to the library
    let format = if sample.is_some_and(|sample| sample.contains('/')) { "%d/%m/%Y" } else { "%Y-%m-%d" };
    date.format(format).to_string()
}

/// The form's state and the `EditOverrides` it sends (iOS `EditDraft`). A date is sent only when it
/// changed; switching off a date the form had sets the matching clear flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDraft {
    /// Selected `PickupChoice.code`; `None` when the form offers no branches (digital holds).
    pub pickup: Option<String>,
    pub initial_pickup: Option<String>,
    pub delay: DateField,
    pub expiry: DateField,
}

impl EditDraft {
    /// The pickup starts on the form's selection, else the branch the reservation lists, else the
    /// first choice.
    pub fn new(options: &EditOptions, current_pickup: Option<&str>, today: NaiveDate) -> Self {
        let choices = &options.pickup_choices;
        let selected = options
            .pickup_selected
            .as_ref()
            .and_then(|code| choices.iter().find(|choice| &choice.code == code))
            .map(|choice| choice.code.clone());
        let listed = current_pickup.and_then(|name| {
            choices
                .iter()
                .find(|choice| choice.name.eq_ignore_ascii_case(name.trim()))
                .map(|choice| choice.code.clone())
        });
        let pickup = selected.or(listed).or_else(|| choices.first().map(|choice| choice.code.clone()));
        Self {
            initial_pickup: pickup.clone(),
            pickup,
            delay: DateField::new(options.delay_until.as_deref(), today + chrono::Duration::days(7)),
            expiry: DateField::new(
                options.not_required_after.as_deref(),
                today.checked_add_months(Months::new(3)).unwrap_or(today),
            ),
        }
    }

    pub fn has_changes(&self) -> bool {
        self.pickup != self.initial_pickup || self.delay.has_changes() || self.expiry.has_changes()
    }

    /// What `edit_reservation` sends. The pickup is always the one shown, like the web app.
    pub fn overrides(&self) -> EditOverrides {
        EditOverrides {
            pickup: self.pickup.clone(),
            delay_until: self.delay.changed_value(),
            not_required_after: self.expiry.changed_value(),
            clear_delay: self.delay.is_cleared(),
            clear_expiry: self.expiry.is_cleared(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Which {
    Delay,
    Expiry,
}

/// The dialog's content: an entity so the load, the draft and the save live across renders.
pub struct EditReservationDialog {
    row: ReservationRow,
    screen: WeakEntity<YourBooksScreen>,
    options: Loadable<EditOptions>,
    draft: Option<EditDraft>,
    is_saving: bool,
    /// The library's refusal or a failed save, shown above the form.
    problem: Option<String>,
    delay_picker: Entity<DatePickerState>,
    expiry_picker: Entity<DatePickerState>,
    _subscriptions: Vec<Subscription>,
}

/// Opens the dialog for `row` (it must have an edit link).
pub fn open(row: ReservationRow, screen: WeakEntity<YourBooksScreen>, window: &mut Window, cx: &mut gpui_kit::App) {
    let view = cx.new(|cx| EditReservationDialog::new(row, screen, window, cx));
    window.open_dialog(cx, move |dialog, _, _| {
        sheet_dialog(dialog)
            .w(px(500.))
            .keyboard(true)
            .title(dialog_title(Copy::EditReservation::title))
            .child(view.clone())
    });
}

impl EditReservationDialog {
    fn new(
        row: ReservationRow,
        screen: WeakEntity<YourBooksScreen>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let start = today();
        let delay_picker = cx.new(|cx| {
            DatePickerState::new(window, cx)
                .date_format("%-d %b %Y")
                .disabled_matcher(move |date: &NaiveDate| *date < start)
        });
        let expiry_picker = cx.new(|cx| {
            DatePickerState::new(window, cx)
                .date_format("%-d %b %Y")
                .disabled_matcher(move |date: &NaiveDate| *date < start)
        });
        let subscriptions = vec![
            cx.subscribe_in(&delay_picker, window, |this, _, event: &DatePickerEvent, _, cx| {
                this.picked(Which::Delay, event, cx)
            }),
            cx.subscribe_in(&expiry_picker, window, |this, _, event: &DatePickerEvent, _, cx| {
                this.picked(Which::Expiry, event, cx)
            }),
        ];
        let mut this = Self {
            row,
            screen,
            options: Loadable::Idle,
            draft: None,
            is_saving: false,
            problem: None,
            delay_picker,
            expiry_picker,
            _subscriptions: subscriptions,
        };
        this.load(window, cx);
        this
    }

    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = AppState::global(cx);
        let (href, session, backend) = {
            let state = state.read(cx);
            (self.row.edit_href.clone(), state.session().map(str::to_owned), state.backend())
        };
        let (Some(href), Some(session)) = (href, session) else {
            window.close_dialog(cx);
            return;
        };
        self.options = Loadable::Loading;
        self.problem = None;
        let fetch = backend.edit_options(href, session);
        cx.spawn_in(window, async move |this, cx| {
            let result = fetch.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(options) => {
                        let current = this.row.item.fields.get("Pickup location").cloned(); // copy-exempt: a field name (a key)
                        let draft = EditDraft::new(&options, current.as_deref(), today());
                        this.delay_picker.update(cx, |picker, cx| picker.set_date(draft.delay.date, window, cx));
                        this.expiry_picker.update(cx, |picker, cx| picker.set_date(draft.expiry.date, window, cx));
                        this.draft = Some(draft);
                        this.options = Loadable::Loaded(options);
                    }
                    Err(error) => {
                        if needs_sign_in(&error) {
                            window.close_dialog(cx);
                        }
                        let shown = AppState::global(cx).update(cx, |state, cx| state.report(&error, cx));
                        this.options = Loadable::Failed(shown);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn picked(&mut self, which: Which, event: &DatePickerEvent, cx: &mut Context<Self>) {
        let DatePickerEvent::Change(value) = event;
        let (Some(draft), Some(date)) = (&mut self.draft, value.start().map(|value| value.date())) else { return };
        let field = match which {
            Which::Delay => &mut draft.delay,
            Which::Expiry => &mut draft.expiry,
        };
        field.date = date;
        field.is_on = true;
        cx.notify();
    }

    fn set_pickup(&mut self, code: String, cx: &mut Context<Self>) {
        if let Some(draft) = &mut self.draft {
            draft.pickup = Some(code);
            cx.notify();
        }
    }

    fn toggle(&mut self, which: Which, on: bool, cx: &mut Context<Self>) {
        if let Some(draft) = &mut self.draft {
            match which {
                Which::Delay => draft.delay.is_on = on,
                Which::Expiry => draft.expiry.is_on = on,
            }
            cx.notify();
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.draft.clone().filter(|draft| draft.has_changes()) else { return };
        if self.is_saving {
            return;
        }
        let state = AppState::global(cx);
        let (Some(href), Some(session)) = (self.row.edit_href.clone(), state.read(cx).session().map(str::to_owned))
        else {
            window.close_dialog(cx);
            return;
        };
        self.is_saving = true;
        self.problem = None;
        let call = state.read(cx).backend().edit_reservation(href, session, draft.overrides());
        // An accepted change fetches the account again (Your Books shows the new pickup).
        let change = state.update(cx, |state, cx| state.perform_account_change(call, |result| result.ok, cx));
        cx.spawn_in(window, async move |this, cx| {
            let result = change.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.is_saving = false;
                match result {
                    Ok(result) if result.ok => {
                        window.close_dialog(cx);
                        this.finish(Some(result), cx);
                    }
                    Ok(result) => {
                        this.problem = Some(
                            non_empty(result.message.as_deref())
                                .unwrap_or_else(|| Copy::EditReservation::changeFailed.to_owned()),
                        );
                    }
                    Err(error) => {
                        if needs_sign_in(&error) {
                            window.close_dialog(cx);
                        }
                        let shown = AppState::global(cx).update(cx, |state, cx| state.report(&error, cx));
                        this.problem = Some(shown.message);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn finish(&self, result: Option<ReserveResult>, cx: &mut Context<Self>) {
        let row_id = self.row.id.clone();
        let _ = self.screen.update(cx, |screen, cx| screen.finish_edit(&row_id, result, cx));
    }

    fn book_header(&self) -> impl IntoElement {
        let row = &self.row;
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::M))
            .p(px(spacing::M))
            .rounded(px(radius::FIELD))
            .bg(theme::color(colors::CHIP_ON_SHEET))
            .child(
                CoverImage::new("edit-cover", row.book.cover_url.clone(), row.book.title.clone())
                    .author(row.book.author.clone())
                    .width(44.)
                    .shadow(false),
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
                            .text_color(theme::color(colors::ON_SHEET))
                            .line_clamp(2)
                            .child(row.book.title.clone()),
                    )
                    .children(crate::state::formatting::byline(row.book.author.as_deref()).map(|line| {
                        div()
                            .typography(Font::Caption)
                            .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                            .truncate()
                            .child(line)
                    }))
                    .child(
                        div()
                            .typography(Font::Caption)
                            .text_color(theme::color(if row.is_ready() {
                                colors::SUCCESS
                            } else {
                                colors::ON_SHEET_SECONDARY
                            }))
                            .child(row.status_label()),
                    ),
            )
    }

    fn section_title(title: &'static str) -> impl IntoElement {
        div().typography(Font::Caption).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(title)
    }

    fn footnote(text: String) -> impl IntoElement {
        div().typography(Font::Caption).text_color(theme::color(colors::ON_SHEET_SECONDARY)).child(text)
    }

    fn pickup_list(&self, choices: &[PickupChoice], cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.draft.as_ref().and_then(|draft| draft.pickup.clone());
        div()
            .id("edit-pickup")
            .role(Role::RadioGroup)
            .aria_label(SharedString::from(Copy::EditReservation::collectFrom))
            .flex()
            .flex_col()
            .max_h(px(220.))
            .overflow_y_scroll()
            .rounded(px(radius::FIELD))
            .bg(theme::color(colors::CHIP_ON_SHEET))
            .children(choices.iter().map(|choice| {
                let checked = selected.as_deref() == Some(choice.code.as_str());
                let name = display_name(Some(&choice.name)).unwrap_or_else(|| choice.name.clone());
                let code = choice.code.clone();
                div()
                    .id(SharedString::from(format!("edit-pickup-{}", choice.code)))
                    .test_support()
                    .role(Role::RadioButton)
                    .aria_label(SharedString::from(name.clone()))
                    .aria_toggled(if checked { Toggled::True } else { Toggled::False })
                    .cursor(CursorStyle::PointingHand)
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(spacing::M))
                    .py(px(spacing::S + 2.))
                    .hover(|style| style.bg(theme::color_alpha(colors::ON_SHEET_SECONDARY, 0.12)))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_pickup(code.clone(), cx)))
                    .child(
                        div()
                            .typography(if checked { Font::BodyStrong } else { Font::Body })
                            .text_color(theme::color(colors::ON_SHEET))
                            .child(name),
                    )
                    .when(checked, |item| item.child(icon(Lucide::Check, px(16.), theme::color(colors::ACCENT))))
            }))
    }

    fn date_section(&self, which: Which, cx: &mut Context<Self>) -> impl IntoElement {
        let (title, footer, id, picker) = match which {
            Which::Delay => (
                Copy::EditReservation::delayUntil,
                Copy::EditReservation::delayFooter,
                "edit-delay",
                &self.delay_picker,
            ),
            Which::Expiry => (
                Copy::EditReservation::notNeededAfter,
                Copy::EditReservation::notNeededFooter,
                "edit-expiry",
                &self.expiry_picker,
            ),
        };
        let field = self.draft.as_ref().map(|draft| match which {
            Which::Delay => draft.delay.clone(),
            Which::Expiry => draft.expiry.clone(),
        });
        let is_on = field.as_ref().is_some_and(|field| field.is_on);
        let cleared = field.as_ref().is_some_and(DateField::is_cleared);
        let footnote = if cleared { Copy::EditReservation::clearedFooter(footer) } else { footer.to_owned() };
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::M))
                    .child(div().typography(Font::Body).text_color(theme::color(colors::ON_SHEET)).child(title))
                    .child(
                        div().id(SharedString::from(format!("{id}-switch"))).test_support().child(
                            Switch::new(SharedString::from(format!("{id}-toggle")))
                                .checked(is_on)
                                .color(theme::color(colors::ACCENT))
                                .accessibility_label(title)
                                .on_click(
                                    cx.listener(move |this, checked: &bool, _, cx| this.toggle(which, *checked, cx)),
                                ),
                        ),
                    ),
            )
            .when(is_on, |section| {
                section.child(
                    div()
                        .id(SharedString::from(format!("{id}-date")))
                        .test_support()
                        .aria_label(SharedString::from(Copy::EditReservation::dateLabel(title)))
                        .rounded(px(radius::FIELD))
                        .bg(theme::color(colors::CHIP_ON_SHEET))
                        .text_color(theme::color(colors::ON_SHEET))
                        .child(DatePicker::new(picker).appearance(false).placeholder(Copy::EditReservation::setDate)),
                )
            })
            .child(Self::footnote(footnote))
    }
}

fn needs_sign_in(error: &SpydusError) -> bool {
    matches!(error, SpydusError::NotLoggedIn | SpydusError::HumanVerificationRequired)
}

impl Render for EditReservationDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = div().id("edit-reservation").test_support().flex().flex_col().gap(px(spacing::L)).w_full();
        match self.options.clone() {
            Loadable::Idle | Loadable::Loading => body.child(
                div()
                    .h(px(180.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(spacing::M))
                    .child(gpui_kit::component::spinner::Spinner::new().color(theme::color(colors::ON_SHEET_SECONDARY)))
                    .child(
                        div()
                            .typography(Font::Body)
                            .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                            .child(Copy::EditReservation::loading),
                    ),
            ),
            Loadable::Failed(error) => body
                .child(self.book_header())
                .child(Banner::new(error.message.clone(), BannerTone::Error))
                .when(error.can_retry(), |body| {
                    body.child(div().flex().flex_row().justify_end().child(DialogButton::new(
                        "edit-retry",
                        Copy::Common::retry,
                        on_click({
                            let view = cx.entity();
                            move |window, cx| view.update(cx, |this, cx| this.load(window, cx))
                        }),
                    )))
                }),
            Loadable::Loaded(options) if !options.can_edit || self.draft.is_none() => {
                body.child(self.book_header()).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::XS))
                        .child(
                            div()
                                .typography(Font::BodyStrong)
                                .text_color(theme::color(colors::ON_SHEET))
                                .child(Copy::EditReservation::cantEditTitle),
                        )
                        .child(Self::footnote(
                            non_empty(options.message.as_deref())
                                .unwrap_or_else(|| Copy::EditReservation::cantEditMessage.to_owned()),
                        )),
                )
            }
            Loadable::Loaded(options) => {
                let can_save = self.draft.as_ref().is_some_and(EditDraft::has_changes) && !self.is_saving;
                let view = cx.entity();
                body.child(self.book_header())
                    .when_some(self.problem.clone(), |body, problem| {
                        body.child(
                            div()
                                .id("edit-problem")
                                .test_support()
                                .p(px(spacing::M))
                                .rounded(px(radius::FIELD))
                                .bg(theme::color_alpha(colors::DANGER, 0.12))
                                .text_color(theme::color(colors::ON_SHEET))
                                .typography(Font::Body)
                                .child(problem),
                        )
                    })
                    .when(!options.pickup_choices.is_empty(), |body| {
                        body.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(spacing::S))
                                .child(Self::section_title(Copy::EditReservation::collectFrom))
                                .child(self.pickup_list(&options.pickup_choices, cx)),
                        )
                    })
                    .child(self.date_section(Which::Delay, cx))
                    .child(self.date_section(Which::Expiry, cx))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .items_center()
                            .gap(px(spacing::S))
                            .when(self.is_saving, |row| {
                                row.child(
                                    gpui_kit::component::spinner::Spinner::new()
                                        .color(theme::color(colors::ON_SHEET_SECONDARY)),
                                )
                            })
                            .child(DialogButton::new(
                                "edit-close",
                                Copy::Common::close,
                                on_click(|window, cx| window.close_dialog(cx)),
                            ))
                            .child(
                                div().when(!can_save, |button| button.opacity(0.45)).child(
                                    DialogButton::new(
                                        "edit-save",
                                        Copy::EditReservation::saveChanges,
                                        on_click(move |window, cx| {
                                            view.update(cx, |this, cx| this.save(window, cx));
                                        }),
                                    )
                                    .confirming(true),
                                ),
                            ),
                    )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn options() -> EditOptions {
        EditOptions {
            can_edit: true,
            pickup_choices: vec![
                PickupChoice { code: "1".into(), name: "CANNINGTON LIBRARY".into() },
                PickupChoice { code: "2".into(), name: "RIVERTON LIBRARY".into() },
            ],
            pickup_selected: None,
            delay_until: Some("2026-11-01".into()),
            not_required_after: None,
            message: None,
        }
    }

    #[test]
    fn the_draft_starts_on_the_listed_branch_and_sends_only_changes() {
        let today = day(2026, 10, 14);
        let mut draft = EditDraft::new(&options(), Some("Riverton Library"), today);
        assert_eq!(draft.pickup.as_deref(), Some("2"));
        assert!(draft.delay.is_on && !draft.expiry.is_on);
        assert_eq!(draft.delay.date, day(2026, 11, 1));
        assert_eq!(draft.expiry.date, day(2027, 1, 14));
        assert!(!draft.has_changes());
        assert_eq!(draft.overrides(), EditOverrides { pickup: Some("2".into()), ..Default::default() });

        draft.pickup = Some("1".into());
        draft.delay.is_on = false;
        draft.expiry.is_on = true;
        assert!(draft.has_changes());
        assert_eq!(
            draft.overrides(),
            EditOverrides {
                pickup: Some("1".into()),
                delay_until: None,
                not_required_after: Some("2027-01-14".into()),
                clear_delay: true,
                clear_expiry: false,
            }
        );
        assert_eq!(form_value(day(2026, 3, 4), Some("01/02/2026")), "04/03/2026");
        let first = EditDraft::new(&EditOptions { pickup_selected: Some("9".into()), ..options() }, None, today);
        assert_eq!(first.pickup.as_deref(), Some("1"));
    }
}

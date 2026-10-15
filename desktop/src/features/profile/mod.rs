//! Profile (D2; docs/native-apps.md §6, docs/desktop-app.md §6; iOS `ProfileView.swift`): a
//! settings-style page with the library card and Change library, the account (Sign in / Sign out),
//! human verification for a library that needed it, About, and (debug builds) the developer tools.
//! v1 has no Face ID switch and no alert switches.

use gpui_kit::{
    AnyElement, App, Context, CursorStyle, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, Render,
    Role, SharedString, StatefulInteractiveElement, Styled, Subscription, TestSupportExt, Window, div,
    prelude::FluentBuilder, px,
};
use librespydus_core::LibraryInfo;

use crate::copy::Copy;
use crate::features::onboarding::LibraryLogo;
use crate::features::onboarding::directory::{self, Region};
use crate::state::AppState;
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{Chip, ConfirmDialog, EmptyStateView, FontExt, Lucide, OnClick, PrimaryPillButton, icon, on_click};

pub struct ProfileScreen {
    _observe: Subscription,
}

impl ProfileScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&AppState::global(cx), |_, _, cx| cx.notify());
        #[cfg(feature = "evidence")]
        if crate::features::onboarding::evidence_step("profile").as_deref() == Some("sign-out-confirm") {
            // Screenshots: the sign-out confirmation, once the screen is up.
            cx.spawn_in(window, async move |_, cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
                let _ = cx.update(confirm_sign_out);
            })
            .detach();
        }
        #[cfg(not(feature = "evidence"))]
        let _ = window;
        Self { _observe: observe }
    }
}

/// "Sign out of City of Canning?" — Cancel / Sign out (red).
fn confirm_sign_out(window: &mut Window, cx: &mut App) {
    let library = AppState::global(cx).read(cx).library_name();
    ConfirmDialog::new(
        Copy::Profile::signOutTitle(&library),
        Copy::Account::signOut,
        on_click(|_, cx| AppState::global(cx).update(cx, |state, cx| state.sign_out(cx))),
    )
    .message(Copy::Profile::signOutMessage)
    .destructive(true)
    .open(window, cx)
}

/// "Version 0.1.0".
pub fn version_text() -> String {
    Copy::Profile::appVersion(env!("CARGO_PKG_VERSION"))
}

/// Up to two initials of the borrower: libraries often write "Surname, Given".
pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name.split(',').map(str::trim).collect();
    let ordered = if parts.len() == 2 { format!("{} {}", parts[1], parts[0]) } else { name.to_owned() };
    directory::monogram(&ordered)
}

fn section_header(title: &'static str) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("profile-header-{title}")))
        .test_support()
        .role(Role::Heading)
        .aria_label(SharedString::from(title))
        .typography(Font::Section)
        .text_color(theme::color(colors::TEXT_PRIMARY))
        .child(title)
}

/// A `surface` card like Your Books'.
fn card() -> gpui_kit::Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::M))
        .p(px(spacing::L))
        .rounded(px(radius::CARD))
        .bg(theme::color(colors::SURFACE))
}

fn divider() -> impl IntoElement {
    div().w_full().h(px(1.)).bg(theme::color(colors::DIVIDER))
}

/// A full-width text action inside a card ("Change library", "Sign out").
fn row_button(id: &'static str, title: &'static str, glyph: Lucide, tint: Hsla, action: OnClick) -> impl IntoElement {
    div()
        .id(ElementId::from(SharedString::from(id)))
        .test_support()
        .role(Role::Button)
        .aria_label(SharedString::from(title))
        .cursor(CursorStyle::PointingHand)
        .w_full()
        .min_h(px(32.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::M))
        .rounded(px(radius::FIELD - 6.))
        .hover(|style| style.opacity(0.8))
        .typography(Font::BodyStrong)
        .text_color(tint)
        .on_click(move |_, window, cx| action(window, cx))
        .child(div().w(px(24.)).flex().justify_center().child(icon(glyph, px(18.), tint)))
        .child(div().flex_1().child(title))
        .child(icon(Lucide::ChevronRight, px(14.), theme::color(colors::TEXT_MUTED)))
}

fn caption(text: impl Into<SharedString>) -> impl IntoElement {
    div().typography(Font::Caption).text_color(theme::color(colors::TEXT_SECONDARY)).child(text.into())
}

fn request_library_picker() -> OnClick {
    on_click(|_, cx| AppState::global(cx).update(cx, |state, cx| state.request_library_picker(cx)))
}

impl ProfileScreen {
    fn library_card(&self, library: &LibraryInfo) -> impl IntoElement {
        let region = Region::from_code(&library.region);
        let place = library
            .country
            .clone()
            .filter(|country| !country.trim().is_empty())
            .unwrap_or_else(|| region.title().to_owned());
        card()
            .id("profile-library")
            .test_support()
            .child(
                div()
                    .id("profile-library-name")
                    .test_support()
                    .aria_label(SharedString::from(library.name.clone()))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::L))
                    .child(LibraryLogo::library("profile-logo", library).size(60.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap(px(spacing::XS))
                            .child(
                                div()
                                    .typography(Font::Section)
                                    .text_color(theme::color(colors::TEXT_PRIMARY))
                                    .child(library.name.clone()),
                            )
                            .child(caption(directory::host(&library.origin)))
                            .child(div().pt(px(2.)).child(Chip::new(place).icon(Lucide::Globe))),
                    ),
            )
            .child(divider())
            .child(row_button(
                "profile-change-library",
                Copy::Profile::changeLibrary,
                Lucide::Building2,
                theme::color(colors::PRIMARY),
                request_library_picker(),
            ))
    }

    fn account_card(&self, cx: &App) -> impl IntoElement {
        let state = AppState::global(cx).read(cx);
        let library_name = state.library_name();
        if !state.is_signed_in() {
            return card()
                .id("profile-account")
                .test_support()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_start()
                        .gap(px(spacing::L))
                        .child(icon(Lucide::CircleUserRound, px(40.), theme::color(colors::TEXT_MUTED)))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(spacing::XS))
                                .child(
                                    div()
                                        .id("profile-signed-out")
                                        .test_support()
                                        .aria_label(SharedString::from(Copy::Profile::notSignedInTitle))
                                        .typography(Font::BodyStrong)
                                        .text_color(theme::color(colors::TEXT_PRIMARY))
                                        .child(Copy::Profile::notSignedInTitle),
                                )
                                .child(caption(Copy::Profile::notSignedInMessage)),
                        ),
                )
                .child(
                    div().pt(px(spacing::XS)).child(
                        PrimaryPillButton::new("profile-sign-in", Copy::Account::signIn)
                            .icon(Lucide::User)
                            .fills_width(true)
                            .on_click(on_click(|_, cx| {
                                AppState::global(cx).update(cx, |state, cx| state.request_sign_in(cx))
                            })),
                    ),
                );
        }
        let account = state.account();
        let name = state.account_name();
        let loading = name.is_none() && account.snapshot().is_none() && account.is_refreshing();
        let failed = account.snapshot().is_none() && account.last_refresh_error().is_some() && !account.is_refreshing();
        let shown_name: SharedString = name.clone().unwrap_or_else(|| Copy::Account::signedIn.to_owned()).into();
        card()
            .id("profile-account")
            .test_support()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::L))
                    .child(
                        div()
                            .flex_none()
                            .size(px(52.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme::color(colors::ACCENT))
                            .text_size(px(20.))
                            .font_weight(gpui_kit::FontWeight::BOLD)
                            .text_color(theme::color(colors::ON_ACCENT))
                            .map(|avatar| match &name {
                                Some(name) => avatar.child(initials(name)),
                                None => avatar.child(icon(Lucide::User, px(22.), theme::color(colors::ON_ACCENT))),
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(spacing::XS))
                            .map(|column| {
                                if loading {
                                    column.child(
                                        div()
                                            .id("profile-account-loading")
                                            .test_support()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap(px(spacing::S))
                                            .child(
                                                gpui_kit::component::spinner::Spinner::new()
                                                    .color(theme::color(colors::TEXT_SECONDARY)),
                                            )
                                            .child(
                                                div()
                                                    .typography(Font::Body)
                                                    .text_color(theme::color(colors::TEXT_SECONDARY))
                                                    .child(Copy::Profile::loadingAccount),
                                            ),
                                    )
                                } else {
                                    column.child(
                                        div()
                                            .id("profile-account-name")
                                            .test_support()
                                            .aria_label(shown_name.clone())
                                            .typography(Font::BodyStrong)
                                            .text_color(theme::color(colors::TEXT_PRIMARY))
                                            .child(shown_name.clone()),
                                    )
                                }
                            })
                            .child(caption(Copy::Profile::signedInTo(&library_name))),
                    ),
            )
            .when(failed, |card| {
                card.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(spacing::S))
                        .child(
                            div()
                                .flex_1()
                                .typography(Font::Caption)
                                .text_color(theme::color(colors::WARNING))
                                .child(Copy::Profile::accountFailed),
                        )
                        .child(
                            div()
                                .id("profile-account-retry")
                                .test_support()
                                .role(Role::Button)
                                .aria_label(SharedString::from(Copy::Common::retry))
                                .cursor(CursorStyle::PointingHand)
                                .typography(Font::BodyStrong)
                                .text_color(theme::color(colors::PRIMARY))
                                .on_click(|_, _, cx| {
                                    AppState::global(cx)
                                        .update(cx, |state, cx| state.refresh_account(true, cx).detach())
                                })
                                .child(Copy::Common::retry),
                        ),
                )
            })
            .child(divider())
            .child(row_button(
                "profile-sign-out",
                Copy::Account::signOut,
                Lucide::LogOut,
                theme::color(colors::DANGER),
                on_click(confirm_sign_out),
            ))
    }

    fn verification_card(&self, library_name: &str) -> impl IntoElement {
        let message = crate::state::formatting::capitalised_first(&Copy::Profile::verifiedMessage(library_name));
        let confirm_message =
            crate::state::formatting::capitalised_first(&Copy::Profile::clearVerificationMessage(library_name));
        card()
            .id("profile-verification")
            .test_support()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(spacing::L))
                    .child(icon(Lucide::ShieldCheck, px(26.), theme::color(colors::SUCCESS)))
                    .child(
                        div()
                            .flex_1()
                            // Lets the message wrap inside the card instead of running past it.
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .gap(px(spacing::XS))
                            .child(
                                div()
                                    .id("profile-verified")
                                    .test_support()
                                    .aria_label(SharedString::from(Copy::Profile::verified))
                                    .typography(Font::BodyStrong)
                                    .text_color(theme::color(colors::TEXT_PRIMARY))
                                    .child(Copy::Profile::verified),
                            )
                            .child(caption(message)),
                    ),
            )
            .child(divider())
            .child(row_button(
                "profile-clear-verification",
                Copy::Profile::clearVerification,
                Lucide::CircleX,
                theme::color(colors::PRIMARY),
                on_click(move |window, cx| {
                    ConfirmDialog::new(
                        Copy::Profile::clearVerificationTitle,
                        Copy::Profile::clearVerification,
                        on_click(|_, cx| AppState::global(cx).update(cx, |state, cx| state.clear_verification(cx))),
                    )
                    .message(confirm_message.clone())
                    .destructive(true)
                    .open(window, cx)
                }),
            ))
    }

    fn about_card(&self) -> impl IntoElement {
        card()
            .id("profile-about")
            .test_support()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .typography(Font::BodyStrong)
                            .text_color(theme::color(colors::TEXT_PRIMARY))
                            .child(Copy::Profile::appName),
                    )
                    .child(
                        div()
                            .id("profile-version")
                            .test_support()
                            .aria_label(SharedString::from(version_text()))
                            .child(caption(version_text())),
                    ),
            )
            .child(divider())
            .child(caption(Copy::Profile::notAffiliated))
    }

    /// Debug builds: demo mode's state and "Expire session" (the next refresh finds the session
    /// ended, so the sign-in dialog shows). Debug-only, so its words aren't in copy.json.
    #[cfg(debug_assertions)]
    fn developer_section(&self, cx: &App) -> Option<AnyElement> {
        let state = AppState::global(cx).read(cx);
        let demo = state.is_demo();
        let signed_in = state.is_signed_in();
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::M))
                .child(section_header("Developer"))
                .child(
                    card()
                        .id("profile-developer")
                        .test_support()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(spacing::S))
                                        .child(icon(Lucide::Settings, px(16.), theme::color(colors::TEXT_PRIMARY)))
                                        .child(
                                            div()
                                                .typography(Font::BodyStrong)
                                                .text_color(theme::color(colors::TEXT_PRIMARY))
                                                .child("Demo mode"),
                                        ),
                                )
                                .child(Chip::new(if demo { "On" } else { "Off" }).tone(if demo {
                                    crate::ui::ChipTone::Success
                                } else {
                                    crate::ui::ChipTone::Neutral
                                })),
                        )
                        .child(caption(if demo {
                            "Account data is sample data and never reaches the library. Sign in with demo / demo. Relaunch without --demo to turn it off."
                        } else {
                            "Relaunch with --demo to use sample account data (debug builds only)."
                        }))
                        .when(signed_in, |card| {
                            card.child(divider()).child(row_button(
                                "profile-expire-session",
                                "Expire session",
                                Lucide::CircleAlert,
                                theme::color(colors::PRIMARY),
                                on_click(|_, cx| {
                                    AppState::global(cx).update(cx, |state, cx| state.expire_session_for_debugging(cx))
                                }),
                            ))
                        }),
                )
                .into_any_element(),
        )
    }

    #[cfg(not(debug_assertions))]
    fn developer_section(&self, _: &App) -> Option<AnyElement> {
        None
    }
}

impl Render for ProfileScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Wave 3 (docs/desktop-app.md §11): Touch ID and Notifications, each owned by its feature.
        let biometrics = crate::features::biometrics::profile_section(window, cx);
        let alerts = crate::features::alerts::profile_section(window, cx);
        let state = AppState::global(cx).read(cx);
        let library = state.library().cloned();
        let library_name = state.library_name();
        let verified = state.is_verified();
        let body = div()
            .w_full()
            .max_w(px(720.))
            .mx_auto()
            .px(px(spacing::XL))
            .pt(px(spacing::L))
            .pb(px(spacing::XXL))
            .flex()
            .flex_col()
            .gap(px(spacing::XL))
            .child(
                div()
                    .id("profile-title")
                    .test_support()
                    .role(Role::Heading)
                    .aria_label(SharedString::from(Copy::Profile::title))
                    .typography(Font::Greeting)
                    .text_color(theme::color(colors::TEXT_PRIMARY))
                    .child(Copy::Profile::title),
            )
            .map(|body| match &library {
                Some(library) => body
                    .child(self.library_card(library))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(spacing::M))
                            .child(section_header(Copy::Profile::account))
                            .child(self.account_card(cx))
                            .children(biometrics),
                    )
                    .children(alerts)
                    .when(verified, |body| {
                        body.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(spacing::M))
                                .child(section_header(Copy::Profile::humanVerification))
                                .child(self.verification_card(&library_name)),
                        )
                    }),
                None => body.child(
                    div().h(px(320.)).child(
                        EmptyStateView::new(Lucide::Library, Copy::Profile::noLibraryTitle)
                            .message(Copy::Profile::noLibraryMessage)
                            .action("profile-choose-library", Copy::Profile::chooseLibrary, request_library_picker()),
                    ),
                ),
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::M))
                    .child(section_header(Copy::Profile::about))
                    .child(self.about_card()),
            )
            .children(self.developer_section(cx));
        div().id("profile-screen").test_support().size_full().overflow_y_scroll().child(body)
    }
}

#[cfg(debug_assertions)]
impl AppState {
    /// Debug builds (Profile's "Expire session"; iOS `expireSessionForDebugging`): stores an
    /// invalid session and refreshes, so the library answers NotLoggedIn and the sign-in dialog
    /// shows. The saved account stays on screen meanwhile.
    pub fn expire_session_for_debugging(&mut self, cx: &mut Context<Self>) {
        if self.session.is_none() {
            return;
        }
        // copy-exempt: a debug session token, never shown
        let expired = "expired-session".to_owned();
        let origin = self.backend.origin();
        self.session = Some(expired.clone());
        self.deps.sessions.set_value(&origin, Some(&expired));
        self.account.use_account(Some(&origin), Some(&expired), self.backend.clone(), true);
        self.picks.use_account(Some(&origin), Some(&expired), self.backend.clone());
        self.refresh_account(true, cx).detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_read_surname_comma_given() {
        assert_eq!(initials("Borrower, Test"), "TB");
        assert_eq!(initials("Test Borrower"), "TB");
        assert_eq!(version_text(), format!("Version {}", env!("CARGO_PKG_VERSION")));
    }
}

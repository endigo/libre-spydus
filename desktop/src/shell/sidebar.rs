//! The left sidebar (docs/desktop-app.md §5.5), in place of the phones' tab bar: white, the
//! active section in navy. The header shows the library (logo, name, Change library); the footer
//! the account name with Sign out, or Sign in.

use gpui_kit::{
    App, CursorStyle, InteractiveElement, IntoElement, ObjectFit, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, StyledImage, TestSupportExt, Window, div, img, prelude::FluentBuilder, px,
};

use crate::copy::Copy;
use crate::state::{AppState, Section};
use crate::theme::{self, Font, colors, radius, spacing};
use crate::ui::{ConfirmDialog, FontExt, Lucide, icon, on_click};

pub const WIDTH: f32 = 232.;

fn section_icon(section: Section) -> Lucide {
    match section {
        Section::Home => Lucide::House,
        Section::YourBooks => Lucide::LibraryBig,
        Section::Search => Lucide::Search,
        Section::MyPicks => Lucide::Bookmark,
        Section::Profile => Lucide::CircleUserRound,
    }
}

#[derive(IntoElement)]
pub struct Sidebar;

impl RenderOnce for Sidebar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = AppState::global(cx).read(cx);
        let current = state.section();
        let library_name = state.library_name();
        let logo = state.library().and_then(|library| library.logo_url.clone());
        let signed_in = state.is_signed_in();
        let account_name = state.account_name();
        let navy = theme::color(colors::TAB_ACTIVE);
        let grey = theme::color(colors::TAB_INACTIVE);

        let header = div()
            .flex()
            .flex_col()
            .gap(px(spacing::S))
            .px(px(spacing::L))
            .pt(px(spacing::XL + spacing::L))
            .pb(px(spacing::L))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::S))
                    .child(
                        div()
                            .id("sidebar-logo")
                            .test_support()
                            .aria_label(SharedString::from(Copy::Desktop::libraryLogo(&library_name)))
                            .flex_none()
                            .size(px(36.))
                            .rounded(px(radius::COVER))
                            .overflow_hidden()
                            .bg(theme::color(colors::CHIP_ON_SHEET))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(Lucide::LibraryBig, px(18.), navy))
                            .when_some(logo, |frame, url| {
                                frame.child(
                                    img(SharedString::from(url))
                                        .id("sidebar-logo-img")
                                        .absolute()
                                        .size(px(36.))
                                        .object_fit(ObjectFit::Contain)
                                        .bg(theme::color(0xFFFFFF)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .typography(Font::BodyStrong)
                            .text_color(navy)
                            .line_clamp(2)
                            .text_ellipsis()
                            .child(library_name.clone()),
                    ),
            )
            .child(
                div()
                    .id("sidebar-change-library")
                    .test_support()
                    .role(Role::Button)
                    .aria_label(SharedString::from(Copy::Profile::changeLibrary))
                    .cursor(CursorStyle::PointingHand)
                    .typography(Font::Caption)
                    .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                    .hover(|style| style.text_color(navy))
                    .on_click(|_, _, cx| AppState::global(cx).update(cx, |state, cx| state.request_library_picker(cx)))
                    .child(Copy::Profile::changeLibrary),
            );

        let items =
            div().flex().flex_col().gap(px(spacing::XS)).px(px(spacing::S)).children(Section::ALL.map(|section| {
                let active = section == current;
                div()
                    .id(section.id())
                    .test_support()
                    .role(Role::Tab)
                    .aria_label(SharedString::from(section.title()))
                    .aria_description(SharedString::from(Copy::Desktop::sidebarHint(section.title())))
                    .aria_selected(active)
                    .cursor(CursorStyle::PointingHand)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::M))
                    .px(px(spacing::M))
                    .h(px(40.))
                    .rounded(px(radius::FIELD))
                    .when(active, |item| item.bg(navy).text_color(theme::color(colors::TAB_BAR)))
                    .when(!active, |item| {
                        item.text_color(navy).hover(|style| style.bg(theme::color_alpha(colors::TAB_ACTIVE, 0.06)))
                    })
                    .on_click(move |_, _, cx| AppState::global(cx).update(cx, |state, cx| state.select(section, cx)))
                    .child(icon(
                        section_icon(section),
                        px(18.),
                        if active { theme::color(colors::TAB_BAR) } else { grey },
                    ))
                    .child(div().typography(if active { Font::BodyStrong } else { Font::Body }).child(section.title()))
            }));

        let footer = div()
            .flex()
            .flex_col()
            .gap(px(spacing::S))
            .p(px(spacing::L))
            .border_t_1()
            .border_color(theme::color_alpha(colors::TAB_INACTIVE, 0.4));
        let footer = if signed_in {
            let name: SharedString = account_name.unwrap_or_else(|| Copy::Account::signedIn.to_owned()).into();
            footer
                .child(
                    div()
                        .id("sidebar-account")
                        .test_support()
                        .role(Role::Label)
                        .aria_label(name.clone())
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(spacing::S))
                        .child(icon(Lucide::CircleUserRound, px(18.), navy))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .typography(Font::BodyStrong)
                                .text_color(navy)
                                .truncate()
                                .child(name),
                        ),
                )
                .child(
                    div()
                        .id("sidebar-sign-out")
                        .test_support()
                        .role(Role::Button)
                        .aria_label(SharedString::from(Copy::Account::signOut))
                        .cursor(CursorStyle::PointingHand)
                        .typography(Font::Caption)
                        .text_color(theme::color(colors::ON_SHEET_SECONDARY))
                        .hover(|style| style.text_color(navy))
                        .on_click(move |_, window, cx| {
                            let library = AppState::global(cx).read(cx).library_name();
                            ConfirmDialog::new(
                                Copy::Profile::signOutTitle(&library),
                                Copy::Account::signOut,
                                on_click(|_, cx| AppState::global(cx).update(cx, |state, cx| state.sign_out(cx))),
                            )
                            .message(Copy::Profile::signOutMessage)
                            .destructive(true)
                            .open(window, cx)
                        })
                        .child(Copy::Account::signOut),
                )
        } else {
            footer.child(
                div()
                    .id("sidebar-sign-in")
                    .test_support()
                    .role(Role::Button)
                    .aria_label(SharedString::from(Copy::Account::signIn))
                    .cursor(CursorStyle::PointingHand)
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .h(px(36.))
                    .rounded(px(radius::PILL))
                    .bg(navy)
                    .text_color(theme::color(colors::TAB_BAR))
                    .typography(Font::BodyStrong)
                    .on_click(|_, _, cx| AppState::global(cx).update(cx, |state, cx| state.request_sign_in(cx)))
                    .child(Copy::Account::signIn),
            )
        };

        div()
            .id("sidebar")
            .test_support()
            .role(Role::Navigation)
            .flex_none()
            .w(px(WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::color(colors::TAB_BAR))
            .border_r_1()
            .border_color(theme::color_alpha(colors::TAB_INACTIVE, 0.4))
            .child(header)
            .child(div().flex_1().child(items))
            .child(footer)
    }
}

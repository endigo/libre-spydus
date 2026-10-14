//! Key bindings and menus (docs/desktop-app.md §5.5): ⌘1–⌘5 select a section, ⌘F opens Search
//! and focuses its field, ⌘[ goes back, ⌘R refreshes the account, ⌘Q quits.

use gpui_kit::{App, KeyBinding, Menu, MenuItem};

use crate::copy::Copy;
use crate::state::{AppState, Section};

/// The key context the root view sets.
pub const CONTEXT: &str = "LibreSpydus";

gpui_kit::actions!(
    librespydus,
    [
        Quit,
        Hide,
        HideOthers,
        ShowAll,
        Minimize,
        Zoom,
        CloseWindow,
        ShowHome,
        ShowYourBooks,
        ShowSearch,
        ShowMyPicks,
        ShowProfile,
        FocusSearch,
        GoBack,
        RefreshAccount
    ]
);

/// Registers the app-wide handlers, key bindings and the menu bar.
pub fn install(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &ShowHome, cx| select(Section::Home, cx));
    cx.on_action(|_: &ShowYourBooks, cx| select(Section::YourBooks, cx));
    cx.on_action(|_: &ShowSearch, cx| select(Section::Search, cx));
    cx.on_action(|_: &ShowMyPicks, cx| select(Section::MyPicks, cx));
    cx.on_action(|_: &ShowProfile, cx| select(Section::Profile, cx));
    cx.on_action(|_: &FocusSearch, cx| AppState::global(cx).update(cx, |state, cx| state.open_search(None, cx)));
    cx.on_action(|_: &GoBack, cx| AppState::global(cx).update(cx, |state, cx| state.pop(cx)));
    cx.on_action(|_: &RefreshAccount, cx| {
        AppState::global(cx).update(cx, |state, cx| state.refresh_account(true, cx).detach())
    });
    cx.bind_keys(key_bindings());
    cx.set_menus(menus());
}

fn select(section: Section, cx: &mut App) {
    AppState::global(cx).update(cx, |state, cx| state.select(section, cx));
}

pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, Some(CONTEXT)),
        KeyBinding::new("cmd-w", CloseWindow, Some(CONTEXT)),
        KeyBinding::new("cmd-1", ShowHome, None),
        KeyBinding::new("cmd-2", ShowYourBooks, None),
        KeyBinding::new("cmd-3", ShowSearch, None),
        KeyBinding::new("cmd-4", ShowMyPicks, None),
        KeyBinding::new("cmd-5", ShowProfile, None),
        KeyBinding::new("cmd-f", FocusSearch, None),
        KeyBinding::new("cmd-[", GoBack, None),
        KeyBinding::new("cmd-r", RefreshAccount, None),
    ]
}

pub fn menus() -> Vec<Menu> {
    vec![
        Menu::new(Copy::Profile::appName).items([
            MenuItem::action(Copy::Desktop::hide, Hide),
            MenuItem::action(Copy::Desktop::hideOthers, HideOthers),
            MenuItem::action(Copy::Desktop::showAll, ShowAll),
            MenuItem::separator(),
            MenuItem::action(Copy::Desktop::quit, Quit),
        ]),
        Menu::new(Copy::Desktop::file).items([MenuItem::action(Copy::Desktop::closeWindow, CloseWindow)]),
        Menu::new(Copy::Desktop::go).items([
            MenuItem::action(Copy::Tabs::home, ShowHome),
            MenuItem::action(Copy::Tabs::yourBooks, ShowYourBooks),
            MenuItem::action(Copy::Tabs::search, ShowSearch),
            MenuItem::action(Copy::Tabs::myPicks, ShowMyPicks),
            MenuItem::action(Copy::Tabs::profile, ShowProfile),
            MenuItem::separator(),
            MenuItem::action(Copy::Common::back, GoBack),
            MenuItem::action(Copy::Search::title, FocusSearch),
            MenuItem::action(Copy::Desktop::refreshAccount, RefreshAccount),
        ]),
        Menu::new(Copy::Desktop::window)
            .items([MenuItem::action(Copy::Desktop::minimize, Minimize), MenuItem::action(Copy::Desktop::zoom, Zoom)]),
    ]
}

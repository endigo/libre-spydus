//! Headless UI tests of the library picker (docs/native-apps.md §6 "Library picker"): onboarding
//! shows the directory grouped by region, search filters it (by name, country or region code), a
//! click selects the library and shows Home; Change library is the same picker in a dialog.

mod support;

use std::sync::Arc;

use futures::FutureExt;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};
use librespydus_core::{LibraryInfo, SpydusError};
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::features::onboarding::set_directory_fetch;
use librespydus_desktop::state::{AppState, Section};

fn library(name: &str, host: &str, region: &str, country: &str) -> LibraryInfo {
    LibraryInfo {
        id: host.into(),
        name: name.into(),
        origin: format!("https://{host}"),
        logo_url: None,
        country: Some(country.into()),
        region: region.into(),
        is_test: false,
    }
}

fn directory() -> Vec<LibraryInfo> {
    vec![
        library("City of Canning", "canning.spydus.com", "AU", "Australia"),
        library("Wellington City Libraries", "wellington.spydus.co.nz", "NZ", "New Zealand"),
        library("Hutt City Libraries", "hutt.spydus.co.nz", "NZ", "New Zealand"),
        library("Aberdeen City Libraries", "aberdeencity.spydus.co.uk", "UK", "United Kingdom"),
        library("Nanyang Library", "nanyang.spydus.sg", "SG", "Singapore"),
    ]
}

fn canned(libraries: Vec<LibraryInfo>) -> librespydus_desktop::features::onboarding::DirectoryFetch {
    Arc::new(move || futures::future::ready(Ok(libraries.clone())).boxed())
}

#[gpui_kit::test]
fn onboarding_lists_regions_filters_and_selects(cx: &mut TestAppContext) {
    let fake = FakeBackend::new("https://canning.spydus.com");
    let state = support::app_state(cx, support::fake_deps(&fake));
    cx.update(|cx| set_directory_fetch(canned(directory()), cx));
    let window = support::open_root(cx);
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("library-picker-screen").is_some());
        for region in ["AU", "NZ", "UK", "SG"] {
            assert!(window.try_find(format!("region-{region}")).is_some(), "{region} header");
        }
        assert!(window.try_find("region-US").is_none(), "empty regions are left out");
        assert_eq!(window.find("region-NZ-count").label(), Some("2 libraries"));
        assert!(window.try_find("library-wellington.spydus.co.nz").is_some());
    })
    .unwrap();

    // A region code: only the UK row is left.
    cx.update_window(window.into(), |_, window, cx| {
        window.click("picker-search", cx);
        window.input("uk", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("library-aberdeencity.spydus.co.uk").is_some());
        assert!(window.try_find("library-canning.spydus.com").is_none());
        assert!(window.try_find("region-AU").is_none());
        // Select all, then a country name: both New Zealand libraries.
        window.press("cmd-a", cx);
        window.input("zealand", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("region-NZ").is_some());
        assert!(window.try_find("library-hutt.spydus.co.nz").is_some());
        assert!(window.try_find("library-wellington.spydus.co.nz").is_some());
        assert!(window.try_find("region-UK").is_none());
        window.press("cmd-a", cx);
        window.input("nowhere", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("region-NZ").is_none());
        window.press("cmd-a", cx);
        window.input("wellington", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("library-wellington.spydus.co.nz", cx);
    })
    .unwrap();
    cx.run_until_parked();
    let (origin, section) =
        state.read_with(cx, |state, _| (state.library().map(|library| library.origin.clone()), state.section()));
    assert_eq!(origin.as_deref(), Some("https://wellington.spydus.co.nz"));
    assert_eq!(section, Section::Home);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("library-picker-screen").is_none());
        assert!(window.try_find("home-screen").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_failed_directory_offers_retry(cx: &mut TestAppContext) {
    let fake = FakeBackend::new("https://canning.spydus.com");
    let _state = support::app_state(cx, support::fake_deps(&fake));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = calls.clone();
    cx.update(|cx| {
        set_directory_fetch(
            Arc::new(move || {
                let first = counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0;
                let result =
                    if first { Err(SpydusError::Network { message: "offline".into() }) } else { Ok(directory()) };
                futures::future::ready(result).boxed()
            }),
            cx,
        )
    });
    let window = support::open_root(cx);
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("picker-error-retry").is_some());
        window.click("picker-error-retry", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("picker-error-retry").is_none());
        assert!(window.try_find("library-canning.spydus.com").is_some());
    })
    .unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[gpui_kit::test]
fn change_library_is_the_picker_in_a_dialog(cx: &mut TestAppContext) {
    let fake = FakeBackend::new("https://canning.spydus.com");
    cx.update(|cx| set_directory_fetch(canned(directory()), cx));
    let (state, window) = support::signed_out_app(cx, &fake);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("sidebar-change-library", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("library-picker-dialog").is_some());
        // The current library is marked.
        assert_eq!(window.find("library-canning.spydus.com").selected(), Some(true));
        window.click("library-hutt.spydus.co.nz", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        state.read_with(cx, |state: &AppState, _| state.library().map(|library| library.origin.clone())).as_deref(),
        Some("https://hutt.spydus.co.nz")
    );
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("library-picker-dialog").is_none(), "choosing closes the dialog");
    })
    .unwrap();
}

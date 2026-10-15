//! Bootstrap (docs/desktop-app.md §5): the runtime and HTTP client, the theme, menus and key
//! bindings, `AppState`, the window and the debug launch flags.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Entity, QuitMode, WindowBounds, WindowOptions, point, px, size,
};

use crate::backend::{CoreBackend, LibraryBackend};
use crate::launch::{LaunchOptions, LaunchRoute, MIN_WINDOW};
use crate::shell::{RootView, actions};
use crate::state::secrets::{CARD_SERVICE, KeychainSecretStore, SESSION_SERVICE, VERIFICATION_SERVICE};
use crate::state::{
    AppState, BookRef, Dependencies, MemorySecretStore, Paths, Route, SecretStore, Section, Settings, YourBooksSegment,
};
use crate::ui::AppAssets;
use crate::{features, runtime, theme, timing};

/// Runs the app with these launch flags; returns when it quits.
pub fn run(options: LaunchOptions) {
    if options.reset {
        reset(&options);
    }
    timing::install();
    gpui_kit::application()
        .with_assets(AppAssets)
        .with_http_client(runtime::HttpClient::new())
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            theme::apply(cx);
            actions::install(cx);
            let state = AppState::init(dependencies(&options), cx);
            if let Some(origin) = &options.library {
                state.update(cx, |state, cx| state.select_library_origin(origin, cx));
            }
            let window = match open_window(&options, cx) {
                Ok(window) => window,
                Err(error) => {
                    eprintln!("librespydus: couldn't open the window: {error}");
                    cx.quit();
                    return;
                }
            };
            apply_route(&options, &state, window.0, &window.1, cx);
            if !options.evidence {
                cx.activate(true);
            }
            if let Some(path) = options.screenshot.clone() {
                schedule_screenshot(window.0, path, options.screenshot_after, cx);
            }
        });
}

/// The app's real stores, or memory with `--demo` / `--data-dir`.
pub fn dependencies(options: &LaunchOptions) -> Dependencies {
    let paths = Paths::new(options.data_dir.as_deref());
    let memory = options.secrets_in_memory();
    let store = |service: &'static str| -> Arc<dyn SecretStore> {
        if memory { Arc::new(MemorySecretStore::new()) } else { Arc::new(KeychainSecretStore::new(service)) }
    };
    let saved_pages = paths.saved_pages.clone();
    let demo = cfg!(debug_assertions) && options.demo;
    let refresh_fails = options.demo_refresh_fails;
    Dependencies {
        sessions: store(SESSION_SERVICE),
        verification_cookies: store(VERIFICATION_SERVICE),
        card_numbers: store(CARD_SERVICE),
        paths,
        is_demo: demo,
        make_backend: Arc::new(move |origin| -> Arc<dyn LibraryBackend> {
            make_backend(origin, saved_pages.clone(), demo, refresh_fails)
        }),
        now: Arc::new(chrono::Utc::now),
        look_up_libraries: true,
    }
}

#[cfg(debug_assertions)]
fn make_backend(
    origin: Option<&str>,
    saved_pages: Option<PathBuf>,
    demo: bool,
    refresh_fails: bool,
) -> Arc<dyn LibraryBackend> {
    if demo {
        Arc::new(crate::backend::demo::DemoBackend::live(origin, saved_pages, refresh_fails))
    } else {
        Arc::new(CoreBackend::new(origin, saved_pages))
    }
}

#[cfg(not(debug_assertions))]
fn make_backend(
    origin: Option<&str>,
    saved_pages: Option<PathBuf>,
    _demo: bool,
    _refresh_fails: bool,
) -> Arc<dyn LibraryBackend> {
    Arc::new(CoreBackend::new(origin, saved_pages))
}

/// `--reset`: deletes the files (and, without `--data-dir`, this app's keychain items).
fn reset(options: &LaunchOptions) {
    let paths = Paths::new(options.data_dir.as_deref());
    if !options.secrets_in_memory() {
        let settings = Settings::load(paths.settings_file());
        let mut origins = settings.data.secret_origins.clone();
        origins.extend(settings.data.library.map(|library| library.origin));
        for service in [SESSION_SERVICE, VERIFICATION_SERVICE, CARD_SERVICE] {
            let store = KeychainSecretStore::new(service);
            for origin in &origins {
                store.set_value(origin, None);
            }
        }
    }
    crate::features::biometrics::reset(options.secrets_in_memory());
    paths.delete_all();
}

fn open_window(options: &LaunchOptions, cx: &mut App) -> anyhow::Result<(AnyWindowHandle, Entity<RootView>)> {
    let window_size = size(px(options.window.0), px(options.window.1));
    let bounds = if options.evidence {
        Bounds { origin: point(px(40.), px(40.)), size: window_size }
    } else {
        Bounds::centered(None, window_size, cx)
    };
    let window_options = WindowOptions {
        titlebar: Some(crate::platform::titlebar()),
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(MIN_WINDOW.0), px(MIN_WINDOW.1))),
        app_id: Some("com.librespydus.desktop".into()),
        focus: !options.evidence,
        show: true,
        ..Default::default()
    };
    gpui_kit::open_window(window_options, cx, |window, cx| cx.new(|cx| RootView::new(window, cx)))
}

/// `--route`: where the window opens.
fn apply_route(
    options: &LaunchOptions,
    state: &Entity<AppState>,
    window: AnyWindowHandle,
    root: &Entity<RootView>,
    cx: &mut App,
) {
    let Some(route) = options.route.clone() else { return };
    let root = root.clone();
    let state = state.clone();
    let _ = window.update(cx, move |_, window, cx| match route {
        LaunchRoute::Home => state.update(cx, |state, cx| state.select(Section::Home, cx)),
        LaunchRoute::Search(query) => {
            state.update(cx, |state, cx| state.open_search((!query.is_empty()).then_some(query), cx))
        }
        LaunchRoute::Results(query) => state.update(cx, |state, cx| {
            state.push_on(
                Section::Home,
                Route::Results { title: query.clone(), query, sort: librespydus_core::SearchSort::Relevance },
                cx,
            )
        }),
        LaunchRoute::Detail(url) => {
            state.update(cx, |state, cx| state.push_on(Section::Home, Route::Detail(BookRef::from_url(&url, "")), cx))
        }
        LaunchRoute::Borrowed => state.update(cx, |state, cx| state.select(Section::YourBooks, cx)),
        LaunchRoute::BorrowedReservations => {
            state.update(cx, |state, cx| state.open_your_books(YourBooksSegment::Reservations, cx))
        }
        LaunchRoute::Favourites => state.update(cx, |state, cx| state.select(Section::MyPicks, cx)),
        LaunchRoute::Profile => state.update(cx, |state, cx| state.select(Section::Profile, cx)),
        LaunchRoute::Libraries => state.update(cx, |state, cx| state.request_library_picker(cx)),
        LaunchRoute::Login => state.update(cx, |state, cx| state.request_sign_in(cx)),
        LaunchRoute::Card => features::my_card::open_my_card(window, cx),
        LaunchRoute::Gallery => show_gallery(&root, window, cx),
    });
}

#[cfg(debug_assertions)]
fn show_gallery(root: &Entity<RootView>, window: &mut gpui_kit::Window, cx: &mut App) {
    let gallery = cx.new(|cx| crate::ui::gallery::GalleryScreen::new(window, cx));
    root.update(cx, |root, cx| root.show_overlay(gallery.into(), cx));
}

#[cfg(not(debug_assertions))]
fn show_gallery(_: &Entity<RootView>, _: &mut gpui_kit::Window, _: &mut App) {}

/// `--screenshot`: after the delay, the app renders its own window to the PNG and quits
/// (docs/gpui-kit-notes.md: `screencapture` can't capture a window over SSH).
#[cfg(feature = "evidence")]
fn schedule_screenshot(window: AnyWindowHandle, path: PathBuf, after: f64, cx: &mut App) {
    cx.spawn(async move |cx| {
        cx.background_executor().timer(std::time::Duration::from_secs_f64(after)).await;
        cx.update(|cx| {
            match window.update(cx, |_, window, _| window.render_to_image()) {
                Ok(Ok(image)) => {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match image.save(&path) {
                        Ok(()) => println!("librespydus: screenshot saved to {}", path.display()),
                        Err(error) => eprintln!("librespydus: couldn't save {}: {error}", path.display()),
                    }
                }
                Ok(Err(error)) => eprintln!("librespydus: couldn't render the window: {error}"),
                Err(error) => eprintln!("librespydus: the window went away: {error}"),
            }
            cx.quit();
        });
    })
    .detach();
}

#[cfg(not(feature = "evidence"))]
fn schedule_screenshot(_: AnyWindowHandle, _: PathBuf, after: f64, cx: &mut App) {
    eprintln!("librespydus: --screenshot needs `--features evidence`; quitting without one");
    cx.spawn(async move |cx| {
        cx.background_executor().timer(std::time::Duration::from_secs_f64(after)).await;
        cx.update(|cx| cx.quit());
    })
    .detach();
}

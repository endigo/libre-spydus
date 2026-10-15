//! Shared test helpers: an app with `FakeBackend`, and the root window.
#![allow(dead_code)]

use std::sync::Arc;

use gpui_kit::base::Root;
use gpui_kit::{
    AppContext, Bounds, Entity, TestAppContext, WindowBounds, WindowHandle, WindowOptions, point, px, size,
};
use librespydus_desktop::backend::LibraryBackend;
use librespydus_desktop::backend::fake::FakeBackend;
use librespydus_desktop::shell::RootView;
use librespydus_desktop::state::library::canning;
use librespydus_desktop::state::{AppState, Dependencies};

/// GPUI and the theme, an `AppState` on `deps`.
pub fn app_state(cx: &mut TestAppContext, deps: Dependencies) -> Entity<AppState> {
    cx.update(|cx| {
        gpui_kit::init(cx);
        librespydus_desktop::theme::apply(cx);
        AppState::init(deps, cx)
    })
}

/// In-memory dependencies whose every library is `fake`.
pub fn fake_deps(fake: &Arc<FakeBackend>) -> Dependencies {
    let fake = fake.clone();
    Dependencies::in_memory(Arc::new(move |_| -> Arc<dyn LibraryBackend> { fake.clone() }))
}

/// An app on Canning (signed out) with `fake`, and its root window.
pub fn signed_out_app(cx: &mut TestAppContext, fake: &Arc<FakeBackend>) -> (Entity<AppState>, WindowHandle<Root>) {
    let state = app_state(cx, fake_deps(fake));
    state.update(cx, |state, cx| state.select_library(canning(), cx));
    let window = open_root(cx);
    (state, window)
}

pub fn open_root(cx: &mut TestAppContext) -> WindowHandle<Root> {
    open_root_sized(cx, 1280., 820.)
}

/// The root window at `width` × `height`.
pub fn open_root_sized(cx: &mut TestAppContext, width: f32, height: f32) -> WindowHandle<Root> {
    cx.update(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(width), px(height)),
            })),
            ..Default::default()
        };
        let (window, _) =
            gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| RootView::new(window, cx))).expect("window");
        window.downcast::<Root>().expect("Base Root")
    })
}

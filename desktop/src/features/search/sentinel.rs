//! "Came into view" for lists without a virtual list: a 1-pt element that reports, after each
//! layout, whether it lies inside the window (iOS `.onAppear` / `.onDisappear`). The results
//! footer uses it for infinite scroll, a Featured collection for loading its shelves in view.

use gpui_kit::{App, IntoElement, Styled, canvas, px};

/// Calls `on_layout(visible, cx)` after every frame that lays it out, outside the frame.
pub(crate) fn sentinel(on_layout: impl FnOnce(bool, &mut App) + 'static) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| {
            let height = window.viewport_size().height;
            let visible = bounds.origin.y < height && bounds.bottom() > px(0.);
            cx.defer(move |cx| on_layout(visible, cx));
        },
        |_, _, _, _| {},
    )
    .w_full()
    .h(px(1.))
}

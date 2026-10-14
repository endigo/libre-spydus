# gpui-kit 0.7.3 notes for the LibreSpydus desktop app

A survey of the gpui-kit 0.7.3 / gpui-pre 0.3.9 sources, made while planning `docs/desktop-app.md`.

The sources are in the cargo registry once you've built:
`R=~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`. The crates are `gpui-kit-0.7.3`,
`gpui-component-0.7.3`, `gpui-base-0.7.3`, `gpui-pre-0.3.9` and `gpui-pre-platform-0.3.9`. Read
them, and the examples in `$R/gpui-kit-0.7.3/examples/`, rather than guessing an API. The upstream
docs are https://github.com/longbridge/gpui-kit (website/component/*.md and
skills/gpui-kit/references/usage.md).

## Spike (works on this Mac: rustc 1.97.1, Xcode 27)
```toml
gpui-kit = "=0.7.3"
librespydus-core = { path = "../core" }
tokio = { version = "1", features = ["rt-multi-thread"] }
```
```rust
use gpui_kit::component::button::*; use gpui_kit::component::*; use gpui_kit::*;
fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap())
}
// in a click handler:
let task = runtime().spawn(async move { librespydus_core::SpydusClient::new(None).search("x".into()).await });
cx.spawn(async move |this, cx| { let r = task.await; let _ = this.update(cx, |this, cx| { /* … */ cx.notify(); }); }).detach();
fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Hello { .. })).expect("window");
    });
}
```
A clean build takes about 60 s and a debug target about 4.2 GB.

## Bootstrap
- `gpui_kit::application()`, then `.with_assets(gpui_kit::assets::Assets)`, then `.run(|cx| { gpui_kit::init(cx); … })`.
- `gpui_kit::open_window(options, cx, build)` wraps the view in `base::Root` for you. Dialogs, sheets, toasts and tooltips are drawn by a root plugin, so there are no layer calls.
- `WindowOptions { titlebar: Some(TitlebarOptions { title, appears_transparent, traffic_light_position }), window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1280.), px(820.)), cx))), window_min_size: Some(size(px(960.), px(640.))), app_id, focus, show, .. }`. A custom title bar is `component::TitleBar`.
- Menus and quit:
  - `actions!(app, [Quit]); cx.on_action(|_: &Quit, cx| cx.quit()); cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]); cx.set_menus([...]); cx.activate(true)`.
  - Skip `activate` with `--evidence`.
  - The default `QuitMode` keeps a macOS app alive with no windows; use `.with_quit_mode(QuitMode::LastWindowClosed)`.
- **Icons:**
  - `Assets` bundles only about 106 Lucide icons (`$R/gpui-kit-assets-0.7.3/default-icons.txt`), and `component::IconName` covers that subset.
  - For others (book-marked, library-big, …), use `icon_assets!(AppAssets, [BookMarked, LibraryBig])` chained to `Assets`. See `examples/extra_assets.rs`.
- **Remote images FAIL by default.** The App starts with `NullHttpClient` ("No HttpClient available"). Implement `gpui_kit::http_client::HttpClient` (`send(&self, http::Request<AsyncBody>) -> BoxFuture<'static, Result<Response<AsyncBody>>>`, `user_agent()`, `proxy()`) with reqwest on our tokio runtime, and install it with `Application::with_http_client(..)` or `cx.set_http_client(..)`. Zed's `reqwest_client` crate isn't published with gpui-pre.

## Theme
- `cx.theme()` (trait `ActiveTheme`) derefs to `ThemeColor`: `background`, `foreground`, `primary`, `primary_foreground`, `muted`, `muted_foreground`, `border`, `input`, `accent`, `sidebar*`, `popover*`, `skeleton`, `link`. Other fields: `mode`, `font_family`, `font_size`, `radius`, `radius_lg`, `shadow`.
- Write path: `Theme::change(ThemeMode::Dark, None, cx);` then a separate `Theme::update(cx, |t| { t.background = …; t.primary = …; t.radius = px(..); })`.
- Colours come from `component::try_parse_color("#2B2D42")`, `rgb(0x2B2D42).into()` or `hsla(..)`.

## Components (`gpui_kit::component::…`)
- **Sidebar:**
  - `Sidebar::new("nav").header(SidebarHeader::new().child(..)).footer(SidebarFooter::new().child(..)).child(SidebarGroup::new("…").child(SidebarMenu::new().child(SidebarMenuItem::new(label).icon(IconName::X).active(bool).on_click(|_, w, cx| ..))))`
  - Also `.collapsible()` and `SidebarToggleButton`.
- **Button:**
  - `Button::new(id).label(..).icon(..).on_click(|&ClickEvent, &mut Window, &mut App| ..)`.
  - Variants: `.primary() .secondary() .danger() .ghost() .link() .text() .outline()`.
  - Other builders: `.loading(bool) .disabled(bool) .tooltip(..) .compact() .small()`.
  - Icon-only: `.icon()` with no label, plus `.accessibility_label(..)`.
- **Input:**
  - State: `cx.new(|cx| InputState::new(window, cx).placeholder(..).default_value(..).masked(true))`, with `.set_value(v, window, cx)`, `.value()` and `.focus(window, cx)`.
  - Element: `Input::new(&state).prefix(..).suffix(..).cleanable(true).mask_toggle()`.
  - Events: `cx.subscribe_in(&state, window, |this, _, ev: &InputEvent, w, cx| ..)` with `InputEvent::{Change, PressEnter{..}, Focus, Blur}`.
- **Switch:** `Switch::new(id).checked(b).label(..).on_change(|&bool, w, cx| ..)`. It is controlled.
- **Segmented control:** `TabBar::new(id).segmented().selected_index(i).on_click(|&usize, w, cx| ..).child(Tab::new().label(..))`.
- **Select:** `SelectState::new(items, Some(IndexPath::default()), window, cx)`, rendered as `Select::new(&state)`. Event: `SelectEvent::Confirm(Option<V>)`.
- **Dialog:**
  - `window.open_dialog(cx, |dialog, w, cx| dialog.title(..).child(..).on_ok(|_, w, cx| true))`. Alert: `window.open_alert_dialog(..)`. Close with `window.close_dialog(cx)`.
  - The builder closure re-runs, so create entities (InputState, …) **before** opening.
- **Toast:** `window.push_notification(Notification::success(msg).autohide(true), cx)`, or a bare `&str`.
- **Sheet:** `window.open_sheet(cx, |s, w, cx| ..)` or `open_sheet_at(Placement::Right, ..)`.
- **Popover:** `Popover::new(id).trigger(Button::new(..)).content(|state, w, cx| ..)`.
- **Context menu:** `div().id("card").context_menu(|menu, w, cx| menu.menu(label, Box::new(Action)).separator())` (trait `menu::ContextMenuExt`). `PopupMenuItem::new(..).on_click(..)` takes closures.
- **Scrolling:** `div().id(..).overflow_y_scroll()`, or `.overflow_y_scrollbar()` (`scroll::ScrollableElement`).
- **Virtual list:** `v_virtual_list(cx.entity(), "id", Rc<Vec<Size<Pixels>>>, |this, range, w, cx| -> Vec<_>)` with `.track_scroll(&handle)`. `list::ListDelegate` adds `has_more()` / `load_more()`.
- **Images:** `img(url).object_fit(ObjectFit::Cover).with_loading(|| ..into_any_element()).with_fallback(|| ..)`. `.id(..)` is needed for the loading state to show.
- **Indicators:** `Skeleton::new()`, `Spinner::new()`.
- **Badges and tags:** `Badge::new().count(n)`, `Tag::primary().outline().rounded_full().child(..)`, `Tag::custom(bg, fg, border)`.
- **Other:** `Link::new(id).href(..)`, `form::{v_form, field}`, `setting::{Settings, SettingPage, SettingGroup, SettingItem}`, `avatar::Avatar`.

## Async
- `cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| { …; this.update(cx, |this, cx| { …; cx.notify() }) }).detach()`. You can also store the `Task` in a field, so dropping it cancels.
- `cx.spawn_in(window, async move |this, cx: &mut AsyncWindowContext| ..)` gives window access, for toasts.
- `cx.background_spawn(..)` and `cx.background_executor().timer(dur)`.
- There is NO tokio integration in gpui. Spawn core calls on our runtime and await the `JoinHandle`.

## Navigation and keys
- `gpui_kit::base::{NavStack, NavStackState, NavMotion}` is a stack of `AnyView`s. We keep our own route stacks in AppState, and NavStack is optional for rendering.
- `actions!(ns, [A, B])`; `cx.bind_keys([KeyBinding::new("cmd-[", Back, Some("LibreSpydus"))])`; `div().key_context("LibreSpydus").on_action(cx.listener(|this, _: &Back, w, cx| ..))`.

## Testing (`$R/gpui-kit-0.7.3/TESTING.md`, `src/test.rs`, `tests/`)
- `[dev-dependencies] gpui-kit = { version = "=0.7.3", features = ["test-support"] }`.
- `#[gpui_kit::test] fn t(cx: &mut TestAppContext)`; `VisualTestContext` is also available.
- `use gpui_kit::test::{TestWindowExt, TestAppContextExt, TestSupportExt, ElementSnapshot};`
  - `window.find("id")` returns an ElementSnapshot with `.value() .focused() .checked() .visible() .bounds()`;
  - `try_find`, `within(scope)`;
  - `click`, `right_click`, `double_click`, `hover`, `scroll`, `drag_to`;
  - `press("cmd-a")`, `input("text")`, `render_frame(cx)`;
  - `cx.wait_for(handle, dur, |w, cx| pred).await`.
  - These use real hit testing and accessibility properties, so give elements `.id()`.
- Headless screenshots (macOS Metal, main thread only, a `test = false` target with a custom main): `HeadlessAppContext::with_platform(..)` with `cx.capture_screenshot(handle) -> RgbaImage` (`$R/gpui-kit-0.7.3/tests/rendering.rs`). Optional; `desktop/scripts/screenshot.sh` is the default evidence path.

## Accessibility
gpui-pre uses AccessKit. An element is exposed only with both `.id()` and `.role(Role::…)`. Other builders: `.aria_label()`, `.aria_selected()`, `.aria_expanded()`, `.aria_toggled()`; components take `accessibility_label(..)`. Guide: `$R/gpui-pre-0.3.9/src/_accessibility.rs`, `examples/a11y.rs`.

## Screenshots: the app renders its own window
`window.render_to_image() -> anyhow::Result<image::RgbaImage>` renders the last frame with Metal,
off-screen, and needs gpui's `test-support` feature. It works over SSH with no Screen Recording
permission. (`screencapture` from an SSH session fails with "could not create image from
window".) A spike checked this on mini-knit:
```rust
let (handle, _) = gpui_kit::open_window(WindowOptions { focus: false, ..Default::default() }, cx, build)?;
cx.spawn(async move |cx| {
    cx.background_executor().timer(Duration::from_secs(3)).await;
    let _ = cx.update(|cx| {
        if let Ok(Ok(img)) = handle.update(cx, |_, window, _| window.render_to_image()) { img.save(path).unwrap(); }
        cx.quit();
    });
}).detach();
```
The desktop app exposes this as the debug flags `--screenshot <png>` / `--screenshot-after <s>`,
behind the cargo feature `evidence` (= `gpui-kit/test-support` + `image` with png).

**Lockfile gotcha:** with `test-support` on, Cargo also resolves gpui-kit's optional `gpui-fast*`
crates, which are referenced through weak `?` features. An old lockfile can then fail with
"failed to select a version for `cocoa`" (or `unicode-properties`). Regenerate the lockfile with
`cargo generate-lockfile` rather than pinning versions by hand.

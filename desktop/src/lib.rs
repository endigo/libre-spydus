//! LibreSpydus for the desktop (docs/desktop-app.md): GPUI + gpui-kit on the shared Rust core,
//! called directly. `app::run` starts it; the modules follow the contract's layout (§3).

pub mod app;
pub mod backend;
#[rustfmt::skip]
pub mod copy;
pub mod features;
pub mod launch;
pub mod platform;
pub mod runtime;
pub mod shell;
pub mod state;
pub mod theme;
pub mod timing;
pub mod ui;

pub use launch::LaunchOptions;

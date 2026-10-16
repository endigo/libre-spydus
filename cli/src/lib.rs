//! LibreSpydus for the terminal (`spydus`, docs/cli.md): a command-line client for a Spydus
//! library on the shared Rust core, calling it directly as the desktop app does.

pub mod app;
pub mod args;
pub mod backend;
pub mod book_ref;
pub mod commands;
#[rustfmt::skip]
#[allow(clippy::module_inception)]
pub mod copy;
pub mod error;
pub mod formatting;
pub mod output;
pub mod patron;
pub mod store;
pub mod text;

#[cfg(test)]
mod golden_tests;
#[cfg(test)]
pub mod testing;

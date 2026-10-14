//! Every feature module (docs/desktop-app.md §3, §5.6). Wave 1 (D1) gives each a placeholder with
//! its exact entry point; a wave-2 worker replaces the inside of its own directories:
//! - D2: `onboarding`, `profile`, `my_card`
//! - D3: `home`, `search`, `featured`
//! - D4: `detail`
//! - D5: `your_books`, `my_picks`
//!
//! Each screen is a struct implementing `gpui::Render`, built by the shell with
//! `cx.new(|cx| X::new(.., window, cx))`; the shell keeps one entity per stack entry, so going back
//! doesn't reload. A feature-only `AppState` operation goes in an `impl AppState { … }` block
//! inside the feature's own directory.

pub mod detail;
pub mod featured;
pub mod home;
pub mod my_card;
pub mod my_picks;
pub mod onboarding;
pub mod profile;
pub mod search;
pub mod your_books;

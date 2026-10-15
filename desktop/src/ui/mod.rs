//! Shared components (docs/desktop-app.md §7.2), the phones' set so the screens line up: covers,
//! shelf covers and row cards; section headers, the search field, pill and circle buttons; genre
//! tiles and chips; loading / empty / error states; toasts, confirmation dialogs and card menus.
//! The debug `--route gallery` shows them all.

pub mod book_views;
pub mod card_menu;
pub mod controls;
pub mod cover;
pub mod dialog;
#[cfg(debug_assertions)]
pub mod gallery;
pub mod icons;
pub mod states;
pub mod style;
pub mod toast;

pub use book_views::{BookRowCard, ShelfCover};
pub use card_menu::{CardAction, card_menu_button, with_context_menu};
pub use controls::{
    Banner, BannerTone, Chip, ChipTone, CircleIconButton, GenreTile, OnClick, PrimaryPillButton, SearchField,
    SearchFieldButton, SecondaryPillButton, SectionHeader, on_click,
};
pub use cover::{CoverImage, CoverPlaceholder};
pub use dialog::{ConfirmDialog, DialogButton, dialog_title, sheet_dialog};
pub use icons::{AppAssets, Lucide, icon};
pub use states::{EmptyStateView, ErrorStateView, LoadingView, PlaceholderScreen, SkeletonBlock, pulse};
pub use style::{FontExt, card_columns, card_grid, content_column};

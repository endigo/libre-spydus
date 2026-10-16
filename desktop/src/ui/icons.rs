//! Icons: gpui-kit's bundled Lucide subset plus the extra Lucide icons the app draws, chained into
//! one asset source (`AppAssets`) installed at launch.

use std::borrow::Cow;

use gpui_kit::component::Icon;
use gpui_kit::{AssetSource, Hsla, Pixels, Result, SharedString, Styled};

pub use gpui_kit::assets::IconName as Lucide;

gpui_kit::assets::icon_assets!(
    ExtraIcons,
    [
        House,
        LibraryBig,
        Library,
        BookMarked,
        Bookmark,
        BookmarkCheck,
        BookOpenText,
        CircleUserRound,
        Barcode,
        ScanBarcode,
        Ellipsis,
        ChevronLeft,
        RefreshCw,
        Sparkles,
        Cloud,
        Atom,
        Landmark,
        WandSparkles,
        ChefHat,
        Baby,
        VenetianMask,
        Skull,
        User,
        Globe,
        PartyPopper,
        Monitor,
        Circle,
        Puzzle,
        MapPinned,
        WifiOff,
        ShieldCheck,
        TriangleAlert,
        UserRoundX,
        BookX,
        Image,
        LogOut,
        Search
    ]
);

/// The app's assets: the extra icons, then gpui-kit's own.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

/// `name` at `size`, in `color`.
pub fn icon(name: Lucide, size: Pixels, color: Hsla) -> Icon {
    Icon::new(name).size(size).text_color(color)
}

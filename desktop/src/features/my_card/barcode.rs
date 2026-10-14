//! Where a card barcode's modules go (iOS `BarcodeLayout.swift` + `BarcodeView.swift`; docs/
//! native-apps.md §5.9): a whole number of device pixels per module, as wide as fits with at least
//! `QUIET_ZONE` white modules each side, centred, each run of black modules one bar. Scanners need
//! crisp, even bars, so the bars are drawn as divs on whole device pixels.

use std::ops::Range;

use gpui_kit::{App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::theme;

/// White modules each side of the bars, at least.
pub const QUIET_ZONE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarcodeLayout {
    /// Device pixels per module: floor(available / (modules + 2 × quiet zone)), at least 1.
    pub module_pixels: usize,
    /// From the leading edge of the available width to the first module.
    pub leading_pixels: usize,
    /// The bars' total width.
    pub bars_pixels: usize,
}

impl BarcodeLayout {
    pub fn new(module_count: usize, available_pixels: usize) -> Self {
        let module_pixels = (available_pixels / (module_count + 2 * QUIET_ZONE)).max(1);
        let bars_pixels = module_count * module_pixels;
        // Rounded down: an odd pixel left over goes on the trailing side.
        let leading_pixels = available_pixels.saturating_sub(bars_pixels) / 2;
        Self { module_pixels, leading_pixels, bars_pixels }
    }

    /// The black bars as pixel ranges from the leading edge, a run of black modules as one bar.
    pub fn bars(&self, modules: &[bool]) -> Vec<Range<usize>> {
        let mut bars = Vec::new();
        let mut start = None;
        for (index, &black) in modules.iter().enumerate() {
            match (black, start) {
                (true, None) => start = Some(index),
                (false, Some(first)) => {
                    bars.push(self.pixels(first..index));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(first) = start {
            bars.push(self.pixels(first..modules.len()));
        }
        bars
    }

    fn pixels(&self, modules: Range<usize>) -> Range<usize> {
        (self.leading_pixels + modules.start * self.module_pixels)
            ..(self.leading_pixels + modules.end * self.module_pixels)
    }
}

/// The bars, black on the white under them, `width` × `height` points at `scale` device pixels per
/// point. Put it on white.
#[derive(IntoElement)]
pub struct BarcodeBars {
    modules: Vec<bool>,
    width: f32,
    height: f32,
}

impl BarcodeBars {
    pub fn new(modules: Vec<bool>, width: f32, height: f32) -> Self {
        Self { modules, width, height }
    }
}

impl RenderOnce for BarcodeBars {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let scale = window.scale_factor().max(1.);
        let available = (self.width * scale + 0.001).floor() as usize;
        let layout = BarcodeLayout::new(self.modules.len(), available);
        div().relative().flex_none().w(px(self.width)).h(px(self.height)).bg(theme::color(0xFFFFFF)).children(
            layout.bars(&self.modules).into_iter().map(|bar| {
                div()
                    .absolute()
                    .top_0()
                    .left(px(bar.start as f32 / scale))
                    .w(px(bar.len() as f32 / scale))
                    .h_full()
                    .bg(theme::color(0x000000))
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_pixels_per_module_with_quiet_zones_centred() {
        // 30 modules + 20 quiet → 500 / 50 = 10 px a module; 300 px of bars, 100 px each side.
        let layout = BarcodeLayout::new(30, 500);
        assert_eq!(layout, BarcodeLayout { module_pixels: 10, leading_pixels: 100, bars_pixels: 300 });
        // An odd pixel left over goes on the trailing side.
        let odd = BarcodeLayout::new(30, 501);
        assert_eq!((odd.module_pixels, odd.leading_pixels), (10, 100));
        // Too narrow: still one pixel a module.
        assert_eq!(BarcodeLayout::new(100, 50).module_pixels, 1);
    }

    #[test]
    fn runs_of_black_modules_are_one_bar() {
        let layout = BarcodeLayout::new(6, 52);
        assert_eq!(layout.module_pixels, 2);
        let bars = layout.bars(&[true, true, false, true, false, true]);
        let lead = layout.leading_pixels;
        assert_eq!(bars, [lead..lead + 4, lead + 6..lead + 8, lead + 10..lead + 12]);
    }

    #[test]
    fn real_card_numbers_fit() {
        use librespydus_core::barcode::{BarcodeSymbology, card_barcode};
        for symbology in [BarcodeSymbology::Code128, BarcodeSymbology::Code39, BarcodeSymbology::Codabar] {
            let barcode = card_barcode("DEMO0012345".into(), symbology).expect("encodable");
            let layout = BarcodeLayout::new(barcode.modules.len(), 800);
            assert!(layout.module_pixels >= 1);
            assert!(layout.leading_pixels >= QUIET_ZONE * layout.module_pixels);
            assert!(layout.leading_pixels + layout.bars_pixels <= 800);
        }
    }
}

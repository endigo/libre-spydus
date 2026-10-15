//! A library's directory logo on a white rounded tile, or a monogram on a genre colour while it
//! loads and when there's none (most directory entries have no logo). iOS `LibraryLogo.swift`,
//! including its `LogoTone` rule: a light logo on a clear background (City of Fremantle's white
//! one) goes on navy, so it doesn't vanish on white.

use std::cell::RefCell;
use std::collections::HashMap;

use gpui_kit::{
    AnyElement, App, ElementId, ImgResourceLoader, InteractiveElement, IntoElement, ObjectFit, ParentElement,
    RenderOnce, Resource, SharedString, Styled, StyledImage, Window, div, img, linear_color_stop, linear_gradient, px,
};
use librespydus_core::LibraryInfo;

use super::directory::monogram;
use crate::theme::{self, colors};

#[derive(IntoElement)]
pub struct LibraryLogo {
    id: ElementId,
    name: String,
    logo_url: Option<String>,
    size: f32,
}

impl LibraryLogo {
    pub fn new(id: impl Into<ElementId>, name: impl Into<String>, logo_url: Option<String>) -> Self {
        Self { id: id.into(), name: name.into(), logo_url, size: 44. }
    }

    pub fn library(id: impl Into<ElementId>, library: &LibraryInfo) -> Self {
        Self::new(id, library.name.clone(), library.logo_url.clone())
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

fn monogram_tile(name: &str, size: f32) -> AnyElement {
    let tint = theme::genre_color(name);
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(135., linear_color_stop(tint.opacity(0.8), 0.), linear_color_stop(tint, 1.)))
        .text_size(px(size * 0.38))
        .font_weight(gpui_kit::FontWeight::BOLD)
        .text_color(theme::color(0xFFFFFF))
        .child(monogram(name))
        .into_any_element()
}

impl RenderOnce for LibraryLogo {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = self.size;
        let frame = div()
            .flex_none()
            .size(px(size))
            .rounded(px(size * 0.26))
            .overflow_hidden()
            .border_1()
            .border_color(theme::color_alpha(0xFFFFFF, 0.08));
        match self.logo_url.filter(|url| url.starts_with("http")) {
            Some(url) => {
                let (loading, fallback) = (self.name.clone(), self.name);
                let tile = if is_light_on_clear(&url, window, cx) { colors::BACKGROUND } else { 0xFFFFFF };
                frame.bg(theme::color(tile)).child(
                    img(SharedString::from(url))
                        .id(self.id)
                        .size_full()
                        .p(px(size * 0.1))
                        .object_fit(ObjectFit::Contain)
                        .with_loading(move || monogram_tile(&loading, size))
                        .with_fallback(move || monogram_tile(&fallback, size)),
                )
            }
            None => frame.child(monogram_tile(&self.name, size)),
        }
    }
}

thread_local! {
    /// `LogoTone`'s verdict per logo URL, decided once per image.
    static VERDICTS: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
}

/// Whether the logo at `url` is light on a clear background, once `img(url)` has loaded it (the
/// same asset, so nothing is fetched twice); `false` until then.
fn is_light_on_clear(url: &str, window: &mut Window, cx: &mut App) -> bool {
    if let Some(verdict) = VERDICTS.with(|verdicts| verdicts.borrow().get(url).copied()) {
        return verdict;
    }
    let Some(Ok(image)) = window.use_asset::<ImgResourceLoader>(&Resource::Uri(url.to_owned().into()), cx) else {
        return false;
    };
    let size = image.size(0);
    let verdict = image.as_bytes(0).is_some_and(|bytes| {
        let (width, height) = (usize::try_from(size.width.0).unwrap_or(0), usize::try_from(size.height.0).unwrap_or(0));
        LogoTone::measure(bytes, width, height)
    });
    VERDICTS.with(|verdicts| verdicts.borrow_mut().insert(url.to_owned(), verdict));
    verdict
}

/// iOS `LogoTone`: some logos are white on a transparent background and vanish on the white tile.
pub struct LogoTone;

impl LogoTone {
    const SIDE: usize = 16;

    /// Light (mean luminance of the visible pixels > 0.85) and at least a quarter transparent,
    /// judged on a 16×16 thumbnail. `bgra` is straight (not premultiplied) BGRA, as GPUI decodes
    /// images, `width` × `height` pixels.
    pub fn measure(bgra: &[u8], width: usize, height: usize) -> bool {
        if width == 0 || height == 0 || bgra.len() < width * height * 4 {
            return false;
        }
        let side = Self::SIDE;
        let (mut luminance, mut coverage, mut clear_cells) = (0.0, 0.0, 0);
        for cell_y in 0..side {
            for cell_x in 0..side {
                // The cell's box of source pixels (at least one), averaged premultiplied.
                let (x0, x1) = (cell_x * width / side, ((cell_x + 1) * width / side).max(cell_x * width / side + 1));
                let (y0, y1) = (cell_y * height / side, ((cell_y + 1) * height / side).max(cell_y * height / side + 1));
                let (mut alpha, mut luma, mut count) = (0.0, 0.0, 0.0);
                for y in y0..y1.min(height) {
                    for x in x0..x1.min(width) {
                        let pixel = &bgra[(y * width + x) * 4..][..4];
                        let a = f64::from(pixel[3]) / 255.;
                        let (blue, green, red) =
                            (f64::from(pixel[0]) / 255., f64::from(pixel[1]) / 255., f64::from(pixel[2]) / 255.);
                        // Rec. 709 luma, weighted by alpha.
                        luma += (0.2126 * red + 0.7152 * green + 0.0722 * blue) * a;
                        alpha += a;
                        count += 1.;
                    }
                }
                if count == 0. {
                    continue;
                }
                let (alpha, luma) = (alpha / count, luma / count);
                if alpha < 0.1 {
                    clear_cells += 1;
                }
                luminance += luma;
                coverage += alpha;
            }
        }
        coverage > 0. && clear_cells * 4 >= side * side && luminance / coverage > 0.85
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `size`×`size` BGRA image: `ink` (grey level, alpha) inside a centred square of `ink_side`,
    /// `paper` (grey level, alpha) around it.
    fn image(size: usize, ink_side: usize, ink: (u8, u8), paper: (u8, u8)) -> Vec<u8> {
        let start = (size - ink_side) / 2;
        let mut bytes = Vec::with_capacity(size * size * 4);
        for y in 0..size {
            for x in 0..size {
                let inside = (start..start + ink_side).contains(&x) && (start..start + ink_side).contains(&y);
                let (grey, alpha) = if inside { ink } else { paper };
                bytes.extend_from_slice(&[grey, grey, grey, alpha]);
            }
        }
        bytes
    }

    #[test]
    fn a_white_logo_on_a_clear_background_is_light() {
        // Fremantle: white ink, transparent around it.
        assert!(LogoTone::measure(&image(64, 40, (255, 255), (0, 0)), 64, 64));
        // Light grey still counts.
        assert!(LogoTone::measure(&image(37, 20, (235, 255), (255, 0)), 37, 37));
    }

    #[test]
    fn dark_or_opaque_logos_stay_on_white() {
        // Dark ink on a clear background.
        assert!(!LogoTone::measure(&image(64, 40, (20, 255), (0, 0)), 64, 64));
        // White ink on an opaque white background: nothing transparent.
        assert!(!LogoTone::measure(&image(64, 40, (255, 255), (255, 255)), 64, 64));
        // Mostly ink: less than a quarter clear.
        assert!(!LogoTone::measure(&image(64, 60, (255, 255), (0, 0)), 64, 64));
        // Fully transparent, empty, or short of bytes.
        assert!(!LogoTone::measure(&image(16, 0, (255, 255), (0, 0)), 16, 16));
        assert!(!LogoTone::measure(&[], 0, 0));
        assert!(!LogoTone::measure(&[255; 8], 4, 4));
    }

    #[test]
    fn tiny_images_are_measured_too() {
        // Smaller than the 16×16 thumbnail: each source pixel fills several cells.
        assert!(LogoTone::measure(&image(4, 2, (255, 255), (0, 0)), 4, 4));
    }
}

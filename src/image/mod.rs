//! Image loading, resizing, and preprocessing

use crate::cli::InputSource;
use crate::terminal::CHAR_ASPECT_RATIO;
use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, GrayImage, ImageReader, Limits, Pixel};
use std::io::{Cursor, Read};
use std::time::Duration;

/// Largest input accepted from a file or URL.
pub const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
/// Time allowed for a whole URL download.
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);
/// Largest decoded image (width or height, in pixels).
pub const MAX_IMAGE_SIDE: u32 = 16_384;
/// Memory the decoder may allocate for one image or GIF canvas.
pub const MAX_DECODE_ALLOC: u64 = 512 * 1024 * 1024;

/// Decoder limits shared by still images and GIF frames.
pub fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    limits
}

/// Read the raw bytes of a file or URL, refusing inputs over [`MAX_INPUT_BYTES`].
pub fn read_source(source: &InputSource) -> Result<Vec<u8>> {
    match source {
        InputSource::File(path) => {
            let file = std::fs::File::open(path)
                .with_context(|| format!("Failed to open {}", path.display()))?;
            read_limited(file, MAX_INPUT_BYTES)
                .with_context(|| format!("Failed to read {}", path.display()))
        }
        InputSource::Url(url) => fetch_url(url, MAX_INPUT_BYTES, DOWNLOAD_TIMEOUT),
    }
}

fn read_limited(reader: impl Read, max_bytes: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        anyhow::bail!("Input is larger than {} MiB", max_bytes / (1024 * 1024));
    }
    Ok(bytes)
}

/// Download a URL with a size cap and an overall timeout. Redirects are followed
/// (at most 10); responses that are clearly not images are refused.
pub fn fetch_url(url: &str, max_bytes: u64, timeout: Duration) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| anyhow::anyhow!("Failed to fetch URL: {}", e))?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }
    if let Some(kind) = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        let kind = kind.to_ascii_lowercase();
        if kind.starts_with("text/") || kind.contains("json") || kind.contains("html") {
            anyhow::bail!("URL did not return an image (content type: {kind})");
        }
    }
    if response.content_length().is_some_and(|n| n > max_bytes) {
        anyhow::bail!("Input is larger than {} MiB", max_bytes / (1024 * 1024));
    }
    read_limited(response, max_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))
}

/// True when the bytes start with a GIF signature.
pub fn is_gif_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")
}

/// Decode a still image (for a GIF, its first frame) within [`decode_limits`].
pub fn decode_image(bytes: &[u8]) -> Result<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    reader.limits(decode_limits());
    let image = reader
        .decode()
        .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;
    if image.width() == 0 || image.height() == 0 {
        anyhow::bail!("Image has no pixels");
    }
    Ok(image)
}

/// Load an image from a file path or URL
pub fn load_image(source: &InputSource) -> Result<DynamicImage> {
    decode_image(&read_source(source)?)
}

/// Resize mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeMode {
    /// The whole image inside the given size, aspect ratio preserved
    Fit,
    /// Cover the given size, cropping the overflow at the center
    Fill,
    /// Exactly the given size, aspect ratio ignored
    Stretch,
    /// The center of the image at full resolution, cropped to the given size
    Crop,
}

impl ResizeMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "fit" => Ok(ResizeMode::Fit),
            "fill" => Ok(ResizeMode::Fill),
            "stretch" => Ok(ResizeMode::Stretch),
            "crop" => Ok(ResizeMode::Crop),
            _ => Err(format!("Unknown resize mode: {}", s)),
        }
    }
}

impl std::str::FromStr for ResizeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ResizeMode::from_str(s)
    }
}

/// Calculate output dimensions (in characters) based on image and terminal size
///
/// Takes into account the character aspect ratio (typically 0.5 for monospace).
pub fn calculate_dimensions(
    img: &DynamicImage,
    term_cols: usize,
    term_rows: usize,
) -> (usize, usize) {
    let (img_w, img_h) = img.dimensions();
    calculate_dimensions_for(img_w, img_h, term_cols, term_rows)
}

/// [`calculate_dimensions`] for a known pixel size.
pub fn calculate_dimensions_for(
    img_w: u32,
    img_h: u32,
    term_cols: usize,
    term_rows: usize,
) -> (usize, usize) {
    let img_aspect = img_w.max(1) as f64 / img_h.max(1) as f64;

    // Adjust aspect ratio for terminal character shape
    // Characters are typically 2x as tall as they are wide
    let adjusted_aspect = img_aspect / CHAR_ASPECT_RATIO;

    // Calculate width that fits within terminal
    let width = term_cols.saturating_sub(2).min(img_w as usize); // Leave margin

    // Calculate height maintaining aspect ratio
    let height =
        ((width as f64 / adjusted_aspect).ceil() as usize).min(term_rows.saturating_sub(2)); // Leave margin

    // When the height was capped, narrow the width to keep the proportions
    let width = width.min((height as f64 * adjusted_aspect).ceil() as usize);

    // Ensure minimum dimensions
    (width.max(1), height.max(1))
}

/// Resize an image for display in `target_width` × `target_height` cells.
///
/// `cell_aspect` is the width:height ratio of one output cell (one pixel of the
/// result): [`CHAR_ASPECT_RATIO`] for characters, 1.0 for Braille dots. It keeps
/// the picture's proportions on screen in `fit`, `fill` and `crop`.
pub fn resize_for_terminal(
    img: &DynamicImage,
    target_width: usize,
    target_height: usize,
    resize_mode: ResizeMode,
    preserve_aspect_ratio: bool,
    cell_aspect: f64,
) -> DynamicImage {
    let target_w = target_width.max(1) as u32;
    let target_h = target_height.max(1) as u32;
    let (orig_w, orig_h) = img.dimensions();
    // How many cells wide per cell high the picture needs to look right.
    let aspect = (orig_w.max(1) as f64 / orig_h.max(1) as f64) / cell_aspect;
    let target_aspect = target_w as f64 / target_h as f64;
    let filter = image::imageops::FilterType::Lanczos3;
    let cells = |v: f64| (v.round() as u32).max(1);

    if !preserve_aspect_ratio || resize_mode == ResizeMode::Stretch {
        return img.resize_exact(target_w, target_h, filter);
    }
    match resize_mode {
        ResizeMode::Fit => {
            let (w, h) = if aspect > target_aspect {
                (target_w, cells(target_w as f64 / aspect).min(target_h))
            } else {
                (cells(target_h as f64 * aspect).min(target_w), target_h)
            };
            img.resize_exact(w, h, filter)
        }
        ResizeMode::Fill => {
            // The centered source region whose on-screen shape matches the box.
            let pixels_per_row = target_aspect * cell_aspect;
            let (w, h) = if aspect > target_aspect {
                (
                    ((orig_h as f64 * pixels_per_row).round() as u32).clamp(1, orig_w),
                    orig_h,
                )
            } else {
                (
                    orig_w,
                    ((orig_w as f64 / pixels_per_row).round() as u32).clamp(1, orig_h),
                )
            };
            crop_to_center(img, w as usize, h as usize).resize_exact(target_w, target_h, filter)
        }
        ResizeMode::Crop => {
            // One source pixel per cell horizontally; the cell shape decides the height.
            let region_w = target_w.min(orig_w);
            let region_h = ((target_h as f64 / cell_aspect).round() as u32).clamp(1, orig_h);
            let center = crop_to_center(img, region_w as usize, region_h as usize);
            let (w, h) = (region_w, cells(region_h as f64 * cell_aspect).min(target_h));
            center.resize_exact(w, h, filter)
        }
        ResizeMode::Stretch => unreachable!(),
    }
}

/// Crop image to center of given dimensions
pub fn crop_to_center(img: &DynamicImage, width: usize, height: usize) -> DynamicImage {
    let (w, h) = img.dimensions();
    let target_w = width as u32;
    let target_h = height as u32;

    if w <= target_w && h <= target_h {
        return img.clone();
    }

    let x: u32 = ((w as i64 - target_w as i64).max(0) / 2) as u32;
    let y: u32 = ((h as i64 - target_h as i64).max(0) / 2) as u32;

    img.crop_imm(x, y, target_w.min(w), target_h.min(h))
}

/// Convert image to grayscale
pub fn to_grayscale(img: &DynamicImage) -> GrayImage {
    img.to_luma8()
}

/// Get pixel data as RGBA
pub fn get_pixel_rgba(img: &DynamicImage, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let pixel = img.get_pixel(x, y);
    let channels = pixel.channels();
    let r = *channels.first().unwrap_or(&0);
    let g = *channels.get(1).unwrap_or(&0);
    let b = *channels.get(2).unwrap_or(&0);
    let a = *channels.get(3).unwrap_or(&255);
    (r, g, b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::new_rgb8(w, h)
    }

    #[test]
    fn test_calculate_dimensions() {
        let (w, h) = calculate_dimensions(&image(100, 100), 80, 40);
        assert!(w <= 80);
        assert!(h <= 40);
    }

    #[test]
    fn auto_size_keeps_the_picture_proportions_in_character_cells() {
        // A square needs twice as many columns as rows (cells are 1:2).
        assert_eq!(calculate_dimensions_for(400, 400, 82, 100), (80, 40));
        // Width capped by the image itself: one column per pixel at most.
        assert_eq!(calculate_dimensions_for(10, 10, 82, 100), (10, 5));
        // A tall image is limited by the terminal height, and narrowed to match.
        assert_eq!(calculate_dimensions_for(100, 1000, 82, 24), (5, 22));
        assert_eq!(calculate_dimensions_for(1000, 1000, 82, 26), (48, 24));
        // A degenerate terminal never produces a zero size.
        assert_eq!(calculate_dimensions_for(100, 100, 0, 0), (1, 1));
    }

    #[test]
    fn resize_modes_produce_the_documented_sizes() {
        let c = CHAR_ASPECT_RATIO;
        let dims = |img: &DynamicImage, w, h, mode, keep, cell| {
            resize_for_terminal(img, w, h, mode, keep, cell).dimensions()
        };
        let square = image(300, 300);
        let wide = image(400, 100);
        // fit: a square in an 80x40 box is 80x40 (2 columns per row).
        assert_eq!(dims(&square, 80, 40, ResizeMode::Fit, true, c), (80, 40));
        // fit: the same square in a 80x10 box is limited by height.
        assert_eq!(dims(&square, 80, 10, ResizeMode::Fit, true, c), (20, 10));
        // fit: a 4:1 image is 8 columns per row; limited by width.
        assert_eq!(dims(&wide, 40, 40, ResizeMode::Fit, true, c), (40, 5));
        // fill and stretch always produce exactly the box.
        assert_eq!(dims(&wide, 40, 40, ResizeMode::Fill, true, c), (40, 40));
        assert_eq!(dims(&square, 33, 7, ResizeMode::Stretch, true, c), (33, 7));
        assert_eq!(dims(&square, 33, 7, ResizeMode::Fit, false, c), (33, 7));
        // crop: the 80-pixel-wide center of the square at one pixel per column.
        assert_eq!(dims(&square, 80, 40, ResizeMode::Crop, true, c), (80, 40));
        // crop: never wider than the image itself.
        assert_eq!(
            dims(&image(20, 300), 80, 40, ResizeMode::Crop, true, c),
            (20, 40)
        );
        // Braille dots are square cells.
        assert_eq!(
            dims(&square, 160, 160, ResizeMode::Fit, true, 1.0),
            (160, 160)
        );
    }

    #[test]
    fn extreme_shapes_never_resize_to_zero() {
        let c = CHAR_ASPECT_RATIO;
        for mode in [
            ResizeMode::Fit,
            ResizeMode::Fill,
            ResizeMode::Stretch,
            ResizeMode::Crop,
        ] {
            for img in [image(5000, 1), image(1, 5000), image(1, 1)] {
                let (w, h) = resize_for_terminal(&img, 80, 24, mode, true, c).dimensions();
                assert!(
                    w >= 1 && h >= 1 && w <= 80 && h <= 24,
                    "{mode:?} {:?} -> {w}x{h}",
                    img.dimensions()
                );
            }
        }
    }

    #[test]
    fn fill_crops_the_center() {
        // Left third red, middle green, right third blue; fill a square box.
        let mut img = image::RgbImage::new(300, 100);
        for (x, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgb(if x < 100 {
                [255, 0, 0]
            } else if x < 200 {
                [0, 255, 0]
            } else {
                [0, 0, 255]
            });
        }
        let out = resize_for_terminal(
            &DynamicImage::ImageRgb8(img),
            10,
            10,
            ResizeMode::Fill,
            true,
            1.0,
        );
        assert_eq!(out.dimensions(), (10, 10));
        let (r, g, b, _) = get_pixel_rgba(&out, 5, 5);
        assert!(
            g > 200 && r < 60 && b < 60,
            "center should be green, got {r},{g},{b}"
        );
    }

    #[test]
    fn test_resize_mode_from_str() {
        assert_eq!(ResizeMode::from_str("fit").unwrap(), ResizeMode::Fit);
        assert_eq!(ResizeMode::from_str("fill").unwrap(), ResizeMode::Fill);
        assert_eq!(
            ResizeMode::from_str("stretch").unwrap(),
            ResizeMode::Stretch
        );
        assert_eq!(ResizeMode::from_str("crop").unwrap(), ResizeMode::Crop);
        assert!(ResizeMode::from_str("zoom").is_err());
    }

    #[test]
    fn gif_signatures_are_recognised_by_content() {
        assert!(is_gif_bytes(b"GIF89a....") && is_gif_bytes(b"GIF87a...."));
        assert!(!is_gif_bytes(b"\x89PNG\r\n") && !is_gif_bytes(b"GIF"));
    }

    #[test]
    fn broken_and_truncated_images_are_errors_not_panics() {
        assert!(decode_image(b"").is_err());
        assert!(decode_image(b"not an image at all").is_err());
        let mut png = Vec::new();
        image(8, 8)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        assert!(decode_image(&png).is_ok());
        assert!(decode_image(&png[..png.len() / 2]).is_err());
    }

    #[test]
    fn oversized_declared_dimensions_are_refused() {
        // A tiny PNG header that claims 100000x100000 pixels.
        let mut png = Vec::new();
        image(1, 1)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        let err = decode_image(&png).unwrap_err().to_string();
        assert!(
            err.contains("decode") || err.contains("limit") || err.contains("Limit"),
            "{err}"
        );
    }

    #[test]
    fn inputs_over_the_size_cap_are_refused() {
        assert!(read_limited(&[0u8; 11][..], 10).is_err());
        assert_eq!(read_limited(&[7u8; 10][..], 10).unwrap().len(), 10);
    }
}

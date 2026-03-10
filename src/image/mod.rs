//! Image loading, resizing, and preprocessing

use crate::cli::InputSource;
use crate::terminal::CHAR_ASPECT_RATIO;
use anyhow::Result;
use image::{DynamicImage, GrayImage, GenericImageView, ImageReader, Pixel};
use std::io::Cursor;
use std::time::Duration;

/// Load an image from a file path
pub fn load_image(source: &InputSource) -> Result<DynamicImage> {
    match source {
        InputSource::File(path) => {
            image::open(path).map_err(|e| anyhow::anyhow!("Failed to load image: {}", e))
        }
        InputSource::Url(url) => {
            load_from_url(url)
        }
    }
}

/// Load an image from a URL
fn load_from_url(url: &str) -> Result<DynamicImage> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    let response = client.get(url).send()
        .map_err(|e| anyhow::anyhow!("Failed to fetch URL: {}", e))?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    let bytes = response.bytes()
        .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

    let cursor = Cursor::new(bytes);
    ImageReader::new(cursor)
        .with_guessed_format()?
        .decode()
        .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))
}

/// Resize mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeMode {
    /// Fit within the given dimensions while preserving aspect ratio
    Fit,
    /// Fill the given dimensions, potentially cropping
    Fill,
    /// Stretch to exactly fit the dimensions
    Stretch,
    /// Crop to the center of the image
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

/// Calculate output dimensions based on image and terminal size
///
/// Takes into account the character aspect ratio (typically 0.5 for monospace).
pub fn calculate_dimensions(
    img: &DynamicImage,
    term_cols: usize,
    term_rows: usize,
) -> (usize, usize) {
    let (img_w, img_h) = img.dimensions();
    let img_aspect = img_w as f64 / img_h as f64;

    // Adjust aspect ratio for terminal character shape
    // Characters are typically 2x as tall as they are wide
    let adjusted_aspect = img_aspect / CHAR_ASPECT_RATIO;

    // Calculate width that fits within terminal
    let width = term_cols.saturating_sub(2).min(img_w as usize); // Leave margin

    // Calculate height maintaining aspect ratio
    let height = ((width as f64 / adjusted_aspect).ceil() as usize)
        .min(term_rows.saturating_sub(2)); // Leave margin

    // Ensure minimum dimensions
    let width = width.max(1);
    let height = height.max(1);

    (width, height)
}

/// Resize image to target dimensions
pub fn resize_for_terminal(
    img: &DynamicImage,
    target_width: usize,
    target_height: usize,
    resize_mode: ResizeMode,
    preserve_aspect_ratio: bool,
) -> DynamicImage {
    let (orig_w, orig_h) = img.dimensions();
    let orig_aspect = orig_w as f64 / orig_h as f64;
    let target_aspect = target_width as f64 / target_height as f64;

    let (final_w, final_h): (u32, u32) = match (resize_mode, preserve_aspect_ratio) {
        (ResizeMode::Fit, true) => {
            // Fit within target while preserving aspect ratio
            if orig_aspect > target_aspect {
                // Image is wider than target
                let w = target_width as u32;
                let h = (target_width as f64 / orig_aspect) as u32;
                (w, h)
            } else {
                // Image is taller than target
                let h = target_height as u32;
                let w = (target_height as f64 * orig_aspect) as u32;
                (w, h)
            }
        }
        (ResizeMode::Fit, false) => (target_width as u32, target_height as u32),
        (ResizeMode::Fill, true) => {
            // Fill target, potentially cropping
            if orig_aspect > target_aspect {
                let h = target_height as u32;
                let w = (target_height as f64 * orig_aspect) as u32;
                (w, h)
            } else {
                let w = target_width as u32;
                let h = (target_width as f64 / orig_aspect) as u32;
                (w, h)
            }
        }
        (ResizeMode::Fill, false) => (target_width as u32, target_height as u32),
        (ResizeMode::Stretch, _) => (target_width as u32, target_height as u32),
        (ResizeMode::Crop, _) => {
            // Crop to center at target dimensions
            (target_width as u32, target_height as u32)
        }
    };

    let resized = img.resize_exact(
        final_w,
        final_h,
        image::imageops::FilterType::Lanczos3,
    );

    match resize_mode {
        ResizeMode::Crop => crop_to_center(&resized, target_width, target_height),
        _ => resized,
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
    let r = *channels.get(0).unwrap_or(&0);
    let g = *channels.get(1).unwrap_or(&0);
    let b = *channels.get(2).unwrap_or(&0);
    let a = *channels.get(3).unwrap_or(&255);
    (r, g, b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dimensions() {
        // Create a simple 100x100 image
        let img = DynamicImage::new_rgb8(100, 100);

        let (w, h) = calculate_dimensions(&img, 80, 40);
        assert!(w <= 80);
        assert!(h <= 40);
    }

    #[test]
    fn test_resize_mode_from_str() {
        assert_eq!(ResizeMode::from_str("fit").unwrap(), ResizeMode::Fit);
        assert_eq!(ResizeMode::from_str("fill").unwrap(), ResizeMode::Fill);
        assert_eq!(ResizeMode::from_str("stretch").unwrap(), ResizeMode::Stretch);
        assert_eq!(ResizeMode::from_str("crop").unwrap(), ResizeMode::Crop);
    }
}

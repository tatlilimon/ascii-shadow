//! Main conversion logic from pixels to ASCII characters

use crate::charset::Charset;
use crate::color::{rgb_to_grayscale, ColorMode};
use crate::image::get_pixel_rgba;
use image::{DynamicImage, GenericImageView};

/// Represents a colored character for output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColoredChar {
    pub char: char,
    pub color: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
}

/// Configuration for the converter
pub struct ConverterConfig {
    pub charset: Box<dyn Charset>,
    pub color_mode: ColorMode,
    pub contrast: i32,
    pub brightness: i32,
    pub invert: bool,
    pub background: Option<(u8, u8, u8)>,
}

impl std::fmt::Debug for ConverterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConverterConfig")
            .field("charset", &"<dyn Charset>")
            .field("color_mode", &self.color_mode)
            .field("contrast", &self.contrast)
            .field("brightness", &self.brightness)
            .field("invert", &self.invert)
            .field("background", &self.background)
            .finish()
    }
}

impl ConverterConfig {
    pub fn new(charset: Box<dyn Charset>) -> Self {
        Self {
            charset,
            color_mode: ColorMode::Grayscale,
            contrast: 0,
            brightness: 0,
            invert: false,
            background: None,
        }
    }
}

/// Convert an image to a grid of colored characters
pub fn convert(config: &ConverterConfig, img: &DynamicImage) -> Vec<ColoredChar> {
    let (width, height) = img.dimensions();
    let mut result = Vec::with_capacity((width * height + height) as usize);

    for y in 0..height {
        for x in 0..width {
            let pixel = get_pixel_rgba(img, x, y);
            let adjusted = adjust_pixel(config, pixel);

            let char = config.charset.map_brightness(adjusted.brightness);

            let color = if config.color_mode == ColorMode::None {
                None
            } else {
                Some(adjusted.color)
            };

            let bg = config.background;

            result.push(ColoredChar { char, color, bg });
        }
        result.push(ColoredChar {
            char: '\n',
            color: None,
            bg: None,
        });
    }

    result
}

/// Represents an adjusted pixel with brightness and color
#[derive(Debug, Clone, Copy)]
struct AdjustedPixel {
    brightness: u8,
    color: (u8, u8, u8),
}

/// Apply contrast, brightness, and inversion to a pixel
fn adjust_pixel(config: &ConverterConfig, pixel: (u8, u8, u8, u8)) -> AdjustedPixel {
    let (r, g, b, a) = pixel;

    // Handle transparency by blending with background or white
    let alpha_ratio = a as f32 / 255.0;
    let (r, g, b) = if a < 255 {
        if let Some((bg_r, bg_g, bg_b)) = config.background {
            (
                (bg_r as f32 * (1.0 - alpha_ratio) + r as f32 * alpha_ratio) as u8,
                (bg_g as f32 * (1.0 - alpha_ratio) + g as f32 * alpha_ratio) as u8,
                (bg_b as f32 * (1.0 - alpha_ratio) + b as f32 * alpha_ratio) as u8,
            )
        } else {
            // Blend with white
            (
                (255.0 * (1.0 - alpha_ratio) + r as f32 * alpha_ratio) as u8,
                (255.0 * (1.0 - alpha_ratio) + g as f32 * alpha_ratio) as u8,
                (255.0 * (1.0 - alpha_ratio) + b as f32 * alpha_ratio) as u8,
            )
        }
    } else {
        (r, g, b)
    };

    let mut brightness = rgb_to_grayscale(r, g, b);

    // Apply contrast
    if config.contrast != 0 {
        let contrast_factor = (config.contrast as f32 / 100.0) * 2.0; // -1 to 1
        let brightness_f = brightness as f32;
        let centered = brightness_f - 128.0;
        let adjusted = centered * (1.0 + contrast_factor) + 128.0;
        brightness = adjusted.clamp(0.0, 255.0) as u8;
    }

    // Apply brightness adjustment
    if config.brightness != 0 {
        let adjustment = config.brightness * 2; // Scale for more noticeable effect
        brightness = (brightness as i32 + adjustment).clamp(0, 255) as u8;
    }

    // Invert if needed
    if config.invert {
        brightness = 255 - brightness;
    }

    AdjustedPixel {
        brightness,
        color: (r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple() {
        let config = ConverterConfig::new(Box::new(crate::charset::StandardCharset));
        let img = DynamicImage::new_rgb8(2, 2);
        let result = convert(&config, &img);
        // Should have 2x2 characters + 2 newlines = 6
        assert_eq!(result.len(), 6);
        assert_eq!(result[2].char, '\n'); // First newline (after 2 chars)
        assert_eq!(result[4].char, ' '); // Character at (1,1) - black maps to space
    }

    #[test]
    fn test_adjust_pixel_no_changes() {
        let config = ConverterConfig::new(Box::new(crate::charset::StandardCharset));
        let pixel = (128, 128, 128, 255);
        let adjusted = adjust_pixel(&config, pixel);
        assert_eq!(adjusted.color, (128, 128, 128));
    }

    #[test]
    fn test_adjust_pixel_invert() {
        let config = ConverterConfig {
            invert: true,
            ..ConverterConfig::new(Box::new(crate::charset::StandardCharset))
        };
        let pixel = (100, 100, 100, 255);
        let adjusted = adjust_pixel(&config, pixel);
        // Grayscale of 100,100,100 is 100, inverted should be ~155
        assert!(adjusted.brightness > 150);
    }
}

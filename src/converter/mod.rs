//! Main conversion logic from pixels to ASCII characters

use crate::charset::Charset;
use crate::charset::braille::pixels_to_braille;
use crate::color::{ColorMode, rgb_to_grayscale};
use crate::image::{ResizeMode, get_pixel_rgba, resize_for_terminal};
use crate::output::format_colored;
use crate::terminal::CHAR_ASPECT_RATIO;
use image::{DynamicImage, GenericImageView};

/// Width:height of one Braille dot. A character cell is 2 dots wide and 4 tall,
/// so with 1:2 cells each dot is square.
pub const BRAILLE_DOT_ASPECT: f64 = CHAR_ASPECT_RATIO * 4.0 / 2.0;

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

/// Convert an image to Braille. Each character covers a 2x4 block of pixels and
/// raises a dot wherever the adjusted pixel is at least half bright; its color is
/// the average of the raised dots (or of the whole block when none are raised).
pub fn convert_braille(config: &ConverterConfig, img: &DynamicImage) -> Vec<ColoredChar> {
    let (width, height) = img.dimensions();
    let (cols, rows) = (width.div_ceil(2), height.div_ceil(4));
    let mut result = Vec::with_capacity(((cols + 1) * rows) as usize);

    for row in 0..rows {
        for col in 0..cols {
            let mut dots = [[false; 2]; 4];
            let (mut all, mut lit) = ([0u32; 4], [0u32; 4]);
            for dy in 0..4 {
                for dx in 0..2 {
                    let (x, y) = (col * 2 + dx, row * 4 + dy);
                    if x >= width || y >= height {
                        continue;
                    }
                    let p = adjust_pixel(config, get_pixel_rgba(img, x, y));
                    let rgb = [p.color.0 as u32, p.color.1 as u32, p.color.2 as u32, 1];
                    all.iter_mut().zip(rgb).for_each(|(sum, v)| *sum += v);
                    if p.brightness >= 128 {
                        dots[dy as usize][dx as usize] = true;
                        lit.iter_mut().zip(rgb).for_each(|(sum, v)| *sum += v);
                    }
                }
            }
            let source = if lit[3] > 0 { lit } else { all };
            let average = |i: usize| (source[i] / source[3].max(1)) as u8;
            let color = (config.color_mode != ColorMode::None)
                .then(|| (average(0), average(1), average(2)));
            result.push(ColoredChar {
                char: pixels_to_braille(dots),
                color,
                bg: config.background,
            });
        }
        result.push(ColoredChar {
            char: '\n',
            color: None,
            bg: None,
        });
    }

    result
}

/// Everything needed to turn a decoded picture into terminal text.
#[derive(Debug)]
pub struct RenderPlan {
    pub config: ConverterConfig,
    pub resize_mode: ResizeMode,
    pub preserve_aspect_ratio: bool,
}

impl RenderPlan {
    /// Render a picture into at most `cols` × `rows` characters, with ANSI colors.
    pub fn render(&self, img: &DynamicImage, cols: usize, rows: usize) -> String {
        let chars = if self.config.charset.is_braille() {
            let resized = resize_for_terminal(
                img,
                cols * 2,
                rows * 4,
                self.resize_mode,
                self.preserve_aspect_ratio,
                BRAILLE_DOT_ASPECT,
            );
            convert_braille(&self.config, &resized)
        } else {
            let resized = resize_for_terminal(
                img,
                cols,
                rows,
                self.resize_mode,
                self.preserve_aspect_ratio,
                CHAR_ASPECT_RATIO,
            );
            convert(&self.config, &resized)
        };
        format_colored(&chars, self.config.color_mode)
    }
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

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    #[test]
    fn braille_raises_dots_where_the_picture_is_bright() {
        // 4x4 image: left column pair white, right pair black -> two cells per row.
        let mut img = image::GrayImage::new(4, 4);
        for (x, _, p) in img.enumerate_pixels_mut() {
            *p = image::Luma([if x < 2 { 255 } else { 0 }]);
        }
        let config = ConverterConfig::new(Box::new(crate::charset::braille::BrailleCharset));
        let chars = convert_braille(&config, &DynamicImage::ImageLuma8(img));
        let text: String = chars.iter().map(|c| c.char).collect();
        assert_eq!(text, "⣿⠀\n");
        let inverted = ConverterConfig {
            invert: true,
            ..ConverterConfig::new(Box::new(crate::charset::braille::BrailleCharset))
        };
        let text: String = convert_braille(
            &inverted,
            &DynamicImage::ImageLuma8(image::GrayImage::new(2, 4)),
        )
        .iter()
        .map(|c| c.char)
        .collect();
        assert_eq!(text, "⣿\n", "invert raises the dots of a black block");
    }

    #[test]
    fn braille_handles_sizes_that_are_not_multiples_of_its_block() {
        let config = ConverterConfig::new(Box::new(crate::charset::braille::BrailleCharset));
        let img = DynamicImage::ImageLuma8(image::GrayImage::from_pixel(3, 5, image::Luma([255])));
        let text: String = convert_braille(&config, &img)
            .iter()
            .map(|c| c.char)
            .collect();
        // 3x5 pixels -> 2x2 cells; the partial cells only raise existing dots.
        assert_eq!(text, "⣿⡇\n⠉⠁\n");
    }

    #[test]
    fn rendering_fits_the_requested_box_in_every_charset() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            64,
            64,
            image::Rgb([200, 120, 40]),
        ));
        for name in [
            "standard",
            "extended",
            "alphanumeric",
            "numbers",
            "blocks",
            "braille",
        ] {
            let plan = RenderPlan {
                config: ConverterConfig {
                    color_mode: ColorMode::None,
                    ..ConverterConfig::new(crate::charset::from_str(name).unwrap())
                },
                resize_mode: ResizeMode::Fit,
                preserve_aspect_ratio: true,
            };
            let text = plan.render(&img, 40, 20);
            let rows = lines(text.trim_end_matches("\x1b[0m"));
            assert_eq!(rows.len(), 20, "{name}");
            assert!(
                rows.iter().all(|r| r.chars().count() == 40),
                "{name}: {rows:?}"
            );
        }
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

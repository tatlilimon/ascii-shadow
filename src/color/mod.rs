//! Color conversion and ANSI escape codes

use std::str::FromStr;

/// Color modes supported by the converter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// 24-bit truecolor (16.7 million colors)
    Truecolor,
    /// 256-color palette
    Color256,
    /// Grayscale only
    #[default]
    Grayscale,
    /// No color
    None,
}

impl FromStr for ColorMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "truecolor" | "24bit" | "rgb" => Ok(ColorMode::Truecolor),
            "256" | "8bit" => Ok(ColorMode::Color256),
            "grayscale" | "gray" | "grey" => Ok(ColorMode::Grayscale),
            "none" | "off" => Ok(ColorMode::None),
            _ => Err(format!("Unknown color mode: {}", s)),
        }
    }
}

/// Convert RGB to grayscale using human eye perception weights
///
/// Uses the formula: 0.299*R + 0.587*G + 0.114*B
pub fn rgb_to_grayscale(r: u8, g: u8, b: u8) -> u8 {
    ((0.299 * r as f64) + (0.587 * g as f64) + (0.114 * b as f64)) as u8
}

/// Convert RGB to 24-bit ANSI truecolor escape sequence (foreground)
pub fn rgb_to_truecolor_fg(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

/// Convert RGB to 24-bit ANSI truecolor escape sequence (background)
pub fn rgb_to_truecolor_bg(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[48;2;{};{};{}m", r, g, b)
}

/// Convert RGB to 256-color ANSI escape sequence (foreground)
pub fn rgb_to_256_fg(r: u8, g: u8, b: u8) -> String {
    let code = rgb_to_256_code(r, g, b);
    format!("\x1b[38;5;{}m", code)
}

/// Convert RGB to 256-color ANSI escape sequence (background)
pub fn rgb_to_256_bg(r: u8, g: u8, b: u8) -> String {
    let code = rgb_to_256_code(r, g, b);
    format!("\x1b[48;5;{}m", code)
}

/// Convert RGB to grayscale ANSI escape sequence (foreground)
pub fn grayscale_to_fg(gray: u8) -> String {
    // Map 0-255 to 232-255 grayscale range
    let code = 232 + (gray as u32 * 23 / 255);
    format!("\x1b[38;5;{}m", code)
}

/// Convert RGB to grayscale ANSI escape sequence (background)
pub fn grayscale_to_bg(gray: u8) -> String {
    let code = 232 + (gray as u32 * 23 / 255);
    format!("\x1b[48;5;{}m", code)
}

/// Reset ANSI color to default
pub fn reset_color() -> &'static str {
    "\x1b[0m"
}

/// Convert RGB to 256-color palette code (0-255)
///
/// The 256-color palette consists of:
/// - 0-15: 16 basic colors
/// - 16-231: 216 colors (6x6x6 RGB cube)
/// - 232-255: 24 grayscale colors
pub fn rgb_to_256_code(r: u8, g: u8, b: u8) -> u8 {
    // Check grayscale (all components are close in value)
    if r.abs_diff(g) < 8 && r.abs_diff(b) < 8 {
        let gray = 232 + (r as u32 * 23 / 255);
        return gray as u8;
    }

    // Map to 6x6x6 color cube
    let six = |v: u8| ((v as f32 * 5.0 / 255.0).round() as u32).min(5);
    let idx = 16 + 36 * six(r) + 6 * six(g) + six(b);
    idx as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_grayscale() {
        assert_eq!(rgb_to_grayscale(0, 0, 0), 0);
        assert_eq!(rgb_to_grayscale(255, 255, 255), 255);
        // Green has highest weight
        assert!(rgb_to_grayscale(0, 255, 0) > rgb_to_grayscale(255, 0, 0));
    }

    #[test]
    fn test_rgb_to_256_code() {
        // Black (all equal) goes to grayscale range
        assert_eq!(rgb_to_256_code(0, 0, 0), 232); // Black in grayscale
        // White (all equal) goes to grayscale range
        assert_eq!(rgb_to_256_code(255, 255, 255), 255); // White in grayscale
        // Red in color cube (not all equal)
        assert_eq!(rgb_to_256_code(255, 0, 0), 196);
    }

    #[test]
    fn test_color_mode_from_str() {
        assert_eq!(ColorMode::from_str("truecolor").unwrap(), ColorMode::Truecolor);
        assert_eq!(ColorMode::from_str("256").unwrap(), ColorMode::Color256);
        assert_eq!(ColorMode::from_str("grayscale").unwrap(), ColorMode::Grayscale);
        assert_eq!(ColorMode::from_str("none").unwrap(), ColorMode::None);
    }
}

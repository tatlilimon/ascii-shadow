//! Output formatting with ANSI color codes

use crate::color::{
    grayscale_to_bg, grayscale_to_fg, rgb_to_256_bg, rgb_to_256_fg, rgb_to_truecolor_bg,
    rgb_to_truecolor_fg, reset_color, ColorMode,
};
use crate::converter::ColoredChar;

/// Format a sequence of colored characters into a string with ANSI escape codes
pub fn format_colored(chars: &[ColoredChar], color_mode: ColorMode) -> String {
    let mut output = String::new();
    let mut last_color = None;
    let mut last_bg = None;

    for c in chars {
        // Newlines don't need color codes
        if c.char == '\n' {
            output.push('\n');
            // Reset after newline to prevent color bleeding
            if last_color.is_some() || last_bg.is_some() {
                output.push_str(reset_color());
                last_color = None;
                last_bg = None;
            }
            continue;
        }

        // Check if we need to change colors
        let color_changed = c.color != last_color;
        let bg_changed = c.bg != last_bg;

        if color_changed || bg_changed {
            output.push_str(reset_color());

            // Apply background color first
            if let Some(bg) = c.bg {
                output.push_str(&bg_ansi(bg, color_mode));
                last_bg = Some(bg);
            } else {
                last_bg = None;
            }

            // Apply foreground color
            if let Some(fg) = c.color {
                output.push_str(&fg_ansi(fg, color_mode));
                last_color = Some(fg);
            } else {
                last_color = None;
            }
        }

        output.push(c.char);
    }

    // Final reset to avoid affecting terminal after output
    output.push_str(reset_color());

    output
}

/// Get foreground ANSI code for a color based on mode
fn fg_ansi((r, g, b): (u8, u8, u8), mode: ColorMode) -> String {
    match mode {
        ColorMode::Truecolor => rgb_to_truecolor_fg(r, g, b),
        ColorMode::Color256 => rgb_to_256_fg(r, g, b),
        ColorMode::Grayscale => grayscale_to_fg((r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) as u8),
        ColorMode::None => String::new(),
    }
}

/// Get background ANSI code for a color based on mode
fn bg_ansi((r, g, b): (u8, u8, u8), mode: ColorMode) -> String {
    match mode {
        ColorMode::Truecolor => rgb_to_truecolor_bg(r, g, b),
        ColorMode::Color256 => rgb_to_256_bg(r, g, b),
        ColorMode::Grayscale => grayscale_to_bg((r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) as u8),
        ColorMode::None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_colored_no_color() {
        let chars = vec![
            ColoredChar {
                char: 'A',
                color: None,
                bg: None,
            },
            ColoredChar {
                char: '\n',
                color: None,
                bg: None,
            },
            ColoredChar {
                char: 'B',
                color: None,
                bg: None,
            },
        ];
        let output = format_colored(&chars, ColorMode::None);
        assert!(output.contains('A'));
        assert!(output.contains('B'));
        assert!(output.contains('\n'));
    }

    #[test]
    fn test_format_colored_simple() {
        let chars = vec![
            ColoredChar {
                char: '@',
                color: Some((255, 0, 0)),
                bg: None,
            },
            ColoredChar {
                char: '#',
                color: Some((0, 255, 0)),
                bg: None,
            },
        ];
        let output = format_colored(&chars, ColorMode::Truecolor);
        assert!(output.contains('@'));
        assert!(output.contains('#'));
        assert!(output.contains("\x1b[38;2;")); // ANSI truecolor prefix
        assert!(output.contains("\x1b[0m")); // Reset code
    }
}

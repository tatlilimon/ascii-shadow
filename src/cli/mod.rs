//! CLI argument parsing using clap

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// ASCII Shadow - Convert images to ASCII art with color support
#[derive(Parser, Debug)]
#[command(name = "ascii-shadow")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input image path
    #[arg(short, long, value_name = "FILE")]
    pub input: PathBuf,

    /// Output file (optional, prints to stdout if not specified)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Charset to use: standard, extended, alphanumeric, numbers, blocks, braille
    #[arg(short, long, default_value = "standard", value_name = "NAME")]
    pub charset: String,

    /// Custom charset string
    #[arg(long, value_name = "CHARS")]
    pub custom_charset: Option<String>,

    /// Character width (auto-detected from terminal if not specified)
    #[arg(short, long, value_name = "N")]
    pub width: Option<usize>,

    /// Character height (auto-calculated based on aspect ratio if not specified)
    #[arg(short, long, value_name = "N")]
    pub height: Option<usize>,

    /// Enable color output
    #[arg(short, long, default_value_t = true, action = clap::ArgAction::SetTrue)]
    pub color: bool,

    /// Color mode: truecolor, 256, grayscale, none
    #[arg(long, default_value = "truecolor", value_name = "MODE")]
    pub color_mode: String,

    /// Contrast adjustment (-100 to 100)
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub contrast: i32,

    /// Brightness adjustment (-100 to 100)
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub brightness: i32,

    /// Invert colors
    #[arg(long, default_value_t = false)]
    pub invert: bool,

    /// Background color in hex format (e.g., 1a1a2e) or "none"
    #[arg(long, value_name = "HEX")]
    pub background: Option<String>,

    /// Resize mode: fit, fill, stretch, crop
    #[arg(long, default_value = "fit", value_name = "MODE")]
    pub resize: String,

    /// Preserve aspect ratio
    #[arg(long, default_value_t = true)]
    pub preserve_aspect_ratio: bool,
}

impl Args {
    /// Print comprehensive help information
    pub fn print_help() {
        println!("Convert images to ASCII art with color support");
        println!();
        println!("USAGE:");
        println!("    ascii-shadow [OPTIONS] --input <FILE>");
        println!();
        println!("OPTIONS:");
        println!("    -i, --input <FILE>         Input image path (required)");
        println!("    -o, --output <FILE>        Output file (prints to stdout if not specified)");
        println!("    -c, --charset <NAME>       Charset: standard, extended, alphanumeric, numbers, blocks, braille");
        println!("        --custom-charset <CHARS>   Custom charset string");
        println!("    -w, --width <N>            Character width (auto-detected from terminal)");
        println!("    -h, --height <N>           Character height (auto-calculated)");
        println!("    --color-mode <MODE>       Color mode: truecolor, 256, grayscale, none");
        println!("    --contrast <N>             Contrast adjustment (-100 to 100)");
        println!("    --brightness <N>           Brightness adjustment (-100 to 100)");
        println!("    --invert                    Invert colors");
        println!("    --background <HEX>         Background color in hex format");
        println!("    --resize <MODE>             Resize mode: fit, fill, stretch, crop");
        println!("    --preserve-aspect-ratio    Preserve aspect ratio");
        println!("    --help                      Show this help message (long flag only)");
        println!();
        println!("CHARSETS:");
        println!("    standard      - Basic ASCII characters (.:-=+*#@)");
        println!("    extended      - Extended ASCII with more detail");
        println!("    alphanumeric  - Full alphanumeric characters");
        println!("    numbers       - Numbers and punctuation only");
        println!("    blocks        - Unicode block characters (░▒▓█)");
        println!("    braille       - Braille patterns for high detail");
        println!();
        println!("COLOR MODES:");
        println!("    truecolor     - 24-bit RGB (recommended)");
        println!("    256           - 256-color palette (wider compatibility)");
        println!("    grayscale     - Grayscale only");
        println!("    none          - No color");
        println!();
        println!("RESIZE MODES:");
        println!("    fit           - Fit within dimensions while preserving ratio");
        println!("    fill          - Fill dimensions, potentially cropping");
        println!("    stretch       - Stretch to exact dimensions");
        println!("    crop          - Crop to center of image");
        println!();
        println!("EXAMPLES:");
        println!("    ascii-shadow --input image.png");
        println!("    ascii-shadow --input photo.jpg --charset braille");
        println!("    ascii-shadow --input logo.png --width 80 --height 40");
        println!("    ascii-shadow --input diagram.png --output art.txt");
    }

    /// Parse and validate CLI arguments
    pub fn parse_args() -> Result<Self> {
        // Check for --help flag (long only, -h is for height)
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|arg| arg == "--help") {
            Self::print_help();
            std::process::exit(0);
        }

        // Try to parse arguments
        let args = match Self::try_parse() {
            Ok(parsed_args) => parsed_args,
            Err(e) => {
                // Show help on parsing error
                eprintln!("\nError: {}", e);
                Self::print_help();
                std::process::exit(1);
            }
        };

        // Validate input file exists
        if !args.input.exists() {
            anyhow::bail!("Input file not found: {}", args.input.display());
        }

        // Validate contrast range
        if args.contrast < -100 || args.contrast > 100 {
            anyhow::bail!("Contrast must be between -100 and 100");
        }

        // Validate brightness range
        if args.brightness < -100 || args.brightness > 100 {
            anyhow::bail!("Brightness must be between -100 and 100");
        }

        Ok(args)
    }

    /// Parse background color from hex string
    pub fn parse_background(&self) -> Result<Option<(u8, u8, u8)>, String> {
        match &self.background {
            None => Ok(None),
            Some(bg) if bg.eq_ignore_ascii_case("none") => Ok(None),
            Some(hex) => {
                let hex = hex.trim_start_matches('#');
                if hex.len() != 6 {
                    return Err("Background color must be 6 hex digits".to_string());
                }
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|e| format!("Invalid hex color: {}", e))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|e| format!("Invalid hex color: {}", e))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|e| format!("Invalid hex color: {}", e))?;
                Ok(Some((r, g, b)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_background_none() {
        let args = Args {
            background: None,
            ..Default::default()
        };
        assert!(args.parse_background().unwrap().is_none());
    }

    #[test]
    fn test_parse_background_hex() {
        let args = Args {
            background: Some("ff0000".to_string()),
            ..Default::default()
        };
        assert_eq!(args.parse_background().unwrap(), Some((255, 0, 0)));
    }

    #[test]
    fn test_parse_background_with_hash() {
        let args = Args {
            background: Some("#1a2b3c".to_string()),
            ..Default::default()
        };
        assert_eq!(args.parse_background().unwrap(), Some((26, 43, 60)));
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            input: PathBuf::from("test.png"),
            output: None,
            charset: "standard".to_string(),
            custom_charset: None,
            width: None,
            height: None,
            color: true,
            color_mode: "truecolor".to_string(),
            contrast: 0,
            brightness: 0,
            invert: false,
            background: None,
            resize: "fit".to_string(),
            preserve_aspect_ratio: true,
        }
    }
}

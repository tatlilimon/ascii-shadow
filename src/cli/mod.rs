//! CLI argument parsing using clap

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Largest accepted character width or height.
pub const MAX_DIMENSION: usize = 10_000;

/// Input source for the image conversion
#[derive(Debug, Clone, PartialEq)]
pub enum InputSource {
    /// Local file path
    File(PathBuf),
    /// Remote URL (http:// or https://)
    Url(String),
}

impl InputSource {
    /// Create an InputSource from a string
    /// Detects URLs starting with http:// or https://
    pub fn from_str(s: String) -> Self {
        let lower = s.to_ascii_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") {
            InputSource::Url(s)
        } else {
            InputSource::File(PathBuf::from(s))
        }
    }

    /// Check if this is a file source
    pub fn is_file(&self) -> bool {
        matches!(self, InputSource::File(_))
    }

    /// Check if this is a URL source
    pub fn is_url(&self) -> bool {
        matches!(self, InputSource::Url(_))
    }

    /// Get display string for error messages
    pub fn display(&self) -> String {
        match self {
            InputSource::File(p) => p.display().to_string(),
            InputSource::Url(u) => u.clone(),
        }
    }
}

fn parse_dimension(value: &str) -> Result<usize, String> {
    let n: usize = value
        .parse()
        .map_err(|_| format!("'{value}' is not a whole number"))?;
    if n == 0 || n > MAX_DIMENSION {
        return Err(format!("must be between 1 and {MAX_DIMENSION}"));
    }
    Ok(n)
}

/// ASCII Shadow - Convert images to ASCII art with color support
///
/// `-h` is the height option, so help is only available as `--help`.
#[derive(Parser, Debug)]
#[command(name = "ascii-shadow")]
#[command(author, version, about, long_about = None, disable_help_flag = true)]
pub struct Args {
    /// Input image path or URL
    #[arg(short, long, value_name = "FILE|URL")]
    pub input: String,

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
    #[arg(short, long, value_name = "N", value_parser = parse_dimension)]
    pub width: Option<usize>,

    /// Character height (auto-calculated based on aspect ratio if not specified)
    #[arg(short, long, value_name = "N", value_parser = parse_dimension)]
    pub height: Option<usize>,

    /// Color output; `--color false` is the same as `--color-mode none`
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set, num_args = 0..=1,
          default_missing_value = "true", value_name = "BOOL")]
    pub color: bool,

    /// Color mode: truecolor, 256, grayscale, none
    #[arg(long, default_value = "truecolor", value_name = "MODE")]
    pub color_mode: String,

    /// Contrast adjustment (-100 to 100)
    #[arg(long, default_value_t = 0, value_name = "N", allow_negative_numbers = true,
          value_parser = clap::value_parser!(i32).range(-100..=100))]
    pub contrast: i32,

    /// Brightness adjustment (-100 to 100)
    #[arg(long, default_value_t = 0, value_name = "N", allow_negative_numbers = true,
          value_parser = clap::value_parser!(i32).range(-100..=100))]
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

    /// Preserve aspect ratio; `--preserve-aspect-ratio false` stretches like `--resize stretch`
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set, num_args = 0..=1,
          default_missing_value = "true", value_name = "BOOL")]
    pub preserve_aspect_ratio: bool,

    /// Frames per second override for GIF playback (1 to 1000)
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub fps: Option<u32>,

    /// Number of animation loops, 0 = infinite (the GIF's own setting if not specified)
    #[arg(long, value_name = "N")]
    pub loops: Option<u32>,

    /// Don't clear screen between frames
    #[arg(long, default_value_t = false)]
    pub no_clear: bool,
}

impl Args {
    /// Print comprehensive help information
    pub fn print_help() {
        println!("Convert images to ASCII art with color support");
        println!();
        println!("USAGE:");
        println!("    ascii-shadow [OPTIONS] --input <FILE|URL>");
        println!();
        println!("OPTIONS:");
        println!("    -i, --input <FILE|URL>          Input image path or URL (required)");
        println!(
            "    -o, --output <FILE>             Output file (prints to stdout if not specified)"
        );
        println!(
            "    -c, --charset <NAME>            Charset: standard, extended, alphanumeric, numbers, blocks, braille"
        );
        println!("        --custom-charset <CHARS>    Custom charset string, darkest to brightest");
        println!(
            "    -w, --width <N>                 Character width (auto-detected from terminal)"
        );
        println!("    -h, --height <N>                Character height (auto-calculated)");
        println!("        --color [true|false]        Color output (default: true)");
        println!("        --color-mode <MODE>         Color mode: truecolor, 256, grayscale, none");
        println!("        --contrast <N>              Contrast adjustment (-100 to 100)");
        println!("        --brightness <N>            Brightness adjustment (-100 to 100)");
        println!("        --invert                    Invert brightness");
        println!(
            "        --background <HEX>          Background color for transparent pixels, or none"
        );
        println!("        --resize <MODE>             Resize mode: fit, fill, stretch, crop");
        println!("        --preserve-aspect-ratio [true|false]");
        println!("                                    Preserve aspect ratio (default: true)");
        println!(
            "        --fps <N>                   Frames per second for GIF playback (1 to 1000)"
        );
        println!(
            "        --loops <N>                 Number of animation loops, 0 = infinite (default: the GIF's)"
        );
        println!("        --no-clear                  Don't clear screen between frames");
        println!("        --help                      Show this help message (long flag only)");
        println!("    -V, --version                   Show the version");
        println!();
        println!("CHARSETS:");
        println!("    standard      - Basic ASCII characters (.:-=+*#%@)");
        println!("    extended      - Extended ASCII with more detail");
        println!("    alphanumeric  - Letters and digits only");
        println!("    numbers       - Digits with light punctuation");
        println!("    blocks        - Unicode block characters (░▒▓█)");
        println!("    braille       - Braille patterns, 2x4 dots per character for high detail");
        println!();
        println!("COLOR MODES:");
        println!("    truecolor     - 24-bit RGB (recommended)");
        println!("    256           - 256-color palette (wider compatibility)");
        println!("    grayscale     - Grayscale only");
        println!("    none          - No color");
        println!();
        println!("RESIZE MODES:");
        println!("    fit           - The whole image, inside the size, aspect ratio preserved");
        println!("    fill          - Covers the size; overflow is cropped at the center");
        println!("    stretch       - Exactly the size, aspect ratio ignored");
        println!("    crop          - The image center at full resolution, cropped to the size");
        println!();
        println!("EXAMPLES:");
        println!("    ascii-shadow --input image.png");
        println!("    ascii-shadow --input photo.jpg --charset braille");
        println!("    ascii-shadow --input logo.png --width 80 --height 40");
        println!("    ascii-shadow --input diagram.png --output art.txt");
        println!("    ascii-shadow --input https://example.com/image.png");
    }

    /// Parse and validate CLI arguments
    pub fn parse_args() -> Result<Self> {
        // `-h` is the height option, so help is only available as `--help`.
        let raw: Vec<String> = std::env::args().collect();
        if raw.iter().skip(1).any(|arg| arg == "--help") {
            Self::print_help();
            std::process::exit(0);
        }

        let args = match Self::try_parse() {
            Ok(parsed_args) => parsed_args,
            Err(e) if e.kind() == clap::error::ErrorKind::DisplayVersion => e.exit(),
            Err(e) => {
                eprintln!("\nError: {}", e);
                Self::print_help();
                std::process::exit(2);
            }
        };

        // Validate input file exists (only for local files, not URLs)
        let input_source = InputSource::from_str(args.input.clone());
        if let InputSource::File(path) = &input_source {
            if !path.exists() {
                anyhow::bail!("Input file not found: {}", path.display());
            }
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
                if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
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
    use clap::CommandFactory;

    fn parse(extra: &[&str]) -> Result<Args, clap::Error> {
        let mut argv = vec!["ascii-shadow", "--input", "x.png"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv)
    }

    #[test]
    fn command_definition_is_consistent() {
        // clap's own consistency check; it panicked on the duplicate `-c` before.
        Args::command().debug_assert();
    }

    #[test]
    fn short_flags_reach_the_documented_options() {
        let a = parse(&["-c", "blocks", "-w", "40", "-h", "12", "-o", "out.txt"]).unwrap();
        assert_eq!(
            (a.charset.as_str(), a.width, a.height),
            ("blocks", Some(40), Some(12))
        );
        assert_eq!(a.output, Some(PathBuf::from("out.txt")));
    }

    #[test]
    fn boolean_options_can_be_turned_off_and_bare_flags_still_work() {
        let a = parse(&["--color", "false", "--preserve-aspect-ratio", "false"]).unwrap();
        assert!(!a.color && !a.preserve_aspect_ratio);
        let a = parse(&["--color", "--preserve-aspect-ratio"]).unwrap();
        assert!(a.color && a.preserve_aspect_ratio);
        let a = parse(&[]).unwrap();
        assert!(a.color && a.preserve_aspect_ratio);
    }

    #[test]
    fn invalid_numbers_are_rejected_before_any_work() {
        for bad in [
            &["--fps", "0"][..],
            &["--fps", "1001"],
            &["--width", "0"],
            &["--height", "0"],
            &["--width", "10001"],
            &["--contrast", "101"],
            &["--brightness", "-101"],
            &["--loops", "-1"],
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should be rejected");
        }
        let a = parse(&["--contrast", "-100", "--brightness", "100", "--fps", "1000"]).unwrap();
        assert_eq!((a.contrast, a.brightness, a.fps), (-100, 100, Some(1000)));
    }

    #[test]
    fn loops_default_to_the_gif_setting() {
        assert_eq!(parse(&[]).unwrap().loops, None);
        assert_eq!(parse(&["--loops", "0"]).unwrap().loops, Some(0));
    }

    #[test]
    fn urls_are_detected_case_insensitively() {
        assert!(InputSource::from_str("HTTPS://example.com/a.png".into()).is_url());
        assert!(InputSource::from_str("http-cat.png".into()).is_file());
    }

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

    #[test]
    fn non_ascii_background_is_an_error_not_a_panic() {
        // Six bytes, but byte offset 2 falls inside "é"; slicing there panicked before.
        let args = Args {
            background: Some("aébcd".to_string()),
            ..Default::default()
        };
        assert!(args.parse_background().is_err());
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            input: "test.png".to_string(),
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
            fps: None,
            loops: None,
            no_clear: false,
        }
    }
}

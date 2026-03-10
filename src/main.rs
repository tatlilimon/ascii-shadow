#![allow(dead_code)]

mod charset;
mod cli;
mod color;
mod converter;
mod image;
mod output;
mod terminal;

use anyhow::{Context, Result};
use cli::{Args, InputSource};
use color::ColorMode;
use converter::{convert, ConverterConfig};
use std::str::FromStr;

fn main() -> Result<()> {
    let args = Args::parse_args()?;

    // Parse the input source
    let input_source = InputSource::from_str(args.input.clone());

    // Load the image
    let img = image::load_image(&input_source)
        .with_context(|| match &input_source {
            InputSource::File(p) => format!("Failed to load image: {}", p.display()),
            InputSource::Url(u) => format!("Failed to load image from URL: {}", u),
        })?;

    // Get terminal size for auto-sizing
    let (term_cols, term_rows) = terminal::get_terminal_size()
        .unwrap_or((80, 24));

    // Calculate dimensions
    let (width, height) = match (args.width, args.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let aspect = img.width() as f64 / img.height() as f64;
            let adjusted_aspect = aspect / terminal::CHAR_ASPECT_RATIO;
            (w, ((w as f64 / adjusted_aspect).ceil() as usize))
        }
        (None, Some(h)) => {
            let aspect = img.width() as f64 / img.height() as f64;
            let adjusted_aspect = aspect / terminal::CHAR_ASPECT_RATIO;
            (((h as f64 * adjusted_aspect).ceil() as usize), h)
        }
        (None, None) => image::calculate_dimensions(&img, term_cols, term_rows),
    };

    // Parse resize mode
    let resize_mode = image::ResizeMode::from_str(&args.resize)
        .map_err(|e| anyhow::anyhow!("Invalid resize mode: {}", e))?;

    // Resize image for terminal
    let resized = image::resize_for_terminal(
        &img,
        width,
        height,
        resize_mode,
        args.preserve_aspect_ratio,
    );

    // Parse charset
    let charset = if let Some(custom) = &args.custom_charset {
        charset::from_custom(custom)?
    } else {
        charset::from_str(&args.charset)?
    };

    // Parse color mode
    let color_mode = if !args.color {
        ColorMode::None
    } else {
        ColorMode::from_str(&args.color_mode)
            .map_err(|e| anyhow::anyhow!("Invalid color mode: {}", e))?
    };

    // Parse background color
    let background = args.parse_background()
        .map_err(|e| anyhow::anyhow!("Invalid background color: {}", e))?;

    // Create converter config
    let config = ConverterConfig {
        charset,
        color_mode,
        contrast: args.contrast,
        brightness: args.brightness,
        invert: args.invert,
        background,
    };

    // Convert image to ASCII
    let chars = convert(&config, &resized);

    // Format output with colors
    let output = output::format_colored(&chars, color_mode);

    // Print or write to file
    if let Some(output_path) = args.output {
        std::fs::write(&output_path, output)
            .with_context(|| format!("Failed to write to: {}", output_path.display()))?;
        eprintln!("Output written to: {}", output_path.display());
    } else {
        print!("{}", output);
    }

    Ok(())
}

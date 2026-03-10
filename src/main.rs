#![allow(dead_code)]

mod charset;
mod cli;
mod color;
mod converter;
mod gif;
mod image;
mod output;
mod terminal;

use anyhow::{Context, Result};
use cli::{Args, InputSource};
use color::ColorMode;
use converter::{convert, ConverterConfig};
use gif::{AnimationConfig, AsciiFrame};
use std::io::{self, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

/// Flag to signal graceful shutdown on SIGINT
static SHOULD_QUIT: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Get the quit flag reference
fn quit_flag() -> &'static Arc<AtomicBool> {
    SHOULD_QUIT.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

fn main() -> Result<()> {
    let args = Args::parse_args()?;

    // Setup SIGINT handler for clean exit
    setup_sigint_handler()?;

    // Parse input source
    let input_source = InputSource::from_str(args.input.clone());

    // Check if input is a GIF
    let is_gif = gif::is_gif(&input_source);

    // Auto-animate GIFs when outputting to terminal
    if is_gif && args.output.is_none() {
        return run_gif_animation(&input_source, &args);
    }

    // Otherwise, proceed with existing single-image logic
    run_single_image(&input_source, &args)
}

/// Run single image conversion (non-GIF or GIF with --output)
fn run_single_image(input_source: &InputSource, args: &Args) -> Result<()> {
    // Load image
    let img = image::load_image(input_source)
        .with_context(|| match input_source {
            InputSource::File(p) => format!("Failed to load image: {}", p.display()),
            InputSource::Url(u) => format!("Failed to load image from URL: {}", u),
        })?;

    // Get terminal size for auto-sizing
    let (term_cols, term_rows) = terminal::get_terminal_size().unwrap_or((80, 24));

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
    if let Some(output_path) = &args.output {
        std::fs::write(output_path, output)
            .with_context(|| format!("Failed to write to: {}", output_path.display()))?;
        eprintln!("Output written to: {}", output_path.display());
    } else {
        print!("{}", output);
    }

    Ok(())
}

/// Run GIF animation in terminal
fn run_gif_animation(input_source: &InputSource, args: &Args) -> Result<()> {
    // Get terminal size for auto-sizing
    let (term_cols, term_rows) = terminal::get_terminal_size().unwrap_or((80, 24));

    // Calculate dimensions
    let (width, height) = match (args.width, args.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let adjusted_aspect = 1.0 / terminal::CHAR_ASPECT_RATIO;
            (w, ((w as f64 / adjusted_aspect).ceil() as usize))
        }
        (None, Some(h)) => {
            let adjusted_aspect = 1.0 / terminal::CHAR_ASPECT_RATIO;
            (((h as f64 * adjusted_aspect).ceil() as usize), h)
        }
        (None, None) => {
            // Use most of terminal for animation
            let w = term_cols.saturating_sub(2);
            let h = term_rows.saturating_sub(2);
            (w, h)
        }
    };

    // Parse resize mode
    let resize_mode = image::ResizeMode::from_str(&args.resize)
        .map_err(|e| anyhow::anyhow!("Invalid resize mode: {}", e))?;

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

    // Load and decode GIF frames
    let decoded = gif::load_gif(input_source, width, height, resize_mode, &config, color_mode)
        .context("Failed to load GIF")?;

    // Check for single-frame GIF
    if decoded.frames.len() == 1 {
        eprintln!("Note: GIF has only one frame, displaying as static image");
        print!("{}", decoded.frames[0].output);
        return Ok(());
    }

    // Setup terminal
    terminal::hide_cursor()?;
    let _guard = CursorGuard;

    // Create animation config
    let animation_config = AnimationConfig {
        loops: args.loops,
        fps: args.fps,
        clear_screen: !args.no_clear,
    };

    // Play animation
    play_animation(&decoded.frames, &animation_config)?;

    // Terminal cleanup is handled by CursorGuard
    Ok(())
}

/// Play animation with proper timing and loop control
fn play_animation(frames: &[AsciiFrame], config: &AnimationConfig) -> Result<()> {
    let mut current_loop = 0u32;

    loop {
        // Check for quit signal
        if should_quit() {
            break;
        }

        for frame in frames {
            // Check for quit signal before each frame
            if should_quit() {
                break;
            }

            // Clear screen or move cursor home
            if config.clear_screen {
                terminal::clear_screen()?;
            } else {
                terminal::move_cursor_home()?;
            }

            // Print frame
            print!("{}", frame.output);
            io::stdout().flush()?;

            // Calculate delay
            let delay = if let Some(fps) = config.fps {
                1000 / fps as u64
            } else {
                frame.delay_ms
            };

            // Sleep for delay
            std::thread::sleep(Duration::from_millis(delay));
        }

        // Loop control
        if config.loops > 0 && current_loop >= config.loops - 1 {
            break;
        }
        current_loop += 1;
    }

    Ok(())
}

/// Setup SIGINT handler for clean exit
fn setup_sigint_handler() -> Result<()> {
    let should_quit = Arc::clone(quit_flag());
    ctrlc::set_handler(move || {
        should_quit.store(true, Ordering::SeqCst);
    })
    .map_err(|e| anyhow::anyhow!("Failed to set Ctrl-C handler: {}", e))
}

/// Check if we should quit (SIGINT received)
fn should_quit() -> bool {
    quit_flag().load(Ordering::SeqCst)
}

/// Guard to restore cursor state when dropped
struct CursorGuard;

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = terminal::show_cursor();
        println!(); // New line after animation
    }
}

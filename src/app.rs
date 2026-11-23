//! The command: read the input, size it for the terminal, render, print or save.

use crate::charset;
use crate::cli::{Args, InputSource, MAX_DIMENSION};
use crate::color::ColorMode;
use crate::converter::{ConverterConfig, RenderPlan};
use crate::gif::{self, AnimationConfig, AsciiFrame};
use crate::image::{self as picture, ResizeMode};
use crate::terminal::{self, CHAR_ASPECT_RATIO};
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Most characters one picture may have (for example 2000 × 2000).
pub const MAX_CELLS: usize = 4_000_000;
/// Size used when the terminal size cannot be read (for example in a pipe).
pub const FALLBACK_TERMINAL: (usize, usize) = (80, 24);

/// Validate every option that does not need the image, so mistakes fail before
/// a download or decode.
pub fn plan_from_args(args: &Args) -> Result<RenderPlan> {
    let charset = match &args.custom_charset {
        Some(custom) => charset::from_custom(custom)?,
        None => charset::from_str(&args.charset)?,
    };
    let color_mode = if args.color {
        ColorMode::from_str(&args.color_mode)
            .map_err(|e| anyhow::anyhow!("Invalid color mode: {}", e))?
    } else {
        ColorMode::None
    };
    let background = args
        .parse_background()
        .map_err(|e| anyhow::anyhow!("Invalid background color: {}", e))?;
    let resize_mode = ResizeMode::from_str(&args.resize)
        .map_err(|e| anyhow::anyhow!("Invalid resize mode: {}", e))?;
    Ok(RenderPlan {
        config: ConverterConfig {
            charset,
            color_mode,
            contrast: args.contrast,
            brightness: args.brightness,
            invert: args.invert,
            background,
        },
        resize_mode,
        preserve_aspect_ratio: args.preserve_aspect_ratio,
    })
}

/// The terminal size, or [`FALLBACK_TERMINAL`] when it is unknown or empty.
pub fn terminal_size() -> (usize, usize) {
    terminal::get_terminal_size()
        .ok()
        .filter(|&(cols, rows)| cols > 0 && rows > 0)
        .unwrap_or(FALLBACK_TERMINAL)
}

/// Output size in characters for a picture of `img_w` × `img_h` pixels.
///
/// A missing width or height follows the picture's proportions (characters are
/// about twice as tall as wide); with neither, the picture fits the terminal.
/// Sizes derived from the picture are scaled down to [`MAX_CELLS`]; an explicit
/// size above it is an error.
pub fn target_size(
    width: Option<usize>,
    height: Option<usize>,
    img_w: u32,
    img_h: u32,
    (term_cols, term_rows): (usize, usize),
) -> Result<(usize, usize)> {
    let cells_per_row = (img_w.max(1) as f64 / img_h.max(1) as f64) / CHAR_ASPECT_RATIO;
    let clamp = |v: f64| (v.ceil() as usize).clamp(1, MAX_DIMENSION);
    let (cols, rows) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, clamp(w as f64 / cells_per_row)),
        (None, Some(h)) => (clamp(h as f64 * cells_per_row), h),
        (None, None) => picture::calculate_dimensions_for(img_w, img_h, term_cols, term_rows),
    };
    if cols * rows <= MAX_CELLS {
        return Ok((cols, rows));
    }
    if width.is_some() && height.is_some() {
        anyhow::bail!("{cols}x{rows} is too large; at most {MAX_CELLS} characters are supported");
    }
    let scale = (MAX_CELLS as f64 / (cols * rows) as f64).sqrt();
    let shrink = |v: usize| ((v as f64 * scale).floor() as usize).max(1);
    Ok(match (width, height) {
        (Some(w), None) => (w, (MAX_CELLS / w).max(1)),
        (None, Some(h)) => ((MAX_CELLS / h).max(1), h),
        _ => (shrink(cols), shrink(rows)),
    })
}

/// Refuse to write the output over the input image.
pub fn check_output_path(source: &InputSource, output: &Path) -> Result<()> {
    let InputSource::File(input) = source else {
        return Ok(());
    };
    if !output.exists() {
        return Ok(());
    }
    let same = match (std::fs::canonicalize(input), std::fs::canonicalize(output)) {
        (Ok(a), Ok(b)) if a == b => true,
        _ => same_file(input, output),
    };
    if same {
        anyhow::bail!(
            "Refusing to overwrite the input image {}; choose another --output",
            input.display()
        );
    }
    Ok(())
}

#[cfg(unix)]
fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(_: &Path, _: &Path) -> bool {
    false
}

/// Run the command for parsed arguments.
pub fn run(args: &Args) -> Result<()> {
    let source = InputSource::from_str(args.input.clone());
    let plan = plan_from_args(args)?;
    if let Some(output) = &args.output {
        check_output_path(&source, output)?;
    }
    let bytes = picture::read_source(&source).with_context(|| match &source {
        InputSource::File(p) => format!("Failed to load image: {}", p.display()),
        InputSource::Url(u) => format!("Failed to load image from URL: {}", u),
    })?;
    let term = terminal_size();

    // Animate GIFs in the terminal; with --output, save the first frame.
    if picture::is_gif_bytes(&bytes) && args.output.is_none() {
        return play_gif(&bytes, args, &plan, term);
    }

    let img = picture::decode_image(&bytes)
        .with_context(|| format!("Failed to load image: {}", source.display()))?;
    let (cols, rows) = target_size(args.width, args.height, img.width(), img.height(), term)?;
    let output = plan.render(&img, cols, rows);

    match &args.output {
        Some(path) => {
            std::fs::write(path, output)
                .with_context(|| format!("Failed to write to: {}", path.display()))?;
            eprintln!("Output written to: {}", path.display());
        }
        None => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(output.as_bytes())?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn play_gif(bytes: &[u8], args: &Args, plan: &RenderPlan, term: (usize, usize)) -> Result<()> {
    let info = gif::info(bytes)?;
    let (cols, rows) = target_size(args.width, args.height, info.width, info.height, term)?;
    let frames = gif::render_frames(bytes, |frame| plan.render(frame, cols, rows))
        .context("Failed to load GIF")?;

    if frames.len() == 1 {
        eprintln!("Note: GIF has only one frame, displaying as static image");
        print!("{}", frames[0].output);
        return Ok(());
    }

    // Ctrl-C ends playback cleanly (cursor restored) instead of killing the process.
    install_sigint_handler()?;
    terminal::hide_cursor()?;
    let _guard = CursorGuard;
    let config = AnimationConfig {
        loops: args.loops.unwrap_or(info.plays),
        fps: args.fps,
        clear_screen: !args.no_clear,
    };
    play_animation(&frames, &config)
}

/// Play animation with proper timing and loop control
pub fn play_animation(frames: &[AsciiFrame], config: &AnimationConfig) -> Result<()> {
    let mut played = 0u32;
    while !should_quit() {
        for frame in frames {
            if should_quit() {
                break;
            }
            if config.clear_screen {
                terminal::clear_screen()?;
            } else {
                terminal::move_cursor_home()?;
            }
            print!("{}", frame.output);
            io::stdout().flush()?;
            std::thread::sleep(Duration::from_millis(gif::frame_delay_ms(frame, config)));
        }
        played += 1;
        if config.loops > 0 && played >= config.loops {
            break;
        }
    }
    Ok(())
}

static SHOULD_QUIT: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn quit_flag() -> &'static Arc<AtomicBool> {
    SHOULD_QUIT.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

fn install_sigint_handler() -> Result<()> {
    let should_quit = Arc::clone(quit_flag());
    ctrlc::set_handler(move || should_quit.store(true, Ordering::SeqCst))
        .map_err(|e| anyhow::anyhow!("Failed to set Ctrl-C handler: {}", e))
}

fn should_quit() -> bool {
    quit_flag().load(Ordering::SeqCst)
}

/// Guard to restore cursor state when dropped
struct CursorGuard;

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = terminal::show_cursor();
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TERM: (usize, usize) = (82, 26);

    #[test]
    fn a_single_given_side_follows_the_picture_proportions() {
        // Square picture: twice as many columns as rows.
        assert_eq!(
            target_size(Some(60), None, 100, 100, TERM).unwrap(),
            (60, 30)
        );
        assert_eq!(
            target_size(None, Some(30), 100, 100, TERM).unwrap(),
            (60, 30)
        );
        // Wide 4:1 picture.
        assert_eq!(
            target_size(Some(80), None, 400, 100, TERM).unwrap(),
            (80, 10)
        );
        assert_eq!(
            target_size(Some(33), Some(7), 400, 100, TERM).unwrap(),
            (33, 7)
        );
    }

    #[test]
    fn auto_size_fits_the_terminal() {
        assert_eq!(target_size(None, None, 1000, 1000, TERM).unwrap(), (48, 24));
        assert_eq!(target_size(None, None, 2000, 500, TERM).unwrap(), (80, 10));
    }

    #[test]
    fn huge_sizes_are_scaled_down_or_refused() {
        // A 1x5000 strip at width 10000 would need 100 million rows.
        let (w, h) = target_size(Some(10_000), None, 1, 5000, TERM).unwrap();
        assert_eq!(w, 10_000);
        assert!(w * h <= MAX_CELLS);
        assert!(target_size(Some(10_000), Some(10_000), 1, 1, TERM).is_err());
        let (w, h) = target_size(Some(5000), Some(800), 1, 1, TERM).unwrap();
        assert_eq!((w, h), (5000, 800));
    }

    #[test]
    fn invalid_options_fail_before_loading_anything() {
        let bad = |f: fn(&mut Args)| {
            let mut args = Args::default();
            f(&mut args);
            plan_from_args(&args).is_err()
        };
        assert!(bad(|a| a.charset = "emoji".into()));
        assert!(bad(|a| a.custom_charset = Some(String::new())));
        assert!(bad(|a| a.color_mode = "cmyk".into()));
        assert!(bad(|a| a.background = Some("12345".into())));
        assert!(bad(|a| a.resize = "zoom".into()));
        assert!(!bad(|_| ()));
        // `--color false` overrides the color mode.
        let mut args = Args::default();
        args.color = false;
        assert_eq!(
            plan_from_args(&args).unwrap().config.color_mode,
            ColorMode::None
        );
    }

    #[test]
    fn the_output_may_replace_another_file_but_never_the_input() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.png");
        let other = dir.path().join("out.txt");
        std::fs::write(&input, b"png").unwrap();
        std::fs::write(&other, b"old").unwrap();
        let source = InputSource::File(input.clone());
        assert!(check_output_path(&source, &other).is_ok());
        assert!(check_output_path(&source, &dir.path().join("new.txt")).is_ok());
        assert!(check_output_path(&source, &input).is_err());
        // The same file through another path.
        let alias = dir.path().join(".").join("in.png");
        assert!(check_output_path(&source, &alias).is_err());
        #[cfg(unix)]
        {
            let link = dir.path().join("hard.png");
            std::fs::hard_link(&input, &link).unwrap();
            assert!(check_output_path(&source, &link).is_err());
        }
        assert!(check_output_path(&InputSource::Url("https://x/y.png".into()), &input).is_ok());
    }
}

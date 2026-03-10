//! GIF decoding and animation handling

use crate::cli::InputSource;
use crate::color::ColorMode;
use crate::converter::{convert, ConverterConfig};
use crate::image::{resize_for_terminal, ResizeMode};
use crate::output::format_colored;
use anyhow::{Context, Result};
use image::DynamicImage;
use std::io::Cursor;
use std::time::Duration;

/// Represents a single ASCII frame with its display duration
#[derive(Debug, Clone)]
pub struct AsciiFrame {
    /// The pre-formatted ASCII output string
    pub output: String,
    /// Frame delay in milliseconds (from GIF, can be overridden by --fps)
    pub delay_ms: u64,
}

/// Configuration for GIF animation playback
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    /// Number of times to loop (0 = infinite)
    pub loops: u32,
    /// Override FPS (None = use GIF native delays)
    pub fps: Option<u32>,
    /// Whether to clear screen between frames
    pub clear_screen: bool,
}

/// Result of GIF decoding with pre-converted ASCII frames
pub struct DecodedGif {
    /// All frames as pre-converted ASCII
    pub frames: Vec<AsciiFrame>,
    /// Native loop count from GIF (0 = infinite)
    pub native_loops: gif::Repeat,
    /// Canvas dimensions
    pub width: u16,
    pub height: u16,
}

/// Detect if an input source is a GIF
pub fn is_gif(source: &InputSource) -> bool {
    match source {
        InputSource::File(path) => {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("gif"))
                .unwrap_or(false)
        }
        InputSource::Url(url) => {
            url.to_lowercase().ends_with(".gif")
        }
    }
}

/// Load and decode a GIF from input source
pub fn load_gif(
    source: &InputSource,
    target_width: usize,
    target_height: usize,
    resize_mode: ResizeMode,
    config: &ConverterConfig,
    color_mode: ColorMode,
) -> Result<DecodedGif> {
    // Read bytes from source
    let bytes = match source {
        InputSource::File(path) => {
            std::fs::read(path).with_context(|| format!("Failed to read GIF file: {}", path.display()))?
        }
        InputSource::Url(url) => {
            load_gif_from_url(url)?
        }
    };

    // Decode GIF frames
    let frames = decode_gif_frames(&bytes, target_width, target_height, resize_mode, config, color_mode)?;

    Ok(frames)
}

/// Load GIF bytes from URL
fn load_gif_from_url(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    let response = client.get(url).send()
        .map_err(|e| anyhow::anyhow!("Failed to fetch URL: {}", e))?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    response.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))
}

/// Decode all frames from a GIF and convert to ASCII
fn decode_gif_frames(
    bytes: &[u8],
    target_width: usize,
    target_height: usize,
    resize_mode: ResizeMode,
    config: &ConverterConfig,
    color_mode: ColorMode,
) -> Result<DecodedGif> {

    // Create decoder with RGBA color output
    let mut decoder_options = gif::DecodeOptions::new();
    decoder_options.set_color_output(gif::ColorOutput::RGBA);

    let mut decoder = decoder_options.read_info(Cursor::new(bytes))
        .context("Failed to decode GIF. Is it a valid GIF file?")?;

    let width = decoder.width();
    let height = decoder.height();
    let native_loops = decoder.repeat();

    // Check if GIF has at least one frame
    let mut frames = Vec::new();

    // Buffer to store decoded RGBA pixel data
    let mut screen_buf = vec
![0u8; (width as usize * height as usize) * 4];

    loop {
        // Read next frame
        let frame = match decoder.read_next_frame() {
            Ok(Some(frame)) => frame,
            Ok(None) => break, // End of GIF
            Err(e) => anyhow::bail!("Failed to read GIF frame: {}", e),
        };

        // Get delay in centiseconds (1/100 second)
        let delay_cs = frame.delay;
        let delay_ms: u64 = if delay_cs == 0 {
            100 // Default to 100ms if delay is 0
        } else {
            (delay_cs * 10) as u64
        };

        // Get frame dimensions and position (partial update handling)
        let frame_width = frame.width as usize;
        let frame_height = frame.height as usize;
        let frame_top = frame.top as usize;
        let frame_left = frame.left as usize;

        // Get the decoded buffer for this frame
        let frame_buffer = &frame.buffer;

        // Copy frame buffer to the correct position on screen_buf
        // This handles partial updates (many GIFs only update changed areas)
        let buf_stride = width as usize * 4;
        let frame_stride = frame_width * 4;

        for y in 0..frame_height {
            let src_start = y * frame_stride;
            let src_end = src_start + frame_stride;
            let screen_row_start = (frame_top + y) * buf_stride + frame_left * 4;
            let screen_row_end = screen_row_start + frame_stride;

            if screen_row_end <= screen_buf.len() {
                screen_buf[screen_row_start..screen_row_end]
                    .copy_from_slice(&frame_buffer[src_start..src_end]);
            }
        }

        // Create RGBA image from buffer
        let rgba_image = image::RgbaImage::from_raw(
            width as u32,
            height as u32,
            screen_buf.clone(),
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create RGBA image from frame data"))?;

        let frame_img: DynamicImage = DynamicImage::ImageRgba8(rgba_image);

        // Resize frame to target dimensions
        let resized = resize_for_terminal(&frame_img, target_width, target_height, resize_mode, false);

        // Convert to ASCII
        let chars = convert(config, &resized);
        let output = format_colored(&chars, color_mode);

        frames.push(AsciiFrame { output, delay_ms });
    }

    if frames.is_empty() {
        anyhow::bail!("GIF contains no frames");
    }

    Ok(DecodedGif {
        frames,
        native_loops,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_gif_file() {
        assert!(is_gif(&InputSource::File(
            std::path::PathBuf::from("test.gif")
        )));
        assert!(is_gif(&InputSource::File(
            std::path::PathBuf::from("test.GIF")
        )));
        assert!(!is_gif(&InputSource::File(
            std::path::PathBuf::from("test.png")
        )));
        assert!(!is_gif(&InputSource::File(
            std::path::PathBuf::from("test.jpg")
        )));
    }

    #[test]
    fn test_is_gif_url() {
        assert!(is_gif(&InputSource::Url(
            "https://example.com/image.gif".to_string()
        )));
        assert!(is_gif(&InputSource::Url(
            "https://example.com/image.GIF".to_string()
        )));
        assert!(!is_gif(&InputSource::Url(
            "https://example.com/image.png".to_string()
        )));
        assert!(!is_gif(&InputSource::Url(
            "https://example.com/image.jpg".to_string()
        )));
    }
}

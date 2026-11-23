//! GIF decoding and animation handling

use crate::image::decode_limits;
use anyhow::{Context, Result};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage, ImageDecoder};
use std::io::Cursor;

/// Most frames an animation may have.
pub const MAX_FRAMES: usize = 10_000;
/// Most memory the pre-rendered frames of one animation may use.
pub const MAX_RENDERED_BYTES: usize = 256 * 1024 * 1024;
/// Delay used for frames that declare none, as browsers do.
pub const DEFAULT_DELAY_MS: u64 = 100;

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
    /// Number of times to play the animation (0 = infinite)
    pub loops: u32,
    /// Override FPS (None = use GIF native delays)
    pub fps: Option<u32>,
    /// Whether to clear screen between frames
    pub clear_screen: bool,
}

/// What the GIF header says before any frame is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GifInfo {
    /// Canvas width in pixels
    pub width: u32,
    /// Canvas height in pixels
    pub height: u32,
    /// How many times the GIF asks to be played; 0 = forever
    pub plays: u32,
}

/// Read the canvas size and loop setting of a GIF.
pub fn info(bytes: &[u8]) -> Result<GifInfo> {
    let decoder = gif::DecodeOptions::new()
        .read_info(Cursor::new(bytes))
        .context("Failed to decode GIF. Is it a valid GIF file?")?;
    // NETSCAPE2.0 loop count n repeats the animation n more times; 0 loops forever.
    let plays = match decoder.repeat() {
        gif::Repeat::Infinite => 0,
        gif::Repeat::Finite(n) => u32::from(n) + 1,
    };
    Ok(GifInfo {
        width: u32::from(decoder.width()).max(1),
        height: u32::from(decoder.height()).max(1),
        plays,
    })
}

/// Decode every frame, composite it onto the canvas (transparency and the
/// keep/background/previous disposal methods included), and render it with
/// `render`. Frames are rendered one at a time, so only the text is kept.
pub fn render_frames(
    bytes: &[u8],
    mut render: impl FnMut(&DynamicImage) -> String,
) -> Result<Vec<AsciiFrame>> {
    let mut decoder = GifDecoder::new(Cursor::new(bytes))
        .context("Failed to decode GIF. Is it a valid GIF file?")?;
    decoder
        .set_limits(decode_limits())
        .context("GIF canvas is too large")?;

    let mut frames = Vec::new();
    let mut rendered_bytes = 0usize;
    for frame in decoder.into_frames() {
        let frame = frame.map_err(|e| anyhow::anyhow!("Failed to read GIF frame: {}", e))?;
        if frames.len() == MAX_FRAMES {
            anyhow::bail!("GIF has more than {MAX_FRAMES} frames");
        }
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay_ms = match u64::from(numerator) / u64::from(denominator.max(1)) {
            0 => DEFAULT_DELAY_MS,
            ms => ms,
        };
        let output = render(&DynamicImage::ImageRgba8(frame.into_buffer()));
        rendered_bytes += output.len();
        if rendered_bytes > MAX_RENDERED_BYTES {
            anyhow::bail!(
                "The rendered animation would use more than 256 MiB; try a smaller --width"
            );
        }
        frames.push(AsciiFrame { output, delay_ms });
    }

    if frames.is_empty() {
        anyhow::bail!("GIF contains no frames");
    }
    Ok(frames)
}

/// How long to show a frame.
pub fn frame_delay_ms(frame: &AsciiFrame, config: &AnimationConfig) -> u64 {
    match config.fps {
        Some(fps) => 1000 / u64::from(fps.max(1)),
        None => frame.delay_ms,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use gif::{DisposalMethod, Encoder, Frame, Repeat};

    /// Build a GIF from (pixels, disposal, delay in centiseconds) frames on a
    /// `w`×`h` canvas. Palette: 0 = black, 1 = white, 2 = red; index 3 is transparent.
    pub(crate) fn make_gif(
        w: u16,
        h: u16,
        repeat: Option<Repeat>,
        frames: &[(Vec<u8>, u16, u16, u16, u16, DisposalMethod, u16)],
    ) -> Vec<u8> {
        let palette = [0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 255, 0];
        let mut out = Vec::new();
        {
            let mut encoder = Encoder::new(&mut out, w, h, &palette).unwrap();
            if let Some(repeat) = repeat {
                encoder.set_repeat(repeat).unwrap();
            }
            for (pixels, left, top, fw, fh, dispose, delay) in frames {
                let frame = Frame {
                    left: *left,
                    top: *top,
                    width: *fw,
                    height: *fh,
                    dispose: *dispose,
                    delay: *delay,
                    transparent: Some(3),
                    buffer: std::borrow::Cow::Borrowed(pixels),
                    ..Frame::default()
                };
                encoder.write_frame(&frame).unwrap();
            }
        }
        out
    }

    fn pixels(bytes: &[u8]) -> Vec<Vec<(u8, u8, u8, u8)>> {
        let mut seen = Vec::new();
        render_frames(bytes, |img| {
            let rgba = img.to_rgba8();
            seen.push(rgba.pixels().map(|p| (p[0], p[1], p[2], p[3])).collect());
            String::from("x")
        })
        .unwrap();
        seen
    }

    #[test]
    fn transparent_pixels_keep_what_is_underneath() {
        // Frame 1: white 2x1. Frame 2 paints only the left pixel red; the right is transparent.
        let gif = make_gif(
            2,
            1,
            None,
            &[
                (vec![1, 1], 0, 0, 2, 1, DisposalMethod::Keep, 5),
                (vec![2, 3], 0, 0, 2, 1, DisposalMethod::Keep, 5),
            ],
        );
        let frames = pixels(&gif);
        assert_eq!(frames[1][0], (255, 0, 0, 255));
        assert_eq!(
            frames[1][1],
            (255, 255, 255, 255),
            "transparent pixel must not erase the white below"
        );
    }

    #[test]
    fn partial_frames_are_placed_at_their_offset() {
        let gif = make_gif(
            3,
            2,
            None,
            &[
                (vec![0; 6], 0, 0, 3, 2, DisposalMethod::Keep, 5),
                (vec![1], 2, 1, 1, 1, DisposalMethod::Keep, 5),
            ],
        );
        let frames = pixels(&gif);
        let white: Vec<usize> = frames[1]
            .iter()
            .enumerate()
            .filter(|(_, p)| p.0 == 255)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(white, vec![5], "only the bottom-right pixel turns white");
    }

    #[test]
    fn background_disposal_clears_the_area_before_the_next_frame() {
        let gif = make_gif(
            2,
            1,
            None,
            &[
                (vec![1, 1], 0, 0, 2, 1, DisposalMethod::Background, 5),
                (vec![3, 2], 0, 0, 2, 1, DisposalMethod::Keep, 5),
            ],
        );
        let frames = pixels(&gif);
        assert_eq!(
            frames[1][0].3, 0,
            "the disposed white pixel is gone, not kept under the transparent one"
        );
        assert_eq!(frames[1][1], (255, 0, 0, 255));
    }

    #[test]
    fn delays_and_loop_settings_follow_the_file() {
        let frames = [
            (vec![1], 0, 0, 1, 1, DisposalMethod::Keep, 0),
            (vec![2], 0, 0, 1, 1, DisposalMethod::Keep, 7),
        ];
        let gif = make_gif(1, 1, Some(Repeat::Infinite), &frames);
        let rendered = render_frames(&gif, |_| String::new()).unwrap();
        assert_eq!(
            rendered.iter().map(|f| f.delay_ms).collect::<Vec<_>>(),
            vec![DEFAULT_DELAY_MS, 70]
        );
        assert_eq!(
            info(&gif).unwrap(),
            GifInfo {
                width: 1,
                height: 1,
                plays: 0
            }
        );
        assert_eq!(
            info(&make_gif(1, 1, Some(Repeat::Finite(2)), &frames))
                .unwrap()
                .plays,
            3
        );
        assert_eq!(info(&make_gif(1, 1, None, &frames)).unwrap().plays, 1);
    }

    #[test]
    fn fps_override_never_divides_by_zero() {
        let frame = AsciiFrame {
            output: String::new(),
            delay_ms: 40,
        };
        let config = |fps| AnimationConfig {
            loops: 1,
            fps,
            clear_screen: true,
        };
        assert_eq!(frame_delay_ms(&frame, &config(None)), 40);
        assert_eq!(frame_delay_ms(&frame, &config(Some(25))), 40);
        assert_eq!(frame_delay_ms(&frame, &config(Some(0))), 1000);
    }

    #[test]
    fn broken_truncated_and_oversized_gifs_are_errors_not_panics() {
        let gif = make_gif(
            2,
            2,
            None,
            &[(vec![1; 4], 0, 0, 2, 2, DisposalMethod::Keep, 5)],
        );
        assert!(render_frames(&gif, |_| String::new()).is_ok());
        assert!(render_frames(b"GIF89a", |_| String::new()).is_err());
        // A truncated file either fails or yields what was complete; it never panics.
        if let Ok(frames) = render_frames(&gif[..gif.len() - 6], |_| String::new()) {
            assert!(frames.len() <= 1);
        }
        // A 13-byte header that claims a 65535x65535 canvas (17 GB of pixels).
        let mut bomb = gif.clone();
        bomb[6..8].copy_from_slice(&u16::MAX.to_le_bytes());
        bomb[8..10].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(render_frames(&bomb, |_| String::new()).is_err());
        assert_eq!(info(&bomb).unwrap().width, 65535);
    }
}

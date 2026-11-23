# ASCII Shadow

Convert images to ASCII art blazing fast with full color support.

## Example

https://github.com/user-attachments/assets/4b9859d6-acf4-4bbe-991c-0ca8718618e0

## Features

- **Multiple charsets**: Standard ASCII, Extended, Alphanumeric (letters and digits), Numbers (digits), Block characters, and Braille patterns
- **High-detail Braille**: each character is a 2×4 grid of dots, so a line holds twice the columns and four times the rows
- **Color support**: Truecolor (24-bit), 256-color palette, Grayscale, or no color (plain text)
- **GIF animation**: Animated GIF playback with frame timing, transparency and disposal, loop control, and screen clear options
- **Auto-sizing**: Fits the terminal and keeps the picture's proportions (terminal characters are about twice as tall as wide)
- **Image adjustments**: Contrast, brightness, and inversion
- **Resize modes**: Fit, fill, stretch, or crop to target dimensions
- **Output options**: Print to terminal or save to file

## Installation

```bash
cargo install --git https://github.com/tatlilimon/ascii-shadow

echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  #if you're using zsh, replace .zshrc with .bashrc
```

Or build from source:
```bash
git clone https://github.com/tatlilimon/ascii-shadow
cd ascii-shadow
cargo install --path .
```

The binary will be at `target/release/ascii-shadow`.

## Usage

```bash
# Basic usage - auto-detect terminal size
ascii-shadow --input image.png

# URL as image input
ascii-shadow --input https://example.com/image.png


# GIF animation with custom settings gif can be also URL format
ascii-shadow --input animation.gif --fps 30 --loops 0 --no-clear 

# Specific dimensions
ascii-shadow --input image.png --width 80 --height 40

# Use different charset
ascii-shadow --input image.png --charset braille
ascii-shadow --input image.png --charset alphanumeric
ascii-shadow --input image.png --charset blocks

# Color modes
ascii-shadow --input image.png --color-mode truecolor
ascii-shadow --input image.png --color-mode 256
ascii-shadow --input image.png --color-mode grayscale

# Adjust image
ascii-shadow --input image.png --contrast 30 --brightness 20
ascii-shadow --input image.png --invert

# Save to file
ascii-shadow --input image.png --output art.txt

# Custom charset
ascii-shadow --input image.png --custom-charset " .oO0@"

# Background color for transparency
ascii-shadow --input image.png --background 1a1a2e
```

## Options

| Option | Description |
|---------|-------------|
| `-i, --input <FILE> or <URL>` | Input image path or URL like 'https://example.com/image.png' (required) |
| `-o, --output <FILE>` | Output file (optional, prints to stdout if not specified). An existing file is replaced; the input image itself is never overwritten |
| `-c, --charset <NAME>` | Charset: `standard`, `extended`, `alphanumeric`, `numbers`, `blocks`, `braille` (default: `standard`) |
| `--custom-charset <CHARS>` | Custom charset string, darkest to brightest |
| `-w, --width <N>` | Character width, 1 to 10000 (auto-detected from terminal if not specified) |
| `-h, --height <N>` | Character height, 1 to 10000 (from the picture's proportions if not specified) |
| `--color [true\|false]` | Color output (default: true); `--color false` is the same as `--color-mode none` |
| `--color-mode <MODE>` | Color mode: `truecolor`, `256`, `grayscale`, `none` (default: `truecolor`) |
| `--contrast <N>` | Contrast adjustment (-100 to 100, default: 0) |
| `--brightness <N>` | Brightness adjustment (-100 to 100, default: 0) |
| `--invert` | Invert brightness |
| `--background <HEX>` | Color behind transparent pixels, in hex (e.g., `1a1a2e`), or `none` for white |
| `--resize <MODE>` | Resize mode: `fit`, `fill`, `stretch`, `crop` (default: `fit`) |
| `--preserve-aspect-ratio [true\|false]` | Preserve aspect ratio (default: true); `false` stretches like `--resize stretch` |
| `--fps <N>` | Frames per second for GIF playback, 1 to 1000 (overrides native timing) |
| `--loops <N>` | Number of times to play the animation, 0 = infinite (uses the GIF's own setting if not specified) |
| `--no-clear` | Don't clear the screen between GIF frames; draw each frame over the last |
| `--help` | Show help. `-h` is `--height`, so help has no short form |
| `-V, --version` | Show the version |

Resize modes:

| Mode | Result |
|------|--------|
| `fit` | The whole picture, as large as fits in the size, proportions kept |
| `fill` | Exactly the size; the picture covers it and the overflow is cropped at the center |
| `stretch` | Exactly the size, proportions ignored |
| `crop` | The center of the picture at one pixel per column, cropped to the size |

Without `--width` and `--height` the picture fits the terminal (80×24 when its
size is unknown). With one of them, the other follows the picture's
proportions.

### Limits

- Inputs up to 64 MiB. URLs: HTTP or HTTPS, up to 10 redirects, 30 seconds for the
  whole download; responses with a text, HTML or JSON content type are refused.
- Decoded pictures up to 16384 pixels per side and 512 MiB of memory; GIF canvases
  the same. GIF files are recognised by their content, not their name.
- Output up to 4,000,000 characters per picture, and animations up to 10,000 frames
  or 256 MiB of rendered text.
- Ctrl-C stops a GIF animation and restores the cursor.

## Supported Image Formats

PNG, JPEG, WebP, GIF, BMP, TIFF, and more via the `image` crate.

## Examples

### Animated GIF
```bash
ascii-shadow --input animation.gif --fps 30 --loops 0 --no-clear
```

### Simple ASCII art
```bash
ascii-shadow --input photo.jpg --color-mode none
```

### High detail Braille patterns
```bash
ascii-shadow --input photo.png --charset braille --width 100
```

### Alphanumeric with adjustments
```bash
ascii-shadow --input logo.png --charset alphanumeric --contrast 20 --brightness -10
```

### Custom output
```bash
ascii-shadow --input diagram.png --custom-charset " .:-=+*#%@" --width 120
```

## Development

```bash
# Run tests (unit tests and end-to-end runs of the binary; URLs use a local test server)
cargo test

# Check formatting
cargo fmt --check

# Build
cargo build --release
```

## License

GPL-V3 License

# ASCII Shadow

Convert images to ASCII art blazing fast with full color support.

## Example

https://github.com/user-attachments/assets/4b9859d6-acf4-4bbe-991c-0ca8718618e0

## Features

- **Multiple charsets**: Standard ASCII, Extended, Alphanumeric, Numbers-only, Block characters, and Braille patterns
- **Color support**: Truecolor (24-bit), 256-color palette, Grayscale, or no color
- **Auto-sizing**: Automatically calculates optimal dimensions based on terminal size
- **Image adjustments**: Contrast, brightness, and color inversion
- **Resize modes**: Fit, fill, stretch, or crop to target dimensions
- **Output options**: Print to terminal or save to file

## Installation

```bash
cargo install --git https://github.com/tatlilimon/ascii-shadow
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
| `-i, --input <FILE>` | Input image path (required) |
| `-o, --output <FILE>` | Output file (optional, prints to stdout if not specified) |
| `-c, --charset <NAME>` | Charset: `standard`, `extended`, `alphanumeric`, `numbers`, `blocks`, `braille` (default: `standard`) |
| `--custom-charset <CHARS>` | Custom charset string |
| `-w, --width <N>` | Character width (auto-detected from terminal if not specified) |
| `-h, --height <N>` | Character height (auto-calculated based on aspect ratio if not specified) |
| `-c, --color` | Enable color output (default: true) |
| `--color-mode <MODE>` | Color mode: `truecolor`, `256`, `grayscale`, `none` (default: `truecolor`) |
| `--contrast <N>` | Contrast adjustment (-100 to 100, default: 0) |
| `--brightness <N>` | Brightness adjustment (-100 to 100, default: 0) |
| `--invert` | Invert colors |
| `--background <HEX>` | Background color in hex format (e.g., `1a1a2e`) or `none` |
| `--resize <MODE>` | Resize mode: `fit`, `fill`, `stretch`, `crop` (default: `fit`) |
| `--preserve-aspect-ratio` | Preserve aspect ratio (default: true) |

## Supported Image Formats

PNG, JPEG, WebP, GIF, BMP, TIFF, and more via the `image` crate.

## Examples

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
# Run tests
cargo test

# Build
cargo build --release
```

## License

GPL-V3 License

//! Braille pattern charset using Unicode Braille characters
//!
//! Braille patterns (U+2800 to U+28FF) provide a 2x4 dot matrix,
//! allowing for much finer detail than single character charsets.

use super::Charset;

/// Braille charset
///
/// Uses Unicode Braille patterns where each character represents a 2x4 pixel block.
/// This provides up to 256 different density levels.
pub struct BrailleCharset;

impl Charset for BrailleCharset {
    fn is_braille(&self) -> bool {
        true
    }

    fn characters(&self) -> &[char] {
        // Braille patterns from U+2800 (empty) to U+28FF (full)
        // We use a subset sorted by visual density
        &[
            '⠀', '⠁', '⠂', '⠃', '⠄', '⠅', '⠆', '⠇', '⠈', '⠉', '⠊', '⠋', '⠌', '⠍', '⠎', '⠏', '⠐',
            '⠑', '⠒', '⠓', '⠔', '⠕', '⠖', '⠗', '⠘', '⠙', '⠚', '⠛', '⠜', '⠝', '⠞', '⠟', '⠠', '⠡',
            '⠢', '⠣', '⠤', '⠥', '⠦', '⠧', '⠨', '⠩', '⠪', '⠫', '⠬', '⠭', '⠮', '⠯', '⠰', '⠱', '⠲',
            '⠳', '⠴', '⠵', '⠶', '⠷', '⠸', '⠹', '⠺', '⠻', '⠼', '⠽', '⠾', '⠿', '⡀', '⡁', '⡂', '⡃',
            '⡄', '⡅', '⡆', '⡇', '⡈', '⡉', '⡊', '⡋', '⡌', '⡍', '⡎', '⡏', '⡐', '⡑', '⡒', '⡓', '⡔',
            '⡕', '⡖', '⡗', '⡘', '⡙', '⡚', '⡛', '⡜', '⡝', '⡞', '⡟', '⡠', '⡡', '⡢', '⡣', '⡤', '⡥',
            '⡦', '⡧', '⡨', '⡩', '⡪', '⡫', '⡬', '⡭', '⡮', '⡯', '⡰', '⡱', '⡲', '⡳', '⡴', '⡵', '⡶',
            '⡷', '⡸', '⡹', '⡺', '⡻', '⡼', '⡽', '⡾', '⡿', '⢀', '⢁', '⢂', '⢃', '⢄', '⢅', '⢆', '⢇',
            '⢈', '⢉', '⢊', '⢋', '⢌', '⢍', '⢎', '⢏', '⢐', '⢑', '⢒', '⢓', '⢔', '⢕', '⢖', '⢗', '⢘',
            '⢙', '⢚', '⢛', '⢜', '⢝', '⢞', '⢟', '⢠', '⢡', '⢢', '⢣', '⢤', '⢥', '⢦', '⢧', '⢨', '⢩',
            '⢪', '⢫', '⢬', '⢭', '⢮', '⢯', '⢰', '⢱', '⢲', '⢳', '⢴', '⢵', '⢶', '⢷', '⢸', '⢹', '⢺',
            '⢻', '⢼', '⢽', '⢾', '⢿', '⣀', '⣁', '⣂', '⣃', '⣄', '⣅', '⣆', '⣇', '⣈', '⣉', '⣊', '⣋',
            '⣌', '⣍', '⣎', '⣏', '⣐', '⣑', '⣒', '⣓', '⣔', '⣕', '⣖', '⣗', '⣘', '⣙', '⣚', '⣛', '⣜',
            '⣝', '⣞', '⣟', '⣠', '⣡', '⣢', '⣣', '⣤', '⣥', '⣦', '⣧', '⣨', '⣩', '⣪', '⣫', '⣬', '⣭',
            '⣮', '⣯', '⣰', '⣱', '⣲', '⣳', '⣴', '⣵', '⣶', '⣷', '⣸', '⣹', '⣺', '⣻', '⣼', '⣽', '⣾',
            '⣿',
        ]
    }
}

/// Convert a 2x4 pixel block to a Braille character
///
/// `pixels[row][col]` is one dot. Unicode numbers the dots column by column:
/// ```text
/// dot 1 (0,0)  dot 4 (0,1)
/// dot 2 (1,0)  dot 5 (1,1)
/// dot 3 (2,0)  dot 6 (2,1)
/// dot 7 (3,0)  dot 8 (3,1)
/// ```
/// and dot `n` is bit `n - 1` of the offset from U+2800.
pub fn pixels_to_braille(pixels: [[bool; 2]; 4]) -> char {
    const BITS: [[u32; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
    let mut code: u32 = 0;
    for (row, bits) in BITS.iter().enumerate() {
        for (col, bit) in bits.iter().enumerate() {
            if pixels[row][col] {
                code |= bit;
            }
        }
    }
    char::from_u32(0x2800 + code).unwrap_or('⠀')
}

/// Starting character for empty Braille (U+2800)
pub const BRAILLE_EMPTY: char = '\u{2800}';

/// Ending character for full Braille (U+28FF)
pub const BRAILLE_FULL: char = '\u{28FF}';

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_charset() {
        let charset = BrailleCharset;
        assert_eq!(charset.len(), 256);
        assert_eq!(charset.map_brightness(0), '⠀'); // U+2800 - first character
        assert_eq!(charset.map_brightness(255), '⣿'); // U+28FF - last character
    }

    #[test]
    fn test_pixels_to_braille_empty() {
        let pixels = [[false; 2]; 4];
        assert_eq!(pixels_to_braille(pixels), '⠀');
    }

    #[test]
    fn test_pixels_to_braille_full() {
        let pixels = [[true; 2]; 4];
        assert_eq!(pixels_to_braille(pixels), '⣿');
    }

    #[test]
    fn every_dot_lands_at_its_unicode_position() {
        // (row, col) -> the single-dot character Unicode defines for that position.
        let expected = [
            ((0, 0), '⠁'),
            ((1, 0), '⠂'),
            ((2, 0), '⠄'),
            ((3, 0), '⡀'),
            ((0, 1), '⠈'),
            ((1, 1), '⠐'),
            ((2, 1), '⠠'),
            ((3, 1), '⢀'),
        ];
        for ((row, col), ch) in expected {
            let mut pixels = [[false; 2]; 4];
            pixels[row][col] = true;
            assert_eq!(pixels_to_braille(pixels), ch, "dot at ({row},{col})");
        }
        // The left column alone is dots 1, 2, 3 and 7.
        assert_eq!(pixels_to_braille([[true, false]; 4]), '⡇');
        assert_eq!(pixels_to_braille([[false, true]; 4]), '⢸');
    }

    #[test]
    fn test_pixels_to_braille_partial() {
        // Just the top-left dot
        let mut pixels = [[false; 2]; 4];
        pixels[0][0] = true;
        assert_eq!(pixels_to_braille(pixels), '⠁');
    }
}

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
    fn characters(&self) -> &[char] {
        // Braille patterns from U+2800 (empty) to U+28FF (full)
        // We use a subset sorted by visual density
        &[
            '⠀', '⠁', '⠂', '⠃', '⠄', '⠅', '⠆', '⠇', '⠈', '⠉', '⠊', '⠋', '⠌', '⠍', '⠎',
            '⠏', '⠐', '⠑', '⠒', '⠓', '⠔', '⠕', '⠖', '⠗', '⠘', '⠙', '⠚', '⠛', '⠜', '⠝',
            '⠞', '⠟', '⠠', '⠡', '⠢', '⠣', '⠤', '⠥', '⠦', '⠧', '⠨', '⠩', '⠪', '⠫', '⠬', '⠭',
            '⠮', '⠯', '⠰', '⠱', '⠲', '⠳', '⠴', '⠵', '⠶', '⠷', '⠸', '⠹', '⠺', '⠻', '⠼', '⠽',
            '⠾', '⠿', '⡀', '⡁', '⡂', '⡃', '⡄', '⡅', '⡆', '⡇', '⡈', '⡉', '⡊', '⡋', '⡌', '⡍',
            '⡎', '⡏', '⡐', '⡑', '⡒', '⡓', '⡔', '⡕', '⡖', '⡗', '⡘', '⡙', '⡚', '⡛', '⡜', '⡝',
            '⡞', '⡟', '⡠', '⡡', '⡢', '⡣', '⡤', '⡥', '⡦', '⡧', '⡨', '⡩', '⡪', '⡫', '⡬', '⡭',
            '⡮', '⡯', '⡰', '⡱', '⡲', '⡳', '⡴', '⡵', '⡶', '⡷', '⡸', '⡹', '⡺', '⡻', '⡼', '⡽',
            '⡾', '⡿', '⢀', '⢁', '⢂', '⢃', '⢄', '⢅', '⢆', '⢇', '⢈', '⢉', '⢊', '⢋', '⢌', '⢍',
            '⢎', '⢏', '⢐', '⢑', '⢒', '⢓', '⢔', '⢕', '⢖', '⢗', '⢘', '⢙', '⢚', '⢛', '⢜', '⢝',
            '⢞', '⢟', '⢠', '⢡', '⢢', '⢣', '⢤', '⢥', '⢦', '⢧', '⢨', '⢩', '⢪', '⢫', '⢬', '⢭',
            '⢮', '⢯', '⢰', '⢱', '⢲', '⢳', '⢴', '⢵', '⢶', '⢷', '⢸', '⢹', '⢺', '⢻', '⢼', '⢽',
            '⢾', '⢿', '⣀', '⣁', '⣂', '⣃', '⣄', '⣅', '⣆', '⣇', '⣈', '⣉', '⣊', '⣋', '⣌', '⣍',
            '⣎', '⣏', '⣐', '⣑', '⣒', '⣓', '⣔', '⣕', '⣖', '⣗', '⣘', '⣙', '⣚', '⣛', '⣜', '⣝',
            '⣞', '⣟', '⣠', '⣡', '⣢', '⣣', '⣤', '⣥', '⣦', '⣧', '⣨', '⣩', '⣪', '⣫', '⣬', '⣭',
            '⣮', '⣯', '⣰', '⣱', '⣲', '⣳', '⣴', '⣵', '⣶', '⣷', '⣸', '⣹', '⣺', '⣻', '⣼', '⣽',
            '⣾', '⣿',
        ]
    }
}

/// Convert a 2x4 pixel block to a Braille character
///
/// Each cell in the 2x4 array represents a dot in the Braille character:
/// ```text
/// (0,0) (0,1)
/// (1,0) (1,1)
/// (2,0) (2,1)
/// (3,0) (3,1)
/// ```
pub fn pixels_to_braille(pixels: [[bool; 2]; 4]) -> char {
    let mut code: u32 = 0;
    // Dot positions in Braille: (row, col) -> bit
    // Positions: (0,0)=0, (0,1)=1, (1,0)=2, (1,1)=3, (2,0)=4, (2,1)=5, (3,0)=6, (3,1)=7
    if pixels[0][0] { code |= 1 << 0; }
    if pixels[0][1] { code |= 1 << 1; }
    if pixels[1][0] { code |= 1 << 2; }
    if pixels[1][1] { code |= 1 << 3; }
    if pixels[2][0] { code |= 1 << 4; }
    if pixels[2][1] { code |= 1 << 5; }
    if pixels[3][0] { code |= 1 << 6; }
    if pixels[3][1] { code |= 1 << 7; }
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
        assert_eq!(charset.map_brightness(0), '⠀');  // U+2800 - first character
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
    fn test_pixels_to_braille_partial() {
        // Just the top-left dot
        let mut pixels = [[false; 2]; 4];
        pixels[0][0] = true;
        assert_eq!(pixels_to_braille(pixels), '⠁');
    }
}

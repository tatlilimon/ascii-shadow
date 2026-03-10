//! Standard ASCII charset

use super::Charset;

/// Standard ASCII charset with simple density levels
pub struct StandardCharset;

impl Charset for StandardCharset {
    fn characters(&self) -> &[char] {
        &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@']
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_charset() {
        let charset = StandardCharset;
        assert_eq!(charset.len(), 10);
        assert_eq!(charset.map_brightness(0), ' ');
        assert_eq!(charset.map_brightness(255), '@');
        assert_eq!(charset.map_brightness(127), '='); // Index 4 for ~50% brightness
        assert_eq!(charset.map_brightness(200), '#'); // Index 7 for ~80% brightness
    }
}

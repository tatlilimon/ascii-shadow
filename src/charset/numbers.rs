//! Numbers-only charset

use super::Charset;

/// Numbers-only charset ordered by visual density
pub struct NumbersCharset;

impl Charset for NumbersCharset {
    fn characters(&self) -> &[char] {
        &[
            ' ', '.', '1', '!', '+', '=', '-', ':', ';', 'i', '|', '>', '?', '<', ']', '[', ')', '(',
            '}', '{', '/', 'l', 'L', 't', 'f', 'j', 'r', 'x', 'n', 'u', 'v', 'c', 'z', 'X', 'Y',
            'U', 'J', 'C', 'L', 'Q', '0', 'O', 'Z', 'm', 'w', 'q', 'p', 'd', 'b', 'k', 'h', 'a',
            'o', '*', '#', 'M', 'W', '&', '8', '%', 'B', '@', '$',
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numbers_charset() {
        let charset = NumbersCharset;
        assert!(charset.len() > 10);
        assert_eq!(charset.map_brightness(0), ' ');
        assert_eq!(charset.map_brightness(255), '$');
    }
}

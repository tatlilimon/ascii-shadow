//! Charset definitions and brightness mapping

pub mod blocks;
pub mod braille;

use std::str::FromStr;

/// Error type for charset operations
#[derive(Debug, thiserror::Error)]
pub enum CharsetError {
    #[error("Unknown charset: {0}")]
    UnknownCharset(String),
    #[error("Custom charset cannot be empty")]
    EmptyCharset,
}

/// Trait for charsets that can map brightness values to characters
pub trait Charset: Send + Sync {
    /// Get the characters in this charset ordered from darkest to lightest
    fn characters(&self) -> &[char];

    /// Map a brightness value (0-255) to a character
    fn map_brightness(&self, brightness: u8) -> char {
        let chars = self.characters();
        if chars.is_empty() {
            return ' ';
        }
        let idx = (brightness as f32 / 255.0 * (chars.len() - 1) as f32).round() as usize;
        chars[idx.min(chars.len() - 1)]
    }

    /// Get the number of characters in this charset
    fn len(&self) -> usize {
        self.characters().len()
    }

    /// Braille draws a 2x4 dot pattern per character instead of mapping brightness.
    fn is_braille(&self) -> bool {
        false
    }
}

/// Standard ASCII charset
pub struct StandardCharset;

impl Charset for StandardCharset {
    fn characters(&self) -> &[char] {
        &[
            ' ', '.', ':', '-', '=', '+', '*', '#', '%', '@',
        ]
    }
}

/// Extended standard ASCII charset with more density levels
pub struct ExtendedStandardCharset;

impl Charset for ExtendedStandardCharset {
    fn characters(&self) -> &[char] {
        &[
            ' ', '`', '^', ',', ':', ';', 'I', 'l', '!', 'i', '>', '<', '~', '+', '_', '-', '?',
            ']', '[', '}', '{', '1', ')', '(', '|', '\\', '/', 't', 'f', 'j', 'r', 'x', 'n', 'u',
            'v', 'c', 'z', 'X', 'Y', 'U', 'J', 'C', 'L', 'Q', '0', 'O', 'Z', 'm', 'w', 'q', 'p',
            'd', 'b', 'k', 'h', 'a', 'o', '*', '#', 'M', 'W', '&', '8', '%', 'B', '@', '$',
        ]
    }
}

/// Alphanumeric charset: letters and digits (and space for black)
pub struct AlphanumericCharset;

impl Charset for AlphanumericCharset {
    fn characters(&self) -> &[char] {
        &[
            ' ', 'i', '1', 'l', 'I', 'J', 'L', 't', 'f', 'j', 'r', 'x', 'n', 'u', 'v', 'c', 'z',
            'X', 'Y', 'U', 'C', 'Q', '0', 'O', 'Z', 'm', 'w', 'q', 'p', 'd', 'b', 'k', 'h', 'a',
            'o', 'M', 'W', '8', 'B',
        ]
    }
}

/// Numbers charset: digits by ink coverage, with light punctuation for faint areas
pub struct NumbersCharset;

impl Charset for NumbersCharset {
    fn characters(&self) -> &[char] {
        &[' ', '.', ':', '1', '7', '4', '3', '2', '5', '9', '6', '0', '8']
    }
}

/// Custom user-defined charset
pub struct CustomCharset {
    chars: Vec<char>,
}

impl CustomCharset {
    pub fn new(s: &str) -> Result<Self, CharsetError> {
        if s.is_empty() {
            return Err(CharsetError::EmptyCharset);
        }
        Ok(Self { chars: s.chars().collect() })
    }
}

impl Charset for CustomCharset {
    fn characters(&self) -> &[char] {
        &self.chars
    }
}

/// Parse charset from string
pub fn from_str(name: &str) -> Result<Box<dyn Charset>, CharsetError> {
    match name.to_lowercase().as_str() {
        "standard" => Ok(Box::new(StandardCharset)),
        "extended" => Ok(Box::new(ExtendedStandardCharset)),
        "alphanumeric" | "alpha" => Ok(Box::new(AlphanumericCharset)),
        "numbers" | "num" => Ok(Box::new(NumbersCharset)),
        "blocks" | "block" => Ok(Box::new(blocks::BlocksCharset)),
        "braille" => Ok(Box::new(braille::BrailleCharset)),
        _ => Err(CharsetError::UnknownCharset(name.to_string())),
    }
}

/// Create charset from custom string
pub fn from_custom(s: &str) -> Result<Box<dyn Charset>, CharsetError> {
    Ok(Box::new(CustomCharset::new(s)?))
}

impl FromStr for Box<dyn Charset> {
    type Err = CharsetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        from_str(s)
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
    }

    #[test]
    fn test_custom_charset() {
        let charset = CustomCharset::new(".#").unwrap();
        assert_eq!(charset.len(), 2);
        assert_eq!(charset.map_brightness(127), '.'); // Index 0 for ~50% brightness
        assert_eq!(charset.map_brightness(255), '#'); // Index 1 for 100% brightness
    }

    #[test]
    fn test_custom_charset_empty() {
        assert!(CustomCharset::new("").is_err());
    }

    #[test]
    fn named_charsets_hold_only_their_documented_characters() {
        let alnum = from_str("alphanumeric").unwrap();
        assert!(alnum.characters()[1..].iter().all(|c| c.is_ascii_alphanumeric()));
        let numbers = from_str("numbers").unwrap();
        assert!(numbers.characters().iter().all(|c| c.is_ascii_digit() || " .:".contains(*c)));
        assert_eq!(numbers.characters().iter().filter(|c| c.is_ascii_digit()).count(), 10);
        let blocks = from_str("blocks").unwrap();
        assert_eq!(blocks.characters(), &[' ', '░', '▒', '▓', '█']);
        assert!(from_str("braille").unwrap().is_braille());
    }

    #[test]
    fn every_named_charset_is_dark_to_bright_without_duplicates() {
        for name in ["standard", "extended", "alphanumeric", "alpha", "numbers", "num", "blocks", "block"] {
            let charset = from_str(name).unwrap();
            let chars = charset.characters();
            assert_eq!(chars[0], ' ', "{name}: black must map to a blank");
            assert_eq!(charset.map_brightness(0), ' ');
            assert_eq!(charset.map_brightness(255), *chars.last().unwrap());
            let mut unique = chars.to_vec();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), chars.len(), "{name} repeats a character");
            assert!(!charset.is_braille());
        }
        assert!(from_str("Standard").is_ok());
        assert!(matches!(from_str("emoji"), Err(CharsetError::UnknownCharset(_))));
    }

    #[test]
    fn brightness_steps_through_every_character_in_order() {
        let charset = StandardCharset;
        let mapped: Vec<char> = (0..=255u8).map(|b| charset.map_brightness(b)).collect();
        let mut seen: Vec<char> = mapped.clone();
        seen.dedup();
        assert_eq!(seen, charset.characters());
    }
}

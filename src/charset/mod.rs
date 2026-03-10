//! Charset definitions and brightness mapping

pub mod blocks;
pub mod braille;
pub mod numbers;
pub mod standard;

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

/// Alphanumeric charset
pub struct AlphanumericCharset;

impl Charset for AlphanumericCharset {
    fn characters(&self) -> &[char] {
        &[
            ' ', '.', ':', 'i', '|', '!', '>', '?', '<', '+', '-', ']', '[', ')', '(', '}', '{',
            '1', 'l', 'I', '/', 'J', 'L', 't', 'f', 'j', 'r', 'x', 'n', 'u', 'v', 'c', 'z', 'X',
            'Y', 'U', 'J', 'C', 'L', 'Q', '0', 'O', 'Z', 'm', 'w', 'q', 'p', 'd', 'b', 'k', 'h',
            'a', 'o', '*', '#', 'M', 'W', '&', '8', '%', 'B', '@', '$',
        ]
    }
}

/// Numbers-only charset
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
}

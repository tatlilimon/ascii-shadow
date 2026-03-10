//! Block character charset using Unicode block characters

use super::Charset;

/// Block character charset
pub struct BlocksCharset;

impl Charset for BlocksCharset {
    fn characters(&self) -> &[char] {
        &[' ', '░', '▒', '▓', '█']
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_charset() {
        let charset = BlocksCharset;
        assert_eq!(charset.len(), 5);
        assert_eq!(charset.map_brightness(0), ' ');
        assert_eq!(charset.map_brightness(255), '█');
    }
}

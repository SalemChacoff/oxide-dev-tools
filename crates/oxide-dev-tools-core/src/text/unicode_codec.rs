//! Unicode escape encoding and decoding.
//!
//! [`convert_unicode`] turns non-ASCII characters into `\u` escape
//! sequences and turns `\uXXXX`, `\u{...}`, and `\UXXXXXXXX` escapes back
//! into characters. ASCII input passes through untouched, and decoding
//! leaves other backslash sequences (`\n`, `\\`, …) literal.
//!
//! Two escape styles are supported for encoding: JSON-style `\uXXXX` with
//! surrogate pairs for characters beyond U+FFFF, and Rust-style `\u{...}`
//! with 1–6 hex digits and no padding.

use std::fmt;

// -------- Public API --------

/// Errors that can occur when encoding or decoding unicode escapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnicodeError {
    /// An escape is malformed: truncated sequence, invalid hex digits, or an empty `\u{}`.
    InvalidEscape,
    /// An escape references a codepoint outside the Unicode range (a surrogate or beyond U+10FFFF).
    InvalidCodepoint,
    /// A high surrogate escape is not followed by a low surrogate escape.
    LoneSurrogate,
}

impl fmt::Display for UnicodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnicodeError::InvalidEscape => write!(f, "malformed unicode escape sequence"),
            UnicodeError::InvalidCodepoint => write!(f, "escape references a codepoint outside the Unicode range"),
            UnicodeError::LoneSurrogate => write!(f, "high surrogate escape is not followed by a low surrogate"),
        }
    }
}

impl std::error::Error for UnicodeError {}

/// Escape style used when encoding.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeStyle {
    /// JSON-style `\uXXXX` with surrogate pairs for characters beyond U+FFFF.
    #[default]
    Json,
    /// Rust-style `\u{...}` with 1–6 hex digits and no padding.
    Rust,
}

/// Options for a unicode encode operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnicodeOptions {
    /// Text to escape.
    pub input: String,
    /// Escape style to use.
    pub style: UnicodeStyle,
}

impl Default for UnicodeOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            style: UnicodeStyle::Json,
        }
    }
}

/// Operations available for unicode escape encoding.
#[derive(Debug)]
pub enum UnicodeKind {
    /// Escape non-ASCII characters in the input.
    Encode(UnicodeOptions),
    /// Turn `\uXXXX`, `\u{...}`, or `\UXXXXXXXX` escapes back into characters.
    Decode(String),
}

/// Encode or decode unicode escapes according to `kind`.
pub fn convert_unicode(kind: UnicodeKind) -> Result<String, UnicodeError> {
    match kind {
        UnicodeKind::Encode(opts) => encode(&opts),
        UnicodeKind::Decode(input) => decode(&input),
    }
}

// -------- Encoding --------

fn encode(opts: &UnicodeOptions) -> Result<String, UnicodeError> {
    let mut encoded = String::with_capacity(opts.input.len());
    for ch in opts.input.chars() {
        if ch.is_ascii() {
            encoded.push(ch);
        } else {
            match opts.style {
                UnicodeStyle::Json => push_json_escape(&mut encoded, ch),
                UnicodeStyle::Rust => push_rust_escape(&mut encoded, ch),
            }
        }
    }
    Ok(encoded)
}

/// Append a JSON-style escape for `ch` (surrogate pairs beyond U+FFFF).
fn push_json_escape(out: &mut String, ch: char) {
    let value = ch as u32;
    if value <= 0xFFFF {
        out.push_str(&format!("\\u{value:04x}"));
    } else {
        let adjusted = value - 0x10000;
        let high = 0xD800 + (adjusted >> 10);
        let low = 0xDC00 + (adjusted & 0x3FF);
        out.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
    }
}

/// Append a Rust-style escape for `ch`.
fn push_rust_escape(out: &mut String, ch: char) {
    out.push_str(&format!("\\u{{{:x}}}", ch as u32));
}

// -------- Decoding --------

fn decode(input: &str) -> Result<String, UnicodeError> {
    let chars: Vec<char> = input.chars().collect();
    let mut decoded = String::with_capacity(input.len());
    let mut index = 0;
    while index < chars.len() {
        let (ch, next) = if chars[index] == '\\' {
            decode_escape(&chars, index)?
        } else {
            (chars[index], index + 1)
        };
        decoded.push(ch);
        index = next;
    }
    Ok(decoded)
}

/// Decode the escape starting at `chars[index] == '\\'`; returns the character
/// and the index just past the sequence. Non-unicode escapes stay literal.
fn decode_escape(chars: &[char], index: usize) -> Result<(char, usize), UnicodeError> {
    match chars.get(index + 1) {
        Some('u') if chars.get(index + 2) == Some(&'{') => decode_braced(chars, index + 3),
        Some('u') => decode_fixed_escape(chars, index),
        Some('U') => decode_fixed(chars, index + 2, 8, index + 10),
        _ => Ok(('\\', index + 1)),
    }
}

/// Decode a `\uXXXX` escape at `index`, combining surrogate pairs into a
/// single character when a low surrogate escape immediately follows.
fn decode_fixed_escape(chars: &[char], index: usize) -> Result<(char, usize), UnicodeError> {
    let value = parse_fixed_hex(chars, index + 2, 4)?;
    if (0xD800..=0xDBFF).contains(&value) {
        decode_surrogate_pair(chars, index, value)
    } else if (0xDC00..=0xDFFF).contains(&value) {
        Err(UnicodeError::LoneSurrogate)
    } else {
        let ch = char::from_u32(value).ok_or(UnicodeError::InvalidCodepoint)?;
        Ok((ch, index + 6))
    }
}

/// Combine a high surrogate escape at `index` with the low surrogate escape
/// that must immediately follow it.
fn decode_surrogate_pair(chars: &[char], index: usize, high: u32) -> Result<(char, usize), UnicodeError> {
    let low_start = index + 6;
    let followed_by_escape = chars.get(low_start) == Some(&'\\') && matches!(chars.get(low_start + 1), Some('u' | 'U'));
    if !followed_by_escape {
        return Err(UnicodeError::LoneSurrogate);
    }
    let low = parse_fixed_hex(chars, low_start + 2, 4)?;
    if !(0xDC00..=0xDFFF).contains(&low) {
        return Err(UnicodeError::LoneSurrogate);
    }
    let codepoint = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
    let ch = char::from_u32(codepoint).ok_or(UnicodeError::InvalidCodepoint)?;
    Ok((ch, low_start + 6))
}

/// Decode a `\u{...}` escape starting at the first hex digit.
fn decode_braced(chars: &[char], start: usize) -> Result<(char, usize), UnicodeError> {
    let end = chars[start..]
        .iter()
        .position(|&ch| ch == '}')
        .map(|offset| start + offset)
        .ok_or(UnicodeError::InvalidEscape)?;
    let digits: String = chars[start..end].iter().collect();
    let value = u64::from_str_radix(&digits, 16).map_err(|_| UnicodeError::InvalidEscape)?;
    let value = u32::try_from(value).map_err(|_| UnicodeError::InvalidCodepoint)?;
    let ch = char::from_u32(value).ok_or(UnicodeError::InvalidCodepoint)?;
    Ok((ch, end + 1))
}

/// Decode exactly `width` hex digits starting at `chars[start]`.
fn decode_fixed(chars: &[char], start: usize, width: usize, next: usize) -> Result<(char, usize), UnicodeError> {
    let value = parse_fixed_hex(chars, start, width)?;
    let ch = char::from_u32(value).ok_or(UnicodeError::InvalidCodepoint)?;
    Ok((ch, next))
}

/// Parse exactly `width` hex digits from `chars[start..]`.
fn parse_fixed_hex(chars: &[char], start: usize, width: usize) -> Result<u32, UnicodeError> {
    let digits: String = chars
        .get(start..start + width)
        .ok_or(UnicodeError::InvalidEscape)?
        .iter()
        .collect();
    u32::from_str_radix(&digits, 16).map_err(|_| UnicodeError::InvalidEscape)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(input: &str) -> UnicodeOptions {
        UnicodeOptions {
            input: input.to_string(),
            ..UnicodeOptions::default()
        }
    }

    fn encode(input: &str) -> String {
        convert_unicode(UnicodeKind::Encode(options(input))).unwrap()
    }

    fn decode(input: &str) -> Result<String, UnicodeError> {
        convert_unicode(UnicodeKind::Decode(input.to_string()))
    }

    #[test]
    fn encode_json_known_vectors() {
        assert_eq!(encode("café"), r"caf\u00e9");
        assert_eq!(encode("🚀"), r"\ud83d\ude80");
        assert_eq!(encode("ascii"), "ascii");
        assert_eq!(encode(""), "");
    }

    #[test]
    fn encode_rust_known_vectors() {
        let rust = |input: &str| {
            convert_unicode(UnicodeKind::Encode(UnicodeOptions {
                input: input.to_string(),
                style: UnicodeStyle::Rust,
            }))
            .unwrap()
        };
        assert_eq!(rust("café"), r"caf\u{e9}");
        assert_eq!(rust("🚀"), r"\u{1f680}");
        assert_eq!(rust("ascii"), "ascii");
    }

    #[test]
    fn decode_known_vectors() {
        assert_eq!(decode(r"caf\u00e9").unwrap(), "café");
        assert_eq!(decode(r"\ud83d\ude80").unwrap(), "🚀");
        assert_eq!(decode(r"\u{e9}").unwrap(), "é");
        assert_eq!(decode(r"\u{1f680}").unwrap(), "🚀");
        assert_eq!(decode(r"\U0001F680").unwrap(), "🚀");
        assert_eq!(decode(r"\U000000E9").unwrap(), "é");
    }

    #[test]
    fn decode_leaves_other_escapes_literal() {
        assert_eq!(decode(r"line\nbreak").unwrap(), r"line\nbreak");
        assert_eq!(decode(r"\\u00e9").unwrap(), r"\é");
        assert_eq!(decode(r"\").unwrap(), r"\");
        assert_eq!(decode("").unwrap(), "");
    }

    #[test]
    fn decode_malformed_escape_errors() {
        for input in [
            r"\u", r"\u0", r"\u00e", r"\uZZZZ", r"\u{", r"\u{}", r"\u{xyz}", r"\U1234",
        ] {
            assert_eq!(decode(input), Err(UnicodeError::InvalidEscape), "input {input:?}");
        }
    }

    #[test]
    fn decode_invalid_codepoint_errors() {
        for input in [r"\u{110000}", r"\u{d800}", r"\U0000D800", r"\u{FFFFFFFF}"] {
            assert_eq!(decode(input), Err(UnicodeError::InvalidCodepoint), "input {input:?}");
        }
    }

    #[test]
    fn decode_lone_surrogate_errors() {
        for input in [
            r"\ud800",
            r"\udc00",
            r"\ud800\u0041",
            r"\ud800\ud800",
            r"\ud800x",
            r"a\ud800",
        ] {
            assert_eq!(decode(input), Err(UnicodeError::LoneSurrogate), "input {input:?}");
        }
    }

    #[test]
    fn roundtrip_all_non_ascii_scalars() {
        for value in 0x80_u32..=0x2FFFF {
            let Some(ch) = char::from_u32(value) else {
                continue;
            };
            for style in [UnicodeStyle::Json, UnicodeStyle::Rust] {
                let encoded = convert_unicode(UnicodeKind::Encode(UnicodeOptions {
                    input: ch.to_string(),
                    style,
                }))
                .unwrap();
                assert_eq!(decode(&encoded).unwrap(), ch.to_string(), "roundtrip {value:#x} in {style:?}");
            }
        }
    }

    #[test]
    fn options_defaults() {
        let defaults = UnicodeOptions::default();
        assert_eq!(defaults.input, "");
        assert_eq!(defaults.style, UnicodeStyle::Json);
    }

    #[test]
    fn dispatch_through_kind() {
        let encoded = convert_unicode(UnicodeKind::Encode(options("héllo"))).unwrap();
        assert_eq!(encoded, r"h\u00e9llo");
        assert_eq!(decode(&encoded).unwrap(), "héllo");
    }
}

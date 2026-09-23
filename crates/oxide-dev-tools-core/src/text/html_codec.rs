//! HTML entity encoding and decoding.
//!
//! [`convert_html`] escapes the characters that carry meaning in HTML (`&`,
//! `<`, `>`, and — in attribute mode — `"` and `'`) and decodes numeric
//! character references plus a curated table of common named entities.
//!
//! Limitation: this is a string codec, not a WHATWG HTML5 parser. Entity
//! references require a trailing semicolon, named entities are limited to
//! the supported table, and no context-sensitive parsing is performed.

use std::fmt;

// -------- Public API --------

/// Errors that can occur when encoding or decoding HTML entities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlError {
    /// A named entity is not in the supported table (names are case-sensitive).
    UnknownEntity,
    /// An entity reference is malformed: missing `&`, missing `;`, or invalid digits.
    InvalidEntity,
    /// A numeric entity references a codepoint outside the Unicode range (a surrogate or beyond U+10FFFF).
    InvalidCodepoint,
}

impl fmt::Display for HtmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HtmlError::UnknownEntity => write!(f, "unknown HTML entity (supported names are case-sensitive)"),
            HtmlError::InvalidEntity => write!(f, "malformed HTML entity reference"),
            HtmlError::InvalidCodepoint => write!(f, "HTML entity references a codepoint outside the Unicode range"),
        }
    }
}

impl std::error::Error for HtmlError {}

/// How aggressively [`convert_html`] escapes its input.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlMode {
    /// Escape all five characters meaningful in HTML: `&`, `<`, `>`, `"`, and `'`.
    #[default]
    Attribute,
    /// Escape only `&`, `<`, and `>`, leaving quotes literal.
    Text,
}

/// Options for an HTML encode operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlOptions {
    /// Text to escape.
    pub input: String,
    /// Which characters get escaped.
    pub mode: HtmlMode,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            mode: HtmlMode::Attribute,
        }
    }
}

/// Operations available for HTML entity encoding.
#[derive(Debug)]
pub enum HtmlKind {
    /// Escape HTML-significant characters in the input.
    Encode(HtmlOptions),
    /// Turn entity references (named or numeric) back into characters.
    Decode(String),
}

/// Encode or decode HTML entities according to `kind`.
pub fn convert_html(kind: HtmlKind) -> Result<String, HtmlError> {
    match kind {
        HtmlKind::Encode(opts) => encode(&opts),
        HtmlKind::Decode(input) => decode(&input),
    }
}

// -------- Named entity table --------

/// Common named character references, sorted by name for binary search.
const NAMED_ENTITIES: &[(&str, char)] = &[
    ("AElig", '\u{C6}'),
    ("Aacute", '\u{C1}'),
    ("Acirc", '\u{C2}'),
    ("Agrave", '\u{C0}'),
    ("Aring", '\u{C5}'),
    ("Atilde", '\u{C3}'),
    ("Auml", '\u{C4}'),
    ("Ccedil", '\u{C7}'),
    ("Dagger", '\u{2021}'),
    ("ETH", '\u{D0}'),
    ("Eacute", '\u{C9}'),
    ("Ecirc", '\u{CA}'),
    ("Egrave", '\u{C8}'),
    ("Euml", '\u{CB}'),
    ("Iacute", '\u{CD}'),
    ("Icirc", '\u{CE}'),
    ("Igrave", '\u{CC}'),
    ("Iuml", '\u{CF}'),
    ("Ntilde", '\u{D1}'),
    ("Oacute", '\u{D3}'),
    ("Ocirc", '\u{D4}'),
    ("Ograve", '\u{D2}'),
    ("Oslash", '\u{D8}'),
    ("Otilde", '\u{D5}'),
    ("Ouml", '\u{D6}'),
    ("Prime", '\u{2033}'),
    ("THORN", '\u{DE}'),
    ("Uacute", '\u{DA}'),
    ("Ucirc", '\u{DB}'),
    ("Ugrave", '\u{D9}'),
    ("Uuml", '\u{DC}'),
    ("Yacute", '\u{DD}'),
    ("aacute", '\u{E1}'),
    ("acirc", '\u{E2}'),
    ("acute", '\u{B4}'),
    ("aelig", '\u{E6}'),
    ("agrave", '\u{E0}'),
    ("alpha", '\u{3B1}'),
    ("amp", '&'),
    ("apos", '\''),
    ("aring", '\u{E5}'),
    ("atilde", '\u{E3}'),
    ("auml", '\u{E4}'),
    ("beta", '\u{3B2}'),
    ("brvbar", '\u{A6}'),
    ("bull", '\u{2022}'),
    ("ccedil", '\u{E7}'),
    ("cedil", '\u{B8}'),
    ("cent", '\u{A2}'),
    ("circ", '\u{2C6}'),
    ("copy", '\u{A9}'),
    ("crarr", '\u{21B5}'),
    ("curren", '\u{A4}'),
    ("dagger", '\u{2020}'),
    ("deg", '\u{B0}'),
    ("divide", '\u{F7}'),
    ("eacute", '\u{E9}'),
    ("ecirc", '\u{EA}'),
    ("egrave", '\u{E8}'),
    ("emsp", '\u{2003}'),
    ("ensp", '\u{2002}'),
    ("eth", '\u{F0}'),
    ("euml", '\u{EB}'),
    ("euro", '\u{20AC}'),
    ("frac12", '\u{BD}'),
    ("frac14", '\u{BC}'),
    ("frac34", '\u{BE}'),
    ("frasl", '\u{2044}'),
    ("ge", '\u{2265}'),
    ("gt", '>'),
    ("harr", '\u{2194}'),
    ("hellip", '\u{2026}'),
    ("iacute", '\u{ED}'),
    ("icirc", '\u{EE}'),
    ("iexcl", '\u{A1}'),
    ("igrave", '\u{EC}'),
    ("infin", '\u{221E}'),
    ("iquest", '\u{BF}'),
    ("iuml", '\u{EF}'),
    ("laquo", '\u{AB}'),
    ("larr", '\u{2190}'),
    ("ldquo", '\u{201C}'),
    ("le", '\u{2264}'),
    ("lrm", '\u{200E}'),
    ("lsaquo", '\u{2039}'),
    ("lsquo", '\u{2018}'),
    ("lt", '<'),
    ("macr", '\u{AF}'),
    ("mdash", '\u{2014}'),
    ("micro", '\u{B5}'),
    ("middot", '\u{B7}'),
    ("minus", '\u{2212}'),
    ("nbsp", '\u{A0}'),
    ("ndash", '\u{2013}'),
    ("ne", '\u{2260}'),
    ("not", '\u{AC}'),
    ("ntilde", '\u{F1}'),
    ("oacute", '\u{F3}'),
    ("ocirc", '\u{F4}'),
    ("ograve", '\u{F2}'),
    ("oline", '\u{203E}'),
    ("ordf", '\u{AA}'),
    ("ordm", '\u{BA}'),
    ("oslash", '\u{F8}'),
    ("otilde", '\u{F5}'),
    ("ouml", '\u{F6}'),
    ("para", '\u{B6}'),
    ("permil", '\u{2030}'),
    ("pi", '\u{3C0}'),
    ("plusmn", '\u{B1}'),
    ("pound", '\u{A3}'),
    ("prime", '\u{2032}'),
    ("quot", '"'),
    ("raquo", '\u{BB}'),
    ("rarr", '\u{2192}'),
    ("rdquo", '\u{201D}'),
    ("reg", '\u{AE}'),
    ("rlm", '\u{200F}'),
    ("rsaquo", '\u{203A}'),
    ("rsquo", '\u{2019}'),
    ("sect", '\u{A7}'),
    ("shy", '\u{AD}'),
    ("sup1", '\u{B9}'),
    ("sup2", '\u{B2}'),
    ("sup3", '\u{B3}'),
    ("szlig", '\u{DF}'),
    ("thinsp", '\u{2009}'),
    ("thorn", '\u{FE}'),
    ("tilde", '\u{2DC}'),
    ("times", '\u{D7}'),
    ("trade", '\u{2122}'),
    ("uacute", '\u{FA}'),
    ("uarr", '\u{2191}'),
    ("ucirc", '\u{FB}'),
    ("ugrave", '\u{F9}'),
    ("uml", '\u{A8}'),
    ("uuml", '\u{FC}'),
    ("yacute", '\u{FD}'),
    ("yen", '\u{A5}'),
    ("yuml", '\u{FF}'),
    ("zwj", '\u{200D}'),
    ("zwnj", '\u{200C}'),
];

/// Look up a named character reference (the name without `&` and `;`).
fn named_entity(name: &str) -> Option<char> {
    NAMED_ENTITIES
        .binary_search_by_key(&name, |&(entity, _)| entity)
        .ok()
        .map(|index| NAMED_ENTITIES[index].1)
}

// -------- Encoding --------

fn encode(opts: &HtmlOptions) -> Result<String, HtmlError> {
    let mut encoded = String::with_capacity(opts.input.len());
    for ch in opts.input.chars() {
        match ch {
            '&' => encoded.push_str("&amp;"),
            '<' => encoded.push_str("&lt;"),
            '>' => encoded.push_str("&gt;"),
            '"' if opts.mode == HtmlMode::Attribute => encoded.push_str("&quot;"),
            '\'' if opts.mode == HtmlMode::Attribute => encoded.push_str("&#39;"),
            _ => encoded.push(ch),
        }
    }
    Ok(encoded)
}

// -------- Decoding --------

fn decode(input: &str) -> Result<String, HtmlError> {
    let mut decoded = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        let end = rest[start + 1..]
            .find(';')
            .map(|offset| start + 1 + offset)
            .ok_or(HtmlError::InvalidEntity)?;
        let body = &rest[start + 1..end];
        let replacement = if let Some(digits) = body.strip_prefix('#') {
            match digits.strip_prefix(['x', 'X']) {
                Some(hex) => decode_numeric(hex, 16)?,
                None => decode_numeric(digits, 10)?,
            }
        } else {
            named_entity(body).ok_or(HtmlError::UnknownEntity)?.to_string()
        };
        decoded.push_str(&replacement);
        rest = &rest[end + 1..];
    }
    decoded.push_str(rest);
    Ok(decoded)
}

/// Decode a numeric character reference body (the digits after `&#`).
fn decode_numeric(digits: &str, radix: u32) -> Result<String, HtmlError> {
    let value = u32::from_str_radix(digits, radix).map_err(|_| HtmlError::InvalidEntity)?;
    char::from_u32(value)
        .map(|ch| ch.to_string())
        .ok_or(HtmlError::InvalidCodepoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(input: &str) -> HtmlOptions {
        HtmlOptions {
            input: input.to_string(),
            ..HtmlOptions::default()
        }
    }

    fn encode(input: &str) -> String {
        convert_html(HtmlKind::Encode(options(input))).unwrap()
    }

    fn decode(input: &str) -> Result<String, HtmlError> {
        convert_html(HtmlKind::Decode(input.to_string()))
    }

    #[test]
    fn encode_attribute_known_vectors() {
        assert_eq!(encode("<b>hi & bye</b>"), "&lt;b&gt;hi &amp; bye&lt;/b&gt;");
        assert_eq!(encode("a\"b'c"), "a&quot;b&#39;c");
        assert_eq!(encode(""), "");
    }

    #[test]
    fn encode_text_mode_leaves_quotes() {
        let text = convert_html(HtmlKind::Encode(HtmlOptions {
            input: "a\"b'c & <d>".to_string(),
            mode: HtmlMode::Text,
        }))
        .unwrap();
        assert_eq!(text, "a\"b'c &amp; &lt;d&gt;");
    }

    #[test]
    fn encode_passes_non_ascii_through() {
        assert_eq!(encode("café 🚀 你好"), "café 🚀 你好");
    }

    #[test]
    fn decode_named_known_vectors() {
        assert_eq!(decode("&lt;b&gt;hi &amp; bye&lt;/b&gt;").unwrap(), "<b>hi & bye</b>");
        assert_eq!(decode("&apos;").unwrap(), "'");
        assert_eq!(decode("").unwrap(), "");
    }

    #[test]
    fn decode_numeric_decimal_and_hex() {
        assert_eq!(decode("&#65;&#x42;&#X43;").unwrap(), "ABC");
        assert_eq!(decode("&#233;").unwrap(), "é");
        assert_eq!(decode("&#x1F680;").unwrap(), "🚀");
    }

    #[test]
    fn decode_curated_spot_checks() {
        assert_eq!(decode("&nbsp; &euro; &hellip; &ndash; &lt;").unwrap(), "\u{a0} € … – <");
        assert_eq!(decode("&zwj;&zwnj;").unwrap(), "\u{200D}\u{200C}");
    }

    #[test]
    fn decode_unknown_entity_errors() {
        for input in ["&bogus;", "&LT;", "&amp&copy;"] {
            assert_eq!(decode(input), Err(HtmlError::UnknownEntity), "input {input:?}");
        }
    }

    #[test]
    fn decode_malformed_reference_errors() {
        for input in ["hello &", "&#;", "&#x;", "&#123", "&#xZZ;", "&#x1F680", "&amp"] {
            assert_eq!(decode(input), Err(HtmlError::InvalidEntity), "input {input:?}");
        }
    }

    #[test]
    fn decode_invalid_codepoint_errors() {
        for input in ["&#x110000;", "&#xD800;", "&#55296;"] {
            assert_eq!(decode(input), Err(HtmlError::InvalidCodepoint), "input {input:?}");
        }
    }

    #[test]
    fn roundtrip_random_inputs() {
        // Deterministic LCG so failures reproduce exactly.
        let mut state = 0x1234_5678_9ABC_DEF0_u64;
        for _ in 0..1_000 {
            let length = (state % 40) as usize;
            let mut input = String::with_capacity(length);
            for _ in 0..length {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let value = (state % 0x2FFFF) as u32 + 0x20;
                if let Some(ch) = char::from_u32(value) {
                    input.push(ch);
                }
            }
            let encoded = encode(&input);
            assert_eq!(decode(&encoded).unwrap(), input, "roundtrip {input:?}");
        }
    }

    #[test]
    fn entity_table_is_sorted_and_unique() {
        for pair in NAMED_ENTITIES.windows(2) {
            assert!(pair[0].0 < pair[1].0, "table out of order at {:?}", pair);
        }
    }

    #[test]
    fn options_defaults() {
        let defaults = HtmlOptions::default();
        assert_eq!(defaults.input, "");
        assert_eq!(defaults.mode, HtmlMode::Attribute);
    }

    #[test]
    fn dispatch_through_kind() {
        let encoded = convert_html(HtmlKind::Encode(options("<a href=\"x\">y</a>"))).unwrap();
        assert_eq!(encoded, "&lt;a href=&quot;x&quot;&gt;y&lt;/a&gt;");
        assert_eq!(decode(&encoded).unwrap(), "<a href=\"x\">y</a>");
    }
}

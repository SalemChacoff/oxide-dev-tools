use std::fmt;

/// Errors that can occur when encoding or decoding URL components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    /// The input contains an invalid `%XX` escape sequence.
    InvalidInput,
    /// The decoded bytes are not valid UTF-8.
    InvalidUtf8,
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlError::InvalidInput => write!(f, "input is not valid percent-encoded text"),
            UrlError::InvalidUtf8 => write!(f, "decoded data is not valid UTF-8"),
        }
    }
}

impl std::error::Error for UrlError {}

/// Encoding scheme used for a URL encode or decode operation.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlMode {
    /// RFC 3986 percent-encoding: unreserved characters stay literal, space becomes `%20`.
    #[default]
    Standard,
    /// `application/x-www-form-urlencoded`: space becomes `+` (and `+` itself is escaped).
    Form,
}

/// Options for a URL encode or decode operation.
#[derive(Debug, Clone)]
pub struct UrlOptions {
    /// Text to encode, or percent-encoded text to decode.
    pub input: String,
    /// Encoding scheme to use.
    pub mode: UrlMode,
}

impl Default for UrlOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            mode: UrlMode::Standard,
        }
    }
}

/// Operations available on URL components.
#[derive(Debug)]
pub enum UrlKind {
    Encode(UrlOptions),
    Decode(UrlOptions),
}

/// Encode or decode a URL component according to `kind`.
pub fn convert_url(kind: UrlKind) -> Result<String, UrlError> {
    match kind {
        UrlKind::Encode(opts) => encode(&opts),
        UrlKind::Decode(opts) => decode(&opts),
    }
}

const HEX_UPPER: [u8; 16] = *b"0123456789ABCDEF";

/// RFC 3986 §2.3: unreserved characters may appear literally in a URL component.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// Hex value of an ASCII hex digit, case-insensitive.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn push_percent_encoded(out: &mut String, byte: u8) {
    out.push('%');
    out.push(HEX_UPPER[(byte >> 4) as usize] as char);
    out.push(HEX_UPPER[(byte & 0x0F) as usize] as char);
}

// -------- Encoding --------

fn encode(opts: &UrlOptions) -> Result<String, UrlError> {
    let mut encoded = String::with_capacity(opts.input.len());
    for &byte in opts.input.as_bytes() {
        if byte == b' ' && opts.mode == UrlMode::Form {
            encoded.push('+');
        } else if is_unreserved(byte) {
            encoded.push(byte as char);
        } else {
            push_percent_encoded(&mut encoded, byte);
        }
    }
    Ok(encoded)
}

// -------- Decoding --------

fn decode(opts: &UrlOptions) -> Result<String, UrlError> {
    let input = opts.input.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        match input[index] {
            b'%' => {
                let high = input.get(index + 1).copied().and_then(hex_value);
                let low = input.get(index + 2).copied().and_then(hex_value);
                let (high, low) = match (high, low) {
                    (Some(high), Some(low)) => (high, low),
                    _ => return Err(UrlError::InvalidInput),
                };
                bytes.push((high << 4) | low);
                index += 3;
            }
            b'+' if opts.mode == UrlMode::Form => {
                bytes.push(b' ');
                index += 1;
            }
            byte => {
                // Non-ASCII bytes pass through raw; UTF-8 validity is checked below.
                bytes.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(bytes).map_err(|_| UrlError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn std_opts(input: &str) -> UrlOptions {
        UrlOptions {
            input: input.to_string(),
            mode: UrlMode::Standard,
        }
    }

    fn form_opts(input: &str) -> UrlOptions {
        UrlOptions {
            input: input.to_string(),
            mode: UrlMode::Form,
        }
    }

    #[test]
    fn encode_standard_known_vectors() {
        for (input, expected) in [
            ("", ""),
            ("hello", "hello"),
            ("hello world", "hello%20world"),
            ("unreserved-._~", "unreserved-._~"),
            ("reserved!*'();:@&=+$,/?#[]", "reserved%21%2A%27%28%29%3B%3A%40%26%3D%2B%24%2C%2F%3F%23%5B%5D"),
        ] {
            assert_eq!(encode(&std_opts(input)).unwrap(), expected, "encoding {input:?}");
        }
    }

    #[test]
    fn encode_standard_escapes_every_non_unreserved_ascii_byte() {
        // Every ASCII byte outside A-Z a-z 0-9 - . _ ~ must be escaped.
        for byte in 0u8..=127 {
            let input = String::from_utf8(vec![byte]).unwrap();
            let encoded = encode(&std_opts(&input)).unwrap();
            if is_unreserved(byte) {
                assert_eq!(encoded, input, "byte {byte:#04x} should stay literal");
            } else {
                assert_eq!(encoded, format!("%{byte:02X}"), "byte {byte:#04x} should be escaped");
            }
        }
    }

    #[test]
    fn encode_form_uses_plus_for_space() {
        assert_eq!(encode(&form_opts("hello world")).unwrap(), "hello+world");
        assert_eq!(encode(&form_opts("a b c")).unwrap(), "a+b+c");
        // `+` itself must still be escaped in form mode.
        assert_eq!(encode(&form_opts("a+b")).unwrap(), "a%2Bb");
    }

    #[test]
    fn encode_multi_byte_input() {
        // "ÿ" is 0xC3 0xBF and "🚀" is 0xF0 0x9F 0x9A 0x80 in UTF-8.
        assert_eq!(encode(&std_opts("ÿ")).unwrap(), "%C3%BF");
        assert_eq!(encode(&std_opts("🚀")).unwrap(), "%F0%9F%9A%80");
    }

    #[test]
    fn decode_standard_known_vectors() {
        for (input, expected) in [
            ("", ""),
            ("hello", "hello"),
            ("hello%20world", "hello world"),
            ("%C3%BF", "ÿ"),
            ("%F0%9F%9A%80", "🚀"),
        ] {
            assert_eq!(decode(&std_opts(input)).unwrap(), expected, "decoding {input:?}");
        }
    }

    #[test]
    fn decode_is_hex_case_insensitive() {
        assert_eq!(decode(&std_opts("hello%2fworld")).unwrap(), "hello/world");
        assert_eq!(decode(&std_opts("hello%2Fworld")).unwrap(), "hello/world");
    }

    #[test]
    fn decode_standard_keeps_plus_literal() {
        assert_eq!(decode(&std_opts("a+b")).unwrap(), "a+b");
        assert_eq!(decode(&std_opts("a%2Bb")).unwrap(), "a+b");
    }

    #[test]
    fn decode_form_turns_plus_into_space() {
        assert_eq!(decode(&form_opts("hello+world")).unwrap(), "hello world");
        assert_eq!(decode(&form_opts("a%2Bb")).unwrap(), "a+b");
    }

    #[test]
    fn decode_passes_raw_utf8_through() {
        assert_eq!(decode(&std_opts("café")).unwrap(), "café");
    }

    #[test]
    fn decode_invalid_escape_errors() {
        assert_eq!(decode(&std_opts("hello%2")), Err(UrlError::InvalidInput));
        assert_eq!(decode(&std_opts("hello%")), Err(UrlError::InvalidInput));
        assert_eq!(decode(&std_opts("hello%GG")), Err(UrlError::InvalidInput));
        assert_eq!(decode(&std_opts("100% sure")), Err(UrlError::InvalidInput));
    }

    #[test]
    fn decode_invalid_utf8_errors() {
        // 0xFF 0xFE is not valid UTF-8.
        assert_eq!(decode(&std_opts("%FF%FE")), Err(UrlError::InvalidUtf8));
    }

    #[test]
    fn roundtrip_unicode() {
        let input = "héllo wörld 🚀 你好";
        for mode in [UrlMode::Standard, UrlMode::Form] {
            let opts = UrlOptions {
                input: input.to_string(),
                mode,
            };
            let encoded = encode(&opts).unwrap();
            let decoded = decode(&UrlOptions {
                input: encoded.clone(),
                mode,
            })
            .unwrap();
            assert_eq!(decoded, input, "roundtrip in {mode:?} mode");
        }
    }

    #[test]
    fn options_defaults() {
        let opts = UrlOptions::default();
        assert_eq!(opts.input, "");
        assert_eq!(opts.mode, UrlMode::Standard);
    }

    #[test]
    fn dispatch_through_kind() {
        let encoded = convert_url(UrlKind::Encode(std_opts("hello world"))).unwrap();
        assert_eq!(encoded, "hello%20world");
        let decoded = convert_url(UrlKind::Decode(std_opts(&encoded))).unwrap();
        assert_eq!(decoded, "hello world");
    }
}

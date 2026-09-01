use std::fmt;

/// Errors that can occur when encoding or decoding hex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HexError {
    /// The input is not valid hex.
    InvalidInput,
    /// The decoded bytes are not valid UTF-8.
    InvalidUtf8,
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HexError::InvalidInput => write!(f, "input is not valid hex"),
            HexError::InvalidUtf8 => write!(f, "decoded data is not valid UTF-8"),
        }
    }
}

impl std::error::Error for HexError {}

/// Letter case used when encoding hex output.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexCase {
    /// Lowercase hex digits (`0-9`, `a-f`).
    #[default]
    Lower,
    /// Uppercase hex digits (`0-9`, `A-F`).
    Upper,
}

/// Options for a hex encode or decode operation.
#[derive(Debug, Clone)]
pub struct HexOptions {
    /// Text to encode, or hex text to decode.
    pub input: String,
    /// Letter case for encoded output. Decoding is always case-insensitive.
    pub case: HexCase,
}

impl Default for HexOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            case: HexCase::Lower,
        }
    }
}

/// Operations available on hex data.
#[derive(Debug)]
pub enum HexKind {
    Encode(HexOptions),
    Decode(HexOptions),
}

/// Encode or decode hex data according to `kind`.
pub fn convert_hex(kind: HexKind) -> Result<String, HexError> {
    match kind {
        HexKind::Encode(opts) => encode(&opts),
        HexKind::Decode(opts) => decode(&opts),
    }
}

// -------- Encoding --------

fn encode(opts: &HexOptions) -> Result<String, HexError> {
    let encoded = match opts.case {
        HexCase::Lower => hex::encode(opts.input.as_bytes()),
        HexCase::Upper => hex::encode_upper(opts.input.as_bytes()),
    };
    Ok(encoded)
}

// -------- Decoding --------

fn decode(opts: &HexOptions) -> Result<String, HexError> {
    // Pasted or line-wrapped hex often contains whitespace; strip it first.
    // The hex digits themselves are matched case-insensitively.
    let compact: String = opts.input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let decoded = hex::decode(compact.as_str()).map_err(|_| HexError::InvalidInput)?;
    String::from_utf8(decoded).map_err(|_| HexError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lower_opts(input: &str) -> HexOptions {
        HexOptions {
            input: input.to_string(),
            case: HexCase::Lower,
        }
    }

    fn upper_opts(input: &str) -> HexOptions {
        HexOptions {
            input: input.to_string(),
            case: HexCase::Upper,
        }
    }

    #[test]
    fn encode_lower_known_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "66"),
            ("fo", "666f"),
            ("foo", "666f6f"),
            ("hello world", "68656c6c6f20776f726c64"),
        ] {
            assert_eq!(encode(&lower_opts(input)).unwrap(), expected, "encoding {input:?}");
        }
    }

    #[test]
    fn encode_upper_known_vectors() {
        assert_eq!(encode(&upper_opts("hello")).unwrap(), "68656C6C6F");
        assert_eq!(encode(&upper_opts("")).unwrap(), "");
    }

    #[test]
    fn encode_multi_byte_input() {
        // "ÿ" is 0xC3 0xBF in UTF-8.
        assert_eq!(encode(&lower_opts("ÿ")).unwrap(), "c3bf");
        assert_eq!(encode(&upper_opts("ÿ")).unwrap(), "C3BF");
    }

    #[test]
    fn decode_known_vectors() {
        for (input, expected) in [("68656c6c6f", "hello"), ("66", "f"), ("c3bf", "ÿ"), ("", "")] {
            assert_eq!(decode(&lower_opts(input)).unwrap(), expected, "decoding {input:?}");
        }
    }

    #[test]
    fn decode_is_case_insensitive() {
        assert_eq!(decode(&lower_opts("68656C6C6F")).unwrap(), "hello");
        assert_eq!(decode(&lower_opts("68656C6c6F")).unwrap(), "hello");
    }

    #[test]
    fn decode_ignores_whitespace() {
        assert_eq!(decode(&lower_opts("68 65 6c 6c 6f")).unwrap(), "hello");
        assert_eq!(decode(&lower_opts("68656c\n6c6f")).unwrap(), "hello");
        assert_eq!(decode(&lower_opts(" 68656c6c6f \r\n")).unwrap(), "hello");
    }

    #[test]
    fn decode_invalid_input_errors() {
        assert_eq!(decode(&lower_opts("686")), Err(HexError::InvalidInput));
        assert_eq!(decode(&lower_opts("686g")), Err(HexError::InvalidInput));
        assert_eq!(decode(&lower_opts("zz")), Err(HexError::InvalidInput));
        assert_eq!(decode(&lower_opts("0x68656c6c6f")), Err(HexError::InvalidInput));
    }

    #[test]
    fn decode_invalid_utf8_errors() {
        // 0xFF 0xFE is not valid UTF-8.
        assert_eq!(decode(&lower_opts("fffe")), Err(HexError::InvalidUtf8));
    }

    #[test]
    fn roundtrip_unicode() {
        let input = "héllo wörld 🚀 你好";
        let encoded = encode(&lower_opts(input)).unwrap();
        assert_eq!(decode(&lower_opts(&encoded)).unwrap(), input);
        let upper_encoded = encode(&upper_opts(input)).unwrap();
        assert_eq!(decode(&lower_opts(&upper_encoded)).unwrap(), input);
    }

    #[test]
    fn options_defaults() {
        let opts = HexOptions::default();
        assert_eq!(opts.input, "");
        assert_eq!(opts.case, HexCase::Lower);
    }

    #[test]
    fn dispatch_through_kind() {
        let encoded = convert_hex(HexKind::Encode(lower_opts("hello"))).unwrap();
        assert_eq!(encoded, "68656c6c6f");
        let decoded = convert_hex(HexKind::Decode(lower_opts(&encoded))).unwrap();
        assert_eq!(decoded, "hello");
    }
}

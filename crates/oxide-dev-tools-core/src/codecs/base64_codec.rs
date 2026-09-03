use std::fmt;

use base64::{Engine as _, engine::general_purpose};

/// Errors that can occur when encoding or decoding base64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base64Error {
    /// The input is not valid base64 for the selected alphabet.
    InvalidInput,
    /// The decoded bytes are not valid UTF-8.
    InvalidUtf8,
}

impl fmt::Display for Base64Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Base64Error::InvalidInput => write!(f, "input is not valid base64"),
            Base64Error::InvalidUtf8 => write!(f, "decoded data is not valid UTF-8"),
        }
    }
}

impl std::error::Error for Base64Error {}

/// Alphabets used for base64 encoding and decoding.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base64Alphabet {
    /// Standard alphabet (`A–Z`, `a–z`, `0–9`, `+`, `/`) with `=` padding.
    #[default]
    Standard,
    /// URL-safe alphabet (`A–Z`, `a–z`, `0–9`, `-`, `_`) without padding.
    UrlSafe,
}

/// Options for a base64 encode or decode operation.
#[derive(Debug, Clone)]
pub struct Base64Options {
    /// Text to encode, or base64 text to decode.
    pub input: String,
    /// Alphabet to use.
    pub alphabet: Base64Alphabet,
}

impl Default for Base64Options {
    fn default() -> Self {
        Self {
            input: String::new(),
            alphabet: Base64Alphabet::Standard,
        }
    }
}

/// Operations available on base64 data.
#[derive(Debug)]
pub enum Base64Kind {
    Encode(Base64Options),
    Decode(Base64Options),
}

/// Encode or decode base64 data according to `kind`.
pub fn convert_base64(kind: Base64Kind) -> Result<String, Base64Error> {
    match kind {
        Base64Kind::Encode(opts) => encode(&opts),
        Base64Kind::Decode(opts) => decode(&opts),
    }
}

// -------- Encoding --------

fn encode(opts: &Base64Options) -> Result<String, Base64Error> {
    let encoded = match opts.alphabet {
        Base64Alphabet::Standard => general_purpose::STANDARD.encode(opts.input.as_bytes()),
        Base64Alphabet::UrlSafe => general_purpose::URL_SAFE_NO_PAD.encode(opts.input.as_bytes()),
    };
    Ok(encoded)
}

// -------- Decoding --------

fn decode(opts: &Base64Options) -> Result<String, Base64Error> {
    // Pasted or line-wrapped base64 often contains whitespace; strip it first.
    let compact: String = opts.input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let decoded = match opts.alphabet {
        Base64Alphabet::Standard => general_purpose::STANDARD.decode(compact.as_str()),
        // URL-safe blobs (e.g. JWT segments) are usually unpadded; tolerate `=`.
        Base64Alphabet::UrlSafe => general_purpose::URL_SAFE_NO_PAD.decode(compact.trim_end_matches('=')),
    };
    let bytes = decoded.map_err(|_| Base64Error::InvalidInput)?;
    String::from_utf8(bytes).map_err(|_| Base64Error::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn std_opts(input: &str) -> Base64Options {
        Base64Options {
            input: input.to_string(),
            alphabet: Base64Alphabet::Standard,
        }
    }

    fn url_opts(input: &str) -> Base64Options {
        Base64Options {
            input: input.to_string(),
            alphabet: Base64Alphabet::UrlSafe,
        }
    }

    #[test]
    fn encode_standard_known_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(&std_opts(input)).unwrap(), expected, "encoding {input:?}");
        }
    }

    #[test]
    fn encode_url_safe_no_padding() {
        assert_eq!(encode(&url_opts("hello")).unwrap(), "aGVsbG8");
        assert_eq!(encode(&url_opts("foobar")).unwrap(), "Zm9vYmFy");
    }

    #[test]
    fn encode_standard_padding() {
        // Multi-byte input exercises padding ("ÿ" is 2 bytes -> 3 sextets + 1 pad).
        assert_eq!(encode(&std_opts("ÿ")).unwrap(), "w78=");
    }

    #[test]
    fn decode_standard_known_vectors() {
        for (input, expected) in [("aGVsbG8=", "hello"), ("Zg==", "f"), ("w78=", "ÿ"), ("", "")] {
            assert_eq!(decode(&std_opts(input)).unwrap(), expected, "decoding {input:?}");
        }
    }

    #[test]
    fn decode_ignores_whitespace() {
        assert_eq!(decode(&std_opts("aGVs\nbG8=")).unwrap(), "hello");
        assert_eq!(decode(&std_opts(" aGVsbG8= \r\n")).unwrap(), "hello");
    }

    #[test]
    fn decode_url_safe_unpadded() {
        assert_eq!(decode(&url_opts("aGVsbG8")).unwrap(), "hello");
    }

    #[test]
    fn decode_url_safe_tolerates_padding() {
        assert_eq!(decode(&url_opts("aGVsbG8=")).unwrap(), "hello");
    }

    #[test]
    fn decode_invalid_input_errors() {
        assert_eq!(decode(&std_opts("aGVsbG8!")), Err(Base64Error::InvalidInput));
        assert_eq!(decode(&url_opts("+/8=")), Err(Base64Error::InvalidInput));
    }

    #[test]
    fn decode_invalid_utf8_errors() {
        // 0xFF 0xFE is not valid UTF-8.
        assert_eq!(decode(&std_opts("//4=")), Err(Base64Error::InvalidUtf8));
    }

    #[test]
    fn roundtrip_unicode() {
        let input = "héllo wörld 🚀 你好";
        let encoded = encode(&std_opts(input)).unwrap();
        assert_eq!(decode(&std_opts(&encoded)).unwrap(), input);
        let url_encoded = encode(&url_opts(input)).unwrap();
        assert_eq!(decode(&url_opts(&url_encoded)).unwrap(), input);
    }

    #[test]
    fn options_defaults() {
        let opts = Base64Options::default();
        assert_eq!(opts.input, "");
        assert_eq!(opts.alphabet, Base64Alphabet::Standard);
    }

    #[test]
    fn dispatch_through_kind() {
        let encoded = convert_base64(Base64Kind::Encode(std_opts("hello"))).unwrap();
        assert_eq!(encoded, "aGVsbG8=");
        let decoded = convert_base64(Base64Kind::Decode(std_opts(&encoded))).unwrap();
        assert_eq!(decoded, "hello");
    }
}

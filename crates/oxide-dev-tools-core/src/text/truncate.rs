//! Text truncation: cut text to a maximum length with an optional ellipsis.
//!
//! [`truncate_text`] counts Unicode scalar values (characters), never bytes,
//! so no character is ever split mid-code-point. The ellipsis counts toward
//! the maximum length; text that already fits is returned unchanged, without
//! an ellipsis.
//!
//! Limitation: truncation operates on scalar values, so grapheme clusters
//! (combining sequences, emoji ZWJ runs) may be split at scalar boundaries.

use std::fmt;

// -------- Public API --------

/// Errors that can occur when truncating text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TruncateError {
    /// Truncation is needed, but the ellipsis alone already exceeds `max_length`.
    EllipsisTooLong,
}

impl fmt::Display for TruncateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TruncateError::EllipsisTooLong => write!(f, "ellipsis is longer than the maximum length"),
        }
    }
}

impl std::error::Error for TruncateError {}

/// Where the ellipsis sits in the truncated output.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncatePosition {
    /// Keep the head of the text, ellipsis at the end (`hello w…`).
    #[default]
    End,
    /// Keep the tail of the text, ellipsis at the start (`…o world`).
    Start,
    /// Keep both ends, ellipsis in the middle (`hell…orld`).
    Middle,
}

/// Options for [`truncate_text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruncateOptions {
    /// Text to truncate.
    pub input: String,
    /// Maximum output length in Unicode scalar values, ellipsis included.
    pub max_length: usize,
    /// Marker added when truncation occurs; an empty string truncates without one.
    pub ellipsis: String,
    /// Where the ellipsis sits in the truncated output.
    pub position: TruncatePosition,
}

impl Default for TruncateOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            max_length: 0,
            ellipsis: "…".to_string(),
            position: TruncatePosition::End,
        }
    }
}

/// Truncate `options.input` to `options.max_length` characters.
///
/// Lengths count Unicode scalar values, never bytes; the ellipsis counts
/// toward the maximum. Input that already fits is returned unchanged.
pub fn truncate_text(options: TruncateOptions) -> Result<String, TruncateError> {
    let input_len = options.input.chars().count();
    if input_len <= options.max_length {
        return Ok(options.input);
    }
    let ellipsis_len = options.ellipsis.chars().count();
    if ellipsis_len > options.max_length {
        return Err(TruncateError::EllipsisTooLong);
    }
    let keep = options.max_length - ellipsis_len;
    let result = match options.position {
        TruncatePosition::End => truncate_end(&options.input, &options.ellipsis, keep),
        TruncatePosition::Start => truncate_start(&options.input, &options.ellipsis, keep),
        TruncatePosition::Middle => truncate_middle(&options.input, &options.ellipsis, keep),
    };
    Ok(result)
}

// -------- Truncation --------

/// Keep the first `keep` characters followed by the ellipsis.
fn truncate_end(input: &str, ellipsis: &str, keep: usize) -> String {
    let mut result = input.chars().take(keep).collect::<String>();
    result.push_str(ellipsis);
    result
}

/// Keep the last `keep` characters preceded by the ellipsis.
fn truncate_start(input: &str, ellipsis: &str, keep: usize) -> String {
    let mut result = String::from(ellipsis);
    let skip = input.chars().count() - keep;
    result.push_str(&input.chars().skip(skip).collect::<String>());
    result
}

/// Keep the first `keep / 2` and last `keep - keep / 2` characters around the ellipsis.
fn truncate_middle(input: &str, ellipsis: &str, keep: usize) -> String {
    let front = keep / 2;
    let back = keep - front;
    let mut result = input.chars().take(front).collect::<String>();
    result.push_str(ellipsis);
    let skip = input.chars().count() - back;
    result.push_str(&input.chars().skip(skip).collect::<String>());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(input: &str, max_length: usize) -> TruncateOptions {
        TruncateOptions {
            input: input.to_string(),
            max_length,
            ..TruncateOptions::default()
        }
    }

    fn options_with(input: &str, max_length: usize, ellipsis: &str, position: TruncatePosition) -> TruncateOptions {
        TruncateOptions {
            input: input.to_string(),
            max_length,
            ellipsis: ellipsis.to_string(),
            position,
        }
    }

    fn truncate(input: &str, max_length: usize) -> String {
        truncate_text(options(input, max_length)).unwrap()
    }

    #[test]
    fn end_known_vectors() {
        assert_eq!(truncate("hello world", 8), "hello w…");
        assert_eq!(truncate("hello world", 1), "…");
        assert_eq!(truncate("hello world", 11), "hello world");
    }

    #[test]
    fn start_known_vectors() {
        let start =
            |input: &str, max: usize| truncate_text(options_with(input, max, "…", TruncatePosition::Start)).unwrap();
        assert_eq!(start("hello world", 8), "…o world");
        assert_eq!(start("hello world", 1), "…");
    }

    #[test]
    fn middle_known_vectors() {
        let middle =
            |input: &str, max: usize| truncate_text(options_with(input, max, "…", TruncatePosition::Middle)).unwrap();
        assert_eq!(middle("hello world", 9), "hell…orld");
        assert_eq!(middle("hello world", 8), "hel…orld");
        assert_eq!(middle("hello", 4), "h…lo");
    }

    #[test]
    fn input_within_max_is_unchanged() {
        for max in [11, 12, 100] {
            assert_eq!(truncate("hello world", max), "hello world", "max {max}");
        }
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn custom_ellipsis_vectors() {
        let end = options_with("hello world", 5, "...", TruncatePosition::End);
        assert_eq!(truncate_text(end).unwrap(), "he...");
        let start = options_with("hello world", 5, "...", TruncatePosition::Start);
        assert_eq!(truncate_text(start).unwrap(), "...ld");
        let middle = options_with("hello world", 6, "...", TruncatePosition::Middle);
        assert_eq!(truncate_text(middle).unwrap(), "h...ld");
    }

    #[test]
    fn empty_ellipsis_hard_truncates() {
        let end = options_with("hello world", 5, "", TruncatePosition::End);
        assert_eq!(truncate_text(end).unwrap(), "hello");
        let middle = options_with("hello world", 5, "", TruncatePosition::Middle);
        assert_eq!(truncate_text(middle).unwrap(), "herld");
        let zero = options_with("hello", 0, "", TruncatePosition::End);
        assert_eq!(truncate_text(zero).unwrap(), "");
    }

    #[test]
    fn ellipsis_longer_than_max_errors() {
        let too_long = options_with("hello", 2, "...", TruncatePosition::End);
        assert_eq!(truncate_text(too_long), Err(TruncateError::EllipsisTooLong));
        let zero = options_with("hello", 0, "…", TruncatePosition::End);
        assert_eq!(truncate_text(zero), Err(TruncateError::EllipsisTooLong));
        let err = truncate_text(options_with("hello", 1, "..", TruncatePosition::Start)).unwrap_err();
        assert!(err.to_string().contains("ellipsis is longer"));
    }

    #[test]
    fn ellipsis_longer_than_max_when_input_fits_is_ignored() {
        // The ellipsis exceeds max_length, but no truncation is needed.
        let fits = options_with("hi", 5, "......", TruncatePosition::End);
        assert_eq!(truncate_text(fits).unwrap(), "hi");
    }

    #[test]
    fn unicode_counts_characters_not_bytes() {
        assert_eq!(truncate("héllo wörld", 8), "héllo w…");
        assert_eq!(truncate("你好世界", 3), "你好…");
    }

    #[test]
    fn combining_sequence_splits_at_scalar_boundary() {
        // "é" written as `e` + combining accent is one grapheme but two
        // scalar values; truncation keeps the first scalar value.
        assert_eq!(truncate("e\u{301}x", 2), "e…");
    }

    #[test]
    fn options_defaults() {
        let defaults = TruncateOptions::default();
        assert_eq!(defaults.input, "");
        assert_eq!(defaults.max_length, 0);
        assert_eq!(defaults.ellipsis, "…");
        assert_eq!(defaults.position, TruncatePosition::End);
    }

    #[test]
    fn entry_returns_ok_for_identity_defaults() {
        assert_eq!(truncate_text(TruncateOptions::default()).unwrap(), "");
    }
}

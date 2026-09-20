//! Text statistics: character, byte, word, line, and paragraph counts.
//!
//! [`count_text_stats`] counts the Unicode scalar values (characters) and
//! UTF-8 bytes of a text, splits it into whitespace-separated words and
//! newline-separated lines, and groups non-blank lines into paragraphs.
//! Counting is total: every input, including the empty string, yields a
//! report.

// -------- Public API --------

/// Options for [`count_text_stats`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextStatsOptions {
    /// The text to count.
    pub input: String,
}

/// Statistics collected from a block of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStatsReport {
    /// Number of Unicode scalar values in the text.
    pub characters: usize,
    /// Size of the text in UTF-8 bytes.
    pub bytes: usize,
    /// Number of whitespace-separated words.
    pub words: usize,
    /// Number of lines; a trailing line break does not start a new line.
    pub lines: usize,
    /// Number of paragraphs (runs of non-blank lines separated by blank lines).
    pub paragraphs: usize,
}

/// Count characters, bytes, words, lines, and paragraphs in `options.input`.
pub fn count_text_stats(options: TextStatsOptions) -> TextStatsReport {
    TextStatsReport {
        characters: options.input.chars().count(),
        bytes: options.input.len(),
        words: options.input.split_whitespace().count(),
        lines: options.input.lines().count(),
        paragraphs: count_paragraphs(&options.input),
    }
}

// -------- Paragraph counting --------

/// Count runs of non-blank lines separated by one or more blank lines.
///
/// A "blank" line contains only whitespace; `\n` and `\r\n` line breaks are
/// handled by [`str::lines`].
fn count_paragraphs(text: &str) -> usize {
    let mut paragraphs = 0;
    let mut in_paragraph = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            in_paragraph = false;
        } else if !in_paragraph {
            paragraphs += 1;
            in_paragraph = true;
        }
    }
    paragraphs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(input: &str) -> TextStatsReport {
        count_text_stats(TextStatsOptions {
            input: input.to_string(),
        })
    }

    fn report(characters: usize, bytes: usize, words: usize, lines: usize, paragraphs: usize) -> TextStatsReport {
        TextStatsReport {
            characters,
            bytes,
            words,
            lines,
            paragraphs,
        }
    }

    #[test]
    fn empty_input_counts_zero() {
        assert_eq!(count(""), report(0, 0, 0, 0, 0));
    }

    #[test]
    fn known_ascii_vector() {
        assert_eq!(count("hello world"), report(11, 11, 2, 1, 1));
    }

    #[test]
    fn multiline_text_counts_lines() {
        assert_eq!(count("one\ntwo\nthree"), report(13, 13, 3, 3, 1));
    }

    #[test]
    fn blank_lines_separate_paragraphs() {
        assert_eq!(count("first\n\nsecond"), report(13, 13, 2, 3, 2));
        assert_eq!(count("first\n\n\nsecond").paragraphs, 2);
        assert_eq!(count("first\n \nsecond").paragraphs, 2);
    }

    #[test]
    fn trailing_newline_adds_no_line() {
        assert_eq!(count("hello\n").lines, 1);
        assert_eq!(count("hello\nworld\n").lines, 2);
    }

    #[test]
    fn crlf_line_endings_group_paragraphs() {
        assert_eq!(count("first\r\n\r\nsecond").lines, 3);
        assert_eq!(count("first\r\n\r\nsecond").paragraphs, 2);
    }

    #[test]
    fn unicode_characters_and_bytes_differ() {
        assert_eq!(count("héllo wörld"), report(11, 13, 2, 1, 1));
        assert_eq!(count("你好"), report(2, 6, 1, 1, 1));
    }

    #[test]
    fn runs_of_whitespace_form_one_separator() {
        assert_eq!(count("hello   world").words, 2);
        assert_eq!(count("a\tb\nc").words, 3);
        assert_eq!(count("   ").words, 0);
        assert_eq!(count("\n").lines, 1);
    }

    #[test]
    fn whitespace_only_lines_are_not_paragraphs() {
        assert_eq!(count(" \n \n ").paragraphs, 0);
        assert_eq!(count("\n\n").paragraphs, 0);
        assert_eq!(count("\n\ntext\n\n").paragraphs, 1);
    }

    #[test]
    fn options_defaults_to_empty_input() {
        assert!(TextStatsOptions::default().input.is_empty());
    }
}

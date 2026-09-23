//! Character scanning: detect whitespace, hidden, and non-ASCII characters
//! in plain text.
//!
//! [`scan_characters`] treats any input as plain text — JSON, XML, SQL,
//! source code, or raw prose — and reports every character that is not an
//! ordinary visible character. Each finding carries its byte offset,
//! character index, 1-based line and column, code point, and classification.
//!
//! Findings are grouped in three overlapping sets:
//! - *whitespace*: space, tab, newline, and the other Unicode White_Space
//!   characters (NBSP, em spaces, ideographic space, line/paragraph
//!   separators, ...).
//! - *hidden*: zero-width characters, bidi controls, C0/C1 controls, format
//!   characters (soft hyphen, BOM), private-use code points, noncharacters,
//!   and combining marks.
//! - *non-ASCII*: any character above U+007F, visible or not.
//!
//! Counts in the report always reflect the whole input; the selection flags
//! in [`TextScanOptions`] only decide which findings appear. Without any
//! explicit selection, the whitespace and hidden groups are reported.
//!
//! Limitations: lone surrogates cannot occur in a valid [`String`];
//! unassigned code points are not detected (they would need Unicode data
//! tables); combining-mark detection covers the common blocks, not the full
//! Unicode repertoire. A CRLF pair is reported as two findings, with the LF
//! sitting at column 1 of the following line.

// -------- Public API --------

/// How a scanned character is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    /// U+0020, the regular space.
    Space,
    /// U+0009, the horizontal tab.
    Tab,
    /// U+000A line feed or U+000D carriage return.
    Newline,
    /// Vertical tab (U+000B) or form feed (U+000C).
    AsciiWhitespace,
    /// White_Space characters outside ASCII (NBSP, em space, U+3000, ...).
    UnicodeWhitespace,
    /// C0/C1 control characters that are not whitespace (NUL, ESC, DEL, ...).
    Control,
    /// Zero-width characters (ZWSP, ZWNJ, ZWJ, word joiner, ...).
    ZeroWidth,
    /// Bidirectional control characters (LRM, RLM, overrides, isolates).
    BidiControl,
    /// Format characters (soft hyphen, BOM, ...).
    Format,
    /// Private-use-area code points.
    PrivateUse,
    /// Unicode noncharacters (U+FDD0–U+FDEF, U+xFFFE/U+xFFFF per plane).
    Noncharacter,
    /// Combining marks in the common Unicode blocks (U+0300–U+036F, ...).
    CombiningMark,
    /// An ordinary visible character.
    Visible,
}

impl CharClass {
    /// True when the class belongs to the whitespace group.
    pub fn is_whitespace(self) -> bool {
        matches!(
            self,
            CharClass::Space
                | CharClass::Tab
                | CharClass::Newline
                | CharClass::AsciiWhitespace
                | CharClass::UnicodeWhitespace
        )
    }

    /// True when the class belongs to the hidden-character group.
    pub fn is_hidden(self) -> bool {
        matches!(
            self,
            CharClass::Control
                | CharClass::ZeroWidth
                | CharClass::BidiControl
                | CharClass::Format
                | CharClass::PrivateUse
                | CharClass::Noncharacter
                | CharClass::CombiningMark
        )
    }

    /// Short human-readable name of the class, used in reports.
    pub fn name(self) -> &'static str {
        match self {
            CharClass::Space => "whitespace (space)",
            CharClass::Tab => "whitespace (tab)",
            CharClass::Newline => "whitespace (newline)",
            CharClass::AsciiWhitespace => "whitespace (ascii)",
            CharClass::UnicodeWhitespace => "whitespace (unicode)",
            CharClass::Control => "control",
            CharClass::ZeroWidth => "zero width",
            CharClass::BidiControl => "bidi control",
            CharClass::Format => "format",
            CharClass::PrivateUse => "private use",
            CharClass::Noncharacter => "noncharacter",
            CharClass::CombiningMark => "combining mark",
            CharClass::Visible => "visible",
        }
    }
}

/// Options for [`scan_characters`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextScanOptions {
    /// The text to scan.
    pub input: String,
    /// Report findings from the whitespace group.
    pub include_whitespace: bool,
    /// Report findings from the hidden-character group.
    pub include_hidden: bool,
    /// Report findings for non-ASCII characters (visible or not).
    pub include_non_ascii: bool,
}

/// One suspicious character found by the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterFinding {
    /// 0-based byte offset of the character in the UTF-8 input.
    pub byte_offset: usize,
    /// 0-based index of the character in Unicode scalar values.
    pub char_index: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column in Unicode scalar values.
    pub column: usize,
    /// The character itself.
    pub character: char,
    /// How the character is classified.
    pub class: CharClass,
}

/// Result of a character scan.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextScanReport {
    /// Findings selected by the options (see [`TextScanOptions`]).
    pub findings: Vec<CharacterFinding>,
    /// Characters in the whitespace group, regardless of selection.
    pub whitespace_count: usize,
    /// Characters in the hidden group, regardless of selection.
    pub hidden_count: usize,
    /// Characters above U+007F, regardless of selection.
    pub non_ascii_count: usize,
    /// Ordinary visible characters, regardless of selection.
    pub visible_count: usize,
    /// Total Unicode scalar values in the input.
    pub total_characters: usize,
}

/// Scan `options.input` for whitespace, hidden, and non-ASCII characters.
///
/// Counts always reflect the whole input; the selection flags only decide
/// which findings appear. With every flag off, the whitespace and hidden
/// groups are reported (the default).
pub fn scan_characters(options: TextScanOptions) -> TextScanReport {
    let selection = Selection::from_options(&options);
    let mut report = TextScanReport::default();
    let mut line = 1;
    let mut column = 1;
    let mut previous_was_cr = false;

    for (char_index, (byte_offset, character)) in options.input.char_indices().enumerate() {
        report.total_characters += 1;
        let class = classify_char(character);
        if class.is_whitespace() {
            report.whitespace_count += 1;
        }
        if class.is_hidden() {
            report.hidden_count += 1;
        }
        if !character.is_ascii() {
            report.non_ascii_count += 1;
        }
        if class == CharClass::Visible {
            report.visible_count += 1;
        }
        if selection.matches(class, character) {
            report.findings.push(CharacterFinding {
                byte_offset,
                char_index,
                line,
                column,
                character,
                class,
            });
        }
        match character {
            '\r' => {
                line += 1;
                column = 1;
                previous_was_cr = true;
            }
            '\n' => {
                if !previous_was_cr {
                    line += 1;
                }
                column = 1;
                previous_was_cr = false;
            }
            '\u{0085}' | '\u{2028}' | '\u{2029}' => {
                line += 1;
                column = 1;
                previous_was_cr = false;
            }
            _ => {
                column += 1;
                previous_was_cr = false;
            }
        }
    }
    report
}

/// Unicode name of a known suspicious character, or an empty string.
///
/// Covers whitespace, zero-width, bidi, and format characters that scanners
/// commonly look for. Unknown characters yield an empty string so callers
/// can fall back to a class description or an escape sequence.
pub fn character_label(character: char) -> &'static str {
    match character {
        ' ' => "SPACE",
        '\t' => "TAB",
        '\n' => "LINE FEED",
        '\r' => "CARRIAGE RETURN",
        '\u{000B}' => "LINE TABULATION",
        '\u{000C}' => "FORM FEED",
        '\u{0085}' => "NEXT LINE",
        '\u{00A0}' => "NO-BREAK SPACE",
        '\u{00AD}' => "SOFT HYPHEN",
        '\u{1680}' => "OGHAM SPACE MARK",
        '\u{180E}' => "MONGOLIAN VOWEL SEPARATOR",
        '\u{2000}' => "EN QUAD",
        '\u{2001}' => "EM QUAD",
        '\u{2002}' => "EN SPACE",
        '\u{2003}' => "EM SPACE",
        '\u{2004}' => "THREE-PER-EM SPACE",
        '\u{2005}' => "FOUR-PER-EM SPACE",
        '\u{2006}' => "SIX-PER-EM SPACE",
        '\u{2007}' => "FIGURE SPACE",
        '\u{2008}' => "PUNCTUATION SPACE",
        '\u{2009}' => "THIN SPACE",
        '\u{200A}' => "HAIR SPACE",
        '\u{200B}' => "ZERO WIDTH SPACE",
        '\u{200C}' => "ZERO WIDTH NON-JOINER",
        '\u{200D}' => "ZERO WIDTH JOINER",
        '\u{200E}' => "LEFT-TO-RIGHT MARK",
        '\u{200F}' => "RIGHT-TO-LEFT MARK",
        '\u{2028}' => "LINE SEPARATOR",
        '\u{2029}' => "PARAGRAPH SEPARATOR",
        '\u{202A}' => "LEFT-TO-RIGHT EMBEDDING",
        '\u{202B}' => "RIGHT-TO-LEFT EMBEDDING",
        '\u{202C}' => "POP DIRECTIONAL FORMATTING",
        '\u{202D}' => "LEFT-TO-RIGHT OVERRIDE",
        '\u{202E}' => "RIGHT-TO-LEFT OVERRIDE",
        '\u{202F}' => "NARROW NO-BREAK SPACE",
        '\u{205F}' => "MEDIUM MATHEMATICAL SPACE",
        '\u{2060}' => "WORD JOINER",
        '\u{2066}' => "LEFT-TO-RIGHT ISOLATE",
        '\u{2067}' => "RIGHT-TO-LEFT ISOLATE",
        '\u{2068}' => "FIRST STRONG ISOLATE",
        '\u{2069}' => "POP DIRECTIONAL ISOLATE",
        '\u{3000}' => "IDEOGRAPHIC SPACE",
        '\u{FEFF}' => "ZERO WIDTH NO-BREAK SPACE (BOM)",
        _ => "",
    }
}

// -------- Classification --------

fn classify_char(character: char) -> CharClass {
    match character {
        ' ' => CharClass::Space,
        '\t' => CharClass::Tab,
        '\n' | '\r' => CharClass::Newline,
        '\u{000B}' | '\u{000C}' => CharClass::AsciiWhitespace,
        c if c.is_whitespace() => CharClass::UnicodeWhitespace,
        c if is_zero_width(c) => CharClass::ZeroWidth,
        c if is_bidi_control(c) => CharClass::BidiControl,
        c if is_format(c) => CharClass::Format,
        c if c.is_control() => CharClass::Control,
        c if is_private_use(c) => CharClass::PrivateUse,
        c if is_noncharacter(c) => CharClass::Noncharacter,
        c if is_combining_mark(c) => CharClass::CombiningMark,
        _ => CharClass::Visible,
    }
}

fn is_zero_width(character: char) -> bool {
    matches!(character, '\u{180E}' | '\u{200B}'..='\u{200D}' | '\u{2060}')
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

fn is_format(character: char) -> bool {
    matches!(
        character,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{08E2}'
            | '\u{2061}'..='\u{2064}'
            | '\u{206A}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

fn is_private_use(character: char) -> bool {
    matches!(
        character,
        '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}'
    )
}

fn is_noncharacter(character: char) -> bool {
    let value = character as u32;
    (0xFDD0..=0xFDEF).contains(&value) || value & 0xFFFE == 0xFFFE
}

fn is_combining_mark(character: char) -> bool {
    matches!(
        character,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
            | '\u{1D165}'..='\u{1D169}'
            | '\u{1D16D}'..='\u{1D172}'
            | '\u{1D17B}'..='\u{1D182}'
            | '\u{1D185}'..='\u{1D18B}'
            | '\u{1D1AA}'..='\u{1D1AD}'
    )
}

// -------- Selection --------

/// Which groups a scan reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Selection {
    whitespace: bool,
    hidden: bool,
    non_ascii: bool,
}

impl Selection {
    /// Every flag off selects the default groups: whitespace and hidden.
    fn from_options(options: &TextScanOptions) -> Self {
        if options.include_whitespace || options.include_hidden || options.include_non_ascii {
            Selection {
                whitespace: options.include_whitespace,
                hidden: options.include_hidden,
                non_ascii: options.include_non_ascii,
            }
        } else {
            Selection {
                whitespace: true,
                hidden: true,
                non_ascii: false,
            }
        }
    }

    fn matches(self, class: CharClass, character: char) -> bool {
        (self.whitespace && class.is_whitespace())
            || (self.hidden && class.is_hidden())
            || (self.non_ascii && !character.is_ascii())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(input: &str) -> TextScanReport {
        scan_characters(TextScanOptions {
            input: input.to_string(),
            ..TextScanOptions::default()
        })
    }

    fn with_selection(input: &str, whitespace: bool, hidden: bool, non_ascii: bool) -> TextScanReport {
        scan_characters(TextScanOptions {
            input: input.to_string(),
            include_whitespace: whitespace,
            include_hidden: hidden,
            include_non_ascii: non_ascii,
        })
    }

    fn finding(report: &TextScanReport, index: usize) -> &CharacterFinding {
        &report.findings[index]
    }

    #[test]
    fn empty_input_yields_no_findings() {
        let report = scan("");
        assert!(report.findings.is_empty());
        assert_eq!(report.total_characters, 0);
        assert_eq!(report.whitespace_count, 0);
        assert_eq!(report.hidden_count, 0);
        assert_eq!(report.non_ascii_count, 0);
        assert_eq!(report.visible_count, 0);
    }

    #[test]
    fn plain_text_yields_no_findings() {
        assert!(scan("let_value=42;").findings.is_empty());
        assert!(scan("{\"key\":\"value\"}").findings.is_empty());
        assert!(scan("SELECT*FROM(users);").findings.is_empty());
    }

    #[test]
    fn ascii_space_is_classified_with_position() {
        let report = scan("hello world");
        assert_eq!(report.findings.len(), 1);
        let found = finding(&report, 0);
        assert_eq!(found.character, ' ');
        assert_eq!(found.class, CharClass::Space);
        assert_eq!(found.byte_offset, 5);
        assert_eq!(found.char_index, 5);
        assert_eq!(found.line, 1);
        assert_eq!(found.column, 6);
        assert_eq!(report.whitespace_count, 1);
        assert_eq!(report.visible_count, 10);
        assert_eq!(report.total_characters, 11);
    }

    #[test]
    fn tab_newline_and_other_ascii_whitespace_are_classified() {
        let report = scan("a\tb\u{000B}c\u{000C}d\ne");
        assert_eq!(report.findings.len(), 4);
        assert_eq!(finding(&report, 0).class, CharClass::Tab);
        assert_eq!(finding(&report, 1).class, CharClass::AsciiWhitespace);
        assert_eq!(finding(&report, 2).class, CharClass::AsciiWhitespace);
        assert_eq!(finding(&report, 3).class, CharClass::Newline);
        assert_eq!(report.whitespace_count, 4);
    }

    #[test]
    fn unicode_whitespace_is_classified() {
        let report = scan("a\u{00A0}b\u{2003}c\u{3000}d\u{202F}e\u{0085}f");
        assert_eq!(report.findings.len(), 5);
        for found in &report.findings {
            assert_eq!(found.class, CharClass::UnicodeWhitespace);
        }
        assert_eq!(report.whitespace_count, 5);
        assert_eq!(report.non_ascii_count, 5);
    }

    #[test]
    fn zero_width_characters_are_classified() {
        let report = scan("a\u{200B}b\u{200C}c\u{200D}d\u{2060}e");
        assert_eq!(report.findings.len(), 4);
        for found in &report.findings {
            assert_eq!(found.class, CharClass::ZeroWidth);
        }
        assert_eq!(report.hidden_count, 4);
        assert_eq!(report.non_ascii_count, 4);
    }

    #[test]
    fn bidi_controls_are_classified() {
        let report = scan("a\u{200E}b\u{202E}c\u{2066}d\u{061C}e");
        assert_eq!(report.findings.len(), 4);
        for found in &report.findings {
            assert_eq!(found.class, CharClass::BidiControl);
        }
        assert_eq!(report.hidden_count, 4);
    }

    #[test]
    fn format_characters_are_classified() {
        let report = scan("a\u{00AD}b\u{FEFF}c");
        assert_eq!(report.findings.len(), 2);
        assert_eq!(finding(&report, 0).class, CharClass::Format);
        assert_eq!(finding(&report, 1).class, CharClass::Format);
        assert_eq!(report.hidden_count, 2);
        assert_eq!(report.non_ascii_count, 2);
    }

    #[test]
    fn control_characters_are_classified() {
        let report = scan("a\u{0000}b\u{007F}c");
        assert_eq!(report.findings.len(), 2);
        assert_eq!(finding(&report, 0).class, CharClass::Control);
        assert_eq!(finding(&report, 1).class, CharClass::Control);
        assert_eq!(report.hidden_count, 2);
    }

    #[test]
    fn private_use_and_noncharacters_are_classified() {
        let report = scan("a\u{E000}b\u{FDD0}c\u{FFFF}d");
        assert_eq!(report.findings.len(), 3);
        assert_eq!(finding(&report, 0).class, CharClass::PrivateUse);
        assert_eq!(finding(&report, 1).class, CharClass::Noncharacter);
        assert_eq!(finding(&report, 2).class, CharClass::Noncharacter);
        assert_eq!(report.hidden_count, 3);
        assert_eq!(report.non_ascii_count, 3);
    }

    #[test]
    fn combining_marks_are_classified() {
        let report = scan("e\u{0301}x");
        assert_eq!(report.findings.len(), 1);
        assert_eq!(finding(&report, 0).class, CharClass::CombiningMark);
        assert_eq!(report.hidden_count, 1);
    }

    #[test]
    fn visible_non_ascii_is_not_reported_by_default() {
        let report = scan("café");
        assert!(report.findings.is_empty());
        assert_eq!(report.non_ascii_count, 1);
        assert_eq!(report.visible_count, 4);
    }

    #[test]
    fn non_ascii_selection_reports_visible_non_ascii() {
        let report = with_selection("café", false, false, true);
        assert_eq!(report.findings.len(), 1);
        let found = finding(&report, 0);
        assert_eq!(found.character, 'é');
        assert_eq!(found.class, CharClass::Visible);
        assert_eq!(found.byte_offset, 3);
        assert_eq!(found.char_index, 3);
        assert_eq!(found.line, 1);
        assert_eq!(found.column, 4);
    }

    #[test]
    fn whitespace_selection_filters_hidden_findings() {
        let report = with_selection("a\u{200B} b", true, false, false);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(finding(&report, 0).character, ' ');
    }

    #[test]
    fn hidden_selection_filters_whitespace_findings() {
        let report = with_selection("a\u{200B} b", false, true, false);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(finding(&report, 0).character, '\u{200B}');
    }

    #[test]
    fn non_ascii_selection_includes_hidden_non_ascii() {
        let report = with_selection("a\u{200B} b", false, false, true);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(finding(&report, 0).character, '\u{200B}');
    }

    #[test]
    fn counts_are_independent_of_selection() {
        let default = scan("a\u{200B} b");
        let filtered = with_selection("a\u{200B} b", true, false, false);
        assert_eq!(default.whitespace_count, filtered.whitespace_count);
        assert_eq!(default.hidden_count, filtered.hidden_count);
        assert_eq!(default.non_ascii_count, filtered.non_ascii_count);
        assert_eq!(default.visible_count, filtered.visible_count);
        assert_eq!(default.total_characters, filtered.total_characters);
    }

    #[test]
    fn line_and_column_track_multibyte_and_crlf() {
        let report = scan("é\u{00A0}\r\n\tb");
        assert_eq!(report.findings.len(), 4);
        let nbsp = finding(&report, 0);
        assert_eq!(nbsp.character, '\u{00A0}');
        assert_eq!(nbsp.line, 1);
        assert_eq!(nbsp.column, 2);
        assert_eq!(nbsp.byte_offset, 2);
        assert_eq!(nbsp.char_index, 1);
        let carriage = finding(&report, 1);
        assert_eq!(carriage.character, '\r');
        assert_eq!(carriage.line, 1);
        assert_eq!(carriage.column, 3);
        let line_feed = finding(&report, 2);
        assert_eq!(line_feed.character, '\n');
        assert_eq!(line_feed.line, 2);
        assert_eq!(line_feed.column, 1);
        let tab = finding(&report, 3);
        assert_eq!(tab.character, '\t');
        assert_eq!(tab.line, 2);
        assert_eq!(tab.column, 1);
        assert_eq!(tab.char_index, 4);
        assert_eq!(report.whitespace_count, 4);
        assert_eq!(report.non_ascii_count, 2);
    }

    #[test]
    fn unicode_line_separators_advance_the_line() {
        let report = scan("a\u{2028}\u{00A0}b");
        assert_eq!(report.findings.len(), 2);
        assert_eq!(finding(&report, 1).line, 2);
        assert_eq!(finding(&report, 1).column, 1);
    }

    #[test]
    fn known_characters_have_labels() {
        assert_eq!(character_label(' '), "SPACE");
        assert_eq!(character_label('\t'), "TAB");
        assert_eq!(character_label('\n'), "LINE FEED");
        assert_eq!(character_label('\r'), "CARRIAGE RETURN");
        assert_eq!(character_label('\u{00A0}'), "NO-BREAK SPACE");
        assert_eq!(character_label('\u{200B}'), "ZERO WIDTH SPACE");
        assert_eq!(character_label('\u{200E}'), "LEFT-TO-RIGHT MARK");
        assert_eq!(character_label('\u{3000}'), "IDEOGRAPHIC SPACE");
        assert_eq!(character_label('\u{FEFF}'), "ZERO WIDTH NO-BREAK SPACE (BOM)");
        assert_eq!(character_label('a'), "");
        assert_eq!(character_label('é'), "");
        assert_eq!(character_label('\u{0301}'), "");
    }

    #[test]
    fn class_groups_are_consistent() {
        assert!(CharClass::Space.is_whitespace());
        assert!(CharClass::Tab.is_whitespace());
        assert!(CharClass::Newline.is_whitespace());
        assert!(CharClass::AsciiWhitespace.is_whitespace());
        assert!(CharClass::UnicodeWhitespace.is_whitespace());
        assert!(!CharClass::ZeroWidth.is_whitespace());
        assert!(CharClass::Control.is_hidden());
        assert!(CharClass::ZeroWidth.is_hidden());
        assert!(CharClass::BidiControl.is_hidden());
        assert!(CharClass::Format.is_hidden());
        assert!(CharClass::PrivateUse.is_hidden());
        assert!(CharClass::Noncharacter.is_hidden());
        assert!(CharClass::CombiningMark.is_hidden());
        assert!(!CharClass::Space.is_hidden());
        assert!(!CharClass::Visible.is_hidden());
        assert!(!CharClass::Visible.is_whitespace());
    }

    #[test]
    fn class_names_are_stable() {
        assert_eq!(CharClass::Space.name(), "whitespace (space)");
        assert_eq!(CharClass::UnicodeWhitespace.name(), "whitespace (unicode)");
        assert_eq!(CharClass::ZeroWidth.name(), "zero width");
        assert_eq!(CharClass::Visible.name(), "visible");
    }

    #[test]
    fn options_default_to_empty_input() {
        assert!(TextScanOptions::default().input.is_empty());
    }
}

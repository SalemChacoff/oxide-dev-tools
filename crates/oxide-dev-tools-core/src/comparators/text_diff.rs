//! Unified text diffing, git-style: line-based and character-based comparison.
//!
//! [`diff_text`] compares two plain-text inputs and renders a unified diff
//! in the style of `git diff`: `---`/`+++` headers, `@@` hunk ranges, and
//! `-`/`+`/context lines. Line mode compares whole lines; character mode
//! compares individual Unicode scalar values and is meant for short strings.
//!
//! The edit script is computed by a dependency-free implementation of the
//! Myers O(ND) algorithm. Two guards keep pathological inputs fast:
//!
//! - a search distance cap (`MAX_DIFF_DISTANCE`): beyond it, the middle
//!   region is reported as a single whole-region replacement instead of a
//!   minimal diff;
//! - a total-iteration budget (`MAX_DIFF_ITERATIONS`): beyond it, the same
//!   whole-region replacement fallback applies.
//!
//! CRLF line endings are normalized to LF before comparison, so files with
//! mixed line-ending styles diff cleanly. A final line without a trailing
//! newline is marked with git's `\ No newline at end of file` hint.

use std::borrow::Cow;
use std::fmt;

// -------- Public API --------

/// Maximum character-mode input size: 1 MiB of Unicode scalar values.
const MAX_CHARS_INPUT: usize = 1_048_576;

/// Myers search distance cap; beyond it the diff falls back to a
/// whole-region replacement.
const MAX_DIFF_DISTANCE: usize = 2000;

/// Total iteration budget for the Myers search; beyond it the diff falls
/// back to a whole-region replacement as well.
const MAX_DIFF_ITERATIONS: usize = 50_000_000;

/// Errors that can occur when diffing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextDiffError {
    /// Character mode received an input longer than the configured limit.
    InputTooLong { limit: usize },
}

impl fmt::Display for TextDiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextDiffError::InputTooLong { limit } => {
                write!(f, "character diff inputs must be at most {limit} characters")
            }
        }
    }
}

impl std::error::Error for TextDiffError {}

/// How the two inputs are compared.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    /// Compare whole lines and render a git-style unified diff.
    #[default]
    Lines,
    /// Compare individual characters (Unicode scalar values).
    Chars,
}

/// Options for [`diff_text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDiffOptions {
    /// First input to compare.
    pub left: String,
    /// Second input to compare.
    pub right: String,
    /// How the inputs are compared.
    pub mode: DiffMode,
    /// Number of unchanged context lines around each change (line mode).
    pub context_lines: usize,
    /// Label for the first input in the diff headers.
    pub left_label: String,
    /// Label for the second input in the diff headers.
    pub right_label: String,
    /// Maximum input size in characters for character mode.
    pub max_chars: usize,
}

impl Default for TextDiffOptions {
    fn default() -> Self {
        Self {
            left: String::new(),
            right: String::new(),
            mode: DiffMode::Lines,
            context_lines: 3,
            left_label: "left".to_string(),
            right_label: "right".to_string(),
            max_chars: MAX_CHARS_INPUT,
        }
    }
}

/// Compare `options.left` and `options.right` and render the difference.
///
/// Identical inputs produce an empty string (like `git diff`). Line mode
/// has no input size limit; character mode refuses inputs longer than
/// `options.max_chars` scalar values.
pub fn diff_text(options: TextDiffOptions) -> Result<String, TextDiffError> {
    if options.left == options.right {
        return Ok(String::new());
    }
    match options.mode {
        DiffMode::Lines => Ok(diff_lines(
            &options.left,
            &options.right,
            options.context_lines,
            &options.left_label,
            &options.right_label,
        )),
        DiffMode::Chars => {
            if options.left.chars().count() > options.max_chars || options.right.chars().count() > options.max_chars {
                return Err(TextDiffError::InputTooLong {
                    limit: options.max_chars,
                });
            }
            Ok(diff_chars(&options.left, &options.right, &options.left_label, &options.right_label))
        }
    }
}

// -------- Line diff --------

/// Render a unified diff comparing whole lines.
fn diff_lines(left: &str, right: &str, context: usize, left_label: &str, right_label: &str) -> String {
    let left_norm = normalize_crlf(left);
    let right_norm = normalize_crlf(right);
    if left_norm == right_norm {
        return String::new();
    }
    let left_lines = split_lines(&left_norm);
    let right_lines = split_lines(&right_norm);
    let entries = build_entries(&myers_diff(&left_lines, &right_lines));
    let left_has_newline = left_norm.is_empty() || left_norm.ends_with('\n');
    let right_has_newline = right_norm.is_empty() || right_norm.ends_with('\n');
    let mut output = format!("--- {left_label}\n+++ {right_label}\n");
    for hunk in build_hunks(&entries, context) {
        write_hunk(&mut output, &hunk, &entries, &left_lines, &right_lines, left_has_newline, right_has_newline);
    }
    output
}

/// Split into lines that keep their trailing `\n` (CRLF already normalized).
///
/// `"a\nb"` becomes `["a\n", "b"]`; `""` becomes `[]`. A final line without
/// a trailing `\n` keeps none, which the unified output marks like git does.
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split_inclusive('\n').collect()
}

/// Normalize CRLF line endings to LF so files with mixed styles diff cleanly.
fn normalize_crlf(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// Strip a line's trailing `\n` for display.
fn display_line(line: &str) -> &str {
    line.strip_suffix('\n').unwrap_or(line)
}

// -------- Unified hunk building --------

/// How one rendered line of a unified diff is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineTag {
    Context,
    Delete,
    Insert,
}

/// One line of the comparison with its indices into the two inputs. The
/// index that does not apply to the tag is unused.
#[derive(Debug, Clone, Copy)]
struct LineEntry {
    tag: LineTag,
    left_index: usize,
    right_index: usize,
}

/// One unified-diff hunk: a slice of line entries plus the line ranges it
/// covers on both sides.
#[derive(Debug)]
struct UnifiedHunk {
    left_start: usize,
    left_count: usize,
    right_start: usize,
    right_count: usize,
    entry_range: std::ops::Range<usize>,
}

/// Expand edit ops into one entry per line, carrying each line's indices.
fn build_entries(ops: &[EditOp]) -> Vec<LineEntry> {
    let mut entries = Vec::new();
    for op in ops {
        match op {
            EditOp::Equal(left_start, right_start, len) => {
                for offset in 0..*len {
                    entries.push(LineEntry {
                        tag: LineTag::Context,
                        left_index: left_start + offset,
                        right_index: right_start + offset,
                    });
                }
            }
            EditOp::Delete(left_start, len) => {
                for offset in 0..*len {
                    entries.push(LineEntry {
                        tag: LineTag::Delete,
                        left_index: left_start + offset,
                        right_index: 0,
                    });
                }
            }
            EditOp::Insert(right_start, len) => {
                for offset in 0..*len {
                    entries.push(LineEntry {
                        tag: LineTag::Insert,
                        left_index: 0,
                        right_index: right_start + offset,
                    });
                }
            }
        }
    }
    entries
}

/// Group line entries into hunks: each hunk spans a run of changes plus up
/// to `context` unchanged lines on both sides; runs whose context windows
/// touch or overlap are merged into one hunk.
fn build_hunks(entries: &[LineEntry], context: usize) -> Vec<UnifiedHunk> {
    let changed: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.tag != LineTag::Context)
        .map(|(index, _)| index)
        .collect();
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut index = 0;
    while index < changed.len() {
        let first = changed[index];
        let mut end = first + 1;
        let mut next = index + 1;
        while next < changed.len() && changed[next] - end <= 2 * context + 1 {
            end = changed[next] + 1;
            next += 1;
        }
        let start = first.saturating_sub(context);
        let stop = (end + context).min(entries.len());
        ranges.push(start..stop);
        index = next;
    }
    let mut hunks = Vec::with_capacity(ranges.len());
    let mut left_pos = 0;
    let mut right_pos = 0;
    let mut entry_index = 0;
    for range in ranges {
        while entry_index < range.start {
            advance_entry(&entries[entry_index], &mut left_pos, &mut right_pos);
            entry_index += 1;
        }
        let left_start = left_pos;
        let right_start = right_pos;
        while entry_index < range.end {
            advance_entry(&entries[entry_index], &mut left_pos, &mut right_pos);
            entry_index += 1;
        }
        hunks.push(UnifiedHunk {
            left_start,
            left_count: left_pos - left_start,
            right_start,
            right_count: right_pos - right_start,
            entry_range: range,
        });
    }
    hunks
}

/// Move the cumulative line positions past one line entry.
fn advance_entry(entry: &LineEntry, left_pos: &mut usize, right_pos: &mut usize) {
    match entry.tag {
        LineTag::Context => {
            *left_pos += 1;
            *right_pos += 1;
        }
        LineTag::Delete => *left_pos += 1,
        LineTag::Insert => *right_pos += 1,
    }
}

/// Render one side of a `@@` header: `0,0`, a bare start, or `start,count`.
fn format_range(line_start: usize, count: usize) -> String {
    if count == 0 {
        return "0,0".to_string();
    }
    if count == 1 {
        return (line_start + 1).to_string();
    }
    format!("{},{}", line_start + 1, count)
}

/// Append one hunk (header plus body lines) to the output.
fn write_hunk(
    output: &mut String,
    hunk: &UnifiedHunk,
    entries: &[LineEntry],
    left_lines: &[&str],
    right_lines: &[&str],
    left_has_newline: bool,
    right_has_newline: bool,
) {
    output.push_str(&format!(
        "@@ -{} +{} @@\n",
        format_range(hunk.left_start, hunk.left_count),
        format_range(hunk.right_start, hunk.right_count)
    ));
    let range = &hunk.entry_range;
    let last_left = left_lines.len().saturating_sub(1);
    let last_right = right_lines.len().saturating_sub(1);
    for entry in &entries[range.start..range.end] {
        match entry.tag {
            LineTag::Context => {
                output.push(' ');
                output.push_str(display_line(left_lines[entry.left_index]));
                output.push('\n');
            }
            LineTag::Delete => {
                output.push('-');
                output.push_str(display_line(left_lines[entry.left_index]));
                output.push('\n');
                if !left_has_newline && entry.left_index == last_left {
                    output.push_str("\\ No newline at end of file\n");
                }
            }
            LineTag::Insert => {
                output.push('+');
                output.push_str(display_line(right_lines[entry.right_index]));
                output.push('\n');
                if !right_has_newline && entry.right_index == last_right {
                    output.push_str("\\ No newline at end of file\n");
                }
            }
        }
    }
}

// -------- Character diff --------

/// Render a character-by-character diff, one `-`/`+` pair per changed run.
fn diff_chars(left: &str, right: &str, left_label: &str, right_label: &str) -> String {
    let left_norm = normalize_crlf(left);
    let right_norm = normalize_crlf(right);
    if left_norm == right_norm {
        return String::new();
    }
    let left_chars: Vec<char> = left_norm.chars().collect();
    let right_chars: Vec<char> = right_norm.chars().collect();
    let ops = myers_diff(&left_chars, &right_chars);
    let mut output = format!("--- {left_label}\n+++ {right_label}\n");
    let mut removed = String::new();
    let mut inserted = String::new();
    let mut pending = false;
    for op in &ops {
        match op {
            EditOp::Equal(..) => flush_changes(&mut output, &mut removed, &mut inserted, &mut pending),
            EditOp::Delete(start, len) => {
                removed.extend(left_chars[*start..*start + *len].iter());
                pending = true;
            }
            EditOp::Insert(start, len) => {
                inserted.extend(right_chars[*start..*start + *len].iter());
                pending = true;
            }
        }
    }
    flush_changes(&mut output, &mut removed, &mut inserted, &mut pending);
    output
}

/// Append one accumulated `-`/`+` change pair to the output.
fn flush_changes(output: &mut String, removed: &mut String, inserted: &mut String, pending: &mut bool) {
    if !*pending {
        return;
    }
    if !removed.is_empty() {
        output.push('-');
        output.push_str(removed);
        output.push('\n');
    }
    if !inserted.is_empty() {
        output.push('+');
        output.push_str(inserted);
        output.push('\n');
    }
    removed.clear();
    inserted.clear();
    *pending = false;
}

// -------- Myers diff --------

/// One operation of an edit script, as a range into the left and/or right
/// input. `Equal` spans both inputs at once, so its starts differ only when
/// earlier deletes or inserts shifted the positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditOp {
    Equal(usize, usize, usize),
    Delete(usize, usize),
    Insert(usize, usize),
}

/// The kind of an [`EditOp`], used while coalescing raw edit steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpKind {
    Equal,
    Delete,
    Insert,
}

/// Compute the edit script between `left` and `right` with the Myers O(ND)
/// greedy algorithm (forward search plus trace backtracking).
///
/// Common prefix and suffix are trimmed first, so localized changes in
/// otherwise identical inputs stay cheap. If the search exceeds its caps,
/// the untrimmed middle is reported as one whole-region replacement (still
/// a valid edit script, just not necessarily minimal).
fn myers_diff<T: Eq>(left: &[T], right: &[T]) -> Vec<EditOp> {
    let (prefix, suffix) = trim_common(left, right);
    let middle_left = &left[prefix..left.len() - suffix];
    let middle_right = &right[prefix..right.len() - suffix];
    let middle_ops = myers_middle(middle_left, middle_right);
    let mut ops = Vec::new();
    if prefix > 0 {
        push_op(&mut ops, OpKind::Equal, 0, 0, prefix);
    }
    for op in middle_ops {
        match op {
            EditOp::Equal(left_start, right_start, len) => {
                push_op(&mut ops, OpKind::Equal, prefix + left_start, prefix + right_start, len);
            }
            EditOp::Delete(left_start, len) => push_op(&mut ops, OpKind::Delete, prefix + left_start, 0, len),
            EditOp::Insert(right_start, len) => push_op(&mut ops, OpKind::Insert, 0, prefix + right_start, len),
        }
    }
    if suffix > 0 {
        push_op(&mut ops, OpKind::Equal, left.len() - suffix, right.len() - suffix, suffix);
    }
    ops
}

/// Length of the common prefix and suffix of the two inputs.
fn trim_common<T: Eq>(left: &[T], right: &[T]) -> (usize, usize) {
    let mut prefix = 0;
    let shared = left.len().min(right.len());
    while prefix < shared && left[prefix] == right[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < left.len() - prefix
        && suffix < right.len() - prefix
        && left[left.len() - 1 - suffix] == right[right.len() - 1 - suffix]
    {
        suffix += 1;
    }
    (prefix, suffix)
}

/// Diff the trimmed middle region, falling back to a whole-region
/// replacement when the search exceeds its caps.
fn myers_middle<T: Eq>(left: &[T], right: &[T]) -> Vec<EditOp> {
    if left.is_empty() {
        return if right.is_empty() {
            Vec::new()
        } else {
            vec![EditOp::Insert(0, right.len())]
        };
    }
    if right.is_empty() {
        return vec![EditOp::Delete(0, left.len())];
    }
    match myers_search(left, right) {
        Some((distance, trace)) => backtrack(left, right, &trace, distance),
        None => vec![EditOp::Delete(0, left.len()), EditOp::Insert(0, right.len())],
    }
}

/// Run the forward Myers search and return the edit distance plus the
/// frontier snapshot per distance step, or `None` when a cap is exceeded.
fn myers_search<T: Eq>(left: &[T], right: &[T]) -> Option<(usize, Vec<Vec<usize>>)> {
    let left_len = left.len();
    let right_len = right.len();
    let offset = MAX_DIFF_DISTANCE as isize + 1;
    let mut frontier = vec![0usize; 2 * MAX_DIFF_DISTANCE + 3];
    let mut trace: Vec<Vec<usize>> = Vec::new();
    let mut iterations = 0usize;
    for distance in 0..=MAX_DIFF_DISTANCE {
        let mut snapshot = Vec::with_capacity(2 * distance + 1);
        for diag in -(distance as isize)..=(distance as isize) {
            snapshot.push(frontier[(diag + offset) as usize]);
        }
        trace.push(snapshot);
        for diag in (-(distance as isize)..=(distance as isize)).step_by(2) {
            iterations += 1;
            if iterations > MAX_DIFF_ITERATIONS {
                return None;
            }
            let index = (diag + offset) as usize;
            let mut x_pos = if diag == -(distance as isize)
                || (diag != distance as isize && frontier[index - 1] < frontier[index + 1])
            {
                frontier[index + 1]
            } else {
                frontier[index - 1] + 1
            };
            let mut y_pos = x_pos as isize - diag;
            while x_pos < left_len && y_pos >= 0 && (y_pos as usize) < right_len && left[x_pos] == right[y_pos as usize]
            {
                iterations += 1;
                if iterations > MAX_DIFF_ITERATIONS {
                    return None;
                }
                x_pos += 1;
                y_pos += 1;
            }
            frontier[index] = x_pos;
            if x_pos >= left_len && y_pos >= 0 && (y_pos as usize) >= right_len {
                return Some((distance, trace));
            }
        }
    }
    None
}

/// Reconstruct the edit script by walking the search trace backwards.
fn backtrack<T>(left: &[T], right: &[T], trace: &[Vec<usize>], distance: usize) -> Vec<EditOp> {
    let mut raw: Vec<(OpKind, usize, usize)> = Vec::new();
    let mut x_pos = left.len();
    let mut y_pos = right.len();
    for step in (1..=distance).rev() {
        let snapshot = &trace[step];
        let diag = x_pos as isize - y_pos as isize;
        let prev_diag = if diag == -(step as isize)
            || (diag != step as isize
                && snapshot[(diag - 1 + step as isize) as usize] < snapshot[(diag + 1 + step as isize) as usize])
        {
            diag + 1
        } else {
            diag - 1
        };
        let prev_x = snapshot[(prev_diag + step as isize) as usize];
        let prev_y = prev_x as isize - prev_diag;
        while x_pos > prev_x && y_pos > prev_y as usize {
            raw.push((OpKind::Equal, x_pos - 1, y_pos - 1));
            x_pos -= 1;
            y_pos -= 1;
        }
        if x_pos == prev_x {
            raw.push((OpKind::Insert, x_pos, y_pos - 1));
            y_pos -= 1;
        } else {
            raw.push((OpKind::Delete, x_pos - 1, y_pos));
            x_pos -= 1;
        }
    }
    while x_pos > 0 && y_pos > 0 {
        raw.push((OpKind::Equal, x_pos - 1, y_pos - 1));
        x_pos -= 1;
        y_pos -= 1;
    }
    let mut ops: Vec<EditOp> = Vec::new();
    for (kind, left_index, right_index) in raw.iter().rev() {
        push_op(&mut ops, *kind, *left_index, *right_index, 1);
    }
    ops
}

/// Append one edit range, merging it into the previous op when contiguous.
fn push_op(ops: &mut Vec<EditOp>, kind: OpKind, left_start: usize, right_start: usize, len: usize) {
    match (kind, ops.last_mut()) {
        (OpKind::Equal, Some(EditOp::Equal(last_left, last_right, last_len)))
            if *last_left + *last_len == left_start && *last_right + *last_len == right_start =>
        {
            *last_len += len;
        }
        (OpKind::Delete, Some(EditOp::Delete(last_left, last_len))) if *last_left + *last_len == left_start => {
            *last_len += len;
        }
        (OpKind::Insert, Some(EditOp::Insert(last_right, last_len))) if *last_right + *last_len == right_start => {
            *last_len += len;
        }
        _ => ops.push(match kind {
            OpKind::Equal => EditOp::Equal(left_start, right_start, len),
            OpKind::Delete => EditOp::Delete(left_start, len),
            OpKind::Insert => EditOp::Insert(right_start, len),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(left: &str, right: &str) -> String {
        diff_text(TextDiffOptions {
            left: left.to_string(),
            right: right.to_string(),
            ..TextDiffOptions::default()
        })
        .unwrap()
    }

    fn char_diff(left: &str, right: &str) -> String {
        diff_text(TextDiffOptions {
            left: left.to_string(),
            right: right.to_string(),
            mode: DiffMode::Chars,
            ..TextDiffOptions::default()
        })
        .unwrap()
    }

    #[test]
    fn identical_inputs_produce_no_output() {
        assert_eq!(diff("a\nb\n", "a\nb\n"), "");
        assert_eq!(diff("", ""), "");
        assert_eq!(char_diff("hello", "hello"), "");
    }

    #[test]
    fn single_line_replacement() {
        assert_eq!(diff("a\nb\nc\n", "a\nx\nc\n"), "--- left\n+++ right\n@@ -1,3 +1,3 @@\n a\n-b\n+x\n c\n");
    }

    #[test]
    fn line_insertion() {
        assert_eq!(diff("a\nc\n", "a\nb\nc\n"), "--- left\n+++ right\n@@ -1,2 +1,3 @@\n a\n+b\n c\n");
    }

    #[test]
    fn line_deletion() {
        assert_eq!(diff("a\nb\nc\n", "a\nc\n"), "--- left\n+++ right\n@@ -1,3 +1,2 @@\n a\n-b\n c\n");
    }

    #[test]
    fn empty_vs_content_insertion() {
        assert_eq!(diff("", "x\ny\n"), "--- left\n+++ right\n@@ -0,0 +1,2 @@\n+x\n+y\n");
    }

    #[test]
    fn content_vs_empty_deletion() {
        assert_eq!(diff("x\ny\n", ""), "--- left\n+++ right\n@@ -1,2 +0,0 @@\n-x\n-y\n");
    }

    #[test]
    fn empty_file_vs_empty_line() {
        assert_eq!(diff("", "\n"), "--- left\n+++ right\n@@ -0,0 +1 @@\n+\n");
    }

    #[test]
    fn missing_trailing_newline_is_marked() {
        assert_eq!(
            diff("a\nb\n", "a\nb"),
            "--- left\n+++ right\n@@ -1,2 +1,2 @@\n a\n-b\n+b\n\\ No newline at end of file\n"
        );
    }

    #[test]
    fn separated_changes_form_two_hunks() {
        let options = TextDiffOptions {
            left: "a\nb\nc\nd\ne\nf\n".to_string(),
            right: "A\nb\nc\nd\ne\nF\n".to_string(),
            context_lines: 1,
            ..TextDiffOptions::default()
        };
        assert_eq!(
            diff_text(options).unwrap(),
            "--- left\n+++ right\n@@ -1,2 +1,2 @@\n-a\n+A\n b\n@@ -5,2 +5,2 @@\n e\n-f\n+F\n"
        );
    }

    #[test]
    fn zero_context_shows_only_changes() {
        let options = TextDiffOptions {
            left: "a\nb\nc\n".to_string(),
            right: "a\nB\nc\n".to_string(),
            context_lines: 0,
            ..TextDiffOptions::default()
        };
        assert_eq!(diff_text(options).unwrap(), "--- left\n+++ right\n@@ -2 +2 @@\n-b\n+B\n");
    }

    #[test]
    fn crlf_is_normalized() {
        assert_eq!(diff("a\r\nb\r\n", "a\nb\n"), "");
        assert_eq!(diff("a\r\nb\r\n", "a\r\nc\n"), "--- left\n+++ right\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n");
        assert_eq!(char_diff("a\r\nb", "a\nb"), "");
    }

    #[test]
    fn unicode_lines_diff_cleanly() {
        let output = diff("你好\n世界\n", "你好\n地球\n");
        assert!(output.contains("-世界"));
        assert!(output.contains("+地球"));
    }

    #[test]
    fn custom_labels_appear_in_headers() {
        let options = TextDiffOptions {
            left: "a\n".to_string(),
            right: "b\n".to_string(),
            left_label: "old.txt".to_string(),
            right_label: "new.txt".to_string(),
            ..TextDiffOptions::default()
        };
        assert!(diff_text(options).unwrap().starts_with("--- old.txt\n+++ new.txt\n"));
    }

    #[test]
    fn char_mode_single_deletion() {
        assert_eq!(char_diff("hello", "helo"), "--- left\n+++ right\n-l\n");
    }

    #[test]
    fn char_mode_replacement() {
        assert_eq!(char_diff("abc", "axc"), "--- left\n+++ right\n-b\n+x\n");
    }

    #[test]
    fn char_mode_insertion_only_shows_added() {
        assert_eq!(char_diff("abc", "abxc"), "--- left\n+++ right\n+x\n");
    }

    #[test]
    fn char_mode_pure_deletion() {
        assert_eq!(char_diff("abc", ""), "--- left\n+++ right\n-abc\n");
    }

    #[test]
    fn char_mode_multiple_groups() {
        assert_eq!(char_diff("abcde", "abXcdYe"), "--- left\n+++ right\n+X\n+Y\n");
    }

    #[test]
    fn char_mode_unicode_scalar_values() {
        assert_eq!(char_diff("你好世界", "你好地球"), "--- left\n+++ right\n-世界\n+地球\n");
    }

    #[test]
    fn char_mode_rejects_oversized_input() {
        let options = TextDiffOptions {
            left: "hello".to_string(),
            right: "world".to_string(),
            mode: DiffMode::Chars,
            max_chars: 4,
            ..TextDiffOptions::default()
        };
        assert_eq!(diff_text(options), Err(TextDiffError::InputTooLong { limit: 4 }));
        let err = TextDiffError::InputTooLong { limit: 4 };
        assert_eq!(err.to_string(), "character diff inputs must be at most 4 characters");
    }

    #[test]
    fn fallback_replaces_wholly_different_inputs() {
        // 3000 unique lines on each side: the distance between the inputs
        // exceeds MAX_DIFF_DISTANCE, so the diff falls back to replace-all.
        let left = (0..3000).map(|i| format!("left-{i}\n")).collect::<String>();
        let right = (0..3000).map(|i| format!("right-{i}\n")).collect::<String>();
        let output = diff(&left, &right);
        assert!(output.starts_with("--- left\n+++ right\n@@ -1,3000 +1,3000 @@\n"));
        assert_eq!(output.lines().filter(|line| line.starts_with("-left-")).count(), 3000);
        assert_eq!(output.lines().filter(|line| line.starts_with("+right-")).count(), 3000);
    }

    #[test]
    fn options_defaults() {
        let defaults = TextDiffOptions::default();
        assert_eq!(defaults.left, "");
        assert_eq!(defaults.right, "");
        assert_eq!(defaults.mode, DiffMode::Lines);
        assert_eq!(defaults.context_lines, 3);
        assert_eq!(defaults.left_label, "left");
        assert_eq!(defaults.right_label, "right");
        assert_eq!(defaults.max_chars, 1_048_576);
    }

    // -------- Algorithm property tests --------

    #[test]
    fn myers_matches_bruteforce_edit_distance() {
        for alphabet in [
            "ab".chars().collect::<Vec<char>>(),
            "abc".chars().collect::<Vec<char>>(),
        ] {
            let max_len = if alphabet.len() == 2 { 6 } else { 4 };
            for left in all_strings(&alphabet, max_len) {
                for right in all_strings(&alphabet, max_len) {
                    let ops = myers_diff(&left, &right);
                    assert_eq!(ops_cost(&ops), edit_distance(&left, &right), "{left:?} vs {right:?}");
                    assert!(ops_apply(&ops, &left, &right), "{left:?} vs {right:?}");
                }
            }
        }
    }

    fn all_strings(alphabet: &[char], max_len: usize) -> Vec<Vec<char>> {
        let mut result = vec![Vec::new()];
        let mut generation_start = 0;
        for _ in 1..=max_len {
            let generation = result[generation_start..].to_vec();
            generation_start = result.len();
            for prefix in &generation {
                for &ch in alphabet {
                    let mut item = prefix.clone();
                    item.push(ch);
                    result.push(item);
                }
            }
        }
        result
    }

    fn edit_distance(left: &[char], right: &[char]) -> usize {
        let mut previous: Vec<usize> = (0..=right.len()).collect();
        let mut current = vec![0usize; right.len() + 1];
        for (left_index, left_ch) in left.iter().enumerate() {
            current[0] = left_index + 1;
            for (right_index, right_ch) in right.iter().enumerate() {
                current[right_index + 1] = if left_ch == right_ch {
                    previous[right_index]
                } else {
                    1 + current[right_index].min(previous[right_index + 1])
                };
            }
            std::mem::swap(&mut previous, &mut current);
        }
        previous[right.len()]
    }

    fn ops_cost(ops: &[EditOp]) -> usize {
        ops.iter()
            .map(|op| match op {
                EditOp::Equal(..) => 0,
                EditOp::Delete(_, len) | EditOp::Insert(_, len) => *len,
            })
            .sum()
    }

    fn ops_apply(ops: &[EditOp], left: &[char], right: &[char]) -> bool {
        let mut left_pos = 0;
        let mut right_pos = 0;
        for op in ops {
            match op {
                EditOp::Equal(left_start, right_start, len) => {
                    if *left_start != left_pos || *right_start != right_pos {
                        return false;
                    }
                    if left[left_pos..left_pos + len] != right[right_pos..right_pos + len] {
                        return false;
                    }
                    left_pos += len;
                    right_pos += len;
                }
                EditOp::Delete(left_start, len) => {
                    if *left_start != left_pos {
                        return false;
                    }
                    left_pos += len;
                }
                EditOp::Insert(right_start, len) => {
                    if *right_start != right_pos {
                        return false;
                    }
                    right_pos += len;
                }
            }
        }
        left_pos == left.len() && right_pos == right.len()
    }
}

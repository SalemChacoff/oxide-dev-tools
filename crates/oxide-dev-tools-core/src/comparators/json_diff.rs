//! JSON deep comparison.
//!
//! Compares two JSON documents *semantically* rather than textually:
//!
//! - object member order is irrelevant (JSON objects are unordered maps);
//! - arrays are positional by default, or compared as multisets with
//!   [`JsonDiffOptions::ignore_array_order`];
//! - types are strict: `1`, `"1"`, and `true` are all different, and a
//!   missing member is not the same as an explicit `null`;
//! - numbers compare numerically (`1 == 1.0`), optionally within a relative
//!   tolerance ([`JsonDiffOptions::relative_tolerance`]);
//! - members named in [`JsonDiffOptions::ignore_keys`] are skipped at every
//!   depth.
//!
//! [`compare_json`] returns a [`JsonDiffReport`] that renders either as a
//! path-addressed report ([`JsonDiffReport::to_text`]) or as an RFC 6902
//! patch ([`JsonDiffReport::to_patch`]).

use std::fmt;

use serde_json::{Map, Number, Value};

// -------- Public API --------

/// Maximum input size per document: 1 MiB of characters.
const MAX_CHARS_PER_SIDE: usize = 1_048_576;

/// Longest value rendering in [`JsonDiffReport::to_text`] before truncation.
const MAX_VALUE_CHARS: usize = 80;

/// Which document an error refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonSide {
    /// The first input document.
    Left,
    /// The second input document.
    Right,
}

impl JsonSide {
    fn label(self) -> &'static str {
        match self {
            JsonSide::Left => "left",
            JsonSide::Right => "right",
        }
    }
}

/// Errors that can occur when comparing JSON documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonDiffError {
    /// A document is not valid JSON.
    InvalidJson {
        /// Which document failed to parse.
        side: JsonSide,
        /// The parser message, including line and column.
        message: String,
    },
    /// A document is longer than the configured limit.
    InputTooLong {
        /// Which document exceeded the limit.
        side: JsonSide,
        /// The configured limit in characters.
        limit: usize,
    },
    /// The relative tolerance is negative, above 100%, or not finite.
    InvalidTolerance {
        /// Human-readable explanation of the invalid value.
        message: String,
    },
}

impl fmt::Display for JsonDiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonDiffError::InvalidJson { side, message } => {
                write!(f, "cannot parse JSON on the {} side: {message}", side.label())
            }
            JsonDiffError::InputTooLong { side, limit } => {
                write!(f, "{} JSON input must be at most {limit} characters", side.label())
            }
            JsonDiffError::InvalidTolerance { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for JsonDiffError {}

/// How the documents differ at a single location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonChangeKind {
    /// The value exists only in the right document.
    Added,
    /// The value exists only in the left document.
    Removed,
    /// The value exists in both documents but is not equal.
    Changed,
}

/// One detected difference between the two documents.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonDifference {
    /// Human-readable location, e.g. `$.users[0].name`.
    pub path: String,
    /// RFC 6901 JSON pointer to the same location, e.g. `/users/0/name`.
    pub pointer: String,
    /// How the documents differ at this location.
    pub kind: JsonChangeKind,
    /// The value in the left document, when it exists there.
    pub left: Option<Value>,
    /// The value in the right document, when it exists there.
    pub right: Option<Value>,
}

/// The outcome of [`compare_json`].
#[derive(Debug, Clone, PartialEq)]
pub struct JsonDiffReport {
    /// Every detected difference, in left-document order.
    pub differences: Vec<JsonDifference>,
}

impl JsonDiffReport {
    /// Whether the two documents are equal under the comparison options.
    pub fn is_equal(&self) -> bool {
        self.differences.is_empty()
    }

    /// Render the report as one line per difference:
    ///
    /// ```text
    /// ~ $.users[0].name: "ann" -> "bob"
    /// + $.meta.build: true
    /// - $.items[2]: {"id":3}
    /// ```
    ///
    /// Values longer than [`MAX_VALUE_CHARS`] are truncated with `…`.
    pub fn to_text(&self) -> String {
        let mut lines = Vec::with_capacity(self.differences.len());
        for difference in &self.differences {
            let line = match difference.kind {
                JsonChangeKind::Changed => format!(
                    "~ {}: {} -> {}",
                    difference.path,
                    format_value(difference.left.as_ref()),
                    format_value(difference.right.as_ref())
                ),
                JsonChangeKind::Added => {
                    format!("+ {}: {}", difference.path, format_value(difference.right.as_ref()))
                }
                JsonChangeKind::Removed => {
                    format!("- {}: {}", difference.path, format_value(difference.left.as_ref()))
                }
            };
            lines.push(line);
        }
        lines.join("\n")
    }

    /// Render the report as an RFC 6902 JSON patch: a pretty-printed array
    /// of `add`, `remove`, and `replace` operations.
    ///
    /// Removal operations are emitted first, array removals in descending
    /// index order, so the patch applies cleanly to ordered comparisons.
    /// With [`JsonDiffOptions::ignore_array_order`] the patch still lists
    /// every difference, but array indices can shift while applying it —
    /// treat it as a descriptive report in that mode.
    pub fn to_patch(&self) -> String {
        let mut removals: Vec<&JsonDifference> = self
            .differences
            .iter()
            .filter(|difference| difference.kind == JsonChangeKind::Removed)
            .collect();
        removals.sort_by(|a, b| {
            removal_order_key(&a.pointer)
                .cmp(&removal_order_key(&b.pointer))
                .reverse()
        });

        let mut operations = Vec::with_capacity(self.differences.len());
        for difference in removals {
            operations.push(patch_operation("remove", &difference.pointer, None));
        }
        for difference in &self.differences {
            match difference.kind {
                JsonChangeKind::Changed => {
                    operations.push(patch_operation("replace", &difference.pointer, difference.right.as_ref()));
                }
                JsonChangeKind::Added => {
                    operations.push(patch_operation("add", &difference.pointer, difference.right.as_ref()));
                }
                JsonChangeKind::Removed => {}
            }
        }
        match serde_json::to_string_pretty(&Value::Array(operations)) {
            Ok(patch) => patch,
            Err(_) => String::from("[]"),
        }
    }
}

/// Options for [`compare_json`].
#[derive(Debug, Clone, PartialEq)]
pub struct JsonDiffOptions {
    /// First document to compare.
    pub left: String,
    /// Second document to compare.
    pub right: String,
    /// Compare array elements as multisets instead of positionally.
    pub ignore_array_order: bool,
    /// Relative numeric tolerance as a fraction of the larger magnitude:
    /// numbers count as equal when `|a - b| <= tolerance * max(|a|, |b|)`.
    /// `None` compares exactly, treating integer and float forms of the
    /// same value as equal (`1 == 1.0`). Numeric comparisons use `f64`
    /// and are approximate beyond 2^53.
    pub relative_tolerance: Option<f64>,
    /// Object member names skipped at every depth.
    pub ignore_keys: Vec<String>,
    /// Maximum input size in characters per document.
    pub max_chars: usize,
}

impl Default for JsonDiffOptions {
    fn default() -> Self {
        Self {
            left: String::new(),
            right: String::new(),
            ignore_array_order: false,
            relative_tolerance: None,
            ignore_keys: Vec::new(),
            max_chars: MAX_CHARS_PER_SIDE,
        }
    }
}

/// Compare two JSON documents and report every difference.
pub fn compare_json(options: JsonDiffOptions) -> Result<JsonDiffReport, JsonDiffError> {
    validate(&options)?;
    let left = parse_document(&options.left, JsonSide::Left)?;
    let right = parse_document(&options.right, JsonSide::Right)?;
    let mut differences = Vec::new();
    compare_values(&[], &left, &right, &options, &mut differences);
    Ok(JsonDiffReport { differences })
}

// -------- Parsing and validation --------

fn validate(options: &JsonDiffOptions) -> Result<(), JsonDiffError> {
    if options.left.chars().count() > options.max_chars {
        return Err(JsonDiffError::InputTooLong {
            side: JsonSide::Left,
            limit: options.max_chars,
        });
    }
    if options.right.chars().count() > options.max_chars {
        return Err(JsonDiffError::InputTooLong {
            side: JsonSide::Right,
            limit: options.max_chars,
        });
    }
    if let Some(tolerance) = options.relative_tolerance {
        if !(0.0..=1.0).contains(&tolerance) {
            return Err(JsonDiffError::InvalidTolerance {
                message: format!("relative tolerance must be between 0.0 and 1.0, got {tolerance}"),
            });
        }
    }
    Ok(())
}

fn parse_document(input: &str, side: JsonSide) -> Result<Value, JsonDiffError> {
    serde_json::from_str(input).map_err(|error| JsonDiffError::InvalidJson {
        side,
        message: error.to_string(),
    })
}

// -------- Comparison --------

/// One step of a location path: an object member or an array index.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PathStep {
    Key(String),
    Index(usize),
}

fn push_step(path: &[PathStep], step: PathStep) -> Vec<PathStep> {
    let mut extended = Vec::from(path);
    extended.push(step);
    extended
}

/// Compare two values recursively; returns whether the subtrees are equal
/// under `options` and appends every detected difference to `differences`.
fn compare_values(
    path: &[PathStep],
    left: &Value,
    right: &Value,
    options: &JsonDiffOptions,
    differences: &mut Vec<JsonDifference>,
) -> bool {
    match (left, right) {
        (Value::Object(left_object), Value::Object(right_object)) => {
            compare_objects(path, left_object, right_object, options, differences)
        }
        (Value::Array(left_array), Value::Array(right_array)) => {
            if options.ignore_array_order {
                compare_arrays_unordered(path, left_array, right_array, options, differences)
            } else {
                compare_arrays_ordered(path, left_array, right_array, options, differences)
            }
        }
        (Value::Number(left_number), Value::Number(right_number)) => {
            if numbers_equal(left_number, right_number, options) {
                true
            } else {
                push_changed(path, left, right, differences);
                false
            }
        }
        _ => {
            if left == right {
                true
            } else {
                push_changed(path, left, right, differences);
                false
            }
        }
    }
}

fn compare_objects(
    path: &[PathStep],
    left: &Map<String, Value>,
    right: &Map<String, Value>,
    options: &JsonDiffOptions,
    differences: &mut Vec<JsonDifference>,
) -> bool {
    let mut equal = true;
    for (key, left_value) in left {
        if options.ignore_keys.contains(key) {
            continue;
        }
        let child_path = push_step(path, PathStep::Key(key.clone()));
        match right.get(key) {
            Some(right_value) => {
                if !compare_values(&child_path, left_value, right_value, options, differences) {
                    equal = false;
                }
            }
            None => {
                push_removed(&child_path, left_value, differences);
                equal = false;
            }
        }
    }
    for (key, right_value) in right {
        if options.ignore_keys.contains(key) || left.contains_key(key) {
            continue;
        }
        let child_path = push_step(path, PathStep::Key(key.clone()));
        push_added(&child_path, right_value, differences);
        equal = false;
    }
    equal
}

fn compare_arrays_ordered(
    path: &[PathStep],
    left: &[Value],
    right: &[Value],
    options: &JsonDiffOptions,
    differences: &mut Vec<JsonDifference>,
) -> bool {
    let mut equal = true;
    for (index, (left_item, right_item)) in left.iter().zip(right.iter()).enumerate() {
        let child_path = push_step(path, PathStep::Index(index));
        if !compare_values(&child_path, left_item, right_item, options, differences) {
            equal = false;
        }
    }
    let shared = left.len().min(right.len());
    for (index, item) in left.iter().enumerate().skip(shared) {
        push_removed(&push_step(path, PathStep::Index(index)), item, differences);
        equal = false;
    }
    for (index, item) in right.iter().enumerate().skip(shared) {
        push_added(&push_step(path, PathStep::Index(index)), item, differences);
        equal = false;
    }
    equal
}

/// Multiset comparison: every element of one array is matched against an
/// unmatched equal element of the other. Unmatched left elements are
/// reported at their left index, unmatched right elements at their right
/// index, so indices refer to the element's own document.
fn compare_arrays_unordered(
    path: &[PathStep],
    left: &[Value],
    right: &[Value],
    options: &JsonDiffOptions,
    differences: &mut Vec<JsonDifference>,
) -> bool {
    let mut equal = true;
    let mut matched = vec![false; right.len()];
    let mut scratch = Vec::new();
    for (left_index, left_item) in left.iter().enumerate() {
        let mut matched_any = false;
        for (right_index, right_item) in right.iter().enumerate() {
            if matched[right_index] {
                continue;
            }
            scratch.clear();
            if compare_values(&[], left_item, right_item, options, &mut scratch) {
                matched[right_index] = true;
                matched_any = true;
                break;
            }
        }
        if !matched_any {
            push_removed(&push_step(path, PathStep::Index(left_index)), left_item, differences);
            equal = false;
        }
    }
    for (right_index, right_item) in right.iter().enumerate() {
        if !matched[right_index] {
            push_added(&push_step(path, PathStep::Index(right_index)), right_item, differences);
            equal = false;
        }
    }
    equal
}

/// Whether two numbers are equal: same-kind integer pairs compare exactly
/// (so `1` and `2` never collapse into the same `f64`), everything else
/// compares as `f64` — which is why `1` and `1.0` are equal. With a
/// relative tolerance, the difference must stay within
/// `tolerance * max(|a|, |b|)`; note that tolerance comparisons convert to
/// `f64` and are only approximate beyond 2^53.
fn numbers_equal(left: &Number, right: &Number, options: &JsonDiffOptions) -> bool {
    if left == right {
        return true;
    }
    let integer_pair =
        (left.as_i64().is_some() && right.as_i64().is_some()) || (left.as_u64().is_some() && right.as_u64().is_some());
    if options.relative_tolerance.is_none() && integer_pair {
        return false;
    }
    let (Some(left_float), Some(right_float)) = (left.as_f64(), right.as_f64()) else {
        return false;
    };
    if left_float == right_float {
        return true;
    }
    let Some(tolerance) = options.relative_tolerance else {
        return false;
    };
    let scale = left_float.abs().max(right_float.abs());
    if scale == 0.0 {
        return true;
    }
    (left_float - right_float).abs() <= tolerance * scale
}

fn push_changed(path: &[PathStep], left: &Value, right: &Value, differences: &mut Vec<JsonDifference>) {
    differences.push(JsonDifference {
        path: format_path(path),
        pointer: format_pointer(path),
        kind: JsonChangeKind::Changed,
        left: Some(left.clone()),
        right: Some(right.clone()),
    });
}

fn push_added(path: &[PathStep], right: &Value, differences: &mut Vec<JsonDifference>) {
    differences.push(JsonDifference {
        path: format_path(path),
        pointer: format_pointer(path),
        kind: JsonChangeKind::Added,
        left: None,
        right: Some(right.clone()),
    });
}

fn push_removed(path: &[PathStep], left: &Value, differences: &mut Vec<JsonDifference>) {
    differences.push(JsonDifference {
        path: format_path(path),
        pointer: format_pointer(path),
        kind: JsonChangeKind::Removed,
        left: Some(left.clone()),
        right: None,
    });
}

// -------- Rendering --------

fn format_path(steps: &[PathStep]) -> String {
    let mut path = String::from("$");
    for step in steps {
        match step {
            PathStep::Key(key) => {
                if is_simple_key(key) {
                    path.push('.');
                    path.push_str(key);
                } else {
                    path.push_str(&quote_key(key));
                }
            }
            PathStep::Index(index) => {
                path.push('[');
                path.push_str(&index.to_string());
                path.push(']');
            }
        }
    }
    path
}

fn is_simple_key(key: &str) -> bool {
    let mut characters = key.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn quote_key(key: &str) -> String {
    let escaped = key.replace('\\', "\\\\").replace('"', "\\\"");
    format!("[\"{escaped}\"]")
}

/// RFC 6901 pointer: `~` and `/` in member names are escaped as `~0` and
/// `~1`; the empty string is the pointer for the whole document.
fn format_pointer(steps: &[PathStep]) -> String {
    let mut pointer = String::new();
    for step in steps {
        pointer.push('/');
        match step {
            PathStep::Key(key) => pointer.push_str(&key.replace('~', "~0").replace('/', "~1")),
            PathStep::Index(index) => pointer.push_str(&index.to_string()),
        }
    }
    pointer
}

fn format_value(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "<missing>".to_string();
    };
    let text = value.to_string();
    if text.chars().count() <= MAX_VALUE_CHARS {
        return text;
    }
    let shortened: String = text.chars().take(MAX_VALUE_CHARS).collect();
    format!("{shortened}…")
}

/// Build one RFC 6902 operation object.
fn patch_operation(operation: &str, pointer: &str, value: Option<&Value>) -> Value {
    let mut fields = Map::new();
    fields.insert("op".to_string(), Value::String(operation.to_string()));
    fields.insert("path".to_string(), Value::String(pointer.to_string()));
    if let Some(value) = value {
        fields.insert("value".to_string(), value.clone());
    }
    Value::Object(fields)
}

/// Sort key for patch removals: the trailing array index of an array
/// removal, or `usize::MAX` for object removals. The caller sorts
/// descending, so object removals (order-independent) come first and array
/// removals go from highest index down.
fn removal_order_key(pointer: &str) -> usize {
    pointer
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|segment| segment.parse::<usize>().ok())
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn options(left: &str, right: &str) -> JsonDiffOptions {
        JsonDiffOptions {
            left: left.to_string(),
            right: right.to_string(),
            ..JsonDiffOptions::default()
        }
    }

    fn compare(left: &str, right: &str) -> JsonDiffReport {
        compare_json(options(left, right)).unwrap()
    }

    #[test]
    fn identical_nested_documents_report_no_differences() {
        let report = compare(
            r#"{"users":[{"name":"ann","tags":["a","b"]}],"meta":{"build":7}}"#,
            r#"{"users":[{"name":"ann","tags":["a","b"]}],"meta":{"build":7}}"#,
        );
        assert!(report.is_equal());
        assert!(report.differences.is_empty());
        assert_eq!(report.to_text(), "");
        assert_eq!(report.to_patch(), "[]");
    }

    #[test]
    fn member_order_is_irrelevant() {
        assert!(compare(r#"{"a":1,"b":2}"#, r#"{"b":2,"a":1}"#).is_equal());
    }

    #[test]
    fn nested_value_change_reports_path_and_values() {
        let report = compare(r#"{"users":[{"name":"ann"}]}"#, r#"{"users":[{"name":"bob"}]}"#);
        assert_eq!(report.differences.len(), 1);
        let difference = &report.differences[0];
        assert_eq!(difference.path, "$.users[0].name");
        assert_eq!(difference.pointer, "/users/0/name");
        assert_eq!(difference.kind, JsonChangeKind::Changed);
        assert_eq!(difference.left, Some(json!("ann")));
        assert_eq!(difference.right, Some(json!("bob")));
        assert_eq!(report.to_text(), "~ $.users[0].name: \"ann\" -> \"bob\"");
    }

    #[test]
    fn added_member_is_reported() {
        let report = compare("{}", r#"{"a":1}"#);
        assert_eq!(report.differences.len(), 1);
        let difference = &report.differences[0];
        assert_eq!(difference.path, "$.a");
        assert_eq!(difference.kind, JsonChangeKind::Added);
        assert_eq!(difference.right, Some(json!(1)));
        assert_eq!(report.to_text(), "+ $.a: 1");
    }

    #[test]
    fn removed_member_is_reported() {
        let report = compare(r#"{"a":1}"#, "{}");
        assert_eq!(report.differences.len(), 1);
        let difference = &report.differences[0];
        assert_eq!(difference.path, "$.a");
        assert_eq!(difference.kind, JsonChangeKind::Removed);
        assert_eq!(difference.left, Some(json!(1)));
        assert_eq!(report.to_text(), "- $.a: 1");
    }

    #[test]
    fn array_element_change_is_positional() {
        let report = compare("[1,2]", "[1,3]");
        assert_eq!(report.differences.len(), 1);
        assert_eq!(report.differences[0].path, "$[1]");
        assert_eq!(report.differences[0].kind, JsonChangeKind::Changed);
    }

    #[test]
    fn array_tail_added_is_reported() {
        let report = compare("[1]", "[1,2]");
        assert_eq!(report.differences.len(), 1);
        assert_eq!(report.differences[0].path, "$[1]");
        assert_eq!(report.differences[0].kind, JsonChangeKind::Added);
    }

    #[test]
    fn array_tail_removed_is_reported() {
        let report = compare("[1,2]", "[1]");
        assert_eq!(report.differences.len(), 1);
        assert_eq!(report.differences[0].path, "$[1]");
        assert_eq!(report.differences[0].kind, JsonChangeKind::Removed);
    }

    #[test]
    fn ignore_order_matches_permutations() {
        let options = JsonDiffOptions {
            ignore_array_order: true,
            ..options("[1,2,3]", "[3,1,2]")
        };
        assert!(compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn ignore_order_handles_duplicates() {
        let options = JsonDiffOptions {
            ignore_array_order: true,
            ..options("[1,1,2]", "[1,2,2]")
        };
        let report = compare_json(options).unwrap();
        assert_eq!(report.differences.len(), 2);
        assert_eq!(report.differences[0].path, "$[1]");
        assert_eq!(report.differences[0].kind, JsonChangeKind::Removed);
        assert_eq!(report.differences[1].path, "$[2]");
        assert_eq!(report.differences[1].kind, JsonChangeKind::Added);
    }

    #[test]
    fn ordered_arrays_stay_positional_by_default() {
        let report = compare("[1,2]", "[2,1]");
        assert_eq!(report.differences.len(), 2);
        assert!(
            report
                .differences
                .iter()
                .all(|difference| difference.kind == JsonChangeKind::Changed)
        );
    }

    #[test]
    fn types_are_strict() {
        assert!(!compare("1", r#""1""#).is_equal());
        assert!(!compare("true", "1").is_equal());
    }

    #[test]
    fn missing_member_is_not_null() {
        assert!(!compare("{}", r#"{"a":null}"#).is_equal());
        assert!(!compare(r#"{"a":null}"#, "{}").is_equal());
    }

    #[test]
    fn integer_and_float_forms_are_equal() {
        assert!(compare("1", "1.0").is_equal());
    }

    #[test]
    fn tolerance_boundary_is_inclusive() {
        let options = JsonDiffOptions {
            relative_tolerance: Some(0.05),
            ..options("100", "105")
        };
        assert!(compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn tolerance_outside_boundary_differs() {
        let options = JsonDiffOptions {
            relative_tolerance: Some(0.05),
            ..options("100", "106")
        };
        assert!(!compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn tolerance_applies_to_negative_numbers() {
        let options = JsonDiffOptions {
            relative_tolerance: Some(0.05),
            ..options("-100", "-105")
        };
        assert!(compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn tolerance_never_applies_to_non_numbers() {
        let options = JsonDiffOptions {
            relative_tolerance: Some(0.9),
            ..options(r#""100""#, r#""105""#)
        };
        assert!(!compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn zero_tolerance_is_exact() {
        let options = JsonDiffOptions {
            relative_tolerance: Some(0.0),
            ..options("1", "1.001")
        };
        assert!(!compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn ignored_keys_are_skipped_at_any_depth() {
        let options = JsonDiffOptions {
            ignore_keys: vec!["skip".to_string()],
            ..options(r#"{"a":{"skip":1,"keep":1}}"#, r#"{"a":{"skip":9,"keep":1}}"#)
        };
        assert!(compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn ignored_keys_are_skipped_when_added_or_removed() {
        let options = JsonDiffOptions {
            ignore_keys: vec!["skip".to_string()],
            ..options("{}", r#"{"skip":1}"#)
        };
        assert!(compare_json(options).unwrap().is_equal());
    }

    #[test]
    fn invalid_json_reports_the_failing_side() {
        let left = compare_json(options("not json", "1")).unwrap_err();
        assert!(left.to_string().starts_with("cannot parse JSON on the left side"));
        let right = compare_json(options("1", "not json")).unwrap_err();
        assert!(right.to_string().contains("cannot parse JSON on the right side"));
    }

    #[test]
    fn oversized_inputs_error_per_side() {
        let options = JsonDiffOptions {
            left: "x".repeat(11),
            right: "1".to_string(),
            max_chars: 10,
            ..JsonDiffOptions::default()
        };
        let error = compare_json(options).unwrap_err();
        assert_eq!(
            error,
            JsonDiffError::InputTooLong {
                side: JsonSide::Left,
                limit: 10
            }
        );
        assert_eq!(error.to_string(), "left JSON input must be at most 10 characters");

        let options = JsonDiffOptions {
            left: "1".to_string(),
            right: "x".repeat(11),
            max_chars: 10,
            ..JsonDiffOptions::default()
        };
        let error = compare_json(options).unwrap_err();
        assert!(error.to_string().contains("right JSON input"));
    }

    #[test]
    fn invalid_tolerances_error() {
        for tolerance in [Some(-0.1), Some(1.5), Some(f64::NAN)] {
            let options = JsonDiffOptions {
                relative_tolerance: tolerance,
                ..options("1", "1")
            };
            let error = compare_json(options).unwrap_err();
            assert!(matches!(error, JsonDiffError::InvalidTolerance { .. }));
            assert!(
                error
                    .to_string()
                    .contains("relative tolerance must be between 0.0 and 1.0")
            );
        }
    }

    #[test]
    fn whole_document_change_reports_the_root() {
        let report = compare("1", "2");
        assert_eq!(report.differences.len(), 1);
        assert_eq!(report.differences[0].path, "$");
        assert_eq!(report.differences[0].pointer, "");
        assert_eq!(report.to_text(), "~ $: 1 -> 2");
    }

    #[test]
    fn patch_describes_changes_and_orders_removals_first() {
        let report = compare(r#"{"a":1,"b":[1,2]}"#, r#"{"a":2,"b":[1],"c":true}"#);
        let patch: Value = serde_json::from_str(&report.to_patch()).unwrap();
        let operations = patch.as_array().unwrap();
        assert_eq!(operations.len(), 3);
        assert_eq!(operations[0]["op"], "remove");
        assert_eq!(operations[0]["path"], "/b/1");
        assert_eq!(operations[1]["op"], "replace");
        assert_eq!(operations[1]["path"], "/a");
        assert_eq!(operations[1]["value"], 2);
        assert_eq!(operations[2]["op"], "add");
        assert_eq!(operations[2]["path"], "/c");
        assert_eq!(operations[2]["value"], true);
    }

    #[test]
    fn patch_escapes_pointer_characters() {
        let report = compare(r#"{"a/b":1,"m~n":2}"#, r#"{"a/b":3,"m~n":4}"#);
        let patch: Value = serde_json::from_str(&report.to_patch()).unwrap();
        let operations = patch.as_array().unwrap();
        let pointers: Vec<&str> = operations
            .iter()
            .map(|operation| operation["path"].as_str().unwrap())
            .collect();
        assert!(pointers.contains(&"/a~1b"));
        assert!(pointers.contains(&"/m~0n"));
    }

    #[test]
    fn patch_multiple_array_removals_sort_descending() {
        let report = compare("[1,2,3]", "[1]");
        let patch: Value = serde_json::from_str(&report.to_patch()).unwrap();
        let operations = patch.as_array().unwrap();
        let pointers: Vec<&str> = operations
            .iter()
            .map(|operation| operation["path"].as_str().unwrap())
            .collect();
        assert_eq!(pointers, vec!["/2", "/1"]);
    }

    #[test]
    fn long_values_are_truncated_in_the_report() {
        let report = compare(&format!(r#"{{"s":"{}"}}"#, "x".repeat(200)), r#"{"s":"y"}"#);
        let text = report.to_text();
        assert!(text.contains('…'));
        assert!(text.len() < 120);
    }

    #[test]
    fn unicode_keys_and_values_are_reported() {
        let report = compare(r#"{"名前":"こんにちは"}"#, r#"{"名前":"さようなら"}"#);
        assert_eq!(report.differences.len(), 1);
        assert_eq!(report.differences[0].path, "$[\"名前\"]");
        assert_eq!(report.to_text(), "~ $[\"名前\"]: \"こんにちは\" -> \"さようなら\"");
    }

    #[test]
    fn special_characters_in_keys_use_quoted_paths() {
        let report = compare(r#"{"weird key":1}"#, r#"{"weird key":2}"#);
        assert_eq!(report.differences[0].path, "$[\"weird key\"]");
    }

    #[test]
    fn deep_nesting_stays_bounded() {
        let mut left = json!(0);
        let mut right = json!(0);
        for _ in 0..100 {
            left = json!([left]);
            right = json!([right]);
        }
        let report = compare(&left.to_string(), &right.to_string());
        assert!(report.is_equal());
    }

    #[test]
    fn options_defaults_are_sane() {
        let defaults = JsonDiffOptions::default();
        assert_eq!(defaults.left, "");
        assert_eq!(defaults.right, "");
        assert!(!defaults.ignore_array_order);
        assert_eq!(defaults.relative_tolerance, None);
        assert!(defaults.ignore_keys.is_empty());
        assert_eq!(defaults.max_chars, MAX_CHARS_PER_SIDE);
    }
}

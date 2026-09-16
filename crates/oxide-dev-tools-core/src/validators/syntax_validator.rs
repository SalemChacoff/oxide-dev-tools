//! JSON, YAML, and XML syntax validation.
//!
//! [`validate_syntax`] checks whether a document is well-formed for one of
//! the three formats:
//!
//! - JSON — strict RFC 8259 parsing via [`serde_json`]; parser messages
//!   keep their line/column positions.
//! - YAML — parsing via [`serde_yaml_ng`] (single-document streams);
//!   anchors and aliases are resolved by the parser.
//! - XML — a [`quick_xml`] well-formedness scan with end-tag checking.
//!   `<!DOCTYPE>` declarations are rejected unless
//!   [`SyntaxOptions::allow_dtd`] is set, unknown entity references are
//!   rejected, and element nesting is capped at [`SyntaxOptions::max_depth`]
//!   (default: [`MAX_DEPTH`]).
//!
//! The validator performs no semantic checks and never resolves external
//! resources; syntax is checked locally.

use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, escape};
use serde_json::Value as JsonValue;
use serde_yaml_ng::Value as YamlValue;

use crate::converters::doc_converter::MAX_DEPTH;

// -------- Public API --------

/// The document format that was validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxFormat {
    /// JSON (RFC 8259).
    Json,
    /// YAML, parsed as a single document.
    Yaml,
    /// XML 1.0 well-formedness.
    Xml,
}

/// Options for [`validate_syntax`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyntaxOptions {
    /// The document text to validate (surrounding whitespace is ignored).
    pub input: String,
    /// Accept XML `<!DOCTYPE>` declarations; they are reported as a warning
    /// instead of an issue. Applies to [`SyntaxKind::Xml`] only.
    pub allow_dtd: bool,
    /// Maximum XML element nesting depth; `None` means [`MAX_DEPTH`].
    /// Applies to [`SyntaxKind::Xml`] only.
    pub max_depth: Option<usize>,
}

/// One syntax validation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxKind {
    /// Validate a JSON document.
    Json(SyntaxOptions),
    /// Validate a YAML document.
    Yaml(SyntaxOptions),
    /// Validate an XML document.
    Xml(SyntaxOptions),
}

/// The result of validating a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxReport {
    /// Whether the document is well-formed for the chosen format.
    pub valid: bool,
    /// The format the document was validated against.
    pub format: SyntaxFormat,
    /// The document text, without surrounding whitespace.
    pub input: String,
    /// The top-level construct: the XML root element name, or the kind of
    /// the JSON/YAML top-level value (`"object"`, `"array"`, ...).
    pub root: Option<String>,
    /// The nesting depth of the document (1 for a top-level scalar).
    pub depth: usize,
    /// Reasons the document is invalid.
    pub issues: Vec<String>,
    /// Valid but unusual or discouraged constructs.
    pub warnings: Vec<String>,
}

/// Validate a document's syntax and produce a detailed report.
///
/// Validation never fails with an error and never panics: an invalid
/// document is reported via [`SyntaxReport::valid`] and
/// [`SyntaxReport::issues`].
pub fn validate_syntax(kind: SyntaxKind) -> SyntaxReport {
    let (format, options) = match kind {
        SyntaxKind::Json(options) => (SyntaxFormat::Json, options),
        SyntaxKind::Yaml(options) => (SyntaxFormat::Yaml, options),
        SyntaxKind::Xml(options) => (SyntaxFormat::Xml, options),
    };
    let input = options.input.trim().to_string();
    let mut report = SyntaxReport {
        valid: false,
        format,
        input: input.clone(),
        root: None,
        depth: 0,
        issues: Vec::new(),
        warnings: Vec::new(),
    };
    if report.input.is_empty() {
        report.issues.push("input is empty".to_string());
        return report;
    }
    match format {
        SyntaxFormat::Json => check_json(&input, &mut report),
        SyntaxFormat::Yaml => check_yaml(&input, &mut report),
        SyntaxFormat::Xml => check_xml(&input, &options, &mut report),
    }
    report.valid = report.issues.is_empty();
    report
}

// -------- JSON checking --------

fn check_json(input: &str, report: &mut SyntaxReport) {
    match serde_json::from_str::<JsonValue>(input) {
        Ok(value) => {
            report.root = Some(json_kind_name(&value).to_string());
            report.depth = json_depth(&value);
        }
        Err(error) => report.issues.push(error.to_string()),
    }
}

fn json_kind_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

fn json_depth(value: &JsonValue) -> usize {
    match value {
        JsonValue::Array(items) => 1 + items.iter().map(json_depth).max().unwrap_or(0),
        JsonValue::Object(object) => 1 + object.values().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

// -------- YAML checking --------

fn check_yaml(input: &str, report: &mut SyntaxReport) {
    match serde_yaml_ng::from_str::<YamlValue>(input) {
        Ok(value) => {
            report.root = Some(yaml_kind_name(&value).to_string());
            report.depth = yaml_depth(&value);
        }
        Err(error) => report.issues.push(error.to_string()),
    }
}

fn yaml_kind_name(value: &YamlValue) -> &'static str {
    match value {
        YamlValue::Null => "null",
        YamlValue::Bool(_) => "boolean",
        YamlValue::Number(_) => "number",
        YamlValue::String(_) => "string",
        YamlValue::Sequence(_) => "array",
        YamlValue::Mapping(_) => "object",
        YamlValue::Tagged(_) => "tagged",
    }
}

fn yaml_depth(value: &YamlValue) -> usize {
    match value {
        YamlValue::Sequence(items) => 1 + items.iter().map(yaml_depth).max().unwrap_or(0),
        YamlValue::Mapping(mapping) => 1 + mapping.values().map(yaml_depth).max().unwrap_or(0),
        YamlValue::Tagged(tagged) => yaml_depth(&tagged.value),
        _ => 1,
    }
}

// -------- XML checking --------

/// Structural state of the XML well-formedness scan.
struct XmlScan {
    depth: usize,
    roots: usize,
    max_depth: usize,
    limit_hit: bool,
}

impl XmlScan {
    fn new() -> Self {
        Self {
            depth: 0,
            roots: 0,
            max_depth: 0,
            limit_hit: false,
        }
    }

    fn enter(&mut self, start: &BytesStart<'_>, limit: usize, report: &mut SyntaxReport) {
        self.depth += 1;
        self.max_depth = self.max_depth.max(self.depth);
        if self.depth == 1 {
            self.record_root(start.name().local_name().as_ref(), report);
        }
        check_depth(self.depth, limit, &mut self.limit_hit, report);
    }

    fn enter_empty(&mut self, start: &BytesStart<'_>, limit: usize, report: &mut SyntaxReport) {
        if self.depth == 0 {
            self.record_root(start.name().local_name().as_ref(), report);
        }
        self.max_depth = self.max_depth.max(self.depth + 1);
        check_depth(self.depth + 1, limit, &mut self.limit_hit, report);
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn text(&self, text: &str, report: &mut SyntaxReport) {
        if self.depth == 0 && !text.trim().is_empty() {
            let message = if self.roots > 0 {
                "content after the root element"
            } else {
                "text before the root element"
            };
            report.issues.push(message.to_string());
        }
    }

    fn record_root(&mut self, name: &[u8], report: &mut SyntaxReport) {
        self.roots += 1;
        if self.roots > 1 {
            report
                .issues
                .push("XML document has more than one root element".to_string());
        } else {
            report.root = Some(String::from_utf8_lossy(name).into_owned());
        }
    }

    fn finish(&self, report: &mut SyntaxReport) {
        if self.depth > 0 {
            report
                .issues
                .push("unexpected end of input (unclosed element)".to_string());
            return;
        }
        if self.roots == 0 {
            report.issues.push("XML document has no root element".to_string());
        }
    }
}

fn check_depth(depth: usize, limit: usize, limit_hit: &mut bool, report: &mut SyntaxReport) {
    if depth > limit && !*limit_hit {
        *limit_hit = true;
        report
            .issues
            .push(format!("document nesting exceeds the maximum depth of {limit}"));
    }
}

fn dtd_policy(options: &SyntaxOptions, report: &mut SyntaxReport) {
    if options.allow_dtd {
        report
            .warnings
            .push("XML document contains a DOCTYPE declaration".to_string());
    } else {
        report
            .issues
            .push("XML DOCTYPE/DTD declarations are not allowed".to_string());
    }
}

fn check_xml(input: &str, options: &SyntaxOptions, report: &mut SyntaxReport) {
    let mut reader = Reader::from_str(input);
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let limit = options.max_depth.unwrap_or(MAX_DEPTH);
    let mut scan = XmlScan::new();

    loop {
        let event = match next_xml_event(&mut reader, &mut buffer) {
            Ok(event) => event,
            Err(message) => {
                report.issues.push(message);
                return;
            }
        };
        match event {
            XmlEvent::Start(start) => scan.enter(&start, limit, report),
            XmlEvent::Empty(start) => scan.enter_empty(&start, limit, report),
            XmlEvent::End => scan.leave(),
            XmlEvent::Text(text) => scan.text(&text, report),
            XmlEvent::DocType => dtd_policy(options, report),
            XmlEvent::Eof => {
                scan.finish(report);
                break;
            }
            XmlEvent::Ignored => {}
        }
    }
    report.depth = scan.max_depth;
}

/// Normalized view of the next XML event; read and decode failures become
/// issue messages so the scan loop stays small.
enum XmlEvent {
    Start(BytesStart<'static>),
    Empty(BytesStart<'static>),
    End,
    Text(String),
    DocType,
    Eof,
    Ignored,
}

fn next_xml_event(reader: &mut Reader<&[u8]>, buffer: &mut Vec<u8>) -> Result<XmlEvent, String> {
    let event = reader
        .read_event_into(buffer)
        .map_err(|error| error.to_string())?
        .into_owned();
    Ok(match event {
        Event::Start(start) => XmlEvent::Start(start),
        Event::End(_) => XmlEvent::End,
        Event::Empty(empty) => XmlEvent::Empty(empty),
        Event::Text(text) => {
            XmlEvent::Text(unescape_text(text.decode().map_err(|error| error.to_string())?.into_owned())?)
        }
        Event::CData(data) => XmlEvent::Text(data.decode().map_err(|error| error.to_string())?.into_owned()),
        Event::GeneralRef(reference) => XmlEvent::Text(unescape_reference(&reference)?),
        Event::DocType(_) => XmlEvent::DocType,
        Event::Eof => XmlEvent::Eof,
        Event::Decl(_) | Event::Comment(_) | Event::PI(_) => XmlEvent::Ignored,
    })
}

/// Text events never contain entity references, but unescaping is a
/// harmless safety net if that ever changes.
fn unescape_text(text: String) -> Result<String, String> {
    let decoded = escape::unescape(&text).map_err(|error| error.to_string())?;
    Ok(decoded.into_owned())
}

/// Resolve a `&name;` reference: predefined entities and numeric character
/// references are valid; anything else is an unknown entity.
fn unescape_reference(reference: &BytesRef<'_>) -> Result<String, String> {
    let name = String::from_utf8_lossy(reference.as_ref()).into_owned();
    let full = format!("&{name};");
    let decoded = escape::unescape(&full).map_err(|_| format!("unknown XML entity &{name};"))?;
    Ok(decoded.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(input: &str) -> SyntaxOptions {
        SyntaxOptions {
            input: input.to_string(),
            ..SyntaxOptions::default()
        }
    }

    fn json(input: &str) -> SyntaxReport {
        validate_syntax(SyntaxKind::Json(options(input)))
    }

    fn yaml(input: &str) -> SyntaxReport {
        validate_syntax(SyntaxKind::Yaml(options(input)))
    }

    fn xml(input: &str) -> SyntaxReport {
        validate_syntax(SyntaxKind::Xml(options(input)))
    }

    // -------- Shared behavior --------

    #[test]
    fn empty_input_is_reported() {
        for report in [json(""), yaml(""), xml("")] {
            assert!(!report.valid);
            assert!(report.issues.iter().any(|issue| issue.contains("empty")));
        }
        for report in [json("   \n"), yaml("   \n"), xml("   \n")] {
            assert!(!report.valid);
        }
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        assert!(json("  {\"a\":1}  ").valid);
        assert!(yaml("  a: 1\n").valid);
        assert!(xml("  <root/>  ").valid);
        assert_eq!(xml("  <root/>  ").input, "<root/>");
    }

    #[test]
    fn options_defaults() {
        let defaults = SyntaxOptions::default();
        assert!(defaults.input.is_empty());
        assert!(!defaults.allow_dtd);
        assert_eq!(defaults.max_depth, None);
    }

    // -------- JSON --------

    #[test]
    fn valid_json_documents() {
        for input in [
            r#"{"a":1}"#,
            "[]",
            "{}",
            r#""text""#,
            "42",
            "-1.5e3",
            "true",
            "null",
            r#"{"nested":{"deep":[1,2,{"x":null}]}}"#,
            "  {\"a\":1}\n",
        ] {
            let report = json(input);
            assert!(report.valid, "{input}: {:?}", report.issues);
        }
    }

    #[test]
    fn json_reports_root_kind_and_depth() {
        let report = json(r#"{"a":1}"#);
        assert_eq!(report.format, SyntaxFormat::Json);
        assert_eq!(report.root.as_deref(), Some("object"));
        assert_eq!(report.depth, 2);

        assert_eq!(json("[1]").root.as_deref(), Some("array"));
        assert_eq!(json("[1]").depth, 2);
        assert_eq!(json(r#""text""#).root.as_deref(), Some("string"));
        assert_eq!(json(r#""text""#).depth, 1);
        assert_eq!(json("null").root.as_deref(), Some("null"));
        assert_eq!(json("null").depth, 1);
        assert_eq!(json(r#"{"a":[1,{"b":[2]}]}"#).depth, 5);
    }

    #[test]
    fn invalid_json_documents() {
        for input in ["{broken", r#"{"a":}"#, "[1,]", r#"{"a":1} extra"#, "\u{0}"] {
            let report = json(input);
            assert!(!report.valid, "{input} should be invalid");
            assert!(!report.issues.is_empty());
        }
        assert!(json("{broken").issues[0].contains("line"));
    }

    // -------- YAML --------

    #[test]
    fn valid_yaml_documents() {
        for input in [
            "a: 1\n",
            "- 1\n- 2\n",
            "a: {b: [1, 2]}\n",
            "base: &base\n  x: 1\ncopy: *base\n",
            "quoted: \"text\"\n",
            "42\n",
            "null\n",
            "tagged: !!str hello\n",
        ] {
            let report = yaml(input);
            assert!(report.valid, "{input}: {:?}", report.issues);
        }
    }

    #[test]
    fn yaml_reports_root_kind_and_depth() {
        let report = yaml("a: 1\n");
        assert_eq!(report.format, SyntaxFormat::Yaml);
        assert_eq!(report.root.as_deref(), Some("object"));
        assert_eq!(report.depth, 2);

        assert_eq!(yaml("- 1\n").root.as_deref(), Some("array"));
        assert_eq!(yaml("- 1\n").depth, 2);
        assert_eq!(yaml("text\n").root.as_deref(), Some("string"));
        assert_eq!(yaml("text\n").depth, 1);
        assert_eq!(yaml("a: [1, [2]]\n").depth, 4);
    }

    #[test]
    fn invalid_yaml_documents() {
        for input in ["a: [1, 2", "\"unclosed", "{a: 1", "a: 1\n---\nb: 2\n"] {
            let report = yaml(input);
            assert!(!report.valid, "{input} should be invalid");
            assert!(!report.issues.is_empty());
        }
        assert!(
            yaml("a: 1\n---\nb: 2\n")
                .issues
                .iter()
                .any(|issue| issue.contains("more than one document"))
        );
    }

    // -------- XML --------

    #[test]
    fn valid_xml_documents() {
        for input in [
            "<root/>",
            "<root></root>",
            "<root><a>1</a></root>",
            "<r a=\"1\">text</r>",
            "<r><![CDATA[<not>markup</not>]]></r>",
            "<?xml version=\"1.0\"?><r><!-- comment --><a/></r>",
            "<r>a &amp; b &#65; &#x42;</r>",
            "<r>\n  <a/>\n</r>",
            "<ns:r xmlns:ns=\"urn\"><ns:a/></ns:r>",
        ] {
            let report = xml(input);
            assert!(report.valid, "{input}: {:?}", report.issues);
        }
    }

    #[test]
    fn xml_reports_root_and_depth() {
        let report = xml("<root><a><b/></a></root>");
        assert_eq!(report.format, SyntaxFormat::Xml);
        assert_eq!(report.root.as_deref(), Some("root"));
        assert_eq!(report.depth, 3);
        assert!(report.warnings.is_empty());

        assert_eq!(xml("<root/>").depth, 1);
        assert_eq!(xml("<root/>").root.as_deref(), Some("root"));
    }

    #[test]
    fn invalid_xml_documents() {
        let cases = [
            ("<root><a></root>", "expected `</a>`"),
            ("<root><a>", "unclosed"),
            ("<a/><b/>", "more than one root element"),
            ("text<root/>", "text before the root element"),
            ("<root/>text", "content after the root element"),
            ("</root>", "does not match any open tag"),
            ("<root/></root>", "does not match any open tag"),
            ("<root>&unknown;</root>", "unknown XML entity"),
            ("<!DOCTYPE root><root/>", "DOCTYPE/DTD declarations are not allowed"),
        ];
        for (input, expected) in cases {
            let report = xml(input);
            assert!(!report.valid, "{input} should be invalid");
            assert!(
                report.issues.iter().any(|issue| issue.contains(expected)),
                "{input}: expected an issue containing {expected:?}, got {:?}",
                report.issues
            );
        }
    }

    #[test]
    fn xml_without_root_is_reported() {
        let report = xml("<!-- nothing here -->");
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("no root element")));
    }

    #[test]
    fn xml_dtd_policy() {
        let rejected = xml("<!DOCTYPE root><root/>");
        assert!(!rejected.valid);
        assert!(rejected.issues.iter().any(|issue| issue.contains("DOCTYPE")));

        let accepted = validate_syntax(SyntaxKind::Xml(SyntaxOptions {
            allow_dtd: true,
            ..options("<!DOCTYPE root><root/>")
        }));
        assert!(accepted.valid, "{:?}", accepted.issues);
        assert!(accepted.warnings.iter().any(|warning| warning.contains("DOCTYPE")));
    }

    #[test]
    fn xml_nesting_depth_is_capped() {
        let deep = format!("{}root{}", "<a>".repeat(MAX_DEPTH + 1), "</a>".repeat(MAX_DEPTH + 1));
        let report = xml(&deep);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("maximum depth")));
        assert_eq!(report.depth, MAX_DEPTH + 1);
    }

    #[test]
    fn xml_custom_max_depth() {
        let nested = "<r><a><b/></a></r>";
        let report = validate_syntax(SyntaxKind::Xml(SyntaxOptions {
            max_depth: Some(2),
            ..options(nested)
        }));
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("maximum depth of 2")));

        let shallow = validate_syntax(SyntaxKind::Xml(SyntaxOptions {
            max_depth: Some(2),
            ..options("<r><a/></r>")
        }));
        assert!(shallow.valid, "{:?}", shallow.issues);
    }

    #[test]
    fn validation_never_panics_on_garbage() {
        for input in ["\u{1}\u{2}", "]]>", "<", "&", &"{".repeat(500)] {
            let _ = json(input);
            let _ = yaml(input);
            let _ = xml(input);
        }
    }
}

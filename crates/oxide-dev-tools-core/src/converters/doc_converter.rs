//! Document format conversion: JSON ↔ YAML ↔ XML.
//!
//! # Mapping rules
//!
//! JSON and YAML share a data model, so JSON ↔ YAML conversion is exact:
//! only the syntax changes (YAML mapping keys that are not strings are
//! rendered in their string form; tagged YAML values are rejected).
//!
//! XML has no data-model equivalent, so these rules apply to conversions
//! that involve XML:
//!
//! - The JSON/YAML value becomes the content of a single root element,
//!   named `root` by default (override with [`DocOptions::root_name`]).
//! - Object keys become child element names; array items repeat the name
//!   of the element they are stored under. A top-level array is wrapped
//!   as repeated `<item>` elements inside the root element.
//! - Strings, numbers, and booleans become text content; `null` becomes
//!   an empty element (`<name/>`).
//! - Object keys must be valid XML element names; invalid names are
//!   rejected with [`DocError::InvalidXmlName`] instead of being mangled.
//! - When reading XML, attributes become `"@name"` string entries, and
//!   text inside elements that also carry attributes or child elements
//!   becomes a `"#text"` entry. All text stays a string — no number or
//!   boolean inference is attempted.
//! - Repeated child element names become a JSON array; an element without
//!   attributes, children, or text becomes `null`.
//! - Whitespace-only text between child elements (indentation) is dropped;
//!   text inside text-only elements is preserved verbatim.
//!
//! # Security
//!
//! - XML input is parsed with `quick-xml`, which never resolves external
//!   entities; `<!DOCTYPE>` declarations are rejected outright, unknown
//!   entity references are rejected, and element nesting is capped at
//!   [`MAX_DEPTH`].
//! - JSON and YAML parsing use serde with its built-in recursion limits;
//!   YAML tags are rejected.
//! - XML output is written with quick-xml's escaping enabled.

use std::fmt;

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer, escape};
use serde_json::map::Entry as JsonMapEntry;
use serde_json::{Map, Number, Value};

/// Maximum element nesting depth accepted when reading or writing XML.
/// Protects against stack exhaustion from deeply nested input.
pub const MAX_DEPTH: usize = 512;

/// Default name of the root element when converting data into XML.
pub const DEFAULT_ROOT_NAME: &str = "root";

/// Element name used for items of a top-level array converted into XML.
const ROOT_ARRAY_ITEM_NAME: &str = "item";

/// Key used for text content of elements that also carry attributes or children.
const TEXT_KEY: &str = "#text";

/// Errors that can occur when converting documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocError {
    /// The input is not valid JSON.
    InvalidJson(String),
    /// The input is not valid YAML.
    InvalidYaml(String),
    /// The input is not well-formed XML.
    InvalidXml(String),
    /// A JSON or YAML key is not a valid XML element name.
    InvalidXmlName(String),
    /// The document nests deeper than [`MAX_DEPTH`] elements.
    NestingTooDeep(usize),
    /// The XML document contains a `<!DOCTYPE>` declaration.
    DtdNotAllowed,
    /// The XML document references an unknown entity.
    UnknownEntity(String),
    /// The XML document has more than one top-level element.
    MultipleRootElements,
    /// The XML document has no root element.
    MissingRoot,
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocError::InvalidJson(message) => write!(f, "invalid JSON: {message}"),
            DocError::InvalidYaml(message) => write!(f, "invalid YAML: {message}"),
            DocError::InvalidXml(message) => write!(f, "invalid XML: {message}"),
            DocError::InvalidXmlName(name) => write!(
                f,
                "{name:?} is not a valid XML element name (names must start with a letter, '_', or ':' \
                 and continue with letters, digits, '_', '-', '.', or ':')"
            ),
            DocError::NestingTooDeep(limit) => {
                write!(f, "document nesting exceeds the maximum depth of {limit}")
            }
            DocError::DtdNotAllowed => write!(f, "XML DOCTYPE/DTD declarations are not allowed"),
            DocError::UnknownEntity(name) => write!(f, "unknown XML entity &{name};"),
            DocError::MultipleRootElements => write!(f, "XML document has more than one root element"),
            DocError::MissingRoot => write!(f, "XML document has no root element"),
        }
    }
}

impl std::error::Error for DocError {}

/// Options for a document conversion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocOptions {
    /// The input document text (JSON, YAML, or XML).
    pub input: String,
    /// Name of the root XML element produced by conversions into XML.
    pub root_name: Option<String>,
    /// Pretty-print JSON and XML output with 2-space indentation.
    pub pretty: bool,
}

/// One document conversion between JSON, YAML, and XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocKind {
    /// Convert JSON input to YAML.
    Json2Yaml(DocOptions),
    /// Convert YAML input to JSON.
    Yaml2Json(DocOptions),
    /// Convert JSON input to XML.
    Json2Xml(DocOptions),
    /// Convert XML input to JSON.
    Xml2Json(DocOptions),
    /// Convert YAML input to XML.
    Yaml2Xml(DocOptions),
    /// Convert XML input to YAML.
    Xml2Yaml(DocOptions),
}

/// Convert a document between JSON, YAML, and XML formats.
pub fn convert_doc(kind: DocKind) -> Result<String, DocError> {
    match kind {
        DocKind::Json2Yaml(options) => json_to_yaml(&options.input),
        DocKind::Yaml2Json(options) => yaml_to_json(&options.input),
        DocKind::Json2Xml(options) => {
            let value = parse_json(&options.input)?;
            value_to_xml(&value, &options)
        }
        DocKind::Xml2Json(options) => xml_to_json(&options.input, options.pretty),
        DocKind::Yaml2Xml(options) => {
            let value = parse_yaml(&options.input)?;
            value_to_xml(&value, &options)
        }
        DocKind::Xml2Yaml(options) => {
            let value = parse_xml(&options.input)?;
            serde_yaml_ng::to_string(&value).map_err(|error| DocError::InvalidYaml(error.to_string()))
        }
    }
}

// -------- JSON and YAML parsing --------

fn parse_json(input: &str) -> Result<Value, DocError> {
    serde_json::from_str(input).map_err(|error| DocError::InvalidJson(error.to_string()))
}

fn json_to_yaml(input: &str) -> Result<String, DocError> {
    let value = parse_json(input)?;
    serde_yaml_ng::to_string(&value).map_err(|error| DocError::InvalidYaml(error.to_string()))
}

fn yaml_to_json(input: &str) -> Result<String, DocError> {
    let value = parse_yaml(input)?;
    serde_json::to_string(&value).map_err(|error| DocError::InvalidJson(error.to_string()))
}

fn parse_yaml(input: &str) -> Result<Value, DocError> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(input).map_err(|error| DocError::InvalidYaml(error.to_string()))?;
    yaml_value_to_json(value)
}

/// Convert a YAML value into the equivalent JSON value. Mapping keys that
/// are not strings are rendered in their string form; tagged values and
/// non-finite numbers are rejected.
fn yaml_value_to_json(value: serde_yaml_ng::Value) -> Result<Value, DocError> {
    use serde_yaml_ng::Value as YamlValue;
    match value {
        YamlValue::Null => Ok(Value::Null),
        YamlValue::Bool(flag) => Ok(Value::Bool(flag)),
        YamlValue::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(Value::Number(Number::from(integer)))
            } else if let Some(integer) = number.as_u64() {
                Ok(Value::Number(Number::from(integer)))
            } else if let Some(float) = number.as_f64() {
                Number::from_f64(float)
                    .map(Value::Number)
                    .ok_or_else(|| DocError::InvalidYaml("non-finite number has no JSON equivalent".to_string()))
            } else {
                Err(DocError::InvalidYaml("unrepresentable number".to_string()))
            }
        }
        YamlValue::String(text) => Ok(Value::String(text)),
        YamlValue::Sequence(items) => items
            .into_iter()
            .map(yaml_value_to_json)
            .collect::<Result<Vec<Value>, DocError>>()
            .map(Value::Array),
        YamlValue::Mapping(entries) => {
            let mut object = Map::new();
            for (key, value) in entries {
                object.insert(yaml_key_to_string(key)?, yaml_value_to_json(value)?);
            }
            Ok(Value::Object(object))
        }
        YamlValue::Tagged(_) => Err(DocError::InvalidYaml("tagged YAML values are not supported".to_string())),
    }
}

fn yaml_key_to_string(key: serde_yaml_ng::Value) -> Result<String, DocError> {
    use serde_yaml_ng::Value as YamlValue;
    match key {
        YamlValue::Null => Ok("null".to_string()),
        YamlValue::Bool(flag) => Ok(flag.to_string()),
        YamlValue::Number(number) => Ok(number.to_string()),
        YamlValue::String(text) => Ok(text),
        YamlValue::Sequence(_) | YamlValue::Mapping(_) | YamlValue::Tagged(_) => {
            Err(DocError::InvalidYaml("complex YAML mapping keys are not supported".to_string()))
        }
    }
}

// -------- Writing XML --------

fn value_to_xml(value: &Value, options: &DocOptions) -> Result<String, DocError> {
    let root_name = options.root_name.as_deref().unwrap_or(DEFAULT_ROOT_NAME);
    validate_xml_name(root_name)?;
    let mut writer = if options.pretty {
        Writer::new_with_indent(Vec::new(), b' ', 2)
    } else {
        Writer::new(Vec::new())
    };
    match value {
        Value::Array(items) => {
            // A top-level array has no element name of its own: wrap the
            // items in <item> elements inside the root element.
            write_start(&mut writer, root_name)?;
            for item in items {
                write_element(&mut writer, ROOT_ARRAY_ITEM_NAME, item, 1)?;
            }
            write_end(&mut writer, root_name)?;
        }
        other => write_element(&mut writer, root_name, other, 0)?,
    }
    let bytes = writer.into_inner();
    String::from_utf8(bytes).map_err(|_| DocError::InvalidXml("output is not valid UTF-8".to_string()))
}

/// Write one JSON value as an XML element (recursively for objects and arrays).
fn write_element(writer: &mut Writer<Vec<u8>>, name: &str, value: &Value, depth: usize) -> Result<(), DocError> {
    if depth >= MAX_DEPTH {
        return Err(DocError::NestingTooDeep(MAX_DEPTH));
    }
    validate_xml_name(name)?;
    match value {
        Value::Null => write_event(writer, Event::Empty(BytesStart::new(name)))?,
        Value::Bool(flag) => write_text(writer, name, &flag.to_string())?,
        Value::Number(number) => write_text(writer, name, &number.to_string())?,
        Value::String(text) => write_text(writer, name, text)?,
        Value::Array(items) => {
            for item in items {
                write_element(writer, name, item, depth + 1)?;
            }
        }
        Value::Object(entries) => {
            write_start(writer, name)?;
            for (key, child) in entries {
                write_element(writer, key, child, depth + 1)?;
            }
            write_end(writer, name)?;
        }
    }
    Ok(())
}

fn write_text(writer: &mut Writer<Vec<u8>>, name: &str, text: &str) -> Result<(), DocError> {
    write_start(writer, name)?;
    write_event(writer, Event::Text(BytesText::new(text)))?;
    write_end(writer, name)
}

fn write_start(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), DocError> {
    write_event(writer, Event::Start(BytesStart::new(name)))
}

fn write_end(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), DocError> {
    write_event(writer, Event::End(BytesEnd::new(name)))
}

fn write_event(writer: &mut Writer<Vec<u8>>, event: Event<'_>) -> Result<(), DocError> {
    writer
        .write_event(event)
        .map_err(|error| DocError::InvalidXml(error.to_string()))
}

/// Reject names that would produce malformed XML rather than mangling them.
fn validate_xml_name(name: &str) -> Result<(), DocError> {
    let mut characters = name.chars();
    let valid = match characters.next() {
        Some(first) => is_name_start(first) && characters.all(is_name_char),
        None => false,
    };
    if valid {
        Ok(())
    } else {
        Err(DocError::InvalidXmlName(name.to_string()))
    }
}

fn is_name_start(character: char) -> bool {
    character == '_' || character == ':' || character.is_alphabetic()
}

fn is_name_char(character: char) -> bool {
    is_name_start(character) || character.is_ascii_digit() || character == '-' || character == '.'
}

// -------- Reading XML --------

fn xml_to_json(input: &str, pretty: bool) -> Result<String, DocError> {
    let value = parse_xml(input)?;
    let result = if pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    };
    result.map_err(|error| DocError::InvalidJson(error.to_string()))
}

/// Normalized view of the next XML event. Structural rejections (DTD,
/// unknown entities) are mapped to errors here so call sites stay small.
enum NodeEvent {
    Start(BytesStart<'static>),
    Empty(BytesStart<'static>),
    End,
    Text(String),
    Eof,
    Ignored,
}

fn read_event(reader: &mut Reader<&[u8]>, buffer: &mut Vec<u8>) -> Result<NodeEvent, DocError> {
    let event = reader
        .read_event_into(buffer)
        .map_err(|error| DocError::InvalidXml(error.to_string()))?
        .into_owned();
    Ok(match event {
        Event::Start(start) => NodeEvent::Start(start),
        Event::End(_) => NodeEvent::End,
        Event::Empty(empty) => NodeEvent::Empty(empty),
        Event::Text(text) => {
            let decoded = text.decode().map_err(|error| DocError::InvalidXml(error.to_string()))?;
            // Text events never contain entity references, but unescaping is
            // a harmless safety net if that ever changes.
            NodeEvent::Text(unescape(decoded.into_owned())?)
        }
        Event::CData(data) => {
            // CDATA is literal character data: decode encoding only, never
            // resolve entities inside it.
            let decoded = data.decode().map_err(|error| DocError::InvalidXml(error.to_string()))?;
            NodeEvent::Text(decoded.into_owned())
        }
        Event::GeneralRef(reference) => {
            let name = String::from_utf8_lossy(reference.as_ref()).into_owned();
            // Rebuild the full `&name;` form so unescape resolves both
            // predefined entities and numeric character references.
            let full = format!("&{name};");
            match escape::unescape(&full) {
                Ok(decoded) => NodeEvent::Text(decoded.into_owned()),
                Err(_) => return Err(DocError::UnknownEntity(name)),
            }
        }
        Event::DocType(_) => return Err(DocError::DtdNotAllowed),
        Event::Eof => NodeEvent::Eof,
        Event::Decl(_) | Event::Comment(_) | Event::PI(_) => NodeEvent::Ignored,
    })
}

fn unescape(text: String) -> Result<String, DocError> {
    let decoded = escape::unescape(&text).map_err(|error| DocError::InvalidXml(error.to_string()))?;
    Ok(decoded.into_owned())
}

fn parse_xml(input: &str) -> Result<Value, DocError> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();

    // Locate the root element, skipping the declaration, comments, PIs,
    // and whitespace.
    let (name, content) = loop {
        match read_event(&mut reader, &mut buffer)? {
            NodeEvent::Start(start) => {
                let name = element_name(&start)?;
                let content = build_element(&mut reader, &mut buffer, start, 0)?;
                break (name, content);
            }
            NodeEvent::Empty(empty) => break (element_name(&empty)?, empty_element(empty)?),
            NodeEvent::Eof => return Err(DocError::MissingRoot),
            NodeEvent::Text(text) if !text.trim().is_empty() => {
                return Err(DocError::InvalidXml("text before the root element".to_string()));
            }
            NodeEvent::End => return Err(DocError::InvalidXml("closing tag before the root element".to_string())),
            NodeEvent::Ignored | NodeEvent::Text(_) => {}
        }
    };

    // Reject anything beyond the root element.
    loop {
        match read_event(&mut reader, &mut buffer)? {
            NodeEvent::Eof => break,
            NodeEvent::Start(_) | NodeEvent::Empty(_) => return Err(DocError::MultipleRootElements),
            NodeEvent::Text(text) if !text.trim().is_empty() => {
                return Err(DocError::InvalidXml("content after the root element".to_string()));
            }
            NodeEvent::End => return Err(DocError::InvalidXml("closing tag after the root element".to_string())),
            NodeEvent::Ignored | NodeEvent::Text(_) => {}
        }
    }
    Ok(object_with_root(&name, content))
}

/// Build the JSON value for an element whose start tag was already read.
fn build_element(
    reader: &mut Reader<&[u8]>,
    buffer: &mut Vec<u8>,
    start: BytesStart<'static>,
    depth: usize,
) -> Result<Value, DocError> {
    if depth >= MAX_DEPTH {
        return Err(DocError::NestingTooDeep(MAX_DEPTH));
    }
    let mut object = collect_attributes(&start)?;
    let mut text = String::new();
    loop {
        match read_event(reader, buffer)? {
            NodeEvent::Start(child) => {
                let name = element_name(&child)?;
                let value = build_element(reader, buffer, child, depth + 1)?;
                insert_child(&mut object, name, value);
            }
            NodeEvent::Empty(child) => {
                let name = element_name(&child)?;
                insert_child(&mut object, name, empty_element(child)?);
            }
            NodeEvent::Text(chunk) => text.push_str(&chunk),
            NodeEvent::End => break,
            NodeEvent::Eof => {
                return Err(DocError::InvalidXml("unexpected end of input (unclosed element)".to_string()));
            }
            NodeEvent::Ignored => {}
        }
    }
    if object.is_empty() {
        return Ok(if text.is_empty() {
            Value::Null
        } else {
            Value::String(text)
        });
    }
    // Whitespace-only text between child elements is indentation noise.
    if !text.trim().is_empty() {
        object.insert(TEXT_KEY.to_string(), Value::String(text));
    }
    Ok(Value::Object(object))
}

/// Attributes of an element become `"@name"` string entries.
fn collect_attributes(start: &BytesStart<'_>) -> Result<Map<String, Value>, DocError> {
    let mut object = Map::new();
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|error| DocError::InvalidXml(error.to_string()))?;
        let name = String::from_utf8(attribute.key.local_name().as_ref().to_vec())
            .map_err(|_| DocError::InvalidXml("attribute name is not valid UTF-8".to_string()))?;
        let value = String::from_utf8(attribute.value.into_owned())
            .map_err(|_| DocError::InvalidXml("attribute value is not valid UTF-8".to_string()))?;
        object.insert(format!("@{name}"), Value::String(unescape(value)?));
    }
    Ok(object)
}

fn empty_element(start: BytesStart<'static>) -> Result<Value, DocError> {
    let object = collect_attributes(&start)?;
    Ok(if object.is_empty() {
        Value::Null
    } else {
        Value::Object(object)
    })
}

fn element_name(start: &BytesStart<'_>) -> Result<String, DocError> {
    String::from_utf8(start.name().local_name().as_ref().to_vec())
        .map_err(|_| DocError::InvalidXml("element name is not valid UTF-8".to_string()))
}

/// Insert a child value, promoting duplicates of the same name to an array.
fn insert_child(object: &mut Map<String, Value>, name: String, value: Value) {
    match object.entry(name) {
        JsonMapEntry::Vacant(slot) => {
            slot.insert(value);
        }
        JsonMapEntry::Occupied(mut slot) => match slot.get_mut() {
            Value::Array(items) => items.push(value),
            existing => {
                let previous = existing.take();
                *existing = Value::Array(vec![previous, value]);
            }
        },
    }
}

fn object_with_root(name: &str, value: Value) -> Value {
    let mut object = Map::new();
    object.insert(name.to_string(), value);
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(kind: DocKind) -> String {
        convert_doc(kind).expect("conversion failed")
    }

    fn options(input: &str) -> DocOptions {
        DocOptions {
            input: input.to_string(),
            ..DocOptions::default()
        }
    }

    fn as_json(text: &str) -> Value {
        serde_json::from_str(text).expect("not JSON")
    }

    fn as_yaml(text: &str) -> serde_yaml_ng::Value {
        serde_yaml_ng::from_str(text).expect("not YAML")
    }

    // -------- JSON <-> YAML --------

    #[test]
    fn json_to_yaml_known_vector() {
        let output = convert(DocKind::Json2Yaml(options(r#"{"a":1,"b":[true,null,"x"],"c":{"d":2.5}}"#)));
        assert_eq!(output, "a: 1\nb:\n- true\n- null\n- x\nc:\n  d: 2.5\n");
    }

    #[test]
    fn yaml_to_json_known_vector() {
        let output = convert(DocKind::Yaml2Json(options("a: 1\nb:\n  - true\nc:\n  d: text\n")));
        assert_eq!(output, r#"{"a":1,"b":[true],"c":{"d":"text"}}"#);
    }

    #[test]
    fn yaml_non_string_keys_become_strings() {
        let output = convert(DocKind::Yaml2Json(options("1: one\ntrue: yes\nnull: empty\n")));
        assert_eq!(output, r#"{"1":"one","null":"empty","true":"yes"}"#);
    }

    #[test]
    fn yaml_anchors_resolve_to_values() {
        let output = convert(DocKind::Yaml2Json(options("base: &anchor 7\ncopy: *anchor\n")));
        assert_eq!(as_json(&output), as_json(r#"{"base":7,"copy":7}"#));
    }

    #[test]
    fn yaml_tagged_values_are_rejected() {
        let error = convert_doc(DocKind::Yaml2Json(options("value: !custom something\n"))).unwrap_err();
        assert!(error.to_string().contains("tagged YAML"), "{error}");
    }

    #[test]
    fn invalid_json_errors_with_message() {
        let error = convert_doc(DocKind::Json2Yaml(options("{not json"))).unwrap_err();
        assert!(error.to_string().contains("invalid JSON"), "{error}");
    }

    #[test]
    fn invalid_yaml_errors_with_message() {
        let error = convert_doc(DocKind::Yaml2Json(options("a: [unclosed"))).unwrap_err();
        assert!(error.to_string().contains("invalid YAML"), "{error}");
    }

    // -------- JSON -> XML --------

    #[test]
    fn json_to_xml_known_vector() {
        let output = convert(DocKind::Json2Xml(options(r#"{"a":1,"b":[true,null],"c":{"d":"text"}}"#)));
        assert_eq!(output, r#"<root><a>1</a><b>true</b><b/><c><d>text</d></c></root>"#);
    }

    #[test]
    fn json_to_xml_custom_root_name() {
        let mut opts = options(r#"{"a":1}"#);
        opts.root_name = Some("data".to_string());
        let output = convert(DocKind::Json2Xml(opts));
        assert_eq!(output, r#"<data><a>1</a></data>"#);
    }

    #[test]
    fn json_to_xml_escapes_special_characters() {
        let output = convert(DocKind::Json2Xml(options(r#"{"number":3.5,"text":"<&>"}"#)));
        assert_eq!(output, r#"<root><number>3.5</number><text>&lt;&amp;&gt;</text></root>"#);
    }

    #[test]
    fn json_to_xml_quotes_roundtrip() {
        let xml = convert(DocKind::Json2Xml(options(r#"{"text":"\"'<>"}"#)));
        let json = convert(DocKind::Xml2Json(options(&xml)));
        assert_eq!(as_json(&json), as_json(r#"{"root":{"text":"\"'<>"}}"#));
    }

    #[test]
    fn json_to_xml_top_level_scalar_and_array() {
        let scalar = convert(DocKind::Json2Xml(options("42")));
        assert_eq!(scalar, "<root>42</root>");
        let array = convert(DocKind::Json2Xml(options("[1,2]")));
        assert_eq!(array, "<root><item>1</item><item>2</item></root>");
    }

    #[test]
    fn json_to_xml_rejects_invalid_names() {
        let error = convert_doc(DocKind::Json2Xml(options(r#"{"1abc":true}"#))).unwrap_err();
        assert!(error.to_string().contains("not a valid XML element name"), "{error}");

        let mut opts = options("{}");
        opts.root_name = Some("1 root".to_string());
        let error = convert_doc(DocKind::Json2Xml(opts)).unwrap_err();
        assert!(error.to_string().contains("not a valid XML element name"), "{error}");
    }

    #[test]
    fn json_to_xml_pretty_parses_back() {
        let mut opts = options(r#"{"a":[1,2]}"#);
        opts.pretty = true;
        let output = convert(DocKind::Json2Xml(opts));
        assert!(output.contains('\n'));
        assert_eq!(as_json(&convert(DocKind::Xml2Json(options(&output)))), as_json(r#"{"root":{"a":["1","2"]}}"#));
    }

    // -------- XML -> JSON --------

    #[test]
    fn xml_to_json_known_vector() {
        let output = convert(DocKind::Xml2Json(options(r#"<root><a>1</a><b>x</b></root>"#)));
        assert_eq!(output, r#"{"root":{"a":"1","b":"x"}}"#);
    }

    #[test]
    fn xml_to_json_attributes_and_text() {
        let output = convert(DocKind::Xml2Json(options(r##"<root id="7">hello</root>"##)));
        assert_eq!(output, r##"{"root":{"#text":"hello","@id":"7"}}"##);
    }

    #[test]
    fn xml_to_json_repeated_elements_become_arrays() {
        let output = convert(DocKind::Xml2Json(options("<root><item>a</item><item>b</item></root>")));
        assert_eq!(output, r#"{"root":{"item":["a","b"]}}"#);
    }

    #[test]
    fn xml_to_json_empty_element_is_null() {
        let output = convert(DocKind::Xml2Json(options("<root><a/><b></b></root>")));
        assert_eq!(output, r#"{"root":{"a":null,"b":null}}"#);
    }

    #[test]
    fn xml_to_json_whitespace_indentation_is_dropped() {
        let output = convert(DocKind::Xml2Json(options("<root>\n  <a>1</a>\n  <b>2</b>\n</root>")));
        assert_eq!(output, r#"{"root":{"a":"1","b":"2"}}"#);
    }

    #[test]
    fn xml_to_json_text_is_preserved_verbatim() {
        let output = convert(DocKind::Xml2Json(options("<root><a> spaced </a></root>")));
        assert_eq!(output, r#"{"root":{"a":" spaced "}}"#);
    }

    #[test]
    fn xml_to_json_cdata_is_literal() {
        let output = convert(DocKind::Xml2Json(options("<root><a><![CDATA[x &amp; y]]></a></root>")));
        assert_eq!(output, r#"{"root":{"a":"x &amp; y"}}"#);
    }

    #[test]
    fn xml_to_json_character_references_are_decoded() {
        let output = convert(DocKind::Xml2Json(options("<root><a>&#65;&#x42;</a></root>")));
        assert_eq!(output, r#"{"root":{"a":"AB"}}"#);
    }

    #[test]
    fn xml_to_json_declaration_and_comments_are_ignored() {
        let output =
            convert(DocKind::Xml2Json(options(r#"<?xml version="1.0"?><!-- note --><root><?go now?><a>1</a></root>"#)));
        assert_eq!(output, r#"{"root":{"a":"1"}}"#);
    }

    #[test]
    fn xml_to_json_doctype_is_rejected() {
        let error = convert_doc(DocKind::Xml2Json(options(r#"<!DOCTYPE root><root/>"#))).unwrap_err();
        assert_eq!(error, DocError::DtdNotAllowed);
    }

    #[test]
    fn xml_to_json_unknown_entity_is_rejected() {
        let error = convert_doc(DocKind::Xml2Json(options("<root><a>&nbsp;</a></root>"))).unwrap_err();
        assert_eq!(error, DocError::UnknownEntity("nbsp".to_string()));
    }

    #[test]
    fn xml_to_json_multiple_roots_are_rejected() {
        let error = convert_doc(DocKind::Xml2Json(options("<a/><b/>"))).unwrap_err();
        assert_eq!(error, DocError::MultipleRootElements);
    }

    #[test]
    fn xml_to_json_missing_root_is_rejected() {
        let error = convert_doc(DocKind::Xml2Json(options("   "))).unwrap_err();
        assert_eq!(error, DocError::MissingRoot);
    }

    #[test]
    fn xml_to_json_unclosed_element_is_rejected() {
        let error = convert_doc(DocKind::Xml2Json(options("<root><a></root>"))).unwrap_err();
        assert!(error.to_string().contains("invalid XML"), "{error}");
    }

    #[test]
    fn xml_to_json_nesting_depth_is_capped() {
        let mut xml = String::new();
        for _ in 0..=MAX_DEPTH {
            xml.push_str("<a>");
        }
        for _ in 0..=MAX_DEPTH {
            xml.push_str("</a>");
        }
        let error = convert_doc(DocKind::Xml2Json(options(&xml))).unwrap_err();
        assert_eq!(error, DocError::NestingTooDeep(MAX_DEPTH));
    }

    #[test]
    fn xml_to_json_pretty_output() {
        let mut opts = options("<root><a>1</a></root>");
        opts.pretty = true;
        let output = convert(DocKind::Xml2Json(opts));
        assert_eq!(output, "{\n  \"root\": {\n    \"a\": \"1\"\n  }\n}");
    }

    // -------- XML <-> YAML and roundtrips --------

    #[test]
    fn xml_to_yaml_known_vector() {
        let output = convert(DocKind::Xml2Yaml(options("<root><a>1</a><b>x</b></root>")));
        assert_eq!(as_yaml(&output), as_yaml("root:\n  a: \"1\"\n  b: x\n"));
    }

    #[test]
    fn yaml_to_xml_known_vector() {
        let output = convert(DocKind::Yaml2Xml(options("a: 1\nb:\n  - true\n")));
        assert_eq!(output, "<root><a>1</a><b>true</b></root>");
    }

    #[test]
    fn json_to_xml_to_json_roundtrip() {
        let xml = convert(DocKind::Json2Xml(options(r#"{"a":1,"list":[1,2],"nested":{"b":"x"}}"#)));
        let json = convert(DocKind::Xml2Json(options(&xml)));
        assert_eq!(as_json(&json), as_json(r#"{"root":{"a":"1","list":["1","2"],"nested":{"b":"x"}}}"#));
    }

    #[test]
    fn yaml_to_json_to_yaml_roundtrip() {
        let input = "a: 1\nb:\n  - true\n  - null\n";
        let json = convert(DocKind::Yaml2Json(options(input)));
        let yaml = convert(DocKind::Json2Yaml(options(&json)));
        assert_eq!(as_yaml(&yaml), as_yaml(input));
    }

    #[test]
    fn options_defaults() {
        let defaults = DocOptions::default();
        assert_eq!(defaults.input, "");
        assert_eq!(defaults.root_name, None);
        assert!(!defaults.pretty);
    }

    #[test]
    fn error_display_is_human_readable() {
        assert_eq!(
            DocError::InvalidXmlName("1x".to_string()).to_string(),
            "\"1x\" is not a valid XML element name (names must start with a letter, '_', or ':' \
             and continue with letters, digits, '_', '-', '.', or ':')"
        );
        assert_eq!(DocError::DtdNotAllowed.to_string(), "XML DOCTYPE/DTD declarations are not allowed");
    }
}

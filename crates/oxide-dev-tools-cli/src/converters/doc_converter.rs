use std::path::Path;

use clap::Args;
use oxide_dev_tools_core::*;

use crate::error::CliError;

/// Shared arguments for `oxide convert json2yaml|yaml2json|json2xml|xml2json|yaml2xml|xml2yaml`
#[derive(Args)]
pub struct DocConvertArgs {
    /// Document text, or a path to a file containing the document
    pub input: Option<String>,

    /// Treat <INPUT> as a file path (a missing file is an error)
    #[arg(long)]
    pub input_file: bool,

    /// Write the result to this file instead of stdout ("-" = stdout)
    #[arg(long)]
    pub output: Option<String>,

    /// Pretty-print JSON or XML output (2-space indentation)
    #[arg(long)]
    pub pretty: bool,

    /// Root element name for conversions into XML (default: root)
    #[arg(long)]
    pub root_name: Option<String>,
}

pub fn exec(args: DocConvertArgs, kind: fn(DocOptions) -> DocKind) -> Result<(), CliError> {
    let input = resolve_input(&args)?;
    let options = DocOptions {
        input,
        root_name: args.root_name,
        pretty: args.pretty,
    };
    let result = convert_doc(kind(options))?;
    write_output(&result, args.output.as_deref())
}

fn resolve_input(args: &DocConvertArgs) -> Result<String, CliError> {
    let Some(input) = &args.input else {
        return Err("missing <INPUT> (document text or a path to a file)".into());
    };
    if args.input_file || Path::new(input).is_file() {
        return std::fs::read_to_string(input)
            .map_err(|error| CliError::Io(format!("cannot read input file \"{input}\": {error}")));
    }
    Ok(input.clone())
}

fn write_output(result: &str, output: Option<&str>) -> Result<(), CliError> {
    match output {
        None | Some("-") => {
            println!("{result}");
            Ok(())
        }
        Some(path) => std::fs::write(path, result)
            .map_err(|error| CliError::Io(format!("cannot write output file \"{path}\": {error}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &str) -> DocConvertArgs {
        DocConvertArgs {
            input: Some(input.into()),
            input_file: false,
            output: None,
            pretty: false,
            root_name: None,
        }
    }

    fn temp_path(extension: &str) -> String {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("oxide-doc-test-{nonce}.{extension}"))
            .to_string_lossy()
            .into_owned()
    }

    // -------- exec --------

    type ConvertCase = (fn(DocOptions) -> DocKind, &'static str);

    #[test]
    fn exec_every_direction_with_inline_text() {
        let cases: [ConvertCase; 6] = [
            (DocKind::Json2Yaml, r#"{"a":1}"#),
            (DocKind::Yaml2Json, "a: 1\n"),
            (DocKind::Json2Xml, r#"{"a":1}"#),
            (DocKind::Xml2Json, "<root><a>1</a></root>"),
            (DocKind::Yaml2Xml, "a: 1\n"),
            (DocKind::Xml2Yaml, "<root><a>1</a></root>"),
        ];
        for (kind, input) in cases {
            assert!(exec(args(input), kind).is_ok());
        }
    }

    #[test]
    fn exec_reads_existing_file_by_auto_detection() {
        let path = temp_path("json");
        std::fs::write(&path, r#"{"a":1}"#).expect("write input");
        let result = exec(args(&path), DocKind::Json2Yaml);
        std::fs::remove_file(&path).ok();
        assert!(result.is_ok());
    }

    #[test]
    fn exec_input_file_flag_requires_existing_file() {
        let result = exec(
            DocConvertArgs {
                input_file: true,
                ..args("/nonexistent/definitely-missing.json")
            },
            DocKind::Json2Yaml,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn exec_writes_output_file() {
        let path = temp_path("yaml");
        let result = exec(
            DocConvertArgs {
                output: Some(path.clone()),
                ..args(r#"{"a":1}"#)
            },
            DocKind::Json2Yaml,
        );
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&path).expect("output missing");
        std::fs::remove_file(&path).ok();
        assert_eq!(content, "a: 1\n");
    }

    #[test]
    fn exec_pretty_and_root_name_flags() {
        let result = exec(
            DocConvertArgs {
                pretty: true,
                root_name: Some("data".into()),
                ..args(r#"{"a":1}"#)
            },
            DocKind::Json2Xml,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn exec_missing_input_errors() {
        let result = exec(
            DocConvertArgs {
                input: None,
                ..args("")
            },
            DocKind::Json2Yaml,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_invalid_document_errors() {
        let result = exec(args("{broken"), DocKind::Json2Yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid JSON"));
    }
}

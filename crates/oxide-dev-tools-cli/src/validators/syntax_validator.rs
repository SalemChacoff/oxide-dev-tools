use std::io::Read;
use std::path::Path;

use clap::Args;
use oxide_dev_tools_core::*;

use crate::error::{GenericError, ValidError};

/// Shared arguments for `oxide validate json|yaml|xml`
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide validate json '{\"a\": 1}'\n  oxide validate yaml 'a: 1'\n  oxide validate xml '<root/>'\n  oxide validate json '{\"a\": 1}' --verbose\n  oxide validate json data.json --input-file\n  oxide validate xml '<!DOCTYPE r><r/>' --allow-dtd"
)]
pub struct SyntaxArgs {
    /// Document text, a path to a file containing the document, or "-" to read from stdin
    pub input: Option<String>,

    /// Treat <INPUT> as a file path (a missing file is an error)
    #[arg(long)]
    pub input_file: bool,

    /// Accept XML DOCTYPE/DTD declarations (XML only, reported as a warning)
    #[arg(long)]
    pub allow_dtd: bool,

    /// Maximum XML element nesting depth (XML only, default: 512)
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Print the full validation report instead of the bare format name
    #[arg(long)]
    pub verbose: bool,
}

pub fn exec(args: SyntaxArgs, kind: fn(SyntaxOptions) -> SyntaxKind) -> Result<(), ValidError> {
    let input = resolve_input(&args)?;
    let options = SyntaxOptions {
        input,
        allow_dtd: args.allow_dtd,
        max_depth: args.max_depth,
    };
    let report = validate_syntax(kind(options));
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if args.verbose {
        print_report(&report);
    }
    if report.valid {
        if !args.verbose {
            println!("{}", format_name(report.format));
        }
        Ok(())
    } else {
        Err(ValidError::Doc(report.issues.join("; ")))
    }
}

fn resolve_input(args: &SyntaxArgs) -> Result<String, GenericError> {
    let Some(input) = &args.input else {
        return Err("missing <INPUT> (document text or a path to a file)".into());
    };
    if input == "-" {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|error| GenericError::Io(format!("cannot read document from stdin: {error}")))?;
        return Ok(content);
    }
    if args.input_file || Path::new(input).is_file() {
        return std::fs::read_to_string(input)
            .map_err(|error| GenericError::Io(format!("cannot read input file \"{input}\": {error}")));
    }
    Ok(input.clone())
}

// -------- Report output --------

fn print_report(report: &SyntaxReport) {
    println!("valid:  {}", yes_no(report.valid));
    println!("format: {}", format_name(report.format));
    println!("input:  {}", report.input);
    if let Some(root) = &report.root {
        println!("root:   {root}");
    }
    println!("depth:  {}", report.depth);
    if !report.issues.is_empty() {
        println!("issues:");
        for issue in &report.issues {
            println!("  - {issue}");
        }
    }
    if !report.warnings.is_empty() {
        println!("warnings:");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
}

fn yes_no(valid: bool) -> &'static str {
    if valid { "yes" } else { "no" }
}

fn format_name(format: SyntaxFormat) -> &'static str {
    match format {
        SyntaxFormat::Json => "json",
        SyntaxFormat::Yaml => "yaml",
        SyntaxFormat::Xml => "xml",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &str) -> SyntaxArgs {
        SyntaxArgs {
            input: Some(input.into()),
            input_file: false,
            allow_dtd: false,
            max_depth: None,
            verbose: false,
        }
    }

    // -------- exec --------

    type SyntaxCase = (fn(SyntaxOptions) -> SyntaxKind, &'static str);

    #[test]
    fn exec_every_format_with_inline_text() {
        let cases: [SyntaxCase; 3] = [
            (SyntaxKind::Json, r#"{"a":1}"#),
            (SyntaxKind::Yaml, "a: 1\n"),
            (SyntaxKind::Xml, "<root/>"),
        ];
        for (kind, input) in cases {
            assert!(exec(args(input), kind).is_ok());
        }
    }

    #[test]
    fn exec_invalid_documents() {
        let cases: [SyntaxCase; 3] = [
            (SyntaxKind::Json, "{broken"),
            (SyntaxKind::Yaml, "a: [1, 2"),
            (SyntaxKind::Xml, "<root>"),
        ];
        for (kind, input) in cases {
            let result = exec(args(input), kind);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("invalid document"));
        }
    }

    #[test]
    fn exec_verbose_valid_and_invalid() {
        assert!(
            exec(
                SyntaxArgs {
                    verbose: true,
                    ..args(r#"{"a":1}"#)
                },
                SyntaxKind::Json
            )
            .is_ok()
        );
        assert!(
            exec(
                SyntaxArgs {
                    verbose: true,
                    ..args("{broken")
                },
                SyntaxKind::Json
            )
            .is_err()
        );
    }

    #[test]
    fn exec_reads_existing_file_by_auto_detection() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("oxide-syntax-test-{nonce}.json"));
        std::fs::write(&path, r#"{"a":1}"#).expect("write input");
        let result = exec(args(&path.to_string_lossy()), SyntaxKind::Json);
        std::fs::remove_file(&path).ok();
        assert!(result.is_ok());
    }

    #[test]
    fn exec_input_file_flag_requires_existing_file() {
        let result = exec(
            SyntaxArgs {
                input_file: true,
                ..args("/nonexistent/definitely-missing.json")
            },
            SyntaxKind::Json,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn exec_missing_input_errors() {
        let result = exec(
            SyntaxArgs {
                input: None,
                ..args("")
            },
            SyntaxKind::Json,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_format_specific_flags() {
        let accepted = exec(
            SyntaxArgs {
                allow_dtd: true,
                ..args("<!DOCTYPE root><root/>")
            },
            SyntaxKind::Xml,
        );
        assert!(accepted.is_ok());

        let capped = exec(
            SyntaxArgs {
                max_depth: Some(1),
                ..args("<r><a/></r>")
            },
            SyntaxKind::Xml,
        );
        assert!(capped.is_err());
    }
}

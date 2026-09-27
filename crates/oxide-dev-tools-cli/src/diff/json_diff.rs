use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::{DiffError, GenericError};

/// `oxide diff json <LEFT> <RIGHT> [flags]` — deep-compare two JSON documents
///
/// Parses both documents and compares them semantically: object member order
/// is irrelevant, arrays are positional by default, and numbers can be
/// compared within a relative tolerance. Each operand is inline JSON, a path
/// to an existing file, or "-" for stdin. Identical documents print nothing
/// (like `git diff`); differences print one line per path unless `--format
/// json` or `--check` is given.
#[derive(Args)]
#[command(after_help = r#"Examples:
  oxide diff json "{\"name\":\"ann\"}" "{\"name\":\"bob\"}"
  oxide diff json api-v1.json api-v2.json
  oxide diff json old.json new.json --ignore-order
  oxide diff json before.json after.json --tolerance 5
  oxide diff json draft.json final.json --ignore-key timestamp --ignore-key version
  oxide diff json api-v1.json api-v2.json --format json
  oxide diff json api-v1.json api-v2.json --check
> Note: on Windows shells, JSON quoting differs — e.g. oxide diff json "{\"name\":\"ann\"}" "{\"name\":\"bob\"}"."#)]
pub struct JsonDiffArgs {
    /// First JSON document: inline text, a file path, or "-" for stdin
    pub left: Option<String>,

    /// Second JSON document: inline text, a file path, or "-" for stdin
    pub right: Option<String>,

    /// Treat <LEFT> and <RIGHT> as file paths (a missing file is an error)
    #[arg(long)]
    pub files: bool,

    /// Compare array elements as sets instead of positionally
    #[arg(long)]
    pub ignore_order: bool,

    /// Relative numeric tolerance in percent (100 vs 105 are equal at 5)
    #[arg(long, value_name = "PERCENT")]
    pub tolerance: Option<f64>,

    /// Object member name to skip at any depth (repeatable)
    #[arg(long = "ignore-key", value_name = "KEY")]
    pub ignore_keys: Vec<String>,

    /// Output format: a path-addressed report (default) or an RFC 6902 patch
    #[arg(long, value_enum)]
    pub format: Option<JsonFormatCli>,

    /// Exit 1 without output when the documents differ (CI-friendly)
    #[arg(long)]
    pub check: bool,
}

/// How differences are rendered.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum JsonFormatCli {
    /// One line per difference with a JSON path (default)
    Report,
    /// A JSON array of RFC 6902 patch operations
    Json,
}

pub fn exec(args: JsonDiffArgs) -> Result<(), DiffError> {
    let (left, _) = super::resolve_diff_input(&args.left, args.files, "LEFT", "left")?;
    let (right, _) = super::resolve_diff_input(&args.right, args.files, "RIGHT", "right")?;
    if let Some(percent) = args.tolerance {
        if !(0.0..=100.0).contains(&percent) {
            return Err(GenericError::Argument(format!("--tolerance must be between 0 and 100, got {percent}")).into());
        }
    }
    let options = JsonDiffOptions {
        left,
        right,
        ignore_array_order: args.ignore_order,
        relative_tolerance: args.tolerance.map(|percent| percent / 100.0),
        ignore_keys: args.ignore_keys,
        ..JsonDiffOptions::default()
    };
    let report = compare_json(options)?;
    if report.is_equal() {
        return Ok(());
    }
    if args.check {
        return Err(DiffError::CheckFailed);
    }
    let output = match args.format.unwrap_or(JsonFormatCli::Report) {
        JsonFormatCli::Report => report.to_text(),
        JsonFormatCli::Json => report.to_patch(),
    };
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn args(left: &str, right: &str) -> JsonDiffArgs {
        JsonDiffArgs {
            left: Some(left.to_string()),
            right: Some(right.to_string()),
            files: false,
            ignore_order: false,
            tolerance: None,
            ignore_keys: Vec::new(),
            format: None,
            check: false,
        }
    }

    #[test]
    fn exec_report_mode_succeeds() {
        assert!(exec(args(r#"{"a":1}"#, r#"{"a":2}"#)).is_ok());
    }

    #[test]
    fn exec_json_format_succeeds() {
        let result = exec(JsonDiffArgs {
            format: Some(JsonFormatCli::Json),
            ..args(r#"{"a":1}"#, r#"{"a":2}"#)
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_ignore_order_flag_succeeds() {
        let result = exec(JsonDiffArgs {
            ignore_order: true,
            ..args("[1,2]", "[2,1]")
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_tolerance_flag_succeeds() {
        let result = exec(JsonDiffArgs {
            tolerance: Some(5.0),
            ..args("100", "105")
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_out_of_range_tolerance_errors() {
        for percent in [Some(-0.1), Some(100.1), Some(f64::NAN)] {
            let result = exec(JsonDiffArgs {
                tolerance: percent,
                ..args("100", "100")
            });
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("--tolerance must be between 0 and 100")
            );
        }
    }

    #[test]
    fn exec_ignore_keys_flag_succeeds() {
        let result = exec(JsonDiffArgs {
            ignore_keys: vec!["timestamp".to_string()],
            ..args(r#"{"timestamp":1,"a":1}"#, r#"{"timestamp":2,"a":2}"#)
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_identical_inputs_succeeds() {
        assert!(exec(args(r#"{"a":1}"#, r#"{"a":1}"#)).is_ok());
    }

    #[test]
    fn exec_missing_operands_error() {
        let left = exec(JsonDiffArgs {
            left: None,
            ..args("1", "2")
        });
        assert!(left.is_err());
        assert!(left.unwrap_err().to_string().contains("missing <LEFT>"));
        let right = exec(JsonDiffArgs {
            right: None,
            ..args("1", "2")
        });
        assert!(right.is_err());
        assert!(right.unwrap_err().to_string().contains("missing <RIGHT>"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(JsonDiffArgs {
            files: true,
            ..args("no-such-left-file.json", "no-such-right-file.json")
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn exec_invalid_json_errors() {
        let result = exec(args("not json", "1"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot parse JSON on the left side")
        );
    }

    #[test]
    fn exec_check_identical_succeeds() {
        let result = exec(JsonDiffArgs {
            check: true,
            ..args(r#"{"a":1}"#, r#"{"a":1}"#)
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_check_differ_fails_silently() {
        let result = exec(JsonDiffArgs {
            check: true,
            ..args(r#"{"a":1}"#, r#"{"a":2}"#)
        });
        let error = result.unwrap_err();
        assert!(matches!(error, DiffError::CheckFailed));
        assert!(error.source().is_none());
    }

    #[test]
    fn exec_check_takes_precedence_over_format() {
        let result = exec(JsonDiffArgs {
            check: true,
            format: Some(JsonFormatCli::Json),
            ..args(r#"{"a":1}"#, r#"{"a":2}"#)
        });
        assert!(matches!(result.unwrap_err(), DiffError::CheckFailed));
    }
}

use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::DiffError;

/// `oxide diff text <LEFT> <RIGHT> [flags]` — compare two texts or files
///
/// The default mode compares whole lines and prints a git-style unified
/// diff; `--mode chars` compares individual characters for short strings.
/// Each operand is inline text, a path to an existing file, or "-" for
/// stdin. Identical inputs print nothing (like `git diff`).
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide diff text \"hello\" \"world\"\n  oxide diff text draft.md final.md --files\n  oxide diff text \"abcdef\" \"abxdef\" --mode chars\n  oxide diff text old.txt new.txt --files --context 5\n  cat draft.md | oxide diff text - final.md"
)]
pub struct TextDiffArgs {
    /// First text to compare: inline text, a file path, or "-" for stdin
    pub left: Option<String>,

    /// Second text to compare: inline text, a file path, or "-" for stdin
    pub right: Option<String>,

    /// Treat <LEFT> and <RIGHT> as file paths (a missing file is an error)
    #[arg(long)]
    pub files: bool,

    /// Comparison mode: whole lines (git-style hunks) or single characters
    #[arg(long, value_enum)]
    pub mode: Option<DiffModeCli>,

    /// Number of unchanged context lines around each change (line mode)
    #[arg(long)]
    pub context: Option<usize>,
}

/// How the two inputs are compared.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DiffModeCli {
    /// Unified diff, line by line (default)
    Lines,
    /// Character-by-character diff for short strings
    Chars,
}

impl From<DiffModeCli> for DiffMode {
    fn from(mode: DiffModeCli) -> Self {
        match mode {
            DiffModeCli::Lines => DiffMode::Lines,
            DiffModeCli::Chars => DiffMode::Chars,
        }
    }
}

pub fn exec(args: TextDiffArgs) -> Result<(), DiffError> {
    let (left, left_label) = super::resolve_diff_input(&args.left, args.files, "LEFT", "left")?;
    let (right, right_label) = super::resolve_diff_input(&args.right, args.files, "RIGHT", "right")?;
    let options = TextDiffOptions {
        left,
        right,
        mode: args.mode.unwrap_or(DiffModeCli::Lines).into(),
        context_lines: args.context.unwrap_or(3),
        left_label,
        right_label,
        ..TextDiffOptions::default()
    };
    let output = diff_text(options)?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(left: &str, right: &str) -> TextDiffArgs {
        TextDiffArgs {
            left: Some(left.to_string()),
            right: Some(right.to_string()),
            files: false,
            mode: None,
            context: None,
        }
    }

    #[test]
    fn exec_line_mode_succeeds() {
        assert!(exec(args("a\nb\n", "a\nc\n")).is_ok());
    }

    #[test]
    fn exec_char_mode_succeeds() {
        let result = exec(TextDiffArgs {
            mode: Some(DiffModeCli::Chars),
            ..args("abc", "axc")
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_context_flag_succeeds() {
        let result = exec(TextDiffArgs {
            context: Some(0),
            ..args("a\nb\n", "a\nc\n")
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_identical_inputs_succeeds() {
        assert!(exec(args("same\n", "same\n")).is_ok());
    }

    #[test]
    fn exec_missing_operands_error() {
        let left = exec(TextDiffArgs {
            left: None,
            ..args("a", "b")
        });
        assert!(left.is_err());
        assert!(left.unwrap_err().to_string().contains("missing <LEFT>"));
        let right = exec(TextDiffArgs {
            right: None,
            ..args("a", "b")
        });
        assert!(right.is_err());
        assert!(right.unwrap_err().to_string().contains("missing <RIGHT>"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(TextDiffArgs {
            files: true,
            ..args("no-such-left-file.txt", "no-such-right-file.txt")
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn exec_oversized_char_input_errors() {
        let oversized = "x".repeat(1_048_577);
        let result = exec(TextDiffArgs {
            mode: Some(DiffModeCli::Chars),
            ..args(&oversized, "world")
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at most 1048576"));
    }

    #[test]
    fn mode_maps_to_core() {
        assert_eq!(DiffMode::from(DiffModeCli::Lines), DiffMode::Lines);
        assert_eq!(DiffMode::from(DiffModeCli::Chars), DiffMode::Chars);
    }
}

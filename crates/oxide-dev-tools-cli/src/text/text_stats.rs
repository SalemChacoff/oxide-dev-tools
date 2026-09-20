use std::io::Read;
use std::path::Path;

use clap::Args;
use oxide_dev_tools_core::*;

use crate::error::{GenericError, TextError};

/// `oxide text count <INPUT> [flags]` — count characters, words, lines, and paragraphs
///
/// Selection flags (`--chars`, `--bytes`, `--words`, `--lines`, `--paragraphs`)
/// print bare numbers, in the order characters, bytes, words, lines, paragraphs.
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text count \"hello world\"\n  oxide text count draft.md --input-file\n  oxide text count \"hello world\" --words\n  oxide text count \"hello world\" --words --lines --chars"
)]
pub struct CountArgs {
    /// Text to count, a path to a file, or "-" to read from stdin
    pub input: Option<String>,

    /// Treat <INPUT> as a file path (a missing file is an error)
    #[arg(long)]
    pub input_file: bool,

    /// Print only the character count
    #[arg(long)]
    pub chars: bool,

    /// Print only the byte count
    #[arg(long)]
    pub bytes: bool,

    /// Print only the word count
    #[arg(long)]
    pub words: bool,

    /// Print only the line count
    #[arg(long)]
    pub lines: bool,

    /// Print only the paragraph count
    #[arg(long)]
    pub paragraphs: bool,
}

pub fn exec(args: CountArgs) -> Result<(), TextError> {
    let input = resolve_input(&args)?;
    let options = TextStatsOptions { input };
    print_report(&count_text_stats(options), &args);
    Ok(())
}

// -------- Input resolution --------

fn resolve_input(args: &CountArgs) -> Result<String, GenericError> {
    let Some(input) = &args.input else {
        return Err("missing <INPUT> (text to count or a path to a file)".into());
    };
    if input == "-" {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|error| GenericError::Io(format!("cannot read text from stdin: {error}")))?;
        return Ok(content);
    }
    if args.input_file || Path::new(input).is_file() {
        return std::fs::read_to_string(input)
            .map_err(|error| GenericError::Io(format!("cannot read input file \"{input}\": {error}")));
    }
    Ok(input.clone())
}

// -------- Report output --------

/// Print the selected metrics as bare numbers, or the full labeled report
/// when no metric is selected.
fn print_report(report: &TextStatsReport, args: &CountArgs) {
    let selected = [
        (args.chars, report.characters),
        (args.bytes, report.bytes),
        (args.words, report.words),
        (args.lines, report.lines),
        (args.paragraphs, report.paragraphs),
    ];
    if selected.iter().any(|(enabled, _)| *enabled) {
        let values = selected
            .iter()
            .filter(|(enabled, _)| *enabled)
            .map(|(_, value)| value.to_string())
            .collect::<Vec<_>>();
        println!("{}", values.join(" "));
    } else {
        println!("characters: {}", report.characters);
        println!("bytes:      {}", report.bytes);
        println!("words:      {}", report.words);
        println!("lines:      {}", report.lines);
        println!("paragraphs: {}", report.paragraphs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &str) -> CountArgs {
        CountArgs {
            input: Some(input.to_string()),
            input_file: false,
            chars: false,
            bytes: false,
            words: false,
            lines: false,
            paragraphs: false,
        }
    }

    #[test]
    fn exec_text_succeeds() {
        assert!(exec(args("hello world")).is_ok());
    }

    #[test]
    fn exec_every_selection_combination_succeeds() {
        for (chars, bytes, words, lines, paragraphs) in [
            (true, false, false, false, false),
            (false, true, false, false, false),
            (false, false, true, false, false),
            (false, false, false, true, false),
            (false, false, false, false, true),
            (true, true, true, true, true),
        ] {
            let result = exec(CountArgs {
                chars,
                bytes,
                words,
                lines,
                paragraphs,
                ..args("hello world")
            });
            assert!(result.is_ok());
        }
    }

    #[test]
    fn exec_missing_input_errors() {
        let result = exec(CountArgs {
            input: None,
            ..args("")
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(CountArgs {
            input_file: true,
            ..args("no-such-file-oxide.txt")
        });
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }
}

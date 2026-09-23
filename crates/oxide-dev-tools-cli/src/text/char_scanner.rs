use clap::Args;
use oxide_dev_tools_core::*;

use crate::error::TextError;

/// `oxide text scan <INPUT> [flags]` — detect whitespace and hidden characters
///
/// Scans any plain text (JSON, XML, SQL, source code, logs) and reports
/// suspicious characters with their line, column, code point, and name.
/// Without flags, whitespace and hidden characters are reported.
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text scan \"hello world\"\n  oxide text scan \"hello world\" --whitespace\n  oxide text scan config.json --input-file --hidden\n  cat query.sql | oxide text scan -\n  oxide text scan \"café\" --non-ascii"
)]
pub struct ScanArgs {
    /// Text to scan, a path to a file, or "-" to read from stdin
    pub input: Option<String>,

    /// Treat <INPUT> as a file path (a missing file is an error)
    #[arg(long)]
    pub input_file: bool,

    /// Report whitespace characters only
    #[arg(long)]
    pub whitespace: bool,

    /// Report hidden characters only (zero-width, controls, format, ...)
    #[arg(long)]
    pub hidden: bool,

    /// Report non-ASCII characters only (visible or not)
    #[arg(long)]
    pub non_ascii: bool,
}

pub fn exec(args: ScanArgs) -> Result<(), TextError> {
    let input = super::resolve_text_input(&args.input, args.input_file)?;
    let options = TextScanOptions {
        input,
        include_whitespace: args.whitespace,
        include_hidden: args.hidden,
        include_non_ascii: args.non_ascii,
    };
    print_report(&scan_characters(options));
    Ok(())
}

// -------- Report output --------

/// Print one line per finding, then a summary block with the full-input
/// counts.
fn print_report(report: &TextScanReport) {
    if report.findings.is_empty() {
        println!("no matching characters found");
    }
    for found in &report.findings {
        println!(
            "line {}, col {}: U+{:04X} {} — {}",
            found.line,
            found.column,
            found.character as u32,
            describe(found),
            found.class.name()
        );
    }
    println!();
    println!("findings:   {}", report.findings.len());
    println!("whitespace: {}", report.whitespace_count);
    println!("hidden:     {}", report.hidden_count);
    println!("non-ascii:  {}", report.non_ascii_count);
    println!("characters: {} total, {} visible", report.total_characters, report.visible_count);
}

/// Human-readable description of a finding: its Unicode name when known,
/// the character itself for visible non-ASCII, or its escape sequence.
fn describe(found: &CharacterFinding) -> String {
    let label = character_label(found.character);
    if !label.is_empty() {
        return label.to_string();
    }
    if found.class == CharClass::Visible {
        return found.character.to_string();
    }
    found.character.escape_unicode().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &str) -> ScanArgs {
        ScanArgs {
            input: Some(input.to_string()),
            input_file: false,
            whitespace: false,
            hidden: false,
            non_ascii: false,
        }
    }

    #[test]
    fn exec_inline_text_succeeds() {
        assert!(exec(args("hello world")).is_ok());
    }

    #[test]
    fn exec_every_flag_combination_succeeds() {
        for (whitespace, hidden, non_ascii) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, true),
        ] {
            let result = exec(ScanArgs {
                whitespace,
                hidden,
                non_ascii,
                ..args("hello\u{200B}world")
            });
            assert!(result.is_ok());
        }
    }

    #[test]
    fn exec_missing_input_errors() {
        let result = exec(ScanArgs {
            input: None,
            ..args("")
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(ScanArgs {
            input_file: true,
            ..args("no-such-file-oxide.txt")
        });
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }
}

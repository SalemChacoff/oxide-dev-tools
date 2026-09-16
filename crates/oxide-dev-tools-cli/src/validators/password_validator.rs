use std::io::Read;
use std::path::Path;

use clap::Args;
use oxide_dev_tools_core::*;

use crate::error::{GenericError, ValidError};

/// `oxide validate password <PASSWORD> [options]` — zxcvbn-based strength analysis
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide validate password 'correct horse battery staple'\n  oxide validate password 'Tr0ub4dor&3' --verbose\n  oxide validate password 'P@ssw0rd!' --min-score 3\n  oxide validate password 'summer2021' --user-input summer\n  oxide validate password secrets.txt --input-file\n  echo 'my secret' | oxide validate password -\n> Note: passwords typed inline stay in your shell history; prefer --input-file or stdin for real secrets."
)]
pub struct PasswordArgs {
    /// Password text, a path to a file containing the password, or "-" to read from stdin
    pub input: Option<String>,

    /// Treat <INPUT> as a file path (a missing file is an error)
    #[arg(long)]
    pub input_file: bool,

    /// Require a minimum strength score (0-4); exit non-zero when below
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=4))]
    pub min_score: Option<u8>,

    /// Words the password should not be based on (names, usernames); repeatable
    #[arg(long = "user-input", value_name = "WORD")]
    pub user_inputs: Vec<String>,

    /// Print the full analysis report instead of the bare strength label
    #[arg(long)]
    pub verbose: bool,
}

pub fn exec(args: PasswordArgs) -> Result<(), ValidError> {
    let input = resolve_input(&args)?;
    let options = PasswordStrengthOptions {
        input,
        min_score: args.min_score.unwrap_or(0),
        user_inputs: args.user_inputs,
    };
    let report = validate_password(options);
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if args.verbose {
        print_report(&report);
    }
    if report.valid {
        if !args.verbose {
            println!("{}", report.strength.label());
        }
        Ok(())
    } else {
        Err(ValidError::Password(report.issues.join("; ")))
    }
}

// -------- Input resolution --------

fn resolve_input(args: &PasswordArgs) -> Result<String, GenericError> {
    let Some(input) = &args.input else {
        return Err("missing <INPUT> (password text, a path to a file, or '-' for stdin)".into());
    };
    if input == "-" {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|error| GenericError::Io(format!("cannot read password from stdin: {error}")))?;
        return Ok(strip_trailing_newline(content));
    }
    if args.input_file || Path::new(input).is_file() {
        let content = std::fs::read_to_string(input)
            .map_err(|error| GenericError::Io(format!("cannot read input file \"{input}\": {error}")))?;
        return Ok(strip_trailing_newline(content));
    }
    Ok(input.clone())
}

/// Drop a single trailing line ending added by editors when saving files;
/// inline passwords are never altered.
fn strip_trailing_newline(mut content: String) -> String {
    if content.ends_with('\n') {
        content.pop();
        if content.ends_with('\r') {
            content.pop();
        }
    }
    content
}

// -------- Report output --------

fn print_report(report: &PasswordReport) {
    println!("valid:        {}", yes_no(report.valid));
    println!("score:        {}/4 {}", report.score, report.strength.label());
    println!("length:       {}", report.input_len);
    println!("entropy:      {} bits", report.entropy_bits);
    println!("guesses:      10^{}", report.guesses_log10);
    println!(
        "classes:      lower={} upper={} digits={} symbols={} other={}",
        report.lowercase, report.uppercase, report.digits, report.symbols, report.other
    );
    println!("crack times:");
    println!("  online throttled:   {}", report.crack_times.online_throttled);
    println!("  online unthrottled: {}", report.crack_times.online_unthrottled);
    println!("  offline slow hash:  {}", report.crack_times.offline_slow);
    println!("  offline fast hash:  {}", report.crack_times.offline_fast);
    if !report.patterns.is_empty() {
        println!("patterns:");
        for pattern in &report.patterns {
            println!("  - {pattern}");
        }
    }
    if !report.warnings.is_empty() {
        println!("warnings:");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
    if !report.suggestions.is_empty() {
        println!("suggestions:");
        for suggestion in &report.suggestions {
            println!("  - {suggestion}");
        }
    }
    if !report.issues.is_empty() {
        println!("issues:");
        for issue in &report.issues {
            println!("  - {issue}");
        }
    }
}

fn yes_no(valid: bool) -> &'static str {
    if valid { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: Option<&str>) -> PasswordArgs {
        PasswordArgs {
            input: input.map(ToString::to_string),
            input_file: false,
            min_score: None,
            user_inputs: Vec::new(),
            verbose: false,
        }
    }

    #[test]
    fn exec_analyzes_weak_and_strong_passwords() {
        assert!(exec(args(Some("password"))).is_ok());
        assert!(exec(args(Some("correct horse battery staple"))).is_ok());
    }

    #[test]
    fn exec_min_score_rejects_weak_passwords() {
        let result = exec(PasswordArgs {
            min_score: Some(3),
            ..args(Some("password"))
        });
        let message = result.unwrap_err().to_string();
        assert!(message.contains("invalid password"));
        assert!(message.contains("below the required minimum of 3"));
    }

    #[test]
    fn exec_missing_input_is_an_error() {
        let result = exec(args(None));
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_missing_file_is_an_error() {
        let result = exec(PasswordArgs {
            input_file: true,
            ..args(Some("no-such-file.txt"))
        });
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn exec_user_inputs_and_verbose() {
        let result = exec(PasswordArgs {
            user_inputs: vec!["alice".to_string()],
            ..args(Some("alice1990"))
        });
        assert!(result.is_ok());
        assert!(
            exec(PasswordArgs {
                verbose: true,
                ..args(Some("password"))
            })
            .is_ok()
        );
    }

    #[test]
    fn strip_trailing_newline_handles_lf_and_crlf() {
        assert_eq!(strip_trailing_newline("secret\n".to_string()), "secret");
        assert_eq!(strip_trailing_newline("secret\r\n".to_string()), "secret");
        assert_eq!(strip_trailing_newline("secret".to_string()), "secret");
    }
}

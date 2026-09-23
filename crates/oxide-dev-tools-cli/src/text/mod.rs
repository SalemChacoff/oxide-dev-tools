pub mod case_converter;
pub mod codec;
pub mod html_codec;
pub mod text_stats;
pub mod truncate;
pub mod unicode_codec;

use std::io::Read;
use std::path::Path;

use clap::{Args, Subcommand};

use crate::error::{CliError, GenericError};

/// `oxide text ...` — entry point for all text utilities
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text case camel \"hello world\"\n  oxide text case snake \"helloWorld\"\n  oxide text case kebab \"helloWorld\"\n  oxide text count \"hello world\"\n  oxide text count draft.md --input-file\n  oxide text truncate \"hello world\" --max-length 8\n  oxide text codec html encode \"<b>hi</b>\""
)]
pub struct TextArgs {
    #[command(subcommand)]
    pub kind: TextKind,
}

#[derive(Subcommand)]
pub enum TextKind {
    /// Convert text between letter cases (camelCase, snake_case, kebab-case, etc.).
    Case(case_converter::CaseArgs),
    /// Encode and decode HTML entities and unicode escape sequences.
    Codec(codec::CodecArgs),
    /// Count characters, words, lines, and paragraphs in text.
    Count(text_stats::CountArgs),
    /// Truncate text to a maximum length with an optional ellipsis.
    Truncate(truncate::TruncateArgs),
}

pub fn exec(args: TextArgs) -> Result<(), CliError> {
    match args.kind {
        TextKind::Case(args) => case_converter::exec(args).map_err(Into::into),
        TextKind::Codec(args) => codec::exec(args).map_err(Into::into),
        TextKind::Count(args) => text_stats::exec(args).map_err(Into::into),
        TextKind::Truncate(args) => truncate::exec(args).map_err(Into::into),
    }
}

// -------- Input resolution --------

/// Resolve `<INPUT>` from inline text, a file path, or `-` for stdin.
fn resolve_text_input(input: &Option<String>, input_file: bool) -> Result<String, GenericError> {
    let Some(input) = input else {
        return Err("missing <INPUT> (inline text or a path to a file)".into());
    };
    if input == "-" {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|error| GenericError::Io(format!("cannot read text from stdin: {error}")))?;
        return Ok(content);
    }
    if input_file || Path::new(input).is_file() {
        return std::fs::read_to_string(input)
            .map_err(|error| GenericError::Io(format!("cannot read input file \"{input}\": {error}")));
    }
    Ok(input.clone())
}

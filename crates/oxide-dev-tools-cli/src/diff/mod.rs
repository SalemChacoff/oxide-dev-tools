pub mod text_diff;

use std::io::Read;
use std::path::Path;

use clap::{Args, Subcommand};

use crate::error::{CliError, GenericError};

/// `oxide diff ...` — compare texts and files, git-style
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide diff text \"hello\" \"world\"\n  oxide diff text draft.md final.md --files\n  oxide diff text \"abcdef\" \"abxdef\" --mode chars"
)]
pub struct DiffArgs {
    #[command(subcommand)]
    pub kind: DiffKind,
}

#[derive(Subcommand)]
pub enum DiffKind {
    /// Compare two texts or files line by line or character by character.
    Text(text_diff::TextDiffArgs),
}

pub fn exec(args: DiffArgs) -> Result<(), CliError> {
    match args.kind {
        DiffKind::Text(args) => text_diff::exec(args).map_err(Into::into),
    }
}

// -------- Input resolution --------

/// Resolve one diff operand from inline text, a file path, or `-` for stdin,
/// returning the content plus a label for the unified diff headers.
fn resolve_diff_input(
    input: &Option<String>,
    files: bool,
    operand: &str,
    label: &str,
) -> Result<(String, String), GenericError> {
    let Some(input) = input else {
        return Err(format!("missing <{operand}> (inline text or a path to a file)").into());
    };
    if input == "-" {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|error| GenericError::Io(format!("cannot read text from stdin: {error}")))?;
        return Ok((content, "stdin".to_string()));
    }
    if files || Path::new(input).is_file() {
        let content = std::fs::read_to_string(input)
            .map_err(|error| GenericError::Io(format!("cannot read input file \"{input}\": {error}")))?;
        return Ok((content, input.clone()));
    }
    Ok((input.clone(), label.to_string()))
}

pub mod case_converter;
pub mod text_stats;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide text ...` — entry point for all text utilities
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text case camel \"hello world\"\n  oxide text case snake \"helloWorld\"\n  oxide text case kebab \"helloWorld\"\n  oxide text count \"hello world\"\n  oxide text count draft.md --input-file"
)]
pub struct TextArgs {
    #[command(subcommand)]
    pub kind: TextKind,
}

#[derive(Subcommand)]
pub enum TextKind {
    /// Convert text between letter cases (camelCase, snake_case, kebab-case, etc.).
    Case(case_converter::CaseArgs),
    /// Count characters, words, lines, and paragraphs in text.
    Count(text_stats::CountArgs),
}

pub fn exec(args: TextArgs) -> Result<(), CliError> {
    match args.kind {
        TextKind::Case(args) => case_converter::exec(args).map_err(Into::into),
        TextKind::Count(args) => text_stats::exec(args).map_err(Into::into),
    }
}

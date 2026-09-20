pub mod case_converter;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide text ...` — entry point for all text utilities
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text case camel \"hello world\"\n  oxide text case snake \"helloWorld\"\n  oxide text case kebab \"helloWorld\""
)]
pub struct TextArgs {
    #[command(subcommand)]
    pub kind: TextKind,
}

#[derive(Subcommand)]
pub enum TextKind {
    /// Convert text between letter cases (camelCase, snake_case, kebab-case, etc.).
    Case(case_converter::CaseArgs),
}

pub fn exec(args: TextArgs) -> Result<(), CliError> {
    match args.kind {
        TextKind::Case(args) => case_converter::exec(args).map_err(Into::into),
    }
}

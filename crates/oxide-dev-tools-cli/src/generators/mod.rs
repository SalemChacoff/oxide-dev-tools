pub mod id_generator;
pub mod key_generator;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide gen ...` — entry point for all generators
#[derive(Args)]
pub struct GenArgs {
    #[command(subcommand)]
    pub kind: GenKind,
}

#[derive(Subcommand)]
pub enum GenKind {
    /// Generate UUIDs (v1–v8), ULIDs, NanoIDs, etc.
    Id(id_generator::IdArgs),
    /// Generate cryptographic keys (e.g. passwords, tokens).
    Key(key_generator::KeyArgs),
}

pub fn exec(args: GenArgs) -> Result<(), CliError> {
    match args.kind {
        GenKind::Id(args) => id_generator::exec(args),
        GenKind::Key(args) => key_generator::exec(args),
    }
}

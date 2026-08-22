pub mod fake_generator;
pub mod id_generator;
pub mod key_generator;
pub mod lorem_generator;

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
    /// Generate fake data (personas, names, emails, phones, addresses, companies)
    Fake(fake_generator::FakeArgs),
    /// Generate IDs (UUIDs (v1–v8), ULIDs, NanoIDs, etc.).
    Id(id_generator::IdArgs),
    /// Generate cryptographic keys (e.g. passwords, tokens).
    Key(key_generator::KeyArgs),
    /// Generate lorem ipsum words, sentences, and paragraphs.
    Lorem(lorem_generator::LoremArgs),
}

pub fn exec(args: GenArgs) -> Result<(), CliError> {
    match args.kind {
        GenKind::Fake(args) => fake_generator::exec(args),
        GenKind::Id(args) => id_generator::exec(args),
        GenKind::Key(args) => key_generator::exec(args),
        GenKind::Lorem(args) => lorem_generator::exec(args),
    }
}

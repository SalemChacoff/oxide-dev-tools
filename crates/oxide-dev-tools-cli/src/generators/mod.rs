pub mod fake_generator;
pub mod id_generator;
pub mod key_generator;
pub mod lorem_generator;
pub mod sample_file_generator;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide gen ...` — entry point for all generators
#[derive(Args)]
#[command(after_help = "Examples:\n  oxide gen id uuidv4\n  oxide gen key pass\n  oxide gen fake person")]
pub struct GenArgs {
    #[command(subcommand)]
    pub kind: GenKind,
}

#[derive(Subcommand)]
pub enum GenKind {
    /// Generate fake data (personas, names, emails, phones, addresses, companies)
    #[command(after_help = "Examples:\n  oxide gen fake person\n  oxide gen fake email --count 10")]
    Fake(fake_generator::FakeArgs),
    /// Generate IDs (UUIDs (v1–v8), ULIDs, NanoIDs, etc.).
    #[command(after_help = "Examples:\n  oxide gen id uuidv4\n  oxide gen id uuidv7 2026-06-07")]
    Id(id_generator::IdArgs),
    /// Generate cryptographic keys (e.g. passwords, tokens).
    #[command(after_help = "Examples:\n  oxide gen key pass\n  oxide gen key token --encoding base64")]
    Key(key_generator::KeyArgs),
    /// Generate lorem ipsum words, sentences, and paragraphs.
    #[command(after_help = "Examples:\n  oxide gen lorem words\n  oxide gen lorem paragraphs --length 2")]
    Lorem(lorem_generator::LoremArgs),
    /// Generate sample files (PDF, PNG, JPG) for testing upload endpoints.
    #[command(after_help = "Examples:\n  oxide gen sample pdf --size 5kb\n  oxide gen sample png --color red")]
    Sample(sample_file_generator::SampleArgs),
}

pub fn exec(args: GenArgs) -> Result<(), CliError> {
    match args.kind {
        GenKind::Fake(args) => fake_generator::exec(args).map_err(Into::into),
        GenKind::Id(args) => id_generator::exec(args).map_err(Into::into),
        GenKind::Key(args) => key_generator::exec(args).map_err(Into::into),
        GenKind::Lorem(args) => lorem_generator::exec(args).map_err(Into::into),
        GenKind::Sample(args) => sample_file_generator::exec(args).map_err(Into::into),
    }
}

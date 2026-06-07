pub mod id_generator;
pub mod key_generator;

use clap::{Args, Subcommand};

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
    Key(key_generator::KeyArgs),
    // Data(),
    // File(),
}

pub fn exec(args: GenArgs) {
    match args.kind {
        GenKind::Id(args) => id_generator::exec(args),
        GenKind::Key(args) => key_generator::exec(args),
        // GenKind::Data(args) => data::exec(args),
        // GenKind::File(args) => file::exec(args),
    }
}

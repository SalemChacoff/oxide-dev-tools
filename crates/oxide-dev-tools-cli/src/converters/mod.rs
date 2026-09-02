pub mod timestamp_converter;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide convert ...` — entry point for all converters
#[derive(Args)]
pub struct ConvertArgs {
    #[command(subcommand)]
    pub kind: ConvertKind,
}

#[derive(Subcommand)]
pub enum ConvertKind {
    /// Convert timestamps between Unix, ISO 8601, and human-readable formats.
    Timestamp(timestamp_converter::TimestampArgs),
}

pub fn exec(args: ConvertArgs) -> Result<(), CliError> {
    match args.kind {
        ConvertKind::Timestamp(args) => timestamp_converter::exec(args),
    }
}

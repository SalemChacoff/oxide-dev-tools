pub mod base64_codec;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide codec ...` — entry point for all codecs
#[derive(Args)]
pub struct CodecArgs {
    #[command(subcommand)]
    pub kind: CodecKind,
}

#[derive(Subcommand)]
pub enum CodecKind {
    /// Encode and decode base64 data.
    Base64(base64_codec::Base64Args),
}

pub fn exec(args: CodecArgs) -> Result<(), CliError> {
    match args.kind {
        CodecKind::Base64(args) => base64_codec::exec(args),
    }
}

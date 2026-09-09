pub mod base64_codec;
pub mod hex_codec;
pub mod url_codec;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide codec ...` — entry point for all codecs
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide codec base64 encode \"hello\"\n  oxide codec hex encode \"hello\"\n  oxide codec url encode \"hello world\""
)]
pub struct CodecArgs {
    #[command(subcommand)]
    pub kind: CodecKind,
}

#[derive(Subcommand)]
pub enum CodecKind {
    /// Encode and decode base64 data.
    #[command(
        after_help = "Examples:\n  oxide codec base64 encode \"hello\"\n  oxide codec base64 decode \"aGVsbG8=\""
    )]
    Base64(base64_codec::Base64Args),
    /// Encode and decode hex data.
    #[command(after_help = "Examples:\n  oxide codec hex encode \"hello\"\n  oxide codec hex decode \"68656c6c6f\"")]
    Hex(hex_codec::HexArgs),
    /// Encode and decode URL components.
    #[command(
        after_help = "Examples:\n  oxide codec url encode \"hello world\"\n  oxide codec url decode \"hello%20world\""
    )]
    Url(url_codec::UrlArgs),
}

pub fn exec(args: CodecArgs) -> Result<(), CliError> {
    match args.kind {
        CodecKind::Base64(args) => base64_codec::exec(args).map_err(Into::into),
        CodecKind::Hex(args) => hex_codec::exec(args).map_err(Into::into),
        CodecKind::Url(args) => url_codec::exec(args).map_err(Into::into),
    }
}

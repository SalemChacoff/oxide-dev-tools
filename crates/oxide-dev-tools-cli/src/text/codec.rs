use clap::{Args, Subcommand};

use super::{html_codec, unicode_codec};
use crate::error::TextError;

/// `oxide text codec ...` — string encode/decode dispatch
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text codec html encode \"<b>hi & bye</b>\"\n  oxide text codec html decode \"&lt;b&gt;hi &amp; bye&lt;/b&gt;\"\n  oxide text codec unicode encode \"café 🚀\"\n  oxide text codec unicode decode \"caf\\u00e9\""
)]
pub struct CodecArgs {
    #[command(subcommand)]
    pub kind: CodecKind,
}

#[derive(Subcommand)]
pub enum CodecKind {
    /// Encode and decode HTML entities.
    #[command(
        after_help = "Examples:\n  oxide text codec html encode \"<b>hi & bye</b>\"\n  oxide text codec html decode \"&lt;b&gt;hi &amp; bye&lt;/b&gt;\""
    )]
    Html(html_codec::HtmlArgs),
    /// Encode and decode unicode escape sequences.
    #[command(
        after_help = "Examples:\n  oxide text codec unicode encode \"café 🚀\"\n  oxide text codec unicode decode \"caf\\u00e9\""
    )]
    Unicode(unicode_codec::UnicodeArgs),
}

pub fn exec(args: CodecArgs) -> Result<(), TextError> {
    match args.kind {
        CodecKind::Html(args) => html_codec::exec(args),
        CodecKind::Unicode(args) => unicode_codec::exec(args),
    }
}

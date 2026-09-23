use clap::{Args, Subcommand, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::TextError;

/// `oxide text codec html [subcommand]` — HTML entity encode/decode dispatch
#[derive(Args)]
pub struct HtmlArgs {
    #[command(subcommand)]
    pub kind: HtmlCmd,
}

/// How aggressively HTML encoding escapes its input.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum HtmlModeCli {
    /// Escape & < > " ' (safe for attribute values)
    #[default]
    Attribute,
    /// Escape only & < > (safe for text content)
    Text,
}

impl From<HtmlModeCli> for HtmlMode {
    fn from(mode: HtmlModeCli) -> Self {
        match mode {
            HtmlModeCli::Attribute => HtmlMode::Attribute,
            HtmlModeCli::Text => HtmlMode::Text,
        }
    }
}

#[derive(Subcommand)]
pub enum HtmlCmd {
    /// Escape HTML-significant characters (& < > " ') into entities
    #[command(
        name = "encode",
        after_help = "Examples:\n  oxide text codec html encode \"<b>hi & bye</b>\"\n  oxide text codec html encode \"a < b & c > d\" --mode text\n  oxide text codec html encode page.html --input-file\n  cat page.html | oxide text codec html encode -"
    )]
    Encode {
        /// Text to escape, a path to a file, or "-" to read from stdin
        input: Option<String>,

        /// Treat <INPUT> as a file path (a missing file is an error)
        #[arg(long)]
        input_file: bool,

        /// Which characters get escaped
        #[arg(long, value_enum)]
        mode: Option<HtmlModeCli>,
    },

    /// Turn HTML entities (named or numeric) back into characters
    #[command(
        name = "decode",
        after_help = "Examples:\n  oxide text codec html decode \"&lt;b&gt;hi &amp; bye&lt;/b&gt;\"\n  oxide text codec html decode \"&copy; 2026 &amp; &#x1F680;\"\n  oxide text codec html decode page.html --input-file\n  cat page.html | oxide text codec html decode -"
    )]
    Decode {
        /// HTML-escaped text to decode, a path to a file, or "-" to read from stdin
        input: Option<String>,

        /// Treat <INPUT> as a file path (a missing file is an error)
        #[arg(long)]
        input_file: bool,
    },
}

pub fn exec(args: HtmlArgs) -> Result<(), TextError> {
    match args.kind {
        HtmlCmd::Encode {
            input,
            input_file,
            mode,
        } => {
            let input = super::resolve_text_input(&input, input_file)?;
            let options = HtmlOptions {
                input,
                mode: mode.unwrap_or_default().into(),
            };
            println!("{}", convert_html(HtmlKind::Encode(options))?);
        }
        HtmlCmd::Decode { input, input_file } => {
            let input = super::resolve_text_input(&input, input_file)?;
            println!("{}", convert_html(HtmlKind::Decode(input))?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_args(input: &str) -> HtmlArgs {
        HtmlArgs {
            kind: HtmlCmd::Encode {
                input: Some(input.into()),
                input_file: false,
                mode: None,
            },
        }
    }

    fn decode_args(input: &str) -> HtmlArgs {
        HtmlArgs {
            kind: HtmlCmd::Decode {
                input: Some(input.into()),
                input_file: false,
            },
        }
    }

    #[test]
    fn exec_encode_succeeds() {
        assert!(exec(encode_args("<b>hi & bye</b>")).is_ok());
    }

    #[test]
    fn exec_encode_every_mode_succeeds() {
        for mode in [HtmlModeCli::Attribute, HtmlModeCli::Text] {
            let result = exec(HtmlArgs {
                kind: HtmlCmd::Encode {
                    input: Some("a\"b'c".into()),
                    input_file: false,
                    mode: Some(mode),
                },
            });
            assert!(result.is_ok(), "{mode:?}");
        }
    }

    #[test]
    fn exec_decode_succeeds() {
        assert!(exec(decode_args("&lt;b&gt;hi &amp; bye&lt;/b&gt;")).is_ok());
        assert!(exec(decode_args("&#x1F680;")).is_ok());
    }

    #[test]
    fn exec_decode_unknown_entity_errors() {
        let result = exec(decode_args("&bogus;"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown HTML entity"));
    }

    #[test]
    fn exec_missing_input_errors() {
        let result = exec(HtmlArgs {
            kind: HtmlCmd::Decode {
                input: None,
                input_file: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(HtmlArgs {
            kind: HtmlCmd::Decode {
                input: Some("no-such-file-oxide.txt".into()),
                input_file: true,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn mode_maps_to_core() {
        assert_eq!(HtmlMode::from(HtmlModeCli::Attribute), HtmlMode::Attribute);
        assert_eq!(HtmlMode::from(HtmlModeCli::Text), HtmlMode::Text);
    }
}

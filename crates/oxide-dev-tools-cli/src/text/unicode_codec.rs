use clap::{Args, Subcommand, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::TextError;

/// `oxide text codec unicode [subcommand]` — unicode escape encode/decode dispatch
#[derive(Args)]
pub struct UnicodeArgs {
    #[command(subcommand)]
    pub kind: UnicodeCmd,
}

/// Escape style used when encoding.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum UnicodeStyleCli {
    /// JSON-style \uXXXX with surrogate pairs beyond U+FFFF
    #[default]
    Json,
    /// Rust-style \u{...} with 1-6 hex digits
    Rust,
}

impl From<UnicodeStyleCli> for UnicodeStyle {
    fn from(style: UnicodeStyleCli) -> Self {
        match style {
            UnicodeStyleCli::Json => UnicodeStyle::Json,
            UnicodeStyleCli::Rust => UnicodeStyle::Rust,
        }
    }
}

#[derive(Subcommand)]
pub enum UnicodeCmd {
    /// Escape non-ASCII characters as \u escape sequences
    #[command(
        name = "encode",
        after_help = "Examples:\n  oxide text codec unicode encode \"café 🚀\"\n  oxide text codec unicode encode \"café\" --style rust\n  oxide text codec unicode encode draft.txt --input-file\n  cat draft.txt | oxide text codec unicode encode -"
    )]
    Encode {
        /// Text to escape, a path to a file, or "-" to read from stdin
        input: Option<String>,

        /// Treat <INPUT> as a file path (a missing file is an error)
        #[arg(long)]
        input_file: bool,

        /// Escape style to use
        #[arg(long, value_enum)]
        style: Option<UnicodeStyleCli>,
    },

    /// Turn \uXXXX, \u{...}, or \UXXXXXXXX escapes back into characters
    #[command(
        name = "decode",
        after_help = "Examples:\n  oxide text codec unicode decode \"caf\\u00e9\"\n  oxide text codec unicode decode \"\\ud83d\\ude80\"\n  oxide text codec unicode decode draft.txt --input-file\n  cat draft.txt | oxide text codec unicode decode -"
    )]
    Decode {
        /// Escaped text to decode, a path to a file, or "-" to read from stdin
        input: Option<String>,

        /// Treat <INPUT> as a file path (a missing file is an error)
        #[arg(long)]
        input_file: bool,
    },
}

pub fn exec(args: UnicodeArgs) -> Result<(), TextError> {
    match args.kind {
        UnicodeCmd::Encode {
            input,
            input_file,
            style,
        } => {
            let input = super::resolve_text_input(&input, input_file)?;
            let options = UnicodeOptions {
                input,
                style: style.unwrap_or_default().into(),
            };
            println!("{}", convert_unicode(UnicodeKind::Encode(options))?);
        }
        UnicodeCmd::Decode { input, input_file } => {
            let input = super::resolve_text_input(&input, input_file)?;
            println!("{}", convert_unicode(UnicodeKind::Decode(input))?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_args(input: &str) -> UnicodeArgs {
        UnicodeArgs {
            kind: UnicodeCmd::Encode {
                input: Some(input.into()),
                input_file: false,
                style: None,
            },
        }
    }

    fn decode_args(input: &str) -> UnicodeArgs {
        UnicodeArgs {
            kind: UnicodeCmd::Decode {
                input: Some(input.into()),
                input_file: false,
            },
        }
    }

    #[test]
    fn exec_encode_succeeds() {
        assert!(exec(encode_args("café 🚀")).is_ok());
    }

    #[test]
    fn exec_encode_every_style_succeeds() {
        for style in [UnicodeStyleCli::Json, UnicodeStyleCli::Rust] {
            let result = exec(UnicodeArgs {
                kind: UnicodeCmd::Encode {
                    input: Some("café".into()),
                    input_file: false,
                    style: Some(style),
                },
            });
            assert!(result.is_ok(), "{style:?}");
        }
    }

    #[test]
    fn exec_decode_succeeds() {
        assert!(exec(decode_args(r"caf\u00e9")).is_ok());
        assert!(exec(decode_args(r"\ud83d\ude80")).is_ok());
        assert!(exec(decode_args(r"\u{1f680}")).is_ok());
    }

    #[test]
    fn exec_decode_malformed_escape_errors() {
        let result = exec(decode_args(r"\uZZZZ"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("malformed unicode escape"));
    }

    #[test]
    fn exec_decode_lone_surrogate_errors() {
        let result = exec(decode_args(r"\ud800"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not followed by a low surrogate")
        );
    }

    #[test]
    fn exec_missing_input_errors() {
        let result = exec(UnicodeArgs {
            kind: UnicodeCmd::Decode {
                input: None,
                input_file: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(UnicodeArgs {
            kind: UnicodeCmd::Decode {
                input: Some("no-such-file-oxide.txt".into()),
                input_file: true,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn style_maps_to_core() {
        assert_eq!(UnicodeStyle::from(UnicodeStyleCli::Json), UnicodeStyle::Json);
        assert_eq!(UnicodeStyle::from(UnicodeStyleCli::Rust), UnicodeStyle::Rust);
    }
}

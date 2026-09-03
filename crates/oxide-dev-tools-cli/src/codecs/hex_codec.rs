use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;

use crate::error::CodecError;

/// `oxide codec hex [subcommand]` — hex encode/decode dispatch
#[derive(Args)]
pub struct HexArgs {
    #[command(subcommand)]
    pub kind: HexCmd,
}

#[derive(Subcommand)]
pub enum HexCmd {
    /// Encode text as hex
    #[command(name = "encode")]
    Encode {
        /// Text to encode
        input: String,

        /// Use uppercase hex digits (A-F)
        #[arg(short = 'u', long = "upper")]
        upper: bool,
    },

    /// Decode hex into text
    #[command(name = "decode")]
    Decode {
        /// Hex text to decode (case-insensitive)
        input: String,
    },
}

pub fn exec(args: HexArgs) -> Result<(), CodecError> {
    match args.kind {
        HexCmd::Encode { input, upper } => {
            let opts = HexOptions {
                input,
                case: case_for(upper),
            };
            println!("{}", convert_hex(HexKind::Encode(opts))?);
        }
        HexCmd::Decode { input } => {
            let opts = HexOptions {
                input,
                case: HexCase::Lower,
            };
            println!("{}", convert_hex(HexKind::Decode(opts))?);
        }
    }
    Ok(())
}

fn case_for(upper: bool) -> HexCase {
    if upper { HexCase::Upper } else { HexCase::Lower }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_encode_lower() {
        assert!(
            exec(HexArgs {
                kind: HexCmd::Encode {
                    input: "hello".into(),
                    upper: false,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_encode_upper() {
        assert!(
            exec(HexArgs {
                kind: HexCmd::Encode {
                    input: "hello".into(),
                    upper: true,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode() {
        assert!(
            exec(HexArgs {
                kind: HexCmd::Decode {
                    input: "68656c6c6f".into(),
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode_invalid_input_errors() {
        let result = exec(HexArgs {
            kind: HexCmd::Decode {
                input: "68656c6c6z".into(),
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not valid hex"));
    }

    #[test]
    fn exec_decode_invalid_utf8_errors() {
        let result = exec(HexArgs {
            kind: HexCmd::Decode { input: "fffe".into() },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not valid UTF-8"));
    }
}

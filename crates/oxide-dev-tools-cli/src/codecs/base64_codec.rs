use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;

use crate::error::CodecError;

/// `oxide codec base64 [subcommand]` — base64 encode/decode dispatch
#[derive(Args)]
pub struct Base64Args {
    #[command(subcommand)]
    pub kind: Base64Cmd,
}

#[derive(Subcommand)]
pub enum Base64Cmd {
    /// Encode text as base64
    #[command(name = "encode")]
    Encode {
        /// Text to encode
        input: String,

        /// Use the URL-safe alphabet (A-Z, a-z, 0-9, -, _) without padding
        #[arg(short = 'u', long = "url")]
        url: bool,
    },

    /// Decode base64 into text
    #[command(name = "decode")]
    Decode {
        /// Base64 text to decode
        input: String,

        /// Use the URL-safe alphabet (A-Z, a-z, 0-9, -, _) without padding
        #[arg(short = 'u', long = "url")]
        url: bool,
    },
}

pub fn exec(args: Base64Args) -> Result<(), CodecError> {
    match args.kind {
        Base64Cmd::Encode { input, url } => {
            let opts = Base64Options {
                input,
                alphabet: alphabet_for(url),
            };
            println!("{}", convert_base64(Base64Kind::Encode(opts))?);
        }
        Base64Cmd::Decode { input, url } => {
            let opts = Base64Options {
                input,
                alphabet: alphabet_for(url),
            };
            println!("{}", convert_base64(Base64Kind::Decode(opts))?);
        }
    }
    Ok(())
}

fn alphabet_for(url: bool) -> Base64Alphabet {
    if url {
        Base64Alphabet::UrlSafe
    } else {
        Base64Alphabet::Standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_encode_standard() {
        assert!(
            exec(Base64Args {
                kind: Base64Cmd::Encode {
                    input: "hello".into(),
                    url: false,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_encode_url_safe() {
        assert!(
            exec(Base64Args {
                kind: Base64Cmd::Encode {
                    input: "hello".into(),
                    url: true,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode_standard() {
        assert!(
            exec(Base64Args {
                kind: Base64Cmd::Decode {
                    input: "aGVsbG8=".into(),
                    url: false,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode_url_safe() {
        assert!(
            exec(Base64Args {
                kind: Base64Cmd::Decode {
                    input: "aGVsbG8".into(),
                    url: true,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode_invalid_input_errors() {
        let result = exec(Base64Args {
            kind: Base64Cmd::Decode {
                input: "aGVsbG8!".into(),
                url: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not valid base64"));
    }

    #[test]
    fn exec_decode_invalid_utf8_errors() {
        let result = exec(Base64Args {
            kind: Base64Cmd::Decode {
                input: "//4=".into(),
                url: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not valid UTF-8"));
    }
}

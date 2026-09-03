use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;

use crate::error::CliError;

/// `oxide codec url [subcommand]` — URL encode/decode dispatch
#[derive(Args)]
pub struct UrlArgs {
    #[command(subcommand)]
    pub kind: UrlCmd,
}

#[derive(Subcommand)]
pub enum UrlCmd {
    /// Encode text as a URL component (RFC 3986 percent-encoding)
    #[command(name = "encode")]
    Encode {
        /// Text to encode
        input: String,

        /// Use form encoding (space becomes `+`)
        #[arg(long = "form")]
        form: bool,
    },

    /// Decode a percent-encoded URL component into text
    #[command(name = "decode")]
    Decode {
        /// URL-encoded text to decode
        input: String,

        /// Decode `+` as space (application/x-www-form-urlencoded)
        #[arg(long = "form")]
        form: bool,
    },
}

pub fn exec(args: UrlArgs) -> Result<(), CliError> {
    match args.kind {
        UrlCmd::Encode { input, form } => {
            let opts = UrlOptions {
                input,
                mode: mode_for(form),
            };
            println!("{}", convert_url(UrlKind::Encode(opts))?);
        }
        UrlCmd::Decode { input, form } => {
            let opts = UrlOptions {
                input,
                mode: mode_for(form),
            };
            println!("{}", convert_url(UrlKind::Decode(opts))?);
        }
    }
    Ok(())
}

fn mode_for(form: bool) -> UrlMode {
    if form { UrlMode::Form } else { UrlMode::Standard }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_encode_standard() {
        assert!(
            exec(UrlArgs {
                kind: UrlCmd::Encode {
                    input: "hello world".into(),
                    form: false,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_encode_form() {
        assert!(
            exec(UrlArgs {
                kind: UrlCmd::Encode {
                    input: "hello world".into(),
                    form: true,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode_standard() {
        assert!(
            exec(UrlArgs {
                kind: UrlCmd::Decode {
                    input: "hello%20world".into(),
                    form: false,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode_form() {
        assert!(
            exec(UrlArgs {
                kind: UrlCmd::Decode {
                    input: "hello+world".into(),
                    form: true,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_decode_invalid_input_errors() {
        let result = exec(UrlArgs {
            kind: UrlCmd::Decode {
                input: "hello%GG".into(),
                form: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("percent-encoded"));
    }

    #[test]
    fn exec_decode_invalid_utf8_errors() {
        let result = exec(UrlArgs {
            kind: UrlCmd::Decode {
                input: "%FF%FE".into(),
                form: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not valid UTF-8"));
    }
}

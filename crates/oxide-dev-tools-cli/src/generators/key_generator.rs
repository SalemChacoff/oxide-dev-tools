use clap::{Args, Subcommand, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::GenError;

/// `oxide gen key [subcommand]` — key/token generator dispatch
#[derive(Args)]
pub struct KeyArgs {
    #[command(subcommand)]
    pub kind: KeyCmd,
}

#[derive(Subcommand)]
pub enum KeyCmd {
    /// Generate a password
    #[command(
        name = "pass",
        after_help = "Examples:\n  oxide gen key pass\n  oxide gen key pass --length 32 --special\n  oxide gen key pass --no-digits --special"
    )]
    Pass {
        /// Length of the password
        #[arg(short = 'l', long = "length", default_value_t = 16)]
        length: usize,

        /// Exclude lowercase letters (enabled by default)
        #[arg(short = 'w', long = "no-lowercase")]
        no_lowercase: bool,

        /// Exclude uppercase letters (enabled by default)
        #[arg(short = 'u', long = "no-uppercase")]
        no_uppercase: bool,

        /// Exclude digits (enabled by default)
        #[arg(short = 'd', long = "no-digits")]
        no_digits: bool,

        /// Include special characters (!@#$%^&*)
        #[arg(short = 's', long = "special")]
        special: bool,
    },

    /// Generate a random token
    #[command(
        name = "token",
        after_help = "Examples:\n  oxide gen key token\n  oxide gen key token --length 16 --encoding base64"
    )]
    Token {
        /// Number of random bytes to generate (hex output is 2× this length)
        #[arg(short = 'l', long = "length", default_value_t = 32)]
        length: usize,

        /// Output encoding
        #[arg(short = 'e', long = "encoding", value_enum, default_value_t = TokenCmdEncoding::Hex)]
        encoding: TokenCmdEncoding,
    },

    /// Generate an HS256 JWT signed from a JSON payload
    #[command(
        name = "jwt",
        after_help = r#"Examples:
  oxide gen key jwt '{"sub":"user-1"}' --secret my-secret --exp 1h
  oxide gen key jwt '{"sub":"user-1","exp":1750000000}' --secret my-secret
> Note: on Windows shells, JSON quoting differs — e.g. oxide gen key jwt "{\"sub\":\"user-1\"}" --secret my-secret"#
    )]
    Jwt {
        /// JSON object with the token claims. Must contain a non-empty "sub" claim.
        payload: String,

        /// HMAC-SHA256 secret used to sign the token
        #[arg(long)]
        secret: String,

        /// Expiry: duration (1h, 30m, 90s, 1d), seconds from now (3600),
        /// or absolute Unix timestamp (1750000000). Ignored when the payload
        /// already has an "exp" claim.
        #[arg(long)]
        exp: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TokenCmdEncoding {
    Hex,
    Base64,
}

impl From<TokenCmdEncoding> for TokenEncoding {
    fn from(e: TokenCmdEncoding) -> Self {
        match e {
            TokenCmdEncoding::Hex => TokenEncoding::Hex,
            TokenCmdEncoding::Base64 => TokenEncoding::Base64,
        }
    }
}

pub fn exec(args: KeyArgs) -> Result<(), GenError> {
    match args.kind {
        KeyCmd::Pass {
            length,
            no_lowercase,
            no_uppercase,
            no_digits,
            special,
        } => {
            let opts = PasswordOptions {
                length,
                lowercase: !no_lowercase,
                uppercase: !no_uppercase,
                digits: !no_digits,
                special,
            };
            println!("{}", generate_key(KeyKind::Password(opts))?);
        }
        KeyCmd::Token { length, encoding } => {
            let opts = TokenOptions {
                length,
                encoding: encoding.into(),
            };
            println!("{}", generate_key(KeyKind::Token(opts))?);
        }
        KeyCmd::Jwt { payload, secret, exp } => {
            let options = JwtOptions {
                payload,
                secret,
                exp: parse_exp(exp)?,
            };
            println!("{}", generate_jwt(options)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_pass_default() {
        assert!(
            exec(KeyArgs {
                kind: KeyCmd::Pass {
                    length: 16,
                    no_lowercase: false,
                    no_uppercase: false,
                    no_digits: false,
                    special: false,
                }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_pass_no_character_set_errors() {
        let result = exec(KeyArgs {
            kind: KeyCmd::Pass {
                length: 16,
                no_lowercase: true,
                no_uppercase: true,
                no_digits: true,
                special: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("character set"));
    }

    #[test]
    fn exec_token_hex_default() {
        assert!(
            exec(KeyArgs {
                kind: KeyCmd::Token {
                    length: 32,
                    encoding: TokenCmdEncoding::Hex,
                }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_token_base64() {
        assert!(
            exec(KeyArgs {
                kind: KeyCmd::Token {
                    length: 30,
                    encoding: TokenCmdEncoding::Base64,
                }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_jwt_valid() {
        assert!(
            exec(KeyArgs {
                kind: KeyCmd::Jwt {
                    payload: r#"{"sub":"user-1"}"#.into(),
                    secret: "s3cret".into(),
                    exp: Some("1h".into()),
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_jwt_expiry_in_payload() {
        assert!(
            exec(KeyArgs {
                kind: KeyCmd::Jwt {
                    payload: r#"{"sub":"user-1","exp":1750000000}"#.into(),
                    secret: "s3cret".into(),
                    exp: None,
                },
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_jwt_invalid_json_errors() {
        let result = exec(KeyArgs {
            kind: KeyCmd::Jwt {
                payload: "not json".into(),
                secret: "s3cret".into(),
                exp: None,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid JSON payload"));
    }

    #[test]
    fn exec_jwt_missing_sub_errors() {
        let result = exec(KeyArgs {
            kind: KeyCmd::Jwt {
                payload: r#"{"aud":"app"}"#.into(),
                secret: "s3cret".into(),
                exp: None,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("\"sub\""));
    }

    #[test]
    fn exec_jwt_invalid_exp_errors() {
        let result = exec(KeyArgs {
            kind: KeyCmd::Jwt {
                payload: r#"{"sub":"user-1"}"#.into(),
                secret: "s3cret".into(),
                exp: Some("bogus".into()),
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid expiry"));
    }

    #[test]
    fn exec_jwt_empty_secret_errors() {
        let result = exec(KeyArgs {
            kind: KeyCmd::Jwt {
                payload: r#"{"sub":"user-1"}"#.into(),
                secret: String::new(),
                exp: None,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("secret"));
    }
}

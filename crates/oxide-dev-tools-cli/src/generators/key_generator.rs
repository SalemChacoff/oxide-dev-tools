use clap::{Args, Subcommand, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::CliError;

/// `oxide gen key [subcommand]` — key/token generator dispatch
#[derive(Args)]
pub struct KeyArgs {
    #[command(subcommand)]
    pub kind: KeyCmd,
}

#[derive(Subcommand)]
pub enum KeyCmd {
    /// Generate a password
    #[command(name = "pass")]
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
    #[command(name = "token")]
    Token {
        /// Number of random bytes to generate (hex output is 2× this length)
        #[arg(short = 'l', long = "length", default_value_t = 32)]
        length: usize,

        /// Output encoding
        #[arg(short = 'e', long = "encoding", value_enum, default_value_t = TokenCmdEncoding::Hex)]
        encoding: TokenCmdEncoding,
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

pub fn exec(args: KeyArgs) -> Result<(), CliError> {
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
}

use clap::{Args, Subcommand, ValueEnum};
use oxide_dev_tools_core::*;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{GenError, GenericError};

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

    /// Generate an HS256 JWT signed from a JSON payload
    #[command(name = "jwt")]
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

/// Parse the `--exp` argument into an absolute Unix timestamp in seconds.
///
/// Accepts a duration with a suffix (`1h`, `30m`, `90s`, `1d`), a plain
/// number of seconds from now (`3600`), or an absolute Unix timestamp
/// (`1750000000`, values >= 1_000_000_000). Returns `None` when no argument
/// is given.
fn parse_exp(exp: Option<String>) -> Result<Option<u64>, GenericError> {
    let Some(value) = exp else {
        return Ok(None);
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let seconds = if let Some(ttl) = parse_duration_seconds(&value) {
        now.saturating_add(ttl)
    } else if let Ok(timestamp) = value.parse::<u64>() {
        // Values >= 1e9 are dates after 2001, so treat them as absolute
        // timestamps; smaller values are more naturally TTL seconds.
        if timestamp >= 1_000_000_000 {
            timestamp
        } else {
            now.saturating_add(timestamp)
        }
    } else {
        let message = format!(
            "invalid expiry \"{value}\": expected a duration (1h, 30m, 90s, 1d), \
             seconds from now (3600), or an absolute Unix timestamp (1750000000)"
        );
        return Err(GenericError::from(message));
    };
    Ok(Some(seconds))
}

/// Parse a duration with a unit suffix (`s`, `m`, `h`, `d`) into seconds.
fn parse_duration_seconds(value: &str) -> Option<u64> {
    let (amount, unit) = value.split_at_checked(value.len().saturating_sub(1))?;
    let amount = amount.parse::<u64>().ok()?;
    let multiplier = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        _ => return None,
    };
    Some(amount.saturating_mul(multiplier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn now_secs() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

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

    #[test]
    fn parse_exp_none_returns_none() {
        assert!(parse_exp(None).unwrap().is_none());
    }

    #[test]
    fn parse_exp_duration_suffixes() {
        for (value, ttl) in [
            ("30s", 30u64),
            ("30m", 1_800),
            ("1h", 3_600),
            ("1d", 86_400),
            ("2d", 172_800),
        ] {
            let before = now_secs();
            let parsed = parse_exp(Some(value.into())).unwrap().unwrap();
            let after = now_secs();
            assert!(
                (before + ttl..=after + ttl).contains(&parsed),
                "expected {value} to map to roughly now + {ttl}s, got {parsed}"
            );
        }
    }

    #[test]
    fn parse_exp_seconds_from_now() {
        let before = now_secs();
        let parsed = parse_exp(Some("3600".into())).unwrap().unwrap();
        let after = now_secs();
        assert!((before + 3_600..=after + 3_600).contains(&parsed));
    }

    #[test]
    fn parse_exp_absolute_timestamp() {
        let parsed = parse_exp(Some("1750000000".into())).unwrap().unwrap();
        assert_eq!(parsed, 1_750_000_000);
    }

    #[test]
    fn parse_exp_invalid_errors() {
        let err = parse_exp(Some("bogus".into())).unwrap_err().to_string();
        assert!(err.contains("invalid expiry"));
    }

    #[test]
    fn parse_exp_bad_suffix_errors() {
        let err = parse_exp(Some("12x".into())).unwrap_err().to_string();
        assert!(err.contains("invalid expiry"));
    }
}

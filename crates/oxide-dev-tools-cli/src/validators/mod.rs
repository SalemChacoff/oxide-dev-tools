pub mod email_validator;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide validate ...` — entry point for all validators
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide validate email user@example.com\n  oxide validate email user@example.com --verbose\n  oxide validate email '用户@例子.广告'"
)]
pub struct ValidateArgs {
    #[command(subcommand)]
    pub kind: ValidateKind,
}

#[derive(Subcommand)]
pub enum ValidateKind {
    /// Validate an email address (RFC 5321/5322, IDN, SMTPUTF8, address literals).
    #[command(
        after_help = "Examples:\n  oxide validate email user@example.com\n  oxide validate email '\"john doe\"@example.com'\n  oxide validate email '用户@例子.广告'\n  oxide validate email 'user@[IPv6:2001:db8::1]'\n  oxide validate email user@example.com --verbose\n  oxide validate email 'user (comment) @example.com' --mode header\n  oxide validate email user@localhost --require-tld\n  oxide validate email user@example.com --ascii"
    )]
    Email(email_validator::EmailArgs),
}

pub fn exec(args: ValidateArgs) -> Result<(), CliError> {
    match args.kind {
        ValidateKind::Email(args) => email_validator::exec(args).map_err(Into::into),
    }
}

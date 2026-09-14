pub mod email_validator;
pub mod url_validator;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide validate ...` — entry point for all validators
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide validate email user@example.com\n  oxide validate email user@example.com --verbose\n  oxide validate email '用户@例子.广告'\n  oxide validate url https://example.com"
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
    /// Validate a URL or URI (WHATWG URL Standard, any scheme, IDN, IPv4/IPv6).
    #[command(
        after_help = "Examples:\n  oxide validate url https://example.com/path?q=1\n  oxide validate url 'https://例子.测试/路径'\n  oxide validate url 'http://[2001:db8::1]:8080/'\n  oxide validate url urn:isbn:0451450523\n  oxide validate url https://example.com --verbose\n  oxide validate url ftp://example.com --scheme https --scheme http\n  oxide validate url mailto:user@example.com --require-host"
    )]
    Url(url_validator::UrlArgs),
}

pub fn exec(args: ValidateArgs) -> Result<(), CliError> {
    match args.kind {
        ValidateKind::Email(args) => email_validator::exec(args).map_err(Into::into),
        ValidateKind::Url(args) => url_validator::exec(args).map_err(Into::into),
    }
}

pub mod email_validator;
pub mod ip_validator;
pub mod url_validator;
pub mod uuid_validator;

use clap::{Args, Subcommand};

use crate::error::CliError;

/// `oxide validate ...` — entry point for all validators
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide validate email user@example.com\n  oxide validate email user@example.com --verbose\n  oxide validate email '用户@例子.广告'\n  oxide validate url https://example.com\n  oxide validate ip 192.168.1.1\n  oxide validate uuid 550e8400-e29b-41d4-a716-446655440000"
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
    /// Validate an IP address (IPv4/IPv6, RFC 6890 classification, canonical form).
    #[command(
        after_help = "Examples:\n  oxide validate ip 192.168.1.1\n  oxide validate ip '2001:db8::1'\n  oxide validate ip '2001:0DB8:0:0::1'\n  oxide validate ip 8.8.8.8 --verbose\n  oxide validate ip 127.0.0.1 --require-global\n  oxide validate ip '192.168.001.1' --no-leading-zeros\n  oxide validate ip 'fe80::1%eth0'\n  oxide validate ip 1.2.3.4 --mode ipv4"
    )]
    Ip(ip_validator::IpArgs),
    /// Validate a UUID (v1–v8, nil/max, hyphenated/simple/braced/URN forms).
    #[command(
        after_help = "Examples:\n  oxide validate uuid 550e8400-e29b-41d4-a716-446655440000\n  oxide validate uuid '550E8400E29B41D4A716446655440000'\n  oxide validate uuid '{550e8400-e29b-41d4-a716-446655440000}'\n  oxide validate uuid urn:uuid:550e8400-e29b-41d4-a716-446655440000\n  oxide validate uuid 550e8400-e29b-41d4-a716-446655440000 --version v4\n  oxide validate uuid 550e8400-e29b-41d4-a716-446655440000 --kind simple\n  oxide validate uuid 550e8400-e29b-41d4-a716-446655440000 --variant rfc4122\n  oxide validate uuid 550e8400-e29b-41d4-a716-446655440000 --verbose"
    )]
    Uuid(uuid_validator::UuidArgs),
}

pub fn exec(args: ValidateArgs) -> Result<(), CliError> {
    match args.kind {
        ValidateKind::Email(args) => email_validator::exec(args).map_err(Into::into),
        ValidateKind::Url(args) => url_validator::exec(args).map_err(Into::into),
        ValidateKind::Ip(args) => ip_validator::exec(args).map_err(Into::into),
        ValidateKind::Uuid(args) => uuid_validator::exec(args).map_err(Into::into),
    }
}

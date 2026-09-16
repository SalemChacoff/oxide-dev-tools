pub mod card_validator;
pub mod email_validator;
pub mod ip_validator;
pub mod syntax_validator;
pub mod url_validator;
pub mod uuid_validator;

use clap::{Args, Subcommand};
use oxide_dev_tools_core::SyntaxKind;

use crate::error::CliError;

/// `oxide validate ...` — entry point for all validators
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide validate email user@example.com\n  oxide validate email user@example.com --verbose\n  oxide validate email '用户@例子.广告'\n  oxide validate url https://example.com\n  oxide validate ip 192.168.1.1\n  oxide validate uuid 550e8400-e29b-41d4-a716-446655440000\n  oxide validate card 4111111111111111\n  oxide validate json '{\"a\": 1}'\n  oxide validate yaml 'a: 1'\n  oxide validate xml '<root/>'"
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
    /// Validate a credit card number (Luhn checksum and issuer network detection).
    #[command(
        after_help = "Examples:\n  oxide validate card 4111111111111111\n  oxide validate card '5555 5555 5555 4444'\n  oxide validate card '3782-822463-10005' --verbose\n  oxide validate card 6011111111111117 --network discover\n  oxide validate card 6200000000000005 --network mastercard\n  oxide validate card 79927398713 --require-network\n  oxide validate card '4111 1111 1111 1111' --no-separators"
    )]
    Card(card_validator::CardArgs),
    /// Validate JSON document syntax (well-formedness with line/column errors).
    #[command(
        after_help = "Examples:\n  oxide validate json '{\"a\": 1}'\n  oxide validate json '{\"a\": 1}' --verbose\n  oxide validate json data.json --input-file\n  oxide validate json '{broken'"
    )]
    Json(syntax_validator::SyntaxArgs),
    /// Validate YAML document syntax (single-document streams).
    #[command(
        after_help = "Examples:\n  oxide validate yaml 'a: 1'\n  oxide validate yaml 'items:\n  - one\n  - two'\n  oxide validate yaml doc.yaml --input-file\n  oxide validate yaml 'a: [1'"
    )]
    Yaml(syntax_validator::SyntaxArgs),
    /// Validate XML document syntax (well-formedness, root/depth, DTD policy).
    #[command(
        after_help = "Examples:\n  oxide validate xml '<root/>'\n  oxide validate xml '<r><a>1</a></r>' --verbose\n  oxide validate xml '<?xml version=\"1.0\"?><r/>'\n  oxide validate xml '<!DOCTYPE r><r/>' --allow-dtd\n  oxide validate xml doc.xml --input-file\n  oxide validate xml '<r>'"
    )]
    Xml(syntax_validator::SyntaxArgs),
}

pub fn exec(args: ValidateArgs) -> Result<(), CliError> {
    match args.kind {
        ValidateKind::Email(args) => email_validator::exec(args).map_err(Into::into),
        ValidateKind::Url(args) => url_validator::exec(args).map_err(Into::into),
        ValidateKind::Ip(args) => ip_validator::exec(args).map_err(Into::into),
        ValidateKind::Uuid(args) => uuid_validator::exec(args).map_err(Into::into),
        ValidateKind::Card(args) => card_validator::exec(args).map_err(Into::into),
        ValidateKind::Json(args) => syntax_validator::exec(args, SyntaxKind::Json).map_err(Into::into),
        ValidateKind::Yaml(args) => syntax_validator::exec(args, SyntaxKind::Yaml).map_err(Into::into),
        ValidateKind::Xml(args) => syntax_validator::exec(args, SyntaxKind::Xml).map_err(Into::into),
    }
}

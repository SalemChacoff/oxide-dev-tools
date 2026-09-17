use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::ValidError;

/// `oxide validate email <ADDRESS> [options]` — RFC-compliant email validation
#[derive(Args)]
pub struct EmailArgs {
    /// Email address to validate
    pub address: String,

    /// Validation mode: standard (RFC 5321 mailbox) or header (RFC 5322 addr-spec)
    #[arg(long, value_enum, default_value_t = EmailModeArg::Standard)]
    pub mode: EmailModeArg,

    /// Reject non-ASCII characters (SMTPUTF8 local parts and IDN domains)
    #[arg(long)]
    pub ascii: bool,

    /// Reject quoted-string local parts (e.g. "john doe"@example.com)
    #[arg(long)]
    pub no_quoted: bool,

    /// Reject address literals (e.g. user@[192.0.2.1])
    #[arg(long)]
    pub no_literal: bool,

    /// Reject single-label domains (e.g. user@localhost)
    #[arg(long)]
    pub require_tld: bool,

    /// Print the full validation report instead of the bare normalized address
    #[arg(long)]
    pub verbose: bool,
}

/// Validation mode choices for the `--mode` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EmailModeArg {
    /// Strict RFC 5321 SMTP mailbox grammar.
    Standard,
    /// Lenient RFC 5322 addr-spec grammar (message headers).
    Header,
}

impl From<EmailModeArg> for EmailMode {
    fn from(arg: EmailModeArg) -> Self {
        match arg {
            EmailModeArg::Standard => EmailMode::Standard,
            EmailModeArg::Header => EmailMode::Header,
        }
    }
}

pub fn exec(args: EmailArgs) -> Result<(), ValidError> {
    let options = EmailOptions {
        address: args.address,
        mode: args.mode.into(),
        allow_quoted_local: !args.no_quoted,
        allow_address_literal: !args.no_literal,
        allow_utf8: !args.ascii,
        require_tld: args.require_tld,
    };
    let report = validate_email(options);
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if args.verbose {
        print_report(&report);
    }
    if report.valid {
        if !args.verbose {
            println!("{}", report.normalized);
        }
        Ok(())
    } else {
        Err(ValidError::Email(report.issues.join("; ")))
    }
}

// -------- Report output --------

fn print_report(report: &EmailReport) {
    println!("valid:      {}", yes_no(report.valid));
    println!("input:      {}", report.input);
    println!("normalized: {}", report.normalized);
    println!("local:      {}", report.local_part);
    println!("domain:     {} ({})", report.domain, domain_kind_name(report.domain_kind));
    if report.smtputf8 {
        println!("smtputf8:   yes");
    }
    if report.idn {
        println!("idn:        yes");
    }
    if !report.issues.is_empty() {
        println!("issues:");
        for issue in &report.issues {
            println!("  - {issue}");
        }
    }
    if !report.warnings.is_empty() {
        println!("warnings:");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
}

fn yes_no(valid: bool) -> &'static str {
    if valid { "yes" } else { "no" }
}

fn domain_kind_name(kind: EmailDomainKind) -> &'static str {
    match kind {
        EmailDomainKind::Dns => "dns",
        EmailDomainKind::Ipv4Literal => "ipv4-literal",
        EmailDomainKind::Ipv6Literal => "ipv6-literal",
        EmailDomainKind::GeneralLiteral => "general-literal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(address: &str) -> EmailArgs {
        EmailArgs {
            address: address.to_string(),
            mode: EmailModeArg::Standard,
            ascii: false,
            no_quoted: false,
            no_literal: false,
            require_tld: false,
            verbose: false,
        }
    }

    #[test]
    fn exec_valid_addresses() {
        assert!(exec(args("user@example.com")).is_ok());
        assert!(exec(args("\"john doe\"@example.com")).is_ok());
        assert!(exec(args("user@[IPv6:::1]")).is_ok());
        assert!(exec(args("用户@例子.广告")).is_ok());
    }

    #[test]
    fn exec_invalid_address() {
        let result = exec(args("not-an-address"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid email"));
    }

    #[test]
    fn exec_header_mode() {
        assert!(
            exec(EmailArgs {
                address: "user (comment) @example.com".to_string(),
                mode: EmailModeArg::Header,
                ..args("")
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_flags() {
        let result = exec(EmailArgs {
            ascii: true,
            ..args("用户@例子.广告")
        });
        assert!(result.unwrap_err().to_string().contains("non-ASCII"));

        let result = exec(EmailArgs {
            no_quoted: true,
            ..args("\"john\"@example.com")
        });
        assert!(result.unwrap_err().to_string().contains("quoted"));

        let result = exec(EmailArgs {
            no_literal: true,
            ..args("user@[192.0.2.1]")
        });
        assert!(result.unwrap_err().to_string().contains("literal"));

        let result = exec(EmailArgs {
            require_tld: true,
            ..args("user@localhost")
        });
        assert!(result.unwrap_err().to_string().contains("two labels"));
    }

    #[test]
    fn exec_verbose_valid_and_invalid() {
        assert!(
            exec(EmailArgs {
                verbose: true,
                ..args("user@example.com")
            })
            .is_ok()
        );
        assert!(
            exec(EmailArgs {
                verbose: true,
                ..args("nope")
            })
            .is_err()
        );
    }
}

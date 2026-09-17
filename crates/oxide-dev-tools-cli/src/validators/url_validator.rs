use clap::Args;
use oxide_dev_tools_core::*;

use crate::error::ValidError;

/// `oxide validate url <URL> [options]` — WHATWG URL/URI validation
#[derive(Args)]
pub struct UrlArgs {
    /// URL or URI to validate (absolute form required)
    pub input: String,

    /// Restrict allowed schemes (repeatable, case-insensitive)
    #[arg(long)]
    pub scheme: Vec<String>,

    /// Reject host-less URLs (e.g. mailto:, urn:, data:)
    #[arg(long)]
    pub require_host: bool,

    /// Print the full validation report instead of the bare normalized URL
    #[arg(long)]
    pub verbose: bool,
}

pub fn exec(args: UrlArgs) -> Result<(), ValidError> {
    let options = UrlValidationOptions {
        input: args.input,
        allowed_schemes: if args.scheme.is_empty() {
            None
        } else {
            Some(args.scheme)
        },
        require_host: args.require_host,
    };
    let report = validate_url(options);
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
        Err(ValidError::Url(report.issues.join("; ")))
    }
}

// -------- Report output --------

fn print_report(report: &UrlReport) {
    println!("valid:      {}", yes_no(report.valid));
    println!("input:      {}", report.input);
    println!("normalized: {}", report.normalized);
    println!("scheme:     {}", report.scheme);
    println!("host:       {} ({})", report.host, host_kind_name(report.host_kind));
    if let Some(port) = report.port {
        println!("port:       {port}");
    }
    println!("path:       {}", report.path);
    if let Some(query) = &report.query {
        println!("query:      {query}");
    }
    if let Some(fragment) = &report.fragment {
        println!("fragment:   {fragment}");
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

fn host_kind_name(kind: UrlHostKind) -> &'static str {
    match kind {
        UrlHostKind::None => "none",
        UrlHostKind::Domain => "domain",
        UrlHostKind::Ipv4 => "ipv4",
        UrlHostKind::Ipv6 => "ipv6",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &str) -> UrlArgs {
        UrlArgs {
            input: input.to_string(),
            scheme: Vec::new(),
            require_host: false,
            verbose: false,
        }
    }

    #[test]
    fn exec_valid_urls() {
        assert!(exec(args("https://example.com")).is_ok());
        assert!(exec(args("https://例子.测试/路径")).is_ok());
        assert!(exec(args("http://[2001:db8::1]:8080/")).is_ok());
        assert!(exec(args("urn:isbn:0451450523")).is_ok());
        assert!(exec(args("mailto:user@example.com")).is_ok());
    }

    #[test]
    fn exec_invalid_url() {
        let result = exec(args("not a url"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid url"));
    }

    #[test]
    fn exec_flags() {
        let result = exec(UrlArgs {
            scheme: vec!["https".to_string()],
            ..args("ftp://example.com/")
        });
        assert!(result.unwrap_err().to_string().contains("not allowed"));

        let result = exec(UrlArgs {
            require_host: true,
            ..args("mailto:user@example.com")
        });
        assert!(result.unwrap_err().to_string().contains("has no host"));

        let result = exec(UrlArgs {
            scheme: vec!["https".to_string()],
            ..args("HTTPS://example.com/")
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_verbose_valid_and_invalid() {
        assert!(
            exec(UrlArgs {
                verbose: true,
                ..args("https://example.com")
            })
            .is_ok()
        );
        assert!(
            exec(UrlArgs {
                verbose: true,
                ..args("nope")
            })
            .is_err()
        );
    }
}

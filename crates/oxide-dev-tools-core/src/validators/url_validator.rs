//! WHATWG URL/URI validation.
//!
//! [`validate_url`] checks an absolute URL or URI against the WHATWG URL
//! Standard: scheme, userinfo, host (IDN, IPv4, IPv6), port, path, query, and
//! fragment are parsed and normalized. Non-hierarchical URIs such as
//! `mailto:`, `urn:`, and `data:` are accepted because any absolute scheme
//! parses; use [`UrlValidationOptions::allowed_schemes`] to restrict the set
//! and [`UrlValidationOptions::require_host`] to reject host-less forms.
//!
//! Internationalized domain names are validated via IDNA and reported in
//! their punycode form. Relative references (no scheme) are rejected, as are
//! surrounding whitespace. The validator performs no network lookups; syntax
//! is checked locally.

use url::{Host, Url};

// -------- Public API --------

/// How the host component was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlHostKind {
    /// The URL has no host (e.g. `mailto:`, `urn:`, `data:`).
    None,
    /// A regular domain name, possibly internationalized (punycode form).
    Domain,
    /// An IPv4 address.
    Ipv4,
    /// An IPv6 address.
    Ipv6,
}

/// Options for [`validate_url`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UrlValidationOptions {
    /// The URL or URI to validate (surrounding whitespace is ignored).
    pub input: String,
    /// Restrict allowed schemes (compared case-insensitively); `None`
    /// accepts every scheme.
    pub allowed_schemes: Option<Vec<String>>,
    /// Reject host-less URLs such as `mailto:` and `urn:`.
    pub require_host: bool,
}

/// The result of validating a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlReport {
    /// Whether the URL satisfies the configured rules.
    pub valid: bool,
    /// The URL as given, without surrounding whitespace.
    pub input: String,
    /// The URL with the scheme and host lowercased, dot segments resolved,
    /// and IDN hosts converted to punycode.
    pub normalized: String,
    /// The scheme, lowercased.
    pub scheme: String,
    /// The host as serialized (punycode for IDN domains).
    pub host: String,
    /// How the host was written.
    pub host_kind: UrlHostKind,
    /// The port number, if the URL has an explicit one.
    pub port: Option<u16>,
    /// The path in its percent-encoded form.
    pub path: String,
    /// The query string, if present.
    pub query: Option<String>,
    /// The fragment, if present.
    pub fragment: Option<String>,
    /// Reasons the URL is invalid.
    pub issues: Vec<String>,
    /// Valid but unusual or discouraged constructs.
    pub warnings: Vec<String>,
}

/// Validate a URL or URI and produce a detailed report.
///
/// Validation never fails with an error and never panics: an invalid URL is
/// reported via [`UrlReport::valid`] and [`UrlReport::issues`].
pub fn validate_url(options: UrlValidationOptions) -> UrlReport {
    let trimmed = options.input.trim();
    let mut report = UrlReport {
        valid: false,
        input: trimmed.to_string(),
        normalized: String::new(),
        scheme: String::new(),
        host: String::new(),
        host_kind: UrlHostKind::None,
        port: None,
        path: String::new(),
        query: None,
        fragment: None,
        issues: Vec::new(),
        warnings: Vec::new(),
    };

    if trimmed.is_empty() {
        report.issues.push("input is empty".to_string());
        return report;
    }

    match Url::parse(trimmed) {
        Ok(parsed) => finish(parsed, &options, &mut report),
        Err(err) => report.issues.push(describe_parse_error(err)),
    }
    report.valid = report.issues.is_empty();
    report
}

// -------- Report assembly --------

fn finish(parsed: Url, options: &UrlValidationOptions, report: &mut UrlReport) {
    report.scheme = parsed.scheme().to_string();
    report.host = parsed.host_str().unwrap_or_default().to_string();
    report.host_kind = match parsed.host() {
        Some(Host::Domain(_)) => UrlHostKind::Domain,
        Some(Host::Ipv4(_)) => UrlHostKind::Ipv4,
        Some(Host::Ipv6(_)) => UrlHostKind::Ipv6,
        None => UrlHostKind::None,
    };
    report.port = parsed.port();
    report.path = parsed.path().to_string();
    report.query = parsed.query().map(str::to_string);
    report.fragment = parsed.fragment().map(str::to_string);
    report.normalized = parsed.to_string();

    if let Some(allowed) = &options.allowed_schemes {
        if !allowed.iter().any(|scheme| scheme.eq_ignore_ascii_case(&report.scheme)) {
            report
                .issues
                .push(format!("scheme '{}' is not allowed (allowed: {})", report.scheme, allowed.join(", ")));
        }
    }

    if options.require_host && parsed.host().is_none() {
        report.issues.push("URL has no host".to_string());
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        report.warnings.push("URL contains embedded credentials".to_string());
    }
}

// -------- Parse errors --------

fn describe_parse_error(err: url::ParseError) -> String {
    use url::ParseError;
    match err {
        ParseError::EmptyHost => "host is empty".to_string(),
        ParseError::IdnaError => "invalid internationalized domain name".to_string(),
        ParseError::InvalidPort => "invalid port number".to_string(),
        ParseError::InvalidIpv4Address => "invalid IPv4 address".to_string(),
        ParseError::InvalidIpv6Address => "invalid IPv6 address".to_string(),
        ParseError::InvalidDomainCharacter => "invalid character in domain name".to_string(),
        ParseError::RelativeUrlWithoutBase => "missing scheme (relative URL)".to_string(),
        ParseError::RelativeUrlWithCannotBeABaseBase => "relative URL with a cannot-be-a-base scheme".to_string(),
        ParseError::SetHostOnCannotBeABaseUrl => "cannot set a host on this scheme".to_string(),
        ParseError::Overflow => "URL component is too large".to_string(),
        other => format!("{other}"),
    }
}

// -------- Tests --------

#[cfg(test)]
mod tests {
    use super::*;

    fn check(input: &str) -> UrlReport {
        validate_url(UrlValidationOptions {
            input: input.to_string(),
            ..Default::default()
        })
    }

    #[test]
    fn valid_urls() {
        let valid = [
            "https://example.com",
            "https://example.com/path/to?q=1#frag",
            "HTTP://EXAMPLE.com/Path",
            "http://localhost:3000/",
            "https://user:secret@example.com/",
            "http://192.0.2.1:8080/x",
            "http://[2001:db8::1]/",
            "http://[::1]",
            "https://例子.测试/路径",
            "https://münchen.de/",
            "https://xn--mnchen-3ya.de/",
            "ftp://example.com/files",
            "custom+scheme://example.com/x",
            "mailto:user@example.com",
            "urn:isbn:0451450523",
            "data:text/plain,hello",
        ];
        for input in valid {
            let report = check(input);
            assert!(report.valid, "{input}: {:?}", report.issues);
        }
    }

    #[test]
    fn invalid_urls() {
        let cases = [
            ("", "input is empty"),
            ("   ", "input is empty"),
            ("example.com", "missing scheme"),
            ("/path/only", "missing scheme"),
            ("https://", "host is empty"),
            ("http://exa mple.com/", "internationalized"),
            ("http://example.com:99999/", "invalid port"),
            ("http://[:::]", "invalid IPv6"),
            ("http://999.999.999.999/", "invalid IPv4"),
        ];
        for (input, expected) in cases {
            let report = check(input);
            assert!(!report.valid, "{input} should be invalid");
            assert!(
                report.issues.iter().any(|issue| issue.contains(expected)),
                "{input}: expected '{expected}' in {:?}",
                report.issues
            );
        }
    }

    #[test]
    fn report_parts_for_full_url() {
        let report = check("https://user:pw@example.com:8443/path/to?q=1&r=2#frag");
        assert!(report.valid);
        assert_eq!(report.scheme, "https");
        assert_eq!(report.host, "example.com");
        assert_eq!(report.host_kind, UrlHostKind::Domain);
        assert_eq!(report.port, Some(8443));
        assert_eq!(report.path, "/path/to");
        assert_eq!(report.query.as_deref(), Some("q=1&r=2"));
        assert_eq!(report.fragment.as_deref(), Some("frag"));
        assert!(report.warnings.iter().any(|w| w.contains("credentials")));
    }

    #[test]
    fn normalized_forms() {
        let report = check("HTTPS://EXAMPLE.com");
        assert!(report.valid);
        assert_eq!(report.normalized, "https://example.com/");

        let report = check("https://例子.测试/路径");
        assert!(report.valid);
        assert_eq!(report.normalized, "https://xn--fsqu00a.xn--0zwm56d/%E8%B7%AF%E5%BE%84");
        assert_eq!(report.host, "xn--fsqu00a.xn--0zwm56d");

        let report = check("http://[2001:db8::1]");
        assert_eq!(report.host_kind, UrlHostKind::Ipv6);
        assert_eq!(report.normalized, "http://[2001:db8::1]/");

        let report = check("http://192.0.2.1:8080/x");
        assert_eq!(report.host_kind, UrlHostKind::Ipv4);
        assert_eq!(report.port, Some(8080));
    }

    #[test]
    fn opaque_scheme_urls_have_no_host() {
        let report = check("mailto:user@example.com");
        assert!(report.valid);
        assert_eq!(report.scheme, "mailto");
        assert_eq!(report.host_kind, UrlHostKind::None);
        assert_eq!(report.host, "");
        assert_eq!(report.path, "user@example.com");

        let report = check("urn:isbn:0451450523");
        assert!(report.valid);
        assert_eq!(report.host_kind, UrlHostKind::None);
        assert_eq!(report.path, "isbn:0451450523");
    }

    #[test]
    fn allowed_schemes_flag() {
        let options = UrlValidationOptions {
            input: "ftp://example.com/".to_string(),
            allowed_schemes: Some(vec!["https".to_string(), "http".to_string()]),
            ..Default::default()
        };
        let report = validate_url(options);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("scheme 'ftp' is not allowed (allowed: https, http)")),
            "{:?}",
            report.issues
        );

        let options = UrlValidationOptions {
            input: "HTTPS://example.com/".to_string(),
            allowed_schemes: Some(vec!["https".to_string()]),
            ..Default::default()
        };
        assert!(validate_url(options).valid);
    }

    #[test]
    fn require_host_flag() {
        let options = UrlValidationOptions {
            input: "mailto:user@example.com".to_string(),
            require_host: true,
            ..Default::default()
        };
        let report = validate_url(options);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("has no host")));

        let options = UrlValidationOptions {
            input: "https://example.com/".to_string(),
            require_host: true,
            ..Default::default()
        };
        assert!(validate_url(options).valid);
    }

    #[test]
    fn empty_default_options_report_invalid() {
        let report = validate_url(UrlValidationOptions::default());
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("empty")));
    }
}

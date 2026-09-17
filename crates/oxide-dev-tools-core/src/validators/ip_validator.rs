//! IP address validation (IPv4 and IPv6).
//!
//! [`validate_ip`] parses an address with the [`std::net`] parsers, classifies
//! it against the well-known RFC 6890 special-purpose ranges, and reports the
//! canonical (normalized) form (RFC 5952 compression for IPv6).
//!
//! IPv6 zone identifiers (RFC 4007, e.g. `fe80::1%eth0`) are accepted and
//! kept in the normalized form; disable them with
//! [`IpOptions::allow_zone_id`]. [`IpOptions::require_global`] rejects every
//! address that is not globally routable (private, loopback, link-local,
//! multicast, reserved, and documentation ranges). The validator performs no
//! network lookups; syntax and ranges are checked locally.

use std::net::{Ipv4Addr, Ipv6Addr};

// -------- Public API --------

/// Required address family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IpMode {
    /// Accept either IPv4 or IPv6 (detected from the input).
    #[default]
    Auto,
    /// Require an IPv4 address.
    Ipv4,
    /// Require an IPv6 address.
    Ipv6,
}

/// The address family the input was parsed as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVersion {
    Ipv4,
    Ipv6,
}

/// Options for [`validate_ip`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpOptions {
    /// The address to validate (surrounding whitespace is ignored).
    pub input: String,
    /// Required address family.
    pub mode: IpMode,
    /// Reject addresses that are not globally routable.
    pub require_global: bool,
    /// Accept IPv4 octets with leading zeros such as `192.168.001.1`.
    pub allow_leading_zeros: bool,
    /// Accept IPv6 zone identifiers such as `fe80::1%eth0`.
    pub allow_zone_id: bool,
}

impl Default for IpOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            mode: IpMode::Auto,
            require_global: false,
            allow_leading_zeros: true,
            allow_zone_id: true,
        }
    }
}

/// How the address is classified (RFC 6890 special-purpose ranges).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpClass {
    /// The unspecified address (`0.0.0.0` or `::`).
    Unspecified,
    /// A loopback address (`127.0.0.0/8` or `::1`).
    Loopback,
    /// A private address (RFC 1918).
    Private,
    /// A link-local address (`169.254.0.0/16` or `fe80::/10`).
    LinkLocal,
    /// A multicast address (`224.0.0.0/4` or `ff00::/8`).
    Multicast,
    /// The IPv4 limited broadcast address (`255.255.255.255`).
    Broadcast,
    /// Reserved for documentation (RFC 5737, RFC 3849).
    Documentation,
    /// Reserved for benchmarking (RFC 2544, `198.18.0.0/15`).
    Benchmark,
    /// Carrier-grade NAT (RFC 6598, `100.64.0.0/10`).
    Shared,
    /// IPv6 unique local address (RFC 4193, `fc00::/7`).
    UniqueLocal,
    /// IPv4-mapped IPv6 address (`::ffff:0:0/96`).
    Ipv4Mapped,
    /// Discard-only IPv6 address (RFC 6666, `100::/64`).
    Discard,
    /// Teredo tunnel address (`2001::/32`).
    Teredo,
    /// 6to4 tunnel address (`2002::/16`).
    SixToFour,
    /// A globally routable unicast address.
    Global,
    /// Reserved for special purposes, not otherwise classified.
    Reserved,
}

/// The result of validating an address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpReport {
    /// Whether the address satisfies the configured rules.
    pub valid: bool,
    /// The address as given, without surrounding whitespace.
    pub input: String,
    /// The canonical form (RFC 5952 for IPv6; zone identifiers are kept).
    pub normalized: String,
    /// The address family that was validated (set even when invalid).
    pub version: IpVersion,
    /// How the address is classified.
    pub class: IpClass,
    /// Reasons the address is invalid.
    pub issues: Vec<String>,
    /// Valid but unusual or discouraged constructs.
    pub warnings: Vec<String>,
}

/// Validate an IP address and produce a detailed report.
///
/// Validation never fails with an error and never panics: an invalid
/// address is reported via [`IpReport::valid`] and [`IpReport::issues`].
pub fn validate_ip(options: IpOptions) -> IpReport {
    let trimmed = options.input.trim();
    let looks_ipv6 = trimmed.contains(':');
    let mut report = IpReport {
        valid: false,
        input: trimmed.to_string(),
        normalized: String::new(),
        version: if looks_ipv6 { IpVersion::Ipv6 } else { IpVersion::Ipv4 },
        class: IpClass::Reserved,
        issues: Vec::new(),
        warnings: Vec::new(),
    };

    if trimmed.is_empty() {
        report.issues.push("input is empty".to_string());
        return report;
    }

    match (options.mode, looks_ipv6) {
        (IpMode::Ipv4, true) => {
            report
                .issues
                .push("expected an IPv4 address, found IPv6 syntax".to_string());
            return report;
        }
        (IpMode::Ipv6, false) => {
            report
                .issues
                .push("expected an IPv6 address, found IPv4 syntax".to_string());
            return report;
        }
        (IpMode::Auto, _) | (IpMode::Ipv4, false) | (IpMode::Ipv6, true) => {}
    }

    if looks_ipv6 {
        parse_ipv6_input(trimmed, &options, &mut report);
    } else {
        parse_ipv4_input(trimmed, &options, &mut report);
    }
    report
}

// -------- Parsing --------

fn parse_ipv4_input(input: &str, options: &IpOptions, report: &mut IpReport) {
    if !options.allow_leading_zeros {
        if let Some(octet) = leading_zero_octet(input) {
            report.issues.push(format!("IPv4 octet '{octet}' has a leading zero"));
            return;
        }
    }
    match strip_leading_zeros(input).parse::<Ipv4Addr>() {
        Ok(addr) => {
            report.version = IpVersion::Ipv4;
            report.normalized = addr.to_string();
            report.class = classify_ipv4(addr);
            finish_check(options, report);
        }
        Err(_) => report.issues.push("invalid IPv4 address syntax".to_string()),
    }
}

fn parse_ipv6_input(input: &str, options: &IpOptions, report: &mut IpReport) {
    let (addr_text, zone) = match split_zone(input) {
        Ok(parts) => parts,
        Err(issue) => {
            report.issues.push(issue);
            return;
        }
    };
    if zone.is_some() && !options.allow_zone_id {
        report.issues.push("zone identifiers are disabled".to_string());
    }
    match addr_text.parse::<Ipv6Addr>() {
        Ok(addr) => {
            report.version = IpVersion::Ipv6;
            report.normalized = match zone {
                Some(zone) => format!("{addr}%{zone}"),
                None => addr.to_string(),
            };
            report.class = classify_ipv6(addr);
            if is_ipv4_compatible(addr) {
                report.warnings.push("deprecated IPv4-compatible address".to_string());
            }
            finish_check(options, report);
        }
        Err(_) => report.issues.push("invalid IPv6 address syntax".to_string()),
    }
}

/// Split an optional RFC 4007 zone identifier off the end of the input.
fn split_zone(input: &str) -> Result<(&str, Option<&str>), String> {
    match input.rsplit_once('%') {
        None => Ok((input, None)),
        Some((addr, zone)) => {
            if addr.contains('%') {
                return Err("invalid IPv6 address: multiple '%' separators".to_string());
            }
            if zone.is_empty() {
                return Err("zone identifier is empty".to_string());
            }
            if !zone
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
            {
                return Err(format!("zone identifier '{zone}' contains invalid characters"));
            }
            Ok((addr, Some(zone)))
        }
    }
}

fn leading_zero_octet(input: &str) -> Option<&str> {
    input.split('.').find(|octet| octet.len() > 1 && octet.starts_with('0'))
}

/// Remove leading zeros from every octet so [`Ipv4Addr`]'s strict parser
/// accepts them; empty and non-digit octets are left untouched for the
/// parser to reject.
fn strip_leading_zeros(input: &str) -> String {
    input
        .split('.')
        .map(|octet| {
            if octet.is_empty() {
                return "";
            }
            let trimmed = octet.trim_start_matches('0');
            if trimmed.is_empty() { "0" } else { trimmed }
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn is_ipv4_compatible(addr: Ipv6Addr) -> bool {
    let segments = addr.segments();
    !addr.is_unspecified() && !addr.is_loopback() && segments[0..6] == [0; 6]
}

// -------- Classification --------

fn classify_ipv4(addr: Ipv4Addr) -> IpClass {
    let octets = addr.octets();
    if addr.is_unspecified() {
        return IpClass::Unspecified;
    }
    if addr.is_broadcast() {
        return IpClass::Broadcast;
    }
    if addr.is_loopback() {
        return IpClass::Loopback;
    }
    if addr.is_multicast() {
        return IpClass::Multicast;
    }
    if addr.is_private() {
        return IpClass::Private;
    }
    if addr.is_link_local() {
        return IpClass::LinkLocal;
    }
    if addr.is_documentation() {
        return IpClass::Documentation;
    }
    if octets[0] == 100 && octets[1] & 0xC0 == 0x40 {
        return IpClass::Shared;
    }
    if octets[0] == 198 && (18..=19).contains(&octets[1]) {
        return IpClass::Benchmark;
    }
    let reserved = octets[0] == 0 || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0) || octets[0] >= 240;
    if reserved {
        return IpClass::Reserved;
    }
    IpClass::Global
}

fn classify_ipv6(addr: Ipv6Addr) -> IpClass {
    let segments = addr.segments();
    if addr.is_unspecified() {
        return IpClass::Unspecified;
    }
    if addr.is_loopback() {
        return IpClass::Loopback;
    }
    if addr.is_multicast() {
        return IpClass::Multicast;
    }
    if segments[0..5] == [0; 5] && segments[5] == 0xFFFF {
        return IpClass::Ipv4Mapped;
    }
    if segments[0] == 0x0100 {
        return IpClass::Discard;
    }
    if segments[0] == 0x2001 && segments[1] == 0x0DB8 {
        return IpClass::Documentation;
    }
    if segments[0] == 0x2001 && segments[1] == 0 {
        return IpClass::Teredo;
    }
    if segments[0] == 0x2002 {
        return IpClass::SixToFour;
    }
    if segments[0] & 0xFE00 == 0xFC00 {
        return IpClass::UniqueLocal;
    }
    if segments[0] & 0xFFC0 == 0xFE80 {
        return IpClass::LinkLocal;
    }
    if segments[0..6] == [0; 6] {
        return IpClass::Reserved;
    }
    if segments[0] & 0xE000 == 0x2000 {
        return IpClass::Global;
    }
    IpClass::Reserved
}

// -------- Report assembly --------

fn finish_check(options: &IpOptions, report: &mut IpReport) {
    if report.class == IpClass::Documentation {
        report
            .warnings
            .push("address is reserved for documentation".to_string());
    }
    if report.class == IpClass::Benchmark {
        report.warnings.push("address is reserved for benchmarking".to_string());
    }
    if report.class == IpClass::Ipv4Mapped {
        report.warnings.push("IPv4-mapped IPv6 address".to_string());
    }
    if options.require_global && report.class != IpClass::Global {
        report
            .issues
            .push(format!("address is not globally routable ({})", class_name(report.class)));
    }
    report.valid = report.issues.is_empty();
}

fn class_name(class: IpClass) -> &'static str {
    match class {
        IpClass::Unspecified => "unspecified",
        IpClass::Loopback => "loopback",
        IpClass::Private => "private",
        IpClass::LinkLocal => "link-local",
        IpClass::Multicast => "multicast",
        IpClass::Broadcast => "broadcast",
        IpClass::Documentation => "documentation",
        IpClass::Benchmark => "benchmark",
        IpClass::Shared => "shared",
        IpClass::UniqueLocal => "unique-local",
        IpClass::Ipv4Mapped => "ipv4-mapped",
        IpClass::Discard => "discard",
        IpClass::Teredo => "teredo",
        IpClass::SixToFour => "6to4",
        IpClass::Global => "global",
        IpClass::Reserved => "reserved",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(input: &str) -> IpReport {
        validate_ip(IpOptions {
            input: input.to_string(),
            ..IpOptions::default()
        })
    }

    fn check_with(input: &str, mutate: impl FnOnce(&mut IpOptions)) -> IpReport {
        let mut options = IpOptions {
            input: input.to_string(),
            ..IpOptions::default()
        };
        mutate(&mut options);
        validate_ip(options)
    }

    fn assert_class(input: &str, expected: IpClass) {
        let report = check(input);
        assert!(report.valid, "expected '{input}' to be valid: {:?}", report.issues);
        assert_eq!(report.class, expected, "wrong class for '{input}'");
    }

    fn assert_issue(input: &str, expected: &str) {
        let report = check(input);
        assert!(!report.valid, "expected '{input}' to be invalid");
        assert!(
            report.issues.iter().any(|issue| issue.contains(expected)),
            "expected an issue containing '{expected}' for '{input}', got: {:?}",
            report.issues
        );
    }

    #[test]
    fn valid_ipv4_addresses() {
        for input in [
            "192.168.1.1",
            "8.8.8.8",
            "0.0.0.0",
            "255.255.255.255",
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.255",
            "169.254.10.20",
            "224.0.0.1",
            "240.0.0.1",
        ] {
            let report = check(input);
            assert!(report.valid, "expected '{input}' to be valid: {:?}", report.issues);
            assert_eq!(report.version, IpVersion::Ipv4);
        }
    }

    #[test]
    fn valid_ipv6_addresses() {
        for input in [
            "::",
            "::1",
            "2001:db8::1",
            "2001:db8:0:0:0:0:2:1",
            "2001:DB8::1",
            "::ffff:192.0.2.1",
            "fe80::1",
            "ff02::1",
            "fd00::1",
            "100::1",
            "2002::1",
            "2001::1",
            "fe80::1%eth0",
            "fe80::1%12",
        ] {
            let report = check(input);
            assert!(report.valid, "expected '{input}' to be valid: {:?}", report.issues);
            assert_eq!(report.version, IpVersion::Ipv6);
        }
    }

    #[test]
    fn invalid_ipv4_addresses() {
        for input in [
            "999.1.1.1",
            "1.2.3",
            "1.2.3.4.5",
            "1.2.3.a",
            "256.256.256.256",
            "-1.2.3.4",
            "1..2.3",
        ] {
            assert_issue(input, "invalid IPv4 address syntax");
        }
    }

    #[test]
    fn invalid_ipv6_addresses() {
        for input in [
            "1::2::3",
            ":::",
            "12345::1",
            "1:2:3:4:5:6:7",
            "1:2:3:4:5:6:7:8:9",
            "g::1",
        ] {
            assert_issue(input, "invalid IPv6 address syntax");
        }
    }

    #[test]
    fn empty_input_is_reported() {
        let report = check("");
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("input is empty")));
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        let report = check("  8.8.8.8  ");
        assert!(report.valid);
        assert_eq!(report.input, "8.8.8.8");
        assert_eq!(report.normalized, "8.8.8.8");
    }

    #[test]
    fn normalization_produces_canonical_forms() {
        let cases = [
            ("192.168.001.1", "192.168.1.1"),
            ("2001:0DB8:0000:0000:0000:0000:0000:0001", "2001:db8::1"),
            ("FE80::1", "fe80::1"),
            ("::ffff:192.0.2.1", "::ffff:192.0.2.1"),
            ("fe80::1%eth0", "fe80::1%eth0"),
        ];
        for (input, expected) in cases {
            let report = check(input);
            assert!(report.valid, "expected '{input}' to be valid: {:?}", report.issues);
            assert_eq!(report.normalized, expected, "wrong normalization for '{input}'");
        }
    }

    #[test]
    fn ipv4_classification() {
        let cases = [
            ("0.0.0.0", IpClass::Unspecified),
            ("255.255.255.255", IpClass::Broadcast),
            ("127.0.0.1", IpClass::Loopback),
            ("224.0.0.1", IpClass::Multicast),
            ("10.0.0.1", IpClass::Private),
            ("172.16.0.1", IpClass::Private),
            ("192.168.1.1", IpClass::Private),
            ("169.254.1.1", IpClass::LinkLocal),
            ("192.0.2.1", IpClass::Documentation),
            ("198.51.100.1", IpClass::Documentation),
            ("203.0.113.1", IpClass::Documentation),
            ("198.18.0.1", IpClass::Benchmark),
            ("100.64.0.1", IpClass::Shared),
            ("0.0.0.1", IpClass::Reserved),
            ("192.0.0.1", IpClass::Reserved),
            ("240.0.0.1", IpClass::Reserved),
            ("8.8.8.8", IpClass::Global),
            ("9.9.9.9", IpClass::Global),
        ];
        for (input, expected) in cases {
            assert_class(input, expected);
        }
    }

    #[test]
    fn ipv6_classification() {
        let cases = [
            ("::", IpClass::Unspecified),
            ("::1", IpClass::Loopback),
            ("ff02::1", IpClass::Multicast),
            ("::ffff:192.0.2.1", IpClass::Ipv4Mapped),
            ("100::1", IpClass::Discard),
            ("2001:db8::1", IpClass::Documentation),
            ("2001::1", IpClass::Teredo),
            ("2002::1", IpClass::SixToFour),
            ("fd00::1", IpClass::UniqueLocal),
            ("fe80::1", IpClass::LinkLocal),
            ("2001:4860:4860::8888", IpClass::Global),
            ("2606:4700:4700::1111", IpClass::Global),
        ];
        for (input, expected) in cases {
            assert_class(input, expected);
        }
    }

    #[test]
    fn ipv4_compatible_ipv6_is_reserved_with_warning() {
        let report = check("::2");
        assert!(report.valid);
        assert_eq!(report.class, IpClass::Reserved);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("IPv4-compatible"))
        );
    }

    #[test]
    fn require_global_rejects_special_ranges() {
        for input in [
            "10.0.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "224.0.0.1",
            "192.0.2.1",
            "::1",
            "fe80::1",
            "fd00::1",
        ] {
            let report = check_with(input, |options| options.require_global = true);
            assert!(!report.valid, "expected '{input}' to fail --require-global");
            assert!(
                report
                    .issues
                    .iter()
                    .any(|issue| issue.contains("not globally routable"))
            );
        }
        for input in ["8.8.8.8", "2001:4860:4860::8888"] {
            let report = check_with(input, |options| options.require_global = true);
            assert!(report.valid, "expected '{input}' to pass --require-global: {:?}", report.issues);
        }
    }

    #[test]
    fn leading_zero_flag() {
        let report = check_with("192.168.001.1", |options| options.allow_leading_zeros = false);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("leading zero")));

        let report = check_with("0.0.0.0", |options| options.allow_leading_zeros = false);
        assert!(report.valid, "single-zero octets are not leading zeros: {:?}", report.issues);
    }

    #[test]
    fn zone_id_flag_and_validation() {
        let report = check_with("fe80::1%eth0", |options| options.allow_zone_id = false);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("zone identifiers are disabled"))
        );

        assert_issue("fe80::1%eth 0", "invalid characters");
        assert_issue("fe80::1%eth0%eth1", "multiple '%' separators");
        assert_issue("fe80::1%", "zone identifier is empty");
    }

    #[test]
    fn mode_mismatch_is_reported() {
        let report = check_with("::1", |options| options.mode = IpMode::Ipv4);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("expected an IPv4")));

        let report = check_with("1.2.3.4", |options| options.mode = IpMode::Ipv6);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("expected an IPv6")));

        assert!(check_with("1.2.3.4", |options| options.mode = IpMode::Ipv4).valid);
        assert!(check_with("::1", |options| options.mode = IpMode::Ipv6).valid);
    }

    #[test]
    fn warnings_for_discouraged_ranges() {
        let report = check("192.0.2.1");
        assert!(report.warnings.iter().any(|warning| warning.contains("documentation")));

        let report = check("198.18.0.1");
        assert!(report.warnings.iter().any(|warning| warning.contains("benchmarking")));

        let report = check("::ffff:192.0.2.1");
        assert!(report.warnings.iter().any(|warning| warning.contains("IPv4-mapped")));
    }
}

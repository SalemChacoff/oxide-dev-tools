//! Credit card number validation (Luhn checksum and issuer detection).
//!
//! [`validate_card`] checks a card number's Luhn checksum and detects the
//! issuing network from the IIN (the leading digits). Spaces and hyphens are
//! tolerated by default and can be rejected with
//! [`CardOptions::no_separators`]; a specific network can be required with
//! [`CardOptions::network`], and unrecognized networks can be rejected with
//! [`CardOptions::require_network`].
//!
//! The IIN ranges are static and approximate: networks occasionally issue
//! numbers outside the documented ranges, and some ranges are shared between
//! networks (shared ranges are resolved by the longest prefix). The validator
//! performs no network lookups and does not check whether a number actually
//! exists — it validates structure only.

// -------- Public API --------

/// Card-issuing network, detected from the IIN (leading digits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CardNetwork {
    /// Accept any network (requirement only; never reported as a detection).
    #[default]
    Any,
    /// Visa (prefix `4`).
    Visa,
    /// Mastercard (prefixes `51`-`55` and `2221`-`2720`).
    Mastercard,
    /// American Express (prefixes `34`, `37`).
    AmericanExpress,
    /// Discover (prefixes `6011`, `65`, `644`-`649`, `622126`-`622925`).
    Discover,
    /// Diners Club (prefixes `300`-`305`, `36`, `38`, `39`, `3095`, `3096`).
    DinersClub,
    /// JCB (prefixes `3528`-`3589`).
    Jcb,
    /// UnionPay (prefixes `62` except Discover's `622126`-`622925`, and `81`).
    UnionPay,
    /// Maestro (prefixes `50`, `56`-`69`).
    Maestro,
    /// Mir (prefixes `2200`-`2204`).
    Mir,
    /// RuPay (prefixes `60`, `6521`, `6522`).
    RuPay,
    /// Elo (Brazilian scheme; several 4-digit prefixes).
    Elo,
    /// Hipercard (Brazilian scheme; prefixes `606282`, `3841`).
    Hipercard,
    /// Verve (Nigerian scheme; prefixes `5060`-`5064`, `5066`, `6500`-`6503`).
    Verve,
    /// UATP, the airline travel card (prefix `1`).
    Uatp,
    /// An unrecognized IIN (report-only; cannot be required).
    Unknown,
}

/// Options for [`validate_card`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardOptions {
    /// The card number to validate (surrounding whitespace is ignored).
    pub input: String,
    /// Required network, or [`CardNetwork::Any`] to accept every network.
    pub network: CardNetwork,
    /// Reject numbers whose network cannot be recognized.
    pub require_network: bool,
    /// Reject spaces and hyphens instead of tolerating them.
    pub no_separators: bool,
}

impl Default for CardOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            network: CardNetwork::Any,
            require_network: false,
            no_separators: false,
        }
    }
}

/// The result of validating a card number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardReport {
    /// Whether the number satisfies the configured rules.
    pub valid: bool,
    /// The number as given, without surrounding whitespace.
    pub input: String,
    /// The digits only, without spaces or hyphens.
    pub normalized: String,
    /// The detected issuing network, or [`CardNetwork::Unknown`].
    pub network: CardNetwork,
    /// Whether the number passes the Luhn checksum.
    pub luhn: bool,
    /// The number of digits after normalization.
    pub length: usize,
    /// Reasons the number is invalid.
    pub issues: Vec<String>,
    /// Valid but unusual or advisory findings.
    pub warnings: Vec<String>,
}

/// Validate a card number and produce a detailed report.
///
/// Validation never fails with an error and never panics: an invalid number
/// is reported via [`CardReport::valid`] and [`CardReport::issues`].
pub fn validate_card(options: CardOptions) -> CardReport {
    let trimmed = options.input.trim();
    let mut report = CardReport {
        valid: false,
        input: trimmed.to_string(),
        normalized: String::new(),
        network: CardNetwork::Unknown,
        luhn: false,
        length: 0,
        issues: Vec::new(),
        warnings: Vec::new(),
    };

    if trimmed.is_empty() {
        report.issues.push("input is empty".to_string());
        return report;
    }
    if options.no_separators && trimmed.contains([' ', '-']) {
        report.issues.push("separators are not allowed".to_string());
    }

    let mut digits = String::with_capacity(trimmed.len());
    for byte in trimmed.bytes() {
        match byte {
            b' ' | b'-' => {}
            b'0'..=b'9' => digits.push(char::from(byte)),
            _ => {
                report
                    .issues
                    .push("contains non-digit characters (only digits, spaces, and hyphens are allowed)".to_string());
                return report;
            }
        }
    }
    if digits.is_empty() {
        report.issues.push("input contains no digits".to_string());
        return report;
    }

    report.normalized = digits;
    report.length = report.normalized.len();
    if !(8..=19).contains(&report.length) {
        report
            .issues
            .push(format!("length {} is outside the supported range (8-19 digits)", report.length));
        return report;
    }

    report.luhn = check_luhn(&report.normalized);
    if !report.luhn {
        report.issues.push("fails the Luhn checksum".to_string());
    }

    report.network = detect_network(&report.normalized);
    if report.network == CardNetwork::Unknown {
        let message = "issuer network is not recognized".to_string();
        if options.require_network {
            report.issues.push(message);
        } else {
            report.warnings.push(message);
        }
    } else {
        check_length(report.network, &mut report);
    }
    check_network(&options, &mut report);
    finish_check(trimmed, &options, &mut report);
    report
}

// -------- Luhn checksum --------

/// Double every second digit from the right and check that the sum is a
/// multiple of 10. The rightmost digit is the check digit and is never
/// doubled (it is at index 0 of the reversed string).
fn check_luhn(digits: &str) -> bool {
    let mut sum = 0u32;
    for (index, byte) in digits.bytes().rev().enumerate() {
        let mut value = u32::from(byte - b'0');
        if index % 2 == 1 {
            value *= 2;
            if value > 9 {
                value -= 9;
            }
        }
        sum += value;
    }
    sum % 10 == 0
}

// -------- Network detection --------

/// A contiguous IIN prefix range, expressed as start/end values of the given
/// digit count (e.g. `51`-`55` are 2-digit prefixes).
struct IinRange {
    start: u32,
    end: u32,
    digits: usize,
}

const IIN_TABLE: &[(CardNetwork, &[IinRange])] = &[
    (
        CardNetwork::Visa,
        &[IinRange {
            start: 4,
            end: 4,
            digits: 1,
        }],
    ),
    (
        CardNetwork::Mastercard,
        &[
            IinRange {
                start: 51,
                end: 55,
                digits: 2,
            },
            IinRange {
                start: 2221,
                end: 2720,
                digits: 4,
            },
        ],
    ),
    (
        CardNetwork::AmericanExpress,
        &[
            IinRange {
                start: 34,
                end: 34,
                digits: 2,
            },
            IinRange {
                start: 37,
                end: 37,
                digits: 2,
            },
        ],
    ),
    (
        CardNetwork::Discover,
        &[
            IinRange {
                start: 6011,
                end: 6011,
                digits: 4,
            },
            IinRange {
                start: 644,
                end: 649,
                digits: 3,
            },
            IinRange {
                start: 65,
                end: 65,
                digits: 2,
            },
            IinRange {
                start: 622_126,
                end: 622_925,
                digits: 6,
            },
        ],
    ),
    (
        CardNetwork::DinersClub,
        &[
            IinRange {
                start: 300,
                end: 305,
                digits: 3,
            },
            IinRange {
                start: 3095,
                end: 3096,
                digits: 4,
            },
            IinRange {
                start: 36,
                end: 36,
                digits: 2,
            },
            IinRange {
                start: 38,
                end: 39,
                digits: 2,
            },
        ],
    ),
    (
        CardNetwork::Jcb,
        &[IinRange {
            start: 3528,
            end: 3589,
            digits: 4,
        }],
    ),
    (
        CardNetwork::UnionPay,
        &[
            IinRange {
                start: 62,
                end: 62,
                digits: 2,
            },
            IinRange {
                start: 81,
                end: 81,
                digits: 2,
            },
        ],
    ),
    // RuPay is listed before Maestro so the shared `60` prefix (both claim
    // it at 2 digits) resolves to RuPay, whose documented range it is.
    (
        CardNetwork::RuPay,
        &[
            IinRange {
                start: 60,
                end: 60,
                digits: 2,
            },
            IinRange {
                start: 6521,
                end: 6522,
                digits: 4,
            },
        ],
    ),
    (
        CardNetwork::Maestro,
        &[
            IinRange {
                start: 50,
                end: 50,
                digits: 2,
            },
            IinRange {
                start: 56,
                end: 69,
                digits: 2,
            },
        ],
    ),
    (
        CardNetwork::Mir,
        &[IinRange {
            start: 2200,
            end: 2204,
            digits: 4,
        }],
    ),
    (
        CardNetwork::Elo,
        &[
            IinRange {
                start: 4011,
                end: 4011,
                digits: 4,
            },
            IinRange {
                start: 4312,
                end: 4312,
                digits: 4,
            },
            IinRange {
                start: 4389,
                end: 4389,
                digits: 4,
            },
            IinRange {
                start: 4514,
                end: 4514,
                digits: 4,
            },
            IinRange {
                start: 4573,
                end: 4573,
                digits: 4,
            },
            IinRange {
                start: 4576,
                end: 4576,
                digits: 4,
            },
            IinRange {
                start: 5041,
                end: 5041,
                digits: 4,
            },
            IinRange {
                start: 5067,
                end: 5067,
                digits: 4,
            },
            IinRange {
                start: 509,
                end: 509,
                digits: 3,
            },
            IinRange {
                start: 6277,
                end: 6277,
                digits: 4,
            },
            IinRange {
                start: 6362,
                end: 6362,
                digits: 4,
            },
            IinRange {
                start: 6504,
                end: 6505,
                digits: 4,
            },
            IinRange {
                start: 6550,
                end: 6550,
                digits: 4,
            },
        ],
    ),
    (
        CardNetwork::Hipercard,
        &[
            IinRange {
                start: 606_282,
                end: 606_282,
                digits: 6,
            },
            IinRange {
                start: 3841,
                end: 3841,
                digits: 4,
            },
        ],
    ),
    (
        CardNetwork::Verve,
        &[
            IinRange {
                start: 5060,
                end: 5064,
                digits: 4,
            },
            IinRange {
                start: 5066,
                end: 5066,
                digits: 4,
            },
            IinRange {
                start: 6500,
                end: 6503,
                digits: 4,
            },
        ],
    ),
    (
        CardNetwork::Uatp,
        &[IinRange {
            start: 1,
            end: 1,
            digits: 1,
        }],
    ),
];

/// Match the leading digits against the IIN table; shared ranges are
/// resolved by the longest matching prefix, so specific ranges win over
/// generic ones (e.g. Discover's `622126`-`622925` beats UnionPay's `62`).
/// Equal-length ties are resolved by table order.
fn detect_network(digits: &str) -> CardNetwork {
    let mut best: Option<(CardNetwork, usize)> = None;
    for (network, ranges) in IIN_TABLE {
        for range in *ranges {
            if digits.len() < range.digits {
                continue;
            }
            let mut prefix = 0u32;
            for byte in digits.bytes().take(range.digits) {
                prefix = prefix * 10 + u32::from(byte - b'0');
            }
            if (range.start..=range.end).contains(&prefix) {
                let better = best.is_none_or(|(_, length)| range.digits > length);
                if better {
                    best = Some((*network, range.digits));
                }
            }
        }
    }
    best.map_or(CardNetwork::Unknown, |(network, _)| network)
}

// -------- Constraint checks --------

/// Check the number's length against the detected network's allowed lengths.
fn check_length(network: CardNetwork, report: &mut CardReport) {
    let lengths = allowed_lengths(network);
    if lengths.contains(&report.length) {
        return;
    }
    report.issues.push(format!(
        "expected a {}-digit {} number, found {} digits",
        length_list(lengths),
        network_name(network),
        report.length
    ));
}

/// Check the detected network against the required network.
fn check_network(options: &CardOptions, report: &mut CardReport) {
    if options.network == CardNetwork::Any || report.network == options.network {
        return;
    }
    let found = match report.network {
        CardNetwork::Unknown => "an unknown network".to_string(),
        network => format!("a {} card", network_name(network)),
    };
    report
        .issues
        .push(format!("expected a {} card, found {found}", network_name(options.network)));
}

// -------- Report assembly --------

fn finish_check(input: &str, options: &CardOptions, report: &mut CardReport) {
    if !options.no_separators && input.contains([' ', '-']) {
        report.warnings.push("spaces and hyphens were removed".to_string());
    }
    report.valid = report.issues.is_empty();
}

// -------- Naming --------

fn network_name(network: CardNetwork) -> &'static str {
    match network {
        CardNetwork::Any => "any",
        CardNetwork::Visa => "Visa",
        CardNetwork::Mastercard => "Mastercard",
        CardNetwork::AmericanExpress => "American Express",
        CardNetwork::Discover => "Discover",
        CardNetwork::DinersClub => "Diners Club",
        CardNetwork::Jcb => "JCB",
        CardNetwork::UnionPay => "UnionPay",
        CardNetwork::Maestro => "Maestro",
        CardNetwork::Mir => "Mir",
        CardNetwork::RuPay => "RuPay",
        CardNetwork::Elo => "Elo",
        CardNetwork::Hipercard => "Hipercard",
        CardNetwork::Verve => "Verve",
        CardNetwork::Uatp => "UATP",
        CardNetwork::Unknown => "unknown",
    }
}

/// The digit counts a network issues, or an empty slice when unconstrained.
fn allowed_lengths(network: CardNetwork) -> &'static [usize] {
    match network {
        CardNetwork::Visa => &[13, 16, 19],
        CardNetwork::Mastercard => &[16],
        CardNetwork::AmericanExpress => &[15],
        CardNetwork::Discover => &[16, 19],
        CardNetwork::DinersClub => &[14, 16],
        CardNetwork::Jcb => &[16, 19],
        CardNetwork::UnionPay => &[16, 17, 18, 19],
        CardNetwork::Maestro => &[12, 13, 14, 15, 16, 17, 18, 19],
        CardNetwork::Mir => &[16, 19],
        CardNetwork::RuPay => &[16],
        CardNetwork::Elo => &[16],
        CardNetwork::Hipercard => &[13, 16, 19],
        CardNetwork::Verve => &[16, 19],
        CardNetwork::Uatp => &[15],
        CardNetwork::Any | CardNetwork::Unknown => &[],
    }
}

/// Format a length list for issue messages: `16`, or `13, 16, or 19`.
fn length_list(lengths: &[usize]) -> String {
    match lengths {
        [] => String::new(),
        [single] => single.to_string(),
        [head @ .., last] => {
            let mut list = head.iter().map(usize::to_string).collect::<Vec<_>>().join(", ");
            list.push_str(", or ");
            list.push_str(&last.to_string());
            list
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical Luhn-valid test numbers.
    const VISA: &str = "4111111111111111";
    const VISA_13: &str = "4222222222222";
    const MASTERCARD: &str = "5555555555554444";
    const MASTERCARD_2_SERIES: &str = "2223003122003222";
    const AMEX: &str = "378282246310005";
    const DISCOVER: &str = "6011111111111117";
    const JCB: &str = "3530111333300000";
    const DINERS: &str = "30569309025904";
    const UNIONPAY: &str = "6200000000000005";
    const MAESTRO: &str = "6759649826438453";

    fn check(input: &str) -> CardReport {
        validate_card(CardOptions {
            input: input.to_string(),
            ..CardOptions::default()
        })
    }

    fn check_with(input: &str, mutate: impl FnOnce(&mut CardOptions)) -> CardReport {
        let mut options = CardOptions {
            input: input.to_string(),
            ..CardOptions::default()
        };
        mutate(&mut options);
        validate_card(options)
    }

    fn assert_valid(input: &str) -> CardReport {
        let report = check(input);
        assert!(report.valid, "expected '{input}' to be valid: {:?}", report.issues);
        assert!(report.luhn, "expected '{input}' to pass the Luhn checksum");
        report
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

    /// Build a Luhn-valid number from a prefix, padded with zeros.
    fn pad_to_valid(prefix: &str, length: usize) -> String {
        assert!(prefix.len() < length, "prefix must be shorter than the target length");
        let mut digits = prefix.to_string();
        digits.push_str(&"0".repeat(length - prefix.len() - 1));
        let check = check_digit(&digits);
        digits.push(char::from(b'0' + check));
        digits
    }

    /// Compute the check digit that makes the payload Luhn-valid.
    fn check_digit(payload: &str) -> u8 {
        let mut sum = 0u32;
        for (index, byte) in payload.bytes().rev().enumerate() {
            let mut value = u32::from(byte - b'0');
            if index % 2 == 0 {
                value *= 2;
                if value > 9 {
                    value -= 9;
                }
            }
            sum += value;
        }
        u8::try_from((10 - sum % 10) % 10).expect("remainder fits in a u8")
    }

    #[test]
    fn known_networks_are_detected() {
        for (input, expected) in [
            (VISA, CardNetwork::Visa),
            (VISA_13, CardNetwork::Visa),
            (MASTERCARD, CardNetwork::Mastercard),
            (MASTERCARD_2_SERIES, CardNetwork::Mastercard),
            (AMEX, CardNetwork::AmericanExpress),
            (DISCOVER, CardNetwork::Discover),
            (JCB, CardNetwork::Jcb),
            (DINERS, CardNetwork::DinersClub),
            (UNIONPAY, CardNetwork::UnionPay),
            (MAESTRO, CardNetwork::Maestro),
        ] {
            let report = assert_valid(input);
            assert_eq!(report.network, expected, "wrong network for '{input}'");
            assert_eq!(report.normalized, input);
        }
    }

    #[test]
    fn synthetic_numbers_detect_remaining_networks() {
        for (input, expected) in [
            (pad_to_valid("2204", 16), CardNetwork::Mir),
            (pad_to_valid("60", 16), CardNetwork::RuPay),
            (pad_to_valid("6522", 16), CardNetwork::RuPay),
            (pad_to_valid("4011", 16), CardNetwork::Elo),
            (pad_to_valid("606282", 16), CardNetwork::Hipercard),
            (pad_to_valid("3841", 16), CardNetwork::Hipercard),
            (pad_to_valid("6500", 16), CardNetwork::Verve),
            (pad_to_valid("5066", 16), CardNetwork::Verve),
            (pad_to_valid("1", 15), CardNetwork::Uatp),
        ] {
            let report = assert_valid(&input);
            assert_eq!(report.network, expected, "wrong network for '{input}'");
        }
    }

    #[test]
    fn shared_prefixes_resolve_to_the_longest_match() {
        for (input, expected) in [
            (pad_to_valid("622126", 16), CardNetwork::Discover),
            (pad_to_valid("622925", 16), CardNetwork::Discover),
            (pad_to_valid("622000", 16), CardNetwork::UnionPay),
            (pad_to_valid("2204", 16), CardNetwork::Mir),
            (pad_to_valid("2221", 16), CardNetwork::Mastercard),
            (pad_to_valid("4011", 16), CardNetwork::Elo),
            (pad_to_valid("4111", 16), CardNetwork::Visa),
            (pad_to_valid("509", 16), CardNetwork::Elo),
            (pad_to_valid("3095", 16), CardNetwork::DinersClub),
            (pad_to_valid("606282", 16), CardNetwork::Hipercard),
        ] {
            let report = assert_valid(&input);
            assert_eq!(report.network, expected, "wrong network for '{input}'");
        }
    }

    #[test]
    fn luhn_failure_is_reported() {
        let report = check("4111111111111112");
        assert!(!report.valid);
        assert!(!report.luhn);
        assert!(report.issues.iter().any(|issue| issue.contains("Luhn")));
    }

    #[test]
    fn separators_are_tolerated_and_normalized() {
        for input in ["4111 1111 1111 1111", "4111-1111-1111-1111"] {
            let report = assert_valid(input);
            assert_eq!(report.normalized, "4111111111111111");
            assert_eq!(report.network, CardNetwork::Visa);
            assert!(
                report.warnings.iter().any(|warning| warning.contains("removed")),
                "expected a separator warning for '{input}'"
            );
        }
    }

    #[test]
    fn empty_and_digitless_inputs_are_rejected() {
        assert_issue("", "input is empty");
        assert_issue("   ", "input is empty");
        assert_issue("---", "input contains no digits");
        assert_issue("4111abcd11111111", "non-digit");
    }

    #[test]
    fn length_limits_are_enforced() {
        assert_issue("1234567", "outside the supported range");
        assert_issue("41111111111111111111", "outside the supported range");
    }

    #[test]
    fn classic_vector_with_unknown_network() {
        let report = assert_valid("79927398713");
        assert_eq!(report.network, CardNetwork::Unknown);
        assert!(
            report.warnings.iter().any(|warning| warning.contains("not recognized")),
            "expected an unknown-network warning"
        );
    }

    #[test]
    fn require_network_rejects_unknown_networks() {
        let report = check_with("79927398713", |options| options.require_network = true);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("not recognized")));
    }

    #[test]
    fn network_requirement_is_enforced() {
        let report = check_with(VISA, |options| options.network = CardNetwork::Mastercard);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected a Mastercard card, found a Visa card"))
        );

        let report = check_with(VISA, |options| options.network = CardNetwork::Visa);
        assert!(report.valid);
    }

    #[test]
    fn no_separators_rejects_formatted_input() {
        for input in ["4111 1111 1111 1111", "4111-1111-1111-1111"] {
            let report = check_with(input, |options| options.no_separators = true);
            assert!(!report.valid);
            assert!(report.issues.iter().any(|issue| issue.contains("separators")));
        }
    }

    #[test]
    fn network_length_constraints_are_enforced() {
        let report = check(&pad_to_valid("37", 16));
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected a 15-digit American Express number, found 16 digits"))
        );

        let report = check(&pad_to_valid("4", 15));
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected a 13, 16, or 19-digit Visa number, found 15 digits")),
            "unexpected issues for 15-digit Visa: {:?}",
            report.issues
        );
    }
}

//! Semantic version comparison.
//!
//! Compares versions by SemVer 2.0.0 precedence and checks versions against
//! Cargo-style requirement ranges:
//!
//! - precedence compares major, minor, and patch numerically, then
//!   prerelease identifiers dot by dot, with numeric identifiers below
//!   alphanumeric ones;
//! - a release outranks all of its prereleases (`1.0.0 > 1.0.0-rc.1`);
//! - build metadata never affects precedence (`1.0.0+a == 1.0.0+b`);
//! - requirements use Cargo syntax: `^1.2`, `~1.0`, `>=1.0.0, <2.0.0`,
//!   wildcards like `1.2.*`, and comma-separated comparator lists;
//! - per Cargo's rule, a prerelease version (`1.0.0-alpha.3`) only satisfies
//!   a requirement that has a comparator with a prerelease tag on the same
//!   `major.minor.patch` (so `*` and `^1.2` never match prereleases).
//!
//! [`compare_semver`] returns a [`SemverReport`] whose variant mirrors the
//! requested [`SemverKind`]; both variants render as a single line.

use std::cmp::Ordering;
use std::fmt;

use semver::{Version, VersionReq};

// -------- Public API --------

/// Maximum input size per operand: 1 MiB of characters.
const MAX_CHARS_PER_SIDE: usize = 1_048_576;

/// Which operand an error refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemverSide {
    /// The first input operand.
    Left,
    /// The second input operand.
    Right,
}

impl SemverSide {
    fn label(self) -> &'static str {
        match self {
            SemverSide::Left => "left",
            SemverSide::Right => "right",
        }
    }
}

/// Errors that can occur when comparing semantic versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemverError {
    /// An operand is not a valid SemVer 2.0.0 version.
    InvalidVersion {
        /// Which operand failed to parse.
        side: SemverSide,
        /// The offending input.
        input: String,
        /// The parser message.
        message: String,
    },
    /// A requirement is not a valid Cargo-style version range.
    InvalidRequirement {
        /// The offending input.
        input: String,
        /// The parser message.
        message: String,
    },
    /// An operand is longer than the configured limit.
    InputTooLong {
        /// Which operand exceeded the limit.
        side: SemverSide,
        /// The configured limit in characters.
        limit: usize,
    },
}

impl fmt::Display for SemverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemverError::InvalidVersion { side, input, message } => {
                write!(f, "cannot parse \"{input}\" as a semver version on the {} side: {message}", side.label())
            }
            SemverError::InvalidRequirement { input, message } => {
                write!(f, "cannot parse \"{input}\" as a version requirement: {message}")
            }
            SemverError::InputTooLong { side, limit } => {
                write!(f, "{} semver input must be at most {limit} characters", side.label())
            }
        }
    }
}

impl std::error::Error for SemverError {}

/// How two versions relate under SemVer precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemverRelation {
    /// The left version has lower precedence.
    Less,
    /// The two versions have equal precedence (build metadata ignored).
    Equal,
    /// The left version has higher precedence.
    Greater,
}

impl SemverRelation {
    /// The operator symbol for this relation: `<`, `=`, or `>`.
    pub fn symbol(self) -> &'static str {
        match self {
            SemverRelation::Less => "<",
            SemverRelation::Equal => "=",
            SemverRelation::Greater => ">",
        }
    }

    fn from_ordering(ordering: Ordering) -> Self {
        match ordering {
            Ordering::Less => SemverRelation::Less,
            Ordering::Equal => SemverRelation::Equal,
            Ordering::Greater => SemverRelation::Greater,
        }
    }
}

/// A relation asserted by the caller, e.g. via `--expect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemverExpectation {
    /// The left version must be strictly lower.
    Lt,
    /// The left version must be lower or equal.
    Le,
    /// The versions must have equal precedence.
    Eq,
    /// The left version must be higher or equal.
    Ge,
    /// The left version must be strictly higher.
    Gt,
}

impl SemverExpectation {
    /// Whether `relation` meets this expectation.
    pub fn is_met_by(self, relation: SemverRelation) -> bool {
        match self {
            SemverExpectation::Lt => relation == SemverRelation::Less,
            SemverExpectation::Le => relation != SemverRelation::Greater,
            SemverExpectation::Eq => relation == SemverRelation::Equal,
            SemverExpectation::Ge => relation != SemverRelation::Less,
            SemverExpectation::Gt => relation == SemverRelation::Greater,
        }
    }

    /// The operator symbol for this expectation: `<`, `<=`, `=`, `>=`, or `>`.
    pub fn symbol(self) -> &'static str {
        match self {
            SemverExpectation::Lt => "<",
            SemverExpectation::Le => "<=",
            SemverExpectation::Eq => "=",
            SemverExpectation::Ge => ">=",
            SemverExpectation::Gt => ">",
        }
    }
}

/// Which semver operation to run.
#[derive(Debug, Clone, PartialEq)]
pub enum SemverKind {
    /// Compare two versions by precedence.
    Compare(SemverCompareOptions),
    /// Check a version against a requirement range.
    Satisfies(SemverSatisfiesOptions),
}

/// Options for comparing two versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemverCompareOptions {
    /// First version to compare.
    pub left: String,
    /// Second version to compare.
    pub right: String,
    /// Maximum input size per operand in characters.
    pub max_chars: usize,
}

impl Default for SemverCompareOptions {
    fn default() -> Self {
        Self {
            left: String::new(),
            right: String::new(),
            max_chars: MAX_CHARS_PER_SIDE,
        }
    }
}

/// Options for checking a version against a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemverSatisfiesOptions {
    /// Version to check, e.g. `1.2.3`.
    pub version: String,
    /// Cargo-style requirement, e.g. `^1.2` or `>=1.0.0, <2.0.0`.
    pub requirement: String,
}

/// The outcome of comparing two versions.
#[derive(Debug, Clone, PartialEq)]
pub struct SemverCompareReport {
    /// The first version, as parsed.
    pub left: Version,
    /// The second version, as parsed.
    pub right: Version,
    /// How the versions relate under SemVer precedence.
    pub relation: SemverRelation,
}

impl SemverCompareReport {
    /// Whether the two versions have equal precedence (build metadata ignored).
    pub fn is_equal(&self) -> bool {
        self.relation == SemverRelation::Equal
    }

    /// Whether the relation meets an asserted expectation; `None` always holds.
    pub fn meets(&self, expect: Option<SemverExpectation>) -> bool {
        expect.is_none_or(|expectation| expectation.is_met_by(self.relation))
    }

    /// Render the comparison as a single line, e.g. `1.2.3 < 2.0.0`.
    pub fn to_text(&self) -> String {
        format!("{} {} {}", self.left, self.relation.symbol(), self.right)
    }
}

/// The outcome of checking a version against a requirement.
#[derive(Debug, Clone, PartialEq)]
pub struct SemverSatisfiesReport {
    /// The version checked, as parsed.
    pub version: Version,
    /// The requirement checked, as parsed.
    pub requirement: VersionReq,
}

impl SemverSatisfiesReport {
    /// Whether the version satisfies the requirement.
    pub fn is_satisfied(&self) -> bool {
        self.requirement.matches(&self.version)
    }

    /// Render the check as a single line, e.g. `1.2.3 satisfies ^1.2`.
    pub fn to_text(&self) -> String {
        format!("{} satisfies {}", self.version, self.requirement)
    }
}

/// The result of [`compare_semver`]; the variant mirrors the [`SemverKind`] given.
#[derive(Debug, Clone, PartialEq)]
pub enum SemverReport {
    /// Result of [`SemverKind::Compare`].
    Comparison(SemverCompareReport),
    /// Result of [`SemverKind::Satisfies`].
    Satisfaction(SemverSatisfiesReport),
}

/// Compare semantic versions by SemVer 2.0.0 precedence, or check a version
/// against a Cargo-style requirement range.
pub fn compare_semver(kind: SemverKind) -> Result<SemverReport, SemverError> {
    match kind {
        SemverKind::Compare(options) => compare_versions(&options).map(SemverReport::Comparison),
        SemverKind::Satisfies(options) => check_requirement(&options).map(SemverReport::Satisfaction),
    }
}

// -------- Comparison helpers --------

fn compare_versions(options: &SemverCompareOptions) -> Result<SemverCompareReport, SemverError> {
    let left = parse_version(&options.left, SemverSide::Left, options.max_chars)?;
    let right = parse_version(&options.right, SemverSide::Right, options.max_chars)?;
    let relation = SemverRelation::from_ordering(left.cmp_precedence(&right));
    Ok(SemverCompareReport { left, right, relation })
}

fn check_requirement(options: &SemverSatisfiesOptions) -> Result<SemverSatisfiesReport, SemverError> {
    let version = parse_version(&options.version, SemverSide::Left, MAX_CHARS_PER_SIDE)?;
    let requirement = VersionReq::parse(&options.requirement).map_err(|error| SemverError::InvalidRequirement {
        input: options.requirement.clone(),
        message: error.to_string(),
    })?;
    Ok(SemverSatisfiesReport { version, requirement })
}

fn parse_version(input: &str, side: SemverSide, max_chars: usize) -> Result<Version, SemverError> {
    if input.chars().count() > max_chars {
        return Err(SemverError::InputTooLong { side, limit: max_chars });
    }
    Version::parse(input).map_err(|error| SemverError::InvalidVersion {
        side,
        input: input.to_string(),
        message: error.to_string(),
    })
}

// -------- Tests --------

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(left: &str, right: &str) -> SemverCompareReport {
        let options = SemverCompareOptions {
            left: left.to_string(),
            right: right.to_string(),
            ..SemverCompareOptions::default()
        };
        match compare_semver(SemverKind::Compare(options)).unwrap() {
            SemverReport::Comparison(report) => report,
            SemverReport::Satisfaction(_) => panic!("expected a comparison report"),
        }
    }

    fn compare_err(left: &str, right: &str) -> SemverError {
        let options = SemverCompareOptions {
            left: left.to_string(),
            right: right.to_string(),
            ..SemverCompareOptions::default()
        };
        compare_semver(SemverKind::Compare(options)).unwrap_err()
    }

    fn satisfies(version: &str, requirement: &str) -> SemverSatisfiesReport {
        let options = SemverSatisfiesOptions {
            version: version.to_string(),
            requirement: requirement.to_string(),
        };
        match compare_semver(SemverKind::Satisfies(options)).unwrap() {
            SemverReport::Satisfaction(report) => report,
            SemverReport::Comparison(_) => panic!("expected a satisfaction report"),
        }
    }

    fn satisfies_err(version: &str, requirement: &str) -> SemverError {
        let options = SemverSatisfiesOptions {
            version: version.to_string(),
            requirement: requirement.to_string(),
        };
        compare_semver(SemverKind::Satisfies(options)).unwrap_err()
    }

    #[test]
    fn identical_versions_are_equal() {
        let report = compare("1.2.3", "1.2.3");
        assert_eq!(report.relation, SemverRelation::Equal);
        assert!(report.is_equal());
        assert_eq!(report.to_text(), "1.2.3 = 1.2.3");
    }

    #[test]
    fn major_minor_patch_compare_numerically() {
        assert_eq!(compare("2.0.0", "1.9.9").relation, SemverRelation::Greater);
        assert_eq!(compare("1.3.0", "1.2.9").relation, SemverRelation::Greater);
        assert_eq!(compare("1.2.4", "1.2.3").relation, SemverRelation::Greater);
        assert_eq!(compare("1.2.3", "1.2.4").relation, SemverRelation::Less);
        assert_eq!(compare("10.0.0", "9.0.0").relation, SemverRelation::Greater);
    }

    #[test]
    fn spec_precedence_chain_is_ordered() {
        let chain = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in chain.windows(2) {
            assert_eq!(compare(pair[0], pair[1]).relation, SemverRelation::Less, "{} < {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn prerelease_sorts_below_release() {
        assert_eq!(compare("1.0.0-rc.1", "1.0.0").relation, SemverRelation::Less);
        assert_eq!(compare("1.0.0", "1.0.0-alpha").relation, SemverRelation::Greater);
    }

    #[test]
    fn numeric_prerelease_identifiers_compare_numerically() {
        assert_eq!(compare("1.0.0-alpha.9", "1.0.0-alpha.10").relation, SemverRelation::Less);
    }

    #[test]
    fn numeric_identifier_sorts_below_alphanumeric() {
        assert_eq!(compare("1.0.0-alpha.1", "1.0.0-alpha.beta").relation, SemverRelation::Less);
    }

    #[test]
    fn build_metadata_is_ignored_in_precedence() {
        let report = compare("1.0.0+build.1", "1.0.0+build.2");
        assert_eq!(report.relation, SemverRelation::Equal);
        assert!(report.is_equal());
        let report = compare("1.0.0+abc", "1.0.0");
        assert_eq!(report.relation, SemverRelation::Equal);
        let report = compare("1.0.0+abc", "1.0.1+def");
        assert_eq!(report.relation, SemverRelation::Less);
    }

    #[test]
    fn report_text_uses_relation_symbols() {
        assert_eq!(compare("1.2.3", "2.0.0").to_text(), "1.2.3 < 2.0.0");
        assert_eq!(compare("2.0.0", "1.2.3").to_text(), "2.0.0 > 1.2.3");
    }

    #[test]
    fn report_text_keeps_build_metadata() {
        assert_eq!(compare("1.0.0+build.1", "1.0.0+build.2").to_text(), "1.0.0+build.1 = 1.0.0+build.2");
    }

    #[test]
    fn meets_checks_all_expectations() {
        let less = compare("1.2.3", "2.0.0");
        assert!(less.meets(Some(SemverExpectation::Lt)));
        assert!(less.meets(Some(SemverExpectation::Le)));
        assert!(!less.meets(Some(SemverExpectation::Eq)));
        assert!(!less.meets(Some(SemverExpectation::Ge)));
        assert!(!less.meets(Some(SemverExpectation::Gt)));

        let equal = compare("1.2.3", "1.2.3");
        assert!(!equal.meets(Some(SemverExpectation::Lt)));
        assert!(equal.meets(Some(SemverExpectation::Le)));
        assert!(equal.meets(Some(SemverExpectation::Eq)));
        assert!(equal.meets(Some(SemverExpectation::Ge)));
        assert!(!equal.meets(Some(SemverExpectation::Gt)));

        let greater = compare("2.0.0", "1.2.3");
        assert!(!greater.meets(Some(SemverExpectation::Lt)));
        assert!(!greater.meets(Some(SemverExpectation::Le)));
        assert!(!greater.meets(Some(SemverExpectation::Eq)));
        assert!(greater.meets(Some(SemverExpectation::Ge)));
        assert!(greater.meets(Some(SemverExpectation::Gt)));
    }

    #[test]
    fn meets_without_expectation_always_holds() {
        for (left, right) in [("1.2.3", "2.0.0"), ("1.2.3", "1.2.3"), ("2.0.0", "1.2.3")] {
            assert!(compare(left, right).meets(None));
        }
    }

    #[test]
    fn invalid_versions_report_the_failing_side() {
        let error = compare_err("1.x", "1.0.0");
        assert!(
            error
                .to_string()
                .contains("cannot parse \"1.x\" as a semver version on the left side")
        );
        let error = compare_err("1.0.0", "nope");
        assert!(
            error
                .to_string()
                .contains("cannot parse \"nope\" as a semver version on the right side")
        );
    }

    #[test]
    fn leading_zero_versions_are_invalid() {
        let error = compare_err("01.2.3", "1.0.0");
        assert!(error.to_string().contains("cannot parse \"01.2.3\""));
    }

    #[test]
    fn missing_components_are_invalid() {
        for input in ["1", "1.2", "1.2.3.4"] {
            assert!(compare_err(input, "1.0.0").to_string().contains("cannot parse"));
        }
    }

    #[test]
    fn empty_version_is_invalid() {
        assert!(compare_err("", "1.0.0").to_string().contains("cannot parse \"\""));
    }

    #[test]
    fn oversized_inputs_error_per_side() {
        let options = SemverCompareOptions {
            left: "1.2.3-rc.1".to_string(),
            right: "1.0.0".to_string(),
            max_chars: 8,
        };
        let error = compare_semver(SemverKind::Compare(options)).unwrap_err();
        assert_eq!(
            error,
            SemverError::InputTooLong {
                side: SemverSide::Left,
                limit: 8
            }
        );
        assert!(
            error
                .to_string()
                .contains("left semver input must be at most 8 characters")
        );

        let options = SemverCompareOptions {
            left: "1.0.0".to_string(),
            right: "1.2.3-rc.1".to_string(),
            max_chars: 8,
        };
        let error = compare_semver(SemverKind::Compare(options)).unwrap_err();
        assert_eq!(
            error,
            SemverError::InputTooLong {
                side: SemverSide::Right,
                limit: 8
            }
        );
    }

    #[test]
    fn satisfies_caret_range() {
        assert!(satisfies("1.2.3", "^1.2").is_satisfied());
        assert!(satisfies("1.9.9", "^1.2").is_satisfied());
        assert!(!satisfies("2.0.0", "^1.2").is_satisfied());
        assert!(!satisfies("0.9.0", "^1.2").is_satisfied());
    }

    #[test]
    fn satisfies_tilde_range() {
        assert!(satisfies("1.2.9", "~1.2").is_satisfied());
        assert!(!satisfies("1.3.0", "~1.2").is_satisfied());
        assert!(!satisfies("2.0.0", "~1.2").is_satisfied());
    }

    #[test]
    fn satisfies_comparator_list() {
        assert!(satisfies("1.5.0", ">=1.0.0, <2.0.0").is_satisfied());
        assert!(satisfies("1.0.0", ">=1.0.0, <2.0.0").is_satisfied());
        assert!(!satisfies("2.0.0", ">=1.0.0, <2.0.0").is_satisfied());
        assert!(!satisfies("0.9.9", ">=1.0.0, <2.0.0").is_satisfied());
    }

    #[test]
    fn satisfies_wildcard() {
        assert!(satisfies("1.2.9", "1.2.*").is_satisfied());
        assert!(!satisfies("1.3.0", "1.2.*").is_satisfied());
    }

    #[test]
    fn satisfies_prerelease_range() {
        assert!(satisfies("1.0.0-rc.2", "^1.0.0-rc.1").is_satisfied());
        assert!(!satisfies("2.0.0", "^1.0.0-rc.1").is_satisfied());
    }

    #[test]
    fn satisfies_star_matches_releases_only() {
        assert!(satisfies("0.1.0", "*").is_satisfied());
        assert!(satisfies("99.0.0", "*").is_satisfied());
        // Cargo rule: a prerelease only satisfies a requirement whose
        // comparators carry a prerelease on the same major.minor.patch.
        assert!(!satisfies("99.0.0-alpha", "*").is_satisfied());
        assert!(!satisfies("1.2.3-rc.1", "^1.2").is_satisfied());
    }

    #[test]
    fn satisfies_report_text() {
        let report = satisfies("1.2.3", "^1.2");
        assert!(report.is_satisfied());
        assert_eq!(report.to_text(), "1.2.3 satisfies ^1.2");
    }

    #[test]
    fn invalid_requirement_errors() {
        let error = satisfies_err("1.2.3", "abc");
        assert!(
            error
                .to_string()
                .contains("cannot parse \"abc\" as a version requirement")
        );
        let error = satisfies_err("1.2.3", "");
        assert!(error.to_string().contains("cannot parse \"\""));
    }

    #[test]
    fn invalid_version_in_satisfies_errors() {
        let error = satisfies_err("1.x", "^1.0");
        assert!(
            error
                .to_string()
                .contains("cannot parse \"1.x\" as a semver version on the left side")
        );
    }

    #[test]
    fn options_defaults_are_sane() {
        let options = SemverCompareOptions::default();
        assert!(options.left.is_empty());
        assert!(options.right.is_empty());
        assert_eq!(options.max_chars, MAX_CHARS_PER_SIDE);
        let options = SemverSatisfiesOptions::default();
        assert!(options.version.is_empty());
        assert!(options.requirement.is_empty());
    }
}

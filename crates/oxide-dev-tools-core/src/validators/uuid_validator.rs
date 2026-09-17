//! UUID validation (RFC 9562 forms, versions, and variants).
//!
//! [`validate_uuid`] checks a UUID against the RFC 9562 syntax using the
//! [`uuid`] crate parser, which accepts four serialization forms: the
//! canonical hyphenated form, the simple form without hyphens, the braced
//! form (`{...}`), and the URN form (`urn:uuid:...`). A specific input form
//! can be required with [`UuidOptions::kind`].
//!
//! The version field (v1–v8, plus the special nil and max UUIDs) and the
//! variant field (RFC 4122, NCS, Microsoft, future) can be required with
//! [`UuidOptions::version`] and [`UuidOptions::variant`]. The version field
//! is only defined for the RFC 4122 variant, so a version requirement can
//! never be satisfied by a UUID of another variant — except the nil and max
//! UUIDs, whose versions RFC 9562 defines regardless of variant.
//!
//! The validator performs no network lookups; syntax and fields are checked
//! locally.

use uuid::{Uuid, Variant, Version};

// -------- Public API --------

/// Required input form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UuidKind {
    /// Accept any supported form.
    #[default]
    Any,
    /// 32 hex digits without hyphens.
    Simple,
    /// The canonical 8-4-4-4-12 hyphenated form.
    Hyphenated,
    /// The hyphenated form wrapped in braces (`{...}`).
    Braced,
    /// The `urn:uuid:` prefixed form.
    Urn,
}

/// Required version (RFC 9562 § 4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UuidVersion {
    /// Accept any version.
    #[default]
    Any,
    /// The nil UUID (all zeros).
    Nil,
    /// The max UUID (all ones).
    Max,
    /// Version 1 (Gregorian timestamp + node).
    V1,
    /// Version 2 (DCE security).
    V2,
    /// Version 3 (MD5 namespace + name).
    V3,
    /// Version 4 (random).
    V4,
    /// Version 5 (SHA-1 namespace + name).
    V5,
    /// Version 6 (reordered Gregorian timestamp + node).
    V6,
    /// Version 7 (Unix timestamp + random).
    V7,
    /// Version 8 (custom).
    V8,
    /// An unrecognized version field value (report-only; cannot be required).
    Unknown,
}

/// Required variant (RFC 9562 § 4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UuidVariant {
    /// Accept any variant.
    #[default]
    Any,
    /// Reserved by the NCS for backward compatibility.
    Ncs,
    /// The RFC 4122 variant (the majority of UUIDs).
    Rfc4122,
    /// Reserved by Microsoft for backward compatibility.
    Microsoft,
    /// Reserved for future definition.
    Future,
}

/// Options for [`validate_uuid`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UuidOptions {
    /// The UUID to validate (surrounding whitespace is ignored).
    pub input: String,
    /// Required input form.
    pub kind: UuidKind,
    /// Required version.
    pub version: UuidVersion,
    /// Required variant.
    pub variant: UuidVariant,
}

impl Default for UuidOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            kind: UuidKind::Any,
            version: UuidVersion::Any,
            variant: UuidVariant::Any,
        }
    }
}

/// The result of validating a UUID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UuidReport {
    /// Whether the UUID satisfies the configured rules.
    pub valid: bool,
    /// The UUID as given, without surrounding whitespace.
    pub input: String,
    /// The canonical lowercase hyphenated form.
    pub normalized: String,
    /// The input form that was detected (best effort when invalid).
    pub kind: UuidKind,
    /// The raw version field, or [`UuidVersion::Unknown`] when the value is
    /// not recognized. The field is only meaningful for the RFC 4122 variant.
    pub version: UuidVersion,
    /// The variant field.
    pub variant: UuidVariant,
    /// Reasons the UUID is invalid.
    pub issues: Vec<String>,
    /// Valid but unusual or discouraged constructs.
    pub warnings: Vec<String>,
}

/// Validate a UUID and produce a detailed report.
///
/// Validation never fails with an error and never panics: an invalid UUID is
/// reported via [`UuidReport::valid`] and [`UuidReport::issues`].
pub fn validate_uuid(options: UuidOptions) -> UuidReport {
    let trimmed = options.input.trim();
    let mut report = UuidReport {
        valid: false,
        input: trimmed.to_string(),
        normalized: String::new(),
        kind: detect_kind(trimmed),
        version: UuidVersion::Unknown,
        variant: UuidVariant::Any,
        issues: Vec::new(),
        warnings: Vec::new(),
    };

    if trimmed.is_empty() {
        report.issues.push("input is empty".to_string());
        return report;
    }

    let uuid = match Uuid::try_parse(trimmed) {
        Ok(uuid) => uuid,
        Err(_) => {
            report.issues.push("invalid UUID syntax".to_string());
            return report;
        }
    };

    report.kind = detect_kind(trimmed);
    report.normalized = uuid.to_string();
    report.variant = variant_of(uuid.get_variant());
    report.version = version_of(uuid.get_version());

    check_form(&options, &mut report);
    check_variant(&options, &mut report);
    check_version(uuid, &options, &mut report);
    finish_check(trimmed, uuid, &options, &mut report);
    report
}

// -------- Detection --------

/// Classify the serialization form from the input syntax.
fn detect_kind(input: &str) -> UuidKind {
    if input.starts_with("urn:uuid:") {
        UuidKind::Urn
    } else if input.starts_with('{') || input.ends_with('}') {
        UuidKind::Braced
    } else if input.len() == 32 && !input.contains('-') {
        UuidKind::Simple
    } else {
        UuidKind::Hyphenated
    }
}

fn variant_of(variant: Variant) -> UuidVariant {
    match variant {
        Variant::NCS => UuidVariant::Ncs,
        Variant::RFC4122 => UuidVariant::Rfc4122,
        Variant::Microsoft => UuidVariant::Microsoft,
        _ => UuidVariant::Future,
    }
}

fn version_of(version: Option<Version>) -> UuidVersion {
    match version {
        Some(Version::Nil) => UuidVersion::Nil,
        Some(Version::Mac) => UuidVersion::V1,
        Some(Version::Dce) => UuidVersion::V2,
        Some(Version::Md5) => UuidVersion::V3,
        Some(Version::Random) => UuidVersion::V4,
        Some(Version::Sha1) => UuidVersion::V5,
        Some(Version::SortMac) => UuidVersion::V6,
        Some(Version::SortRand) => UuidVersion::V7,
        Some(Version::Custom) => UuidVersion::V8,
        Some(Version::Max) => UuidVersion::Max,
        _ => UuidVersion::Unknown,
    }
}

// -------- Constraint checks --------

fn check_form(options: &UuidOptions, report: &mut UuidReport) {
    if options.kind == UuidKind::Any {
        return;
    }
    if report.kind != options.kind {
        report.issues.push(format!(
            "expected the {} form, found the {} form",
            kind_name(options.kind),
            kind_name(report.kind)
        ));
    }
}

fn check_variant(options: &UuidOptions, report: &mut UuidReport) {
    if options.variant == UuidVariant::Any {
        return;
    }
    if report.variant != options.variant {
        report.issues.push(format!(
            "expected the {} variant, found the {} variant",
            variant_name(options.variant),
            variant_name(report.variant)
        ));
    }
}

fn check_version(uuid: Uuid, options: &UuidOptions, report: &mut UuidReport) {
    if options.version == UuidVersion::Any {
        return;
    }
    if report.variant == UuidVariant::Rfc4122 {
        if report.version != options.version {
            report.issues.push(format!(
                "expected version {}, found {}",
                version_name(options.version),
                version_name(report.version)
            ));
        }
        return;
    }
    // The version field is only defined for the RFC 4122 variant; RFC 9562
    // defines the nil and max UUID versions themselves, so those two are
    // the only non-RFC 4122 UUIDs that can satisfy a version requirement.
    let special = (uuid.is_nil() && options.version == UuidVersion::Nil)
        || (uuid.is_max() && options.version == UuidVersion::Max);
    if !special {
        report
            .issues
            .push(format!("version is undefined for the {} variant", variant_name(report.variant)));
    }
}

// -------- Report assembly --------

fn finish_check(input: &str, uuid: Uuid, options: &UuidOptions, report: &mut UuidReport) {
    if options.version == UuidVersion::Any && uuid.is_nil() {
        report.warnings.push("nil UUID (all zeros)".to_string());
    }
    if options.version == UuidVersion::Any && uuid.is_max() {
        report.warnings.push("max UUID (all ones)".to_string());
    }
    if options.variant == UuidVariant::Any && report.variant != UuidVariant::Rfc4122 && !uuid.is_nil() && !uuid.is_max()
    {
        report
            .warnings
            .push(format!("variant is not RFC 4122 ({})", variant_name(report.variant)));
    }
    if input != input.to_ascii_lowercase() {
        report
            .warnings
            .push("uppercase input was normalized to lowercase".to_string());
    }
    report.valid = report.issues.is_empty();
}

// -------- Naming --------

fn kind_name(kind: UuidKind) -> &'static str {
    match kind {
        UuidKind::Any => "any",
        UuidKind::Simple => "simple",
        UuidKind::Hyphenated => "hyphenated",
        UuidKind::Braced => "braced",
        UuidKind::Urn => "urn",
    }
}

fn version_name(version: UuidVersion) -> &'static str {
    match version {
        UuidVersion::Any => "any",
        UuidVersion::Nil => "nil",
        UuidVersion::Max => "max",
        UuidVersion::V1 => "v1",
        UuidVersion::V2 => "v2",
        UuidVersion::V3 => "v3",
        UuidVersion::V4 => "v4",
        UuidVersion::V5 => "v5",
        UuidVersion::V6 => "v6",
        UuidVersion::V7 => "v7",
        UuidVersion::V8 => "v8",
        UuidVersion::Unknown => "unknown",
    }
}

fn variant_name(variant: UuidVariant) -> &'static str {
    match variant {
        UuidVariant::Any => "any",
        UuidVariant::Ncs => "NCS",
        UuidVariant::Rfc4122 => "RFC 4122",
        UuidVariant::Microsoft => "Microsoft",
        UuidVariant::Future => "future",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One well-formed UUID per version (all RFC 4122 variant), plus the
    // special nil/max UUIDs and a Microsoft-variant GUID.
    const V1: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    const V2: &str = "000003e8-0000-2000-8000-000000000000";
    const V3: &str = "3d813cbb-47fb-32ba-91df-831e1593ac29";
    const V4: &str = "550e8400-e29b-41d4-a716-446655440000";
    const V5: &str = "886313e1-3b8a-5372-9b90-0c9aee199e5d";
    const V6: &str = "1ec9414c-232a-6b00-b3c8-9f6bdeced846";
    const V7: &str = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
    const V8: &str = "3209263e-9317-8cff-b4f4-f36de920dbb4";
    const NIL: &str = "00000000-0000-0000-0000-000000000000";
    const MAX: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";
    const MICROSOFT: &str = "550e8400-e29b-41d4-c716-446655440000";

    fn check(input: &str) -> UuidReport {
        validate_uuid(UuidOptions {
            input: input.to_string(),
            ..UuidOptions::default()
        })
    }

    fn check_with(input: &str, mutate: impl FnOnce(&mut UuidOptions)) -> UuidReport {
        let mut options = UuidOptions {
            input: input.to_string(),
            ..UuidOptions::default()
        };
        mutate(&mut options);
        validate_uuid(options)
    }

    fn assert_valid(input: &str) -> UuidReport {
        let report = check(input);
        assert!(report.valid, "expected '{input}' to be valid: {:?}", report.issues);
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

    #[test]
    fn valid_versions_are_detected() {
        for (input, expected) in [
            (V1, UuidVersion::V1),
            (V2, UuidVersion::V2),
            (V3, UuidVersion::V3),
            (V4, UuidVersion::V4),
            (V5, UuidVersion::V5),
            (V6, UuidVersion::V6),
            (V7, UuidVersion::V7),
            (V8, UuidVersion::V8),
        ] {
            let report = assert_valid(input);
            assert_eq!(report.version, expected, "wrong version for '{input}'");
            assert_eq!(report.variant, UuidVariant::Rfc4122);
            assert_eq!(report.normalized, input);
        }
    }

    #[test]
    fn nil_and_max_are_valid_with_warnings() {
        let report = assert_valid(NIL);
        assert_eq!(report.version, UuidVersion::Nil);
        assert_eq!(report.variant, UuidVariant::Ncs);
        assert!(report.warnings.iter().any(|warning| warning.contains("nil UUID")));

        let report = assert_valid(MAX);
        assert_eq!(report.version, UuidVersion::Max);
        assert_eq!(report.variant, UuidVariant::Future);
        assert!(report.warnings.iter().any(|warning| warning.contains("max UUID")));
    }

    #[test]
    fn alternative_forms_are_accepted() {
        let cases = [
            ("550e8400e29b41d4a716446655440000", UuidKind::Simple),
            ("{550e8400-e29b-41d4-a716-446655440000}", UuidKind::Braced),
            ("urn:uuid:550e8400-e29b-41d4-a716-446655440000", UuidKind::Urn),
            (V4, UuidKind::Hyphenated),
        ];
        for (input, expected) in cases {
            let report = assert_valid(input);
            assert_eq!(report.kind, expected, "wrong form for '{input}'");
            assert_eq!(report.normalized, V4, "wrong normalization for '{input}'");
        }
    }

    #[test]
    fn invalid_syntax_is_reported() {
        for input in [
            "nope",
            "550e8400-e29b-41d4-a716-44665544000",
            "550e8400-e29b-41d4-a716-4466554400000",
            "550e8400-e29b-41d4-a716-44665544000g",
            "550e8400-e29b-41d4-a716-4466554400-00",
            "{550e8400e29b41d4a716446655440000}",
            "{550e8400-e29b-41d4-a716-446655440000",
            "urn:uuid:550e8400e29b41d4a716446655440000",
        ] {
            assert_issue(input, "invalid UUID syntax");
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
        let report = check("  550e8400-e29b-41d4-a716-446655440000  ");
        assert!(report.valid);
        assert_eq!(report.input, V4);
        assert_eq!(report.normalized, V4);
    }

    #[test]
    fn uppercase_input_is_normalized_with_warning() {
        let report = check("550E8400-E29B-41D4-A716-446655440000");
        assert!(report.valid);
        assert_eq!(report.normalized, V4);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("normalized to lowercase"))
        );

        let report = check("550E8400E29B41D4A716446655440000");
        assert!(report.valid);
        assert_eq!(report.kind, UuidKind::Simple);
        assert_eq!(report.normalized, V4);
    }

    #[test]
    fn kind_requirement_is_enforced() {
        let report = check_with(V4, |options| options.kind = UuidKind::Simple);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected the simple form"))
        );

        let report = check_with(V4, |options| options.kind = UuidKind::Urn);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected the urn form"))
        );

        assert!(check_with("550e8400e29b41d4a716446655440000", |options| options.kind = UuidKind::Simple).valid);
        assert!(check_with("{550e8400-e29b-41d4-a716-446655440000}", |options| options.kind = UuidKind::Braced).valid);
        assert!(
            check_with("urn:uuid:550e8400-e29b-41d4-a716-446655440000", |options| options.kind = UuidKind::Urn).valid
        );
        assert!(check_with(V4, |options| options.kind = UuidKind::Hyphenated).valid);
    }

    #[test]
    fn version_requirement_is_enforced() {
        let report = check_with(V1, |options| options.version = UuidVersion::V4);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected version v4, found v1"))
        );

        for (input, version) in [
            (V1, UuidVersion::V1),
            (V3, UuidVersion::V3),
            (V4, UuidVersion::V4),
            (V5, UuidVersion::V5),
            (V6, UuidVersion::V6),
            (V7, UuidVersion::V7),
            (V8, UuidVersion::V8),
        ] {
            assert!(check_with(input, |options| options.version = version).valid);
        }
    }

    #[test]
    fn version_requirement_on_non_rfc4122_variant() {
        let report = check_with(NIL, |options| options.version = UuidVersion::V1);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("version is undefined for the NCS variant"))
        );

        let report = check_with(MAX, |options| options.version = UuidVersion::V4);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("version is undefined for the future variant"))
        );

        // The version bits of a Microsoft GUID are not meaningful.
        let report = check_with(MICROSOFT, |options| options.version = UuidVersion::V4);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("version is undefined for the Microsoft variant"))
        );

        assert!(check_with(NIL, |options| options.version = UuidVersion::Nil).valid);
        assert!(check_with(MAX, |options| options.version = UuidVersion::Max).valid);
    }

    #[test]
    fn variant_requirement_is_enforced() {
        let report = check_with(MICROSOFT, |options| options.variant = UuidVariant::Rfc4122);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected the RFC 4122 variant, found the Microsoft variant"))
        );

        let report = check_with(NIL, |options| options.variant = UuidVariant::Rfc4122);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("expected the RFC 4122 variant, found the NCS variant"))
        );

        assert!(check_with(V4, |options| options.variant = UuidVariant::Rfc4122).valid);
        assert!(check_with(MICROSOFT, |options| options.variant = UuidVariant::Microsoft).valid);
        assert!(check_with(NIL, |options| options.variant = UuidVariant::Ncs).valid);
        assert!(check_with(MAX, |options| options.variant = UuidVariant::Future).valid);
    }

    #[test]
    fn warnings_for_unusual_but_valid_input() {
        let report = assert_valid(MICROSOFT);
        assert!(!report.warnings.is_empty());
        assert!(report.warnings.iter().any(|warning| warning.contains("not RFC 4122")));

        assert!(assert_valid(V4).warnings.is_empty());

        let report = check_with(NIL, |options| options.version = UuidVersion::Nil);
        assert!(report.valid);
        assert!(!report.warnings.iter().any(|warning| warning.contains("nil UUID")));
    }

    #[test]
    fn unrecognized_version_bits_are_reported_as_unknown() {
        let report = check("550e8400-e29b-91d4-a716-446655440000");
        assert!(report.valid, "unexpected issues: {:?}", report.issues);
        assert_eq!(report.version, UuidVersion::Unknown);
        assert_eq!(report.variant, UuidVariant::Rfc4122);
    }
}

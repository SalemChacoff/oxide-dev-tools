use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::ValidError;

/// `oxide validate uuid <UUID> [options]` — UUID validation and inspection
#[derive(Args)]
pub struct UuidArgs {
    /// UUID to validate
    pub value: String,

    /// Required input form
    #[arg(long, value_enum, default_value_t = UuidKindArg::Any)]
    pub kind: UuidKindArg,

    /// Required UUID version
    #[arg(long, value_enum, default_value_t = UuidVersionArg::Any)]
    pub version: UuidVersionArg,

    /// Required UUID variant
    #[arg(long, value_enum, default_value_t = UuidVariantArg::Any)]
    pub variant: UuidVariantArg,

    /// Print the full validation report instead of the bare canonical UUID
    #[arg(long)]
    pub verbose: bool,
}

/// Input form choices for the `--kind` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UuidKindArg {
    /// Accept any supported form.
    Any,
    /// Require 32 hex digits without hyphens.
    Simple,
    /// Require the canonical 8-4-4-4-12 hyphenated form.
    Hyphenated,
    /// Require the hyphenated form wrapped in braces.
    Braced,
    /// Require the `urn:uuid:` prefixed form.
    Urn,
}

impl From<UuidKindArg> for UuidKind {
    fn from(arg: UuidKindArg) -> Self {
        match arg {
            UuidKindArg::Any => UuidKind::Any,
            UuidKindArg::Simple => UuidKind::Simple,
            UuidKindArg::Hyphenated => UuidKind::Hyphenated,
            UuidKindArg::Braced => UuidKind::Braced,
            UuidKindArg::Urn => UuidKind::Urn,
        }
    }
}

/// Version choices for the `--version` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UuidVersionArg {
    /// Accept any version.
    Any,
    /// Require the nil UUID (all zeros).
    Nil,
    /// Require the max UUID (all ones).
    Max,
    /// Require version 1 (Gregorian timestamp + node).
    V1,
    /// Require version 2 (DCE security).
    V2,
    /// Require version 3 (MD5 namespace + name).
    V3,
    /// Require version 4 (random).
    V4,
    /// Require version 5 (SHA-1 namespace + name).
    V5,
    /// Require version 6 (reordered Gregorian timestamp + node).
    V6,
    /// Require version 7 (Unix timestamp + random).
    V7,
    /// Require version 8 (custom).
    V8,
}

impl From<UuidVersionArg> for UuidVersion {
    fn from(arg: UuidVersionArg) -> Self {
        match arg {
            UuidVersionArg::Any => UuidVersion::Any,
            UuidVersionArg::Nil => UuidVersion::Nil,
            UuidVersionArg::Max => UuidVersion::Max,
            UuidVersionArg::V1 => UuidVersion::V1,
            UuidVersionArg::V2 => UuidVersion::V2,
            UuidVersionArg::V3 => UuidVersion::V3,
            UuidVersionArg::V4 => UuidVersion::V4,
            UuidVersionArg::V5 => UuidVersion::V5,
            UuidVersionArg::V6 => UuidVersion::V6,
            UuidVersionArg::V7 => UuidVersion::V7,
            UuidVersionArg::V8 => UuidVersion::V8,
        }
    }
}

/// Variant choices for the `--variant` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UuidVariantArg {
    /// Accept any variant.
    Any,
    /// Require the variant reserved by the NCS.
    Ncs,
    /// Require the RFC 4122 variant.
    Rfc4122,
    /// Require the variant reserved by Microsoft.
    Microsoft,
    /// Require the variant reserved for future definition.
    Future,
}

impl From<UuidVariantArg> for UuidVariant {
    fn from(arg: UuidVariantArg) -> Self {
        match arg {
            UuidVariantArg::Any => UuidVariant::Any,
            UuidVariantArg::Ncs => UuidVariant::Ncs,
            UuidVariantArg::Rfc4122 => UuidVariant::Rfc4122,
            UuidVariantArg::Microsoft => UuidVariant::Microsoft,
            UuidVariantArg::Future => UuidVariant::Future,
        }
    }
}

pub fn exec(args: UuidArgs) -> Result<(), ValidError> {
    let options = UuidOptions {
        input: args.value,
        kind: args.kind.into(),
        version: args.version.into(),
        variant: args.variant.into(),
    };
    let report = validate_uuid(options);
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
        Err(ValidError::Uuid(report.issues.join("; ")))
    }
}

// -------- Report output --------

fn print_report(report: &UuidReport) {
    println!("valid:      {}", yes_no(report.valid));
    println!("input:      {}", report.input);
    println!("normalized: {}", report.normalized);
    println!("kind:       {}", kind_name(report.kind));
    println!("version:    {}", version_name(report.version));
    println!("variant:    {}", variant_name(report.variant));
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
        UuidVariant::Ncs => "ncs",
        UuidVariant::Rfc4122 => "rfc4122",
        UuidVariant::Microsoft => "microsoft",
        UuidVariant::Future => "future",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(value: &str) -> UuidArgs {
        UuidArgs {
            value: value.to_string(),
            kind: UuidKindArg::Any,
            version: UuidVersionArg::Any,
            variant: UuidVariantArg::Any,
            verbose: false,
        }
    }

    #[test]
    fn exec_valid_uuid_forms() {
        assert!(exec(args("550e8400-e29b-41d4-a716-446655440000")).is_ok());
        assert!(exec(args("550e8400e29b41d4a716446655440000")).is_ok());
        assert!(exec(args("{550e8400-e29b-41d4-a716-446655440000}")).is_ok());
        assert!(exec(args("urn:uuid:550e8400-e29b-41d4-a716-446655440000")).is_ok());
    }

    #[test]
    fn exec_invalid_uuid() {
        let result = exec(args("not-a-uuid"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid uuid"));
    }

    #[test]
    fn exec_flags() {
        let result = exec(UuidArgs {
            kind: UuidKindArg::Simple,
            ..args("550e8400-e29b-41d4-a716-446655440000")
        });
        assert!(result.unwrap_err().to_string().contains("expected the simple form"));

        let result = exec(UuidArgs {
            version: UuidVersionArg::V1,
            ..args("550e8400-e29b-41d4-a716-446655440000")
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected version v1, found v4")
        );

        let result = exec(UuidArgs {
            variant: UuidVariantArg::Rfc4122,
            ..args("550e8400-e29b-41d4-c716-446655440000")
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected the RFC 4122 variant")
        );

        assert!(
            exec(UuidArgs {
                version: UuidVersionArg::V4,
                ..args("550e8400-e29b-41d4-a716-446655440000")
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_verbose_valid_and_invalid() {
        assert!(
            exec(UuidArgs {
                verbose: true,
                ..args("550e8400-e29b-41d4-a716-446655440000")
            })
            .is_ok()
        );
        assert!(
            exec(UuidArgs {
                verbose: true,
                ..args("nope")
            })
            .is_err()
        );
    }
}

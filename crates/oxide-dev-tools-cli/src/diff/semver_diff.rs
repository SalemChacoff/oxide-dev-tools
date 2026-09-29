use clap::{Args, Subcommand, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::{DiffError, GenericError};

/// `oxide diff semver ...` — compare versions by SemVer 2.0.0 precedence
///
/// `compare` reports the precedence relation between two versions, with an
/// optional asserted relation (`--expect`) and a CI-friendly silent
/// `--check` mode. `satisfies` checks a version against a Cargo-style
/// requirement range (`^1.2`, `~1.0`, `>=1.0.0, <2.0.0`, `1.2.*`). Build
/// metadata never affects precedence.
#[derive(Args)]
#[command(after_help = r#"Examples:
  oxide diff semver compare 1.2.3 2.0.0
  oxide diff semver compare 1.0.0-rc.1 1.0.0 --expect lt
  oxide diff semver satisfies 1.2.3 "^1.2"
  oxide diff semver satisfies 2.0.0 ">=1.0.0, <2.0.0" --check"#)]
pub struct SemverArgs {
    #[command(subcommand)]
    pub cmd: SemverCmd,
}

#[derive(Subcommand)]
pub enum SemverCmd {
    /// Compare two versions by SemVer 2.0.0 precedence.
    Compare(CompareArgs),
    /// Check whether a version satisfies a Cargo-style requirement.
    Satisfies(SatisfiesArgs),
}

/// `oxide diff semver compare <LEFT> <RIGHT> [flags]` — compare two versions
///
/// Prints one line with the precedence relation, e.g. `1.2.3 < 2.0.0`.
/// `--expect` asserts a relation and exits 1 with an error when it does not
/// hold; `--check` makes the failure silent (CI-friendly). Without
/// `--expect`, `--check` asserts equality. Pre-release identifiers order
/// naturally and build metadata is ignored.
#[derive(Args)]
#[command(after_help = r#"Examples:
  oxide diff semver compare 1.2.3 2.0.0
  oxide diff semver compare 1.0.0-rc.1 1.0.0
  oxide diff semver compare 1.2.3 2.0.0 --expect lt
  oxide diff semver compare 1.2.3 1.2.3 --check
  oxide diff semver compare 2.0.0 1.2.3 --expect gt --check"#)]
pub struct CompareArgs {
    /// First version, e.g. "1.2.3"
    pub left: Option<String>,

    /// Second version, e.g. "2.0.0"
    pub right: Option<String>,

    /// Assert this relation and exit 1 when it does not hold
    #[arg(long, value_enum, value_name = "RELATION")]
    pub expect: Option<SemverExpectCli>,

    /// Exit 1 without output when the assertion fails (CI-friendly)
    #[arg(long)]
    pub check: bool,
}

/// `oxide diff semver satisfies <VERSION> <REQUIREMENT> [--check]` — check a
/// version against a requirement range
///
/// Prints `VERSION satisfies REQUIREMENT` when the requirement holds and
/// exits 0; otherwise exits 1 with an error message, or silently when
/// `--check` is given. Requirements use Cargo syntax.
#[derive(Args)]
#[command(after_help = r#"Examples:
  oxide diff semver satisfies 1.2.3 "^1.2"
  oxide diff semver satisfies 1.2.9 "~1.2"
  oxide diff semver satisfies 1.5.0 ">=1.0.0, <2.0.0"
  oxide diff semver satisfies 2.0.0 "^1.2" --check
> Note: on Windows shells, quoting differs — e.g. oxide diff semver satisfies 1.2.3 "^1.2"."#)]
pub struct SatisfiesArgs {
    /// Version to check, e.g. "1.2.3"
    pub version: Option<String>,

    /// Cargo-style requirement, e.g. "^1.2" or ">=1.0.0, <2.0.0"
    pub requirement: Option<String>,

    /// Exit 1 without output when the version does not satisfy (CI-friendly)
    #[arg(long)]
    pub check: bool,
}

/// Relation asserted with `--expect`.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SemverExpectCli {
    /// LEFT must be strictly lower than RIGHT
    Lt,
    /// LEFT must be lower than or equal to RIGHT
    Le,
    /// LEFT must equal RIGHT
    Eq,
    /// LEFT must be higher than or equal to RIGHT
    Ge,
    /// LEFT must be strictly higher than RIGHT
    Gt,
}

impl From<SemverExpectCli> for SemverExpectation {
    fn from(value: SemverExpectCli) -> Self {
        match value {
            SemverExpectCli::Lt => SemverExpectation::Lt,
            SemverExpectCli::Le => SemverExpectation::Le,
            SemverExpectCli::Eq => SemverExpectation::Eq,
            SemverExpectCli::Ge => SemverExpectation::Ge,
            SemverExpectCli::Gt => SemverExpectation::Gt,
        }
    }
}

pub fn exec(args: SemverArgs) -> Result<(), DiffError> {
    match args.cmd {
        SemverCmd::Compare(args) => exec_compare(args),
        SemverCmd::Satisfies(args) => exec_satisfies(args),
    }
}

// -------- Compare --------

fn exec_compare(args: CompareArgs) -> Result<(), DiffError> {
    let left = require_operand(args.left, "LEFT")?;
    let right = require_operand(args.right, "RIGHT")?;
    let options = SemverCompareOptions {
        left,
        right,
        ..SemverCompareOptions::default()
    };
    let report = match compare_semver(SemverKind::Compare(options))? {
        SemverReport::Comparison(report) => report,
        SemverReport::Satisfaction(_) => unreachable!("Compare kind produces a comparison report"),
    };
    let expectation = match (args.expect, args.check) {
        (Some(expect), _) => Some(SemverExpectation::from(expect)),
        (None, true) => Some(SemverExpectation::Eq),
        (None, false) => None,
    };
    if report.meets(expectation) {
        if !args.check {
            println!("{}", report.to_text());
        }
        return Ok(());
    }
    if args.check {
        return Err(DiffError::CheckFailed);
    }
    println!("{}", report.to_text());
    let Some(expected) = expectation else {
        unreachable!("a failed assertion implies an expectation was set");
    };
    Err(DiffError::ExpectationFailed {
        message: format!(
            "expected {} {} {}, but found {}",
            report.left,
            expected.symbol(),
            report.right,
            report.to_text()
        ),
    })
}

// -------- Satisfies --------

fn exec_satisfies(args: SatisfiesArgs) -> Result<(), DiffError> {
    let version = require_operand(args.version, "VERSION")?;
    let requirement = require_operand(args.requirement, "REQUIREMENT")?;
    let options = SemverSatisfiesOptions { version, requirement };
    let report = match compare_semver(SemverKind::Satisfies(options))? {
        SemverReport::Satisfaction(report) => report,
        SemverReport::Comparison(_) => unreachable!("Satisfies kind produces a satisfaction report"),
    };
    if report.is_satisfied() {
        if !args.check {
            println!("{}", report.to_text());
        }
        return Ok(());
    }
    if args.check {
        return Err(DiffError::CheckFailed);
    }
    Err(DiffError::ExpectationFailed {
        message: format!("{} does not satisfy {}", report.version, report.requirement),
    })
}

// -------- Operand resolution --------

/// Require one positional operand, or fail with a generic argument error.
fn require_operand(input: Option<String>, operand: &str) -> Result<String, DiffError> {
    input.ok_or_else(|| GenericError::Argument(format!("missing <{operand}> (a semver version or requirement)")).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn compare_args(left: &str, right: &str) -> CompareArgs {
        CompareArgs {
            left: Some(left.to_string()),
            right: Some(right.to_string()),
            expect: None,
            check: false,
        }
    }

    fn satisfies_args(version: &str, requirement: &str) -> SatisfiesArgs {
        SatisfiesArgs {
            version: Some(version.to_string()),
            requirement: Some(requirement.to_string()),
            check: false,
        }
    }

    #[test]
    fn exec_compare_reports_relation() {
        assert!(
            exec(SemverArgs {
                cmd: SemverCmd::Compare(compare_args("1.2.3", "2.0.0"))
            })
            .is_ok()
        );
        assert!(
            exec(SemverArgs {
                cmd: SemverCmd::Compare(compare_args("1.2.3", "1.2.3"))
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_expect_matching_relation_succeeds() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Compare(CompareArgs {
                expect: Some(SemverExpectCli::Lt),
                ..compare_args("1.2.3", "2.0.0")
            }),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_expect_mismatch_errors() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Compare(CompareArgs {
                expect: Some(SemverExpectCli::Lt),
                ..compare_args("2.0.0", "1.2.3")
            }),
        });
        let error = result.unwrap_err();
        assert!(matches!(error, DiffError::ExpectationFailed { .. }));
        assert!(
            error
                .to_string()
                .contains("expected 2.0.0 < 1.2.3, but found 2.0.0 > 1.2.3")
        );
        assert!(error.source().is_none());
    }

    #[test]
    fn exec_check_equal_succeeds() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Compare(CompareArgs {
                check: true,
                ..compare_args("1.2.3", "1.2.3")
            }),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_check_differ_fails_silently() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Compare(CompareArgs {
                check: true,
                ..compare_args("1.2.3", "2.0.0")
            }),
        });
        let error = result.unwrap_err();
        assert!(matches!(error, DiffError::CheckFailed));
        assert!(error.source().is_none());
    }

    #[test]
    fn exec_check_with_expect_mismatch_is_silent() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Compare(CompareArgs {
                expect: Some(SemverExpectCli::Gt),
                check: true,
                ..compare_args("1.2.3", "2.0.0")
            }),
        });
        assert!(matches!(result.unwrap_err(), DiffError::CheckFailed));
    }

    #[test]
    fn exec_missing_operands_error() {
        let left = exec(SemverArgs {
            cmd: SemverCmd::Compare(CompareArgs {
                left: None,
                ..compare_args("1.2.3", "2.0.0")
            }),
        });
        assert!(left.unwrap_err().to_string().contains("missing <LEFT>"));
        let right = exec(SemverArgs {
            cmd: SemverCmd::Compare(CompareArgs {
                right: None,
                ..compare_args("1.2.3", "2.0.0")
            }),
        });
        assert!(right.unwrap_err().to_string().contains("missing <RIGHT>"));
    }

    #[test]
    fn exec_invalid_version_errors() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Compare(compare_args("1.x", "2.0.0")),
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot parse \"1.x\" as a semver version")
        );
    }

    #[test]
    fn exec_satisfies_succeeds() {
        assert!(
            exec(SemverArgs {
                cmd: SemverCmd::Satisfies(satisfies_args("1.2.3", "^1.2"))
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_satisfies_unsatisfied_errors() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Satisfies(satisfies_args("2.0.0", "^1.2")),
        });
        let error = result.unwrap_err();
        assert!(matches!(error, DiffError::ExpectationFailed { .. }));
        assert!(error.to_string().contains("2.0.0 does not satisfy ^1.2"));
    }

    #[test]
    fn exec_satisfies_check_unsatisfied_is_silent() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Satisfies(SatisfiesArgs {
                check: true,
                ..satisfies_args("2.0.0", "^1.2")
            }),
        });
        assert!(matches!(result.unwrap_err(), DiffError::CheckFailed));
    }

    #[test]
    fn exec_satisfies_check_satisfied_succeeds() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Satisfies(SatisfiesArgs {
                check: true,
                ..satisfies_args("1.2.3", "^1.2")
            }),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_satisfies_missing_operands_error() {
        let version = exec(SemverArgs {
            cmd: SemverCmd::Satisfies(SatisfiesArgs {
                version: None,
                ..satisfies_args("1.2.3", "^1.2")
            }),
        });
        assert!(version.unwrap_err().to_string().contains("missing <VERSION>"));
        let requirement = exec(SemverArgs {
            cmd: SemverCmd::Satisfies(SatisfiesArgs {
                requirement: None,
                ..satisfies_args("1.2.3", "^1.2")
            }),
        });
        assert!(requirement.unwrap_err().to_string().contains("missing <REQUIREMENT>"));
    }

    #[test]
    fn exec_satisfies_invalid_requirement_errors() {
        let result = exec(SemverArgs {
            cmd: SemverCmd::Satisfies(satisfies_args("1.2.3", "abc")),
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot parse \"abc\" as a version requirement")
        );
    }

    #[test]
    fn expect_cli_maps_to_core_expectation() {
        assert_eq!(SemverExpectation::from(SemverExpectCli::Lt), SemverExpectation::Lt);
        assert_eq!(SemverExpectation::from(SemverExpectCli::Le), SemverExpectation::Le);
        assert_eq!(SemverExpectation::from(SemverExpectCli::Eq), SemverExpectation::Eq);
        assert_eq!(SemverExpectation::from(SemverExpectCli::Ge), SemverExpectation::Ge);
        assert_eq!(SemverExpectation::from(SemverExpectCli::Gt), SemverExpectation::Gt);
    }
}

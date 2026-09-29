use std::path::PathBuf;

use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::{DiffError, GenericError};

/// `oxide diff dir <LEFT> <RIGHT> [flags]` — compare two directory trees
///
/// Walks both directories recursively and compares entries by relative
/// path: entries present in only one tree, kind changes (file vs directory
/// vs symlink), file size changes, and symlink target changes. `--deep`
/// also hashes file contents with SHA-256 to catch same-size
/// modifications; `--ignore` prunes `*` wildcard patterns, where `*`
/// matches any sequence of characters including path separators.
/// Identical directories print nothing (like `git diff`); differences
/// print one line per entry unless `--format json` or `--check` is given.
#[derive(Args)]
#[command(after_help = r#"Examples:
  oxide diff dir src backup-src
  oxide diff dir build-v1 build-v2 --deep
  oxide diff dir old new --ignore "*.tmp" --ignore "target/*"
  oxide diff dir old new --follow-symlinks
  oxide diff dir old new --format json
  oxide diff dir old new --check"#)]
pub struct DirDiffArgs {
    /// First directory to compare
    pub left: Option<String>,

    /// Second directory to compare
    pub right: Option<String>,

    /// Hash file contents with SHA-256 instead of comparing sizes only
    #[arg(long)]
    pub deep: bool,

    /// Follow symlinked directories instead of comparing links as leaves
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Skip relative paths matching a * wildcard pattern (repeatable)
    #[arg(long, value_name = "PATTERN")]
    pub ignore: Vec<String>,

    /// Output format: a line report (default) or a JSON array
    #[arg(long, value_enum)]
    pub format: Option<DirFormatCli>,

    /// Exit 1 without output when the directories differ (CI-friendly)
    #[arg(long)]
    pub check: bool,
}

/// How differences are rendered.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DirFormatCli {
    /// One line per difference (default)
    Report,
    /// A JSON array of difference objects
    Json,
}

pub fn exec(args: DirDiffArgs) -> Result<(), DiffError> {
    let left = args
        .left
        .ok_or_else(|| GenericError::Argument("missing <LEFT> (path to a directory)".to_string()))?;
    let right = args
        .right
        .ok_or_else(|| GenericError::Argument("missing <RIGHT> (path to a directory)".to_string()))?;
    let options = DirDiffOptions {
        left_root: PathBuf::from(left),
        right_root: PathBuf::from(right),
        compare_contents: args.deep,
        follow_symlinks: args.follow_symlinks,
        ignore_globs: args.ignore,
    };
    let report = compare_dirs(options)?;
    if report.is_equal() {
        return Ok(());
    }
    if args.check {
        return Err(DiffError::CheckFailed);
    }
    let output = match args.format.unwrap_or(DirFormatCli::Report) {
        DirFormatCli::Report => report.to_text(),
        DirFormatCli::Json => report.to_json(),
    };
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    fn temp_dir(tag: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("oxide-dir-cli-{tag}-{nonce}"));
        fs::create_dir_all(&path).expect("create fixture dir");
        path
    }

    fn write_file(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("file has parent")).expect("create fixture dir");
        fs::write(path, content).expect("write fixture file");
    }

    fn args(left: &Path, right: &Path) -> DirDiffArgs {
        DirDiffArgs {
            left: Some(left.to_string_lossy().into_owned()),
            right: Some(right.to_string_lossy().into_owned()),
            deep: false,
            follow_symlinks: false,
            ignore: Vec::new(),
            format: None,
            check: false,
        }
    }

    fn cleanup(left: &Path, right: &Path) {
        fs::remove_dir_all(left).ok();
        fs::remove_dir_all(right).ok();
    }

    #[test]
    fn exec_report_mode_succeeds() {
        let left = temp_dir("report-left");
        let right = temp_dir("report-right");
        write_file(&left, "only-left.txt", "old");
        write_file(&right, "only-right.txt", "new");
        assert!(exec(args(&left, &right)).is_ok());
        cleanup(&left, &right);
    }

    #[test]
    fn exec_json_format_succeeds() {
        let left = temp_dir("json-left");
        let right = temp_dir("json-right");
        write_file(&right, "extra.txt", "new");
        let result = exec(DirDiffArgs {
            format: Some(DirFormatCli::Json),
            ..args(&left, &right)
        });
        assert!(result.is_ok());
        cleanup(&left, &right);
    }

    #[test]
    fn exec_deep_flag_succeeds() {
        let left = temp_dir("deep-left");
        let right = temp_dir("deep-right");
        write_file(&left, "data.bin", "same size");
        write_file(&right, "data.bin", "SAME SIZE");
        let result = exec(DirDiffArgs {
            deep: true,
            ..args(&left, &right)
        });
        assert!(result.is_ok());
        cleanup(&left, &right);
    }

    #[test]
    fn exec_ignore_flag_succeeds() {
        let left = temp_dir("ignore-left");
        let right = temp_dir("ignore-right");
        write_file(&left, "cache.tmp", "left");
        write_file(&right, "cache.tmp", "right");
        let result = exec(DirDiffArgs {
            ignore: vec!["*.tmp".to_string()],
            ..args(&left, &right)
        });
        assert!(result.is_ok());
        cleanup(&left, &right);
    }

    #[test]
    fn exec_missing_operands_error() {
        let left = temp_dir("missing-left");
        let right = temp_dir("missing-right");
        let missing_left = exec(DirDiffArgs {
            left: None,
            ..args(&left, &right)
        });
        assert!(missing_left.is_err());
        assert!(missing_left.unwrap_err().to_string().contains("missing <LEFT>"));
        let missing_right = exec(DirDiffArgs {
            right: None,
            ..args(&left, &right)
        });
        assert!(missing_right.is_err());
        assert!(missing_right.unwrap_err().to_string().contains("missing <RIGHT>"));
        cleanup(&left, &right);
    }

    #[test]
    fn exec_missing_directory_errors() {
        let left = temp_dir("absent-left");
        let right = temp_dir("absent-right");
        let result = exec(DirDiffArgs {
            left: Some(left.join("no-such-dir").to_string_lossy().into_owned()),
            ..args(&left, &right)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
        cleanup(&left, &right);
    }

    #[test]
    fn exec_root_is_file_errors() {
        let left = temp_dir("file-root-left");
        let right = temp_dir("file-root-right");
        let file = left.join("plain.txt");
        fs::write(&file, "not a directory").expect("write file");
        let result = exec(DirDiffArgs {
            left: Some(file.to_string_lossy().into_owned()),
            ..args(&left, &right)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not a directory"));
        cleanup(&left, &right);
    }

    #[test]
    fn exec_check_identical_succeeds() {
        let left = temp_dir("check-equal-left");
        let right = temp_dir("check-equal-right");
        write_file(&left, "same.txt", "same");
        write_file(&right, "same.txt", "same");
        let result = exec(DirDiffArgs {
            check: true,
            ..args(&left, &right)
        });
        assert!(result.is_ok());
        cleanup(&left, &right);
    }

    #[test]
    fn exec_check_differ_fails_silently() {
        let left = temp_dir("check-diff-left");
        let right = temp_dir("check-diff-right");
        write_file(&left, "left.txt", "old");
        let result = exec(DirDiffArgs {
            check: true,
            ..args(&left, &right)
        });
        let error = result.unwrap_err();
        assert!(matches!(error, DiffError::CheckFailed));
        assert!(error.source().is_none());
        cleanup(&left, &right);
    }

    #[test]
    fn exec_check_takes_precedence_over_format() {
        let left = temp_dir("check-format-left");
        let right = temp_dir("check-format-right");
        write_file(&right, "extra.txt", "new");
        let result = exec(DirDiffArgs {
            check: true,
            format: Some(DirFormatCli::Json),
            ..args(&left, &right)
        });
        assert!(matches!(result.unwrap_err(), DiffError::CheckFailed));
        cleanup(&left, &right);
    }
}

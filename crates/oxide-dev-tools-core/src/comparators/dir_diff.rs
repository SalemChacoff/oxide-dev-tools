//! Directory tree comparison.
//!
//! Walks two directory trees and reports where they differ, joined on
//! relative paths: entries present in only one tree ([`DirChangeKind::Added`]
//! / [`DirChangeKind::Removed`]), entries whose kind changed
//! ([`DirChangeKind::TypeChanged`]), files whose size or content changed
//! ([`DirChangeKind::Modified`]), and symlinks whose target changed
//! ([`DirChangeKind::LinkTargetDiffers`]).
//!
//! By default the comparison is structural: file sizes decide equality and
//! symlinks compare by target, without following them. With
//! [`DirDiffOptions::compare_contents`] files are additionally hashed with
//! SHA-256, catching same-size content changes. With
//! [`DirDiffOptions::follow_symlinks`] symlinked directories are descended
//! (loop-safe) instead of compared as leaves.
//! [`DirDiffOptions::ignore_globs`] prunes relative paths matching a `*`
//! wildcard pattern, where `*` matches any sequence of characters,
//! including path separators.
//!
//! [`compare_dirs`] returns a [`DirDiffReport`] that renders either as a
//! line report ([`DirDiffReport::to_text`]) or as a JSON array
//! ([`DirDiffReport::to_json`]).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

// -------- Public API --------

/// Buffer size for hashing file contents.
const HASH_BUFFER_BYTES: usize = 8192;

/// Which directory an error refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirSide {
    /// The first directory to compare.
    Left,
    /// The second directory to compare.
    Right,
}

impl DirSide {
    fn label(self) -> &'static str {
        match self {
            DirSide::Left => "left",
            DirSide::Right => "right",
        }
    }
}

/// Errors that can occur when comparing directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirDiffError {
    /// A root path does not exist.
    RootMissing {
        /// Which root is missing.
        side: DirSide,
        /// The path as it was given.
        path: String,
    },
    /// A root path exists but is not a directory.
    NotADirectory {
        /// Which root is not a directory.
        side: DirSide,
        /// The path as it was given.
        path: String,
    },
    /// An entry could not be read while walking a tree.
    Io {
        /// Which tree the entry belongs to.
        side: DirSide,
        /// The entry path that failed.
        path: String,
        /// The underlying I/O error message.
        message: String,
    },
}

impl fmt::Display for DirDiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DirDiffError::RootMissing { side, path } => {
                write!(f, "{} root directory \"{path}\" does not exist", side.label())
            }
            DirDiffError::NotADirectory { side, path } => {
                write!(f, "{} root \"{path}\" is not a directory", side.label())
            }
            DirDiffError::Io { side, path, message } => {
                write!(f, "cannot read \"{path}\" on the {} side: {message}", side.label())
            }
        }
    }
}

impl std::error::Error for DirDiffError {}

/// What kind of filesystem entry a path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirEntryKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link, compared as a leaf unless links are followed.
    Symlink,
}

impl DirEntryKind {
    fn label(self) -> &'static str {
        match self {
            DirEntryKind::File => "file",
            DirEntryKind::Directory => "directory",
            DirEntryKind::Symlink => "symlink",
        }
    }
}

/// How the trees differ at a single relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirChangeKind {
    /// The entry exists only in the right directory.
    Added,
    /// The entry exists only in the left directory.
    Removed,
    /// A file exists in both directories but differs in size or content.
    Modified {
        /// Size of the left file in bytes.
        left_size: u64,
        /// Size of the right file in bytes.
        right_size: u64,
        /// SHA-256 hash of the left file, present in content mode.
        left_hash: Option<String>,
        /// SHA-256 hash of the right file, present in content mode.
        right_hash: Option<String>,
    },
    /// A symlink exists in both directories but points elsewhere.
    LinkTargetDiffers {
        /// Where the left link points.
        left_target: String,
        /// Where the right link points.
        right_target: String,
    },
    /// The same relative path is a different kind of entry on each side.
    TypeChanged {
        /// Kind of the entry in the left directory.
        left: DirEntryKind,
        /// Kind of the entry in the right directory.
        right: DirEntryKind,
    },
}

/// One detected difference between the two directory trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntryDiff {
    /// Path relative to the compared roots, with `/` separators.
    pub relative_path: String,
    /// How the trees differ at this path.
    pub kind: DirChangeKind,
}

/// The outcome of [`compare_dirs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirDiffReport {
    /// Every detected difference, sorted by relative path.
    pub differences: Vec<DirEntryDiff>,
}

impl DirDiffReport {
    /// Whether the two directories are equal under the comparison options.
    pub fn is_equal(&self) -> bool {
        self.differences.is_empty()
    }

    /// Render the report as one line per difference:
    ///
    /// ```text
    /// + config/extra.toml
    /// - cache/old.tmp
    /// ~ build/app.bin: 1024 bytes -> 2048 bytes
    /// ~ build/app.bin: content differs (1024 bytes)
    /// ~ lib/current: link target "v1" -> "v2"
    /// ~ assets/logo: type changed (file -> directory)
    /// ```
    pub fn to_text(&self) -> String {
        let mut lines = Vec::with_capacity(self.differences.len());
        for difference in &self.differences {
            let line = match &difference.kind {
                DirChangeKind::Added => format!("+ {}", difference.relative_path),
                DirChangeKind::Removed => format!("- {}", difference.relative_path),
                DirChangeKind::Modified {
                    left_size,
                    right_size,
                    left_hash,
                    right_hash,
                } => {
                    if left_size == right_size && (left_hash.is_some() || right_hash.is_some()) {
                        format!("~ {}: content differs ({left_size} bytes)", difference.relative_path)
                    } else {
                        format!("~ {}: {left_size} bytes -> {right_size} bytes", difference.relative_path)
                    }
                }
                DirChangeKind::LinkTargetDiffers {
                    left_target,
                    right_target,
                } => {
                    format!("~ {}: link target \"{left_target}\" -> \"{right_target}\"", difference.relative_path)
                }
                DirChangeKind::TypeChanged { left, right } => {
                    format!("~ {}: type changed ({} -> {})", difference.relative_path, left.label(), right.label())
                }
            };
            lines.push(line);
        }
        lines.join("\n")
    }

    /// Render the report as a pretty-printed JSON array of difference
    /// objects:
    ///
    /// ```text
    /// [
    ///   {
    ///     "path": "build/app.bin",
    ///     "kind": "modified",
    ///     "left_size": 1024,
    ///     "right_size": 2048
    ///   }
    /// ]
    /// ```
    ///
    /// Every object carries `path` and a `kind` of `added`, `removed`,
    /// `modified`, `link-target`, or `type-changed`, plus kind-specific
    /// fields (`left_size`/`right_size`/`left_hash`/`right_hash`,
    /// `left_target`/`right_target`, or `left_kind`/`right_kind`).
    pub fn to_json(&self) -> String {
        let mut objects = Vec::with_capacity(self.differences.len());
        for difference in &self.differences {
            let mut object = Map::new();
            object.insert("path".to_string(), Value::String(difference.relative_path.clone()));
            match &difference.kind {
                DirChangeKind::Added => {
                    object.insert("kind".to_string(), Value::String("added".to_string()));
                }
                DirChangeKind::Removed => {
                    object.insert("kind".to_string(), Value::String("removed".to_string()));
                }
                DirChangeKind::Modified {
                    left_size,
                    right_size,
                    left_hash,
                    right_hash,
                } => {
                    object.insert("kind".to_string(), Value::String("modified".to_string()));
                    object.insert("left_size".to_string(), Value::from(*left_size));
                    object.insert("right_size".to_string(), Value::from(*right_size));
                    if let (Some(left_hash), Some(right_hash)) = (left_hash, right_hash) {
                        object.insert("left_hash".to_string(), Value::String(left_hash.clone()));
                        object.insert("right_hash".to_string(), Value::String(right_hash.clone()));
                    }
                }
                DirChangeKind::LinkTargetDiffers {
                    left_target,
                    right_target,
                } => {
                    object.insert("kind".to_string(), Value::String("link-target".to_string()));
                    object.insert("left_target".to_string(), Value::String(left_target.clone()));
                    object.insert("right_target".to_string(), Value::String(right_target.clone()));
                }
                DirChangeKind::TypeChanged { left, right } => {
                    object.insert("kind".to_string(), Value::String("type-changed".to_string()));
                    object.insert("left_kind".to_string(), Value::String(left.label().to_string()));
                    object.insert("right_kind".to_string(), Value::String(right.label().to_string()));
                }
            }
            objects.push(Value::Object(object));
        }
        match serde_json::to_string_pretty(&Value::Array(objects)) {
            Ok(json) => json,
            Err(_) => String::from("[]"),
        }
    }
}

/// Options for [`compare_dirs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirDiffOptions {
    /// First directory to compare.
    pub left_root: PathBuf,
    /// Second directory to compare.
    pub right_root: PathBuf,
    /// Hash file contents with SHA-256 instead of comparing sizes only.
    pub compare_contents: bool,
    /// Descend into symlinked directories instead of comparing links as
    /// leaves. Symlink cycles are detected and skipped.
    pub follow_symlinks: bool,
    /// Relative paths skipped while walking. Each pattern is a `*`
    /// wildcard, where `*` matches any sequence of characters, including
    /// path separators.
    pub ignore_globs: Vec<String>,
}

impl Default for DirDiffOptions {
    fn default() -> Self {
        Self {
            left_root: PathBuf::new(),
            right_root: PathBuf::new(),
            compare_contents: false,
            follow_symlinks: false,
            ignore_globs: Vec::new(),
        }
    }
}

/// Compare two directory trees and report every difference.
pub fn compare_dirs(options: DirDiffOptions) -> Result<DirDiffReport, DirDiffError> {
    validate(&options)?;
    let mut left = BTreeMap::new();
    let mut right = BTreeMap::new();
    {
        let mut visited = HashSet::new();
        let mut context = WalkContext {
            root: &options.left_root,
            side: DirSide::Left,
            compare_contents: options.compare_contents,
            follow_links: options.follow_symlinks,
            ignore_globs: &options.ignore_globs,
            visited: &mut visited,
        };
        walk_dir(&mut context, Path::new(""), &mut left)?;
    }
    {
        let mut visited = HashSet::new();
        let mut context = WalkContext {
            root: &options.right_root,
            side: DirSide::Right,
            compare_contents: options.compare_contents,
            follow_links: options.follow_symlinks,
            ignore_globs: &options.ignore_globs,
            visited: &mut visited,
        };
        walk_dir(&mut context, Path::new(""), &mut right)?;
    }
    Ok(DirDiffReport {
        differences: compare_trees(&left, &right),
    })
}

// -------- Validation --------

fn validate(options: &DirDiffOptions) -> Result<(), DirDiffError> {
    check_root(&options.left_root, DirSide::Left)?;
    check_root(&options.right_root, DirSide::Right)?;
    Ok(())
}

fn check_root(root: &Path, side: DirSide) -> Result<(), DirDiffError> {
    let path = root.to_string_lossy().into_owned();
    let metadata = std::fs::metadata(root).map_err(|_| DirDiffError::RootMissing {
        side,
        path: path.clone(),
    })?;
    if !metadata.is_dir() {
        return Err(DirDiffError::NotADirectory { side, path });
    }
    Ok(())
}

// -------- Walking --------

/// Everything known about one entry during the comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EntryInfo {
    kind: DirEntryKind,
    size: u64,
    hash: Option<String>,
    link_target: Option<String>,
}

/// Shared state for one side's walk.
struct WalkContext<'a> {
    root: &'a Path,
    side: DirSide,
    compare_contents: bool,
    follow_links: bool,
    ignore_globs: &'a [String],
    visited: &'a mut HashSet<PathBuf>,
}

fn walk_dir(
    context: &mut WalkContext<'_>,
    base: &Path,
    entries: &mut BTreeMap<String, EntryInfo>,
) -> Result<(), DirDiffError> {
    let directory = if base.as_os_str().is_empty() {
        context.root.to_path_buf()
    } else {
        context.root.join(base)
    };
    let mut listing: Vec<_> = std::fs::read_dir(&directory)
        .map_err(|error| io_error(context.side, &directory, error))?
        .collect::<Result<_, _>>()
        .map_err(|error| io_error(context.side, &directory, error))?;
    listing.sort_by_key(|entry| entry.file_name());
    for entry in listing {
        let relative = if base.as_os_str().is_empty() {
            PathBuf::from(entry.file_name())
        } else {
            base.join(entry.file_name())
        };
        let relative_text = relative.to_string_lossy().replace('\\', "/");
        if context
            .ignore_globs
            .iter()
            .any(|pattern| glob_matches(pattern, &relative_text))
        {
            continue;
        }
        let full_path = context.root.join(&relative);
        walk_entry(context, &relative, &relative_text, &full_path, entries)?;
    }
    Ok(())
}

fn walk_entry(
    context: &mut WalkContext<'_>,
    relative: &Path,
    relative_text: &str,
    full_path: &Path,
    entries: &mut BTreeMap<String, EntryInfo>,
) -> Result<(), DirDiffError> {
    let link_metadata =
        std::fs::symlink_metadata(full_path).map_err(|error| io_error(context.side, full_path, error))?;
    let file_type = link_metadata.file_type();
    if file_type.is_symlink() && !context.follow_links {
        let target = std::fs::read_link(full_path).map_err(|error| io_error(context.side, full_path, error))?;
        entries.insert(
            relative_text.to_string(),
            EntryInfo {
                kind: DirEntryKind::Symlink,
                size: link_metadata.len(),
                hash: None,
                link_target: Some(target.to_string_lossy().into_owned()),
            },
        );
        return Ok(());
    }
    let metadata = std::fs::metadata(full_path).map_err(|error| io_error(context.side, full_path, error))?;
    if metadata.is_dir() {
        if context.follow_links && file_type.is_symlink() {
            let canonical =
                std::fs::canonicalize(full_path).map_err(|error| io_error(context.side, full_path, error))?;
            if !context.visited.insert(canonical) {
                return Ok(());
            }
        }
        entries.insert(
            relative_text.to_string(),
            EntryInfo {
                kind: DirEntryKind::Directory,
                size: 0,
                hash: None,
                link_target: None,
            },
        );
        walk_dir(context, relative, entries)?;
        return Ok(());
    }
    let hash = if context.compare_contents {
        Some(hash_file(full_path, context.side)?)
    } else {
        None
    };
    entries.insert(
        relative_text.to_string(),
        EntryInfo {
            kind: DirEntryKind::File,
            size: metadata.len(),
            hash,
            link_target: None,
        },
    );
    Ok(())
}

fn hash_file(path: &Path, side: DirSide) -> Result<String, DirDiffError> {
    let mut file = std::fs::File::open(path).map_err(|error| io_error(side, path, error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; HASH_BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(|error| io_error(side, path, error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn io_error<T: fmt::Display>(side: DirSide, path: &Path, error: T) -> DirDiffError {
    DirDiffError::Io {
        side,
        path: path.to_string_lossy().into_owned(),
        message: error.to_string(),
    }
}

// -------- Glob matching --------

/// Whether `pattern` (a `*` wildcard) matches the whole `text`.
fn glob_matches(pattern: &str, text: &str) -> bool {
    glob_matches_at(&pattern.chars().collect::<Vec<_>>(), &text.chars().collect::<Vec<_>>())
}

fn glob_matches_at(pattern: &[char], text: &[char]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    match pattern[0] {
        '*' => {
            if !text.is_empty() && glob_matches_at(pattern, &text[1..]) {
                return true;
            }
            glob_matches_at(&pattern[1..], text)
        }
        character => !text.is_empty() && character == text[0] && glob_matches_at(&pattern[1..], &text[1..]),
    }
}

// -------- Tree comparison --------

fn compare_trees(left: &BTreeMap<String, EntryInfo>, right: &BTreeMap<String, EntryInfo>) -> Vec<DirEntryDiff> {
    let paths: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
    let mut differences = Vec::new();
    for path in paths {
        match (left.get(path), right.get(path)) {
            (None, Some(_)) => differences.push(DirEntryDiff {
                relative_path: path.clone(),
                kind: DirChangeKind::Added,
            }),
            (Some(_), None) => differences.push(DirEntryDiff {
                relative_path: path.clone(),
                kind: DirChangeKind::Removed,
            }),
            (Some(left_info), Some(right_info)) => {
                compare_entries(path, left_info, right_info, &mut differences);
            }
            (None, None) => {}
        }
    }
    differences
}

fn compare_entries(path: &str, left: &EntryInfo, right: &EntryInfo, differences: &mut Vec<DirEntryDiff>) {
    if left.kind != right.kind {
        differences.push(DirEntryDiff {
            relative_path: path.to_string(),
            kind: DirChangeKind::TypeChanged {
                left: left.kind,
                right: right.kind,
            },
        });
        return;
    }
    match left.kind {
        DirEntryKind::File => {
            let content_differs = match (&left.hash, &right.hash) {
                (Some(left_hash), Some(right_hash)) => left_hash != right_hash,
                _ => false,
            };
            if content_differs || left.size != right.size {
                differences.push(DirEntryDiff {
                    relative_path: path.to_string(),
                    kind: DirChangeKind::Modified {
                        left_size: left.size,
                        right_size: right.size,
                        left_hash: left.hash.clone(),
                        right_hash: right.hash.clone(),
                    },
                });
            }
        }
        DirEntryKind::Symlink => {
            if left.link_target != right.link_target {
                let (Some(left_target), Some(right_target)) = (&left.link_target, &right.link_target) else {
                    return;
                };
                differences.push(DirEntryDiff {
                    relative_path: path.to_string(),
                    kind: DirChangeKind::LinkTargetDiffers {
                        left_target: left_target.clone(),
                        right_target: right_target.clone(),
                    },
                });
            }
        }
        DirEntryKind::Directory => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a unique scratch directory under the system temp dir.
    fn temp_root(tag: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("oxide-dir-diff-{tag}-{nonce}"));
        fs::create_dir_all(&path).expect("create fixture root");
        path
    }

    /// Create a file under `root` with the given relative path and content.
    fn write_file(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("file has parent")).expect("create fixture dir");
        fs::write(path, content).expect("write fixture file");
    }

    fn options(left: &Path, right: &Path) -> DirDiffOptions {
        DirDiffOptions {
            left_root: left.to_path_buf(),
            right_root: right.to_path_buf(),
            ..DirDiffOptions::default()
        }
    }

    fn cleanup(left: &Path, right: &Path) {
        fs::remove_dir_all(left).ok();
        fs::remove_dir_all(right).ok();
    }

    #[test]
    fn identical_nested_trees_report_no_differences() {
        let left = temp_root("identical-left");
        let right = temp_root("identical-right");
        for relative in ["src/main.rs", "src/lib/mod.rs", "README.md"] {
            write_file(&left, relative, "same content");
            write_file(&right, relative, "same content");
        }
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert!(report.is_equal());
        assert!(report.differences.is_empty());
        cleanup(&left, &right);
    }

    #[test]
    fn empty_directories_are_equal() {
        let left = temp_root("empty-left");
        let right = temp_root("empty-right");
        assert!(compare_dirs(options(&left, &right)).expect("compare").is_equal());
        cleanup(&left, &right);
    }

    #[test]
    fn added_and_removed_entries_are_reported() {
        let left = temp_root("addremove-left");
        let right = temp_root("addremove-right");
        write_file(&left, "gone.txt", "old");
        write_file(&right, "fresh.txt", "new");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(report.differences.len(), 2);
        assert_eq!(report.differences[0].relative_path, "fresh.txt");
        assert_eq!(report.differences[0].kind, DirChangeKind::Added);
        assert_eq!(report.differences[1].relative_path, "gone.txt");
        assert_eq!(report.differences[1].kind, DirChangeKind::Removed);
        cleanup(&left, &right);
    }

    #[test]
    fn new_directory_reports_the_directory_and_its_contents() {
        let left = temp_root("newdir-left");
        let right = temp_root("newdir-right");
        write_file(&right, "newdir/inner.txt", "content");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(report.differences.len(), 2);
        assert_eq!(report.differences[0].relative_path, "newdir");
        assert_eq!(report.differences[0].kind, DirChangeKind::Added);
        assert_eq!(report.differences[1].relative_path, "newdir/inner.txt");
        assert_eq!(report.differences[1].kind, DirChangeKind::Added);
        cleanup(&left, &right);
    }

    #[test]
    fn size_changes_are_reported_structurally() {
        let left = temp_root("size-left");
        let right = temp_root("size-right");
        write_file(&left, "app.bin", "a");
        write_file(&right, "app.bin", "much longer");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(report.differences.len(), 1);
        assert_eq!(
            report.differences[0].kind,
            DirChangeKind::Modified {
                left_size: 1,
                right_size: 11,
                left_hash: None,
                right_hash: None,
            }
        );
        cleanup(&left, &right);
    }

    #[test]
    fn same_size_changes_need_content_mode() {
        let left = temp_root("deep-left");
        let right = temp_root("deep-right");
        write_file(&left, "data.bin", "same size");
        write_file(&right, "data.bin", "SAME SIZE");
        assert!(compare_dirs(options(&left, &right)).expect("compare").is_equal());
        let report = compare_dirs(DirDiffOptions {
            compare_contents: true,
            ..options(&left, &right)
        })
        .expect("compare");
        assert_eq!(report.differences.len(), 1);
        match &report.differences[0].kind {
            DirChangeKind::Modified {
                left_size,
                right_size,
                left_hash,
                right_hash,
            } => {
                assert_eq!(*left_size, 9);
                assert_eq!(*right_size, 9);
                assert_ne!(left_hash, right_hash);
            }
            other => panic!("expected modified, got {other:?}"),
        }
        cleanup(&left, &right);
    }

    #[test]
    fn content_mode_accepts_identical_content() {
        let left = temp_root("deep-equal-left");
        let right = temp_root("deep-equal-right");
        for index in 0..10 {
            let relative = format!("dir{}/file{index}.bin", index % 3);
            write_file(&left, &relative, "binary-ish content");
            write_file(&right, &relative, "binary-ish content");
        }
        let report = compare_dirs(DirDiffOptions {
            compare_contents: true,
            ..options(&left, &right)
        })
        .expect("compare");
        assert!(report.is_equal());
        cleanup(&left, &right);
    }

    #[test]
    fn type_changes_are_reported() {
        let left = temp_root("type-left");
        let right = temp_root("type-right");
        write_file(&left, "node", "a file");
        fs::create_dir_all(right.join("node")).expect("create dir");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(report.differences.len(), 1);
        assert_eq!(
            report.differences[0].kind,
            DirChangeKind::TypeChanged {
                left: DirEntryKind::File,
                right: DirEntryKind::Directory,
            }
        );
        cleanup(&left, &right);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_targets_are_compared_as_leaves() {
        let left = temp_root("link-left");
        let right = temp_root("link-right");
        write_file(&left, "target.txt", "content");
        write_file(&right, "target.txt", "content");
        std::os::unix::fs::symlink("target.txt", left.join("current")).expect("create link");
        std::os::unix::fs::symlink("other.txt", right.join("current")).expect("create link");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(report.differences.len(), 1);
        assert_eq!(
            report.differences[0].kind,
            DirChangeKind::LinkTargetDiffers {
                left_target: "target.txt".to_string(),
                right_target: "other.txt".to_string(),
            }
        );
        cleanup(&left, &right);
    }

    #[cfg(unix)]
    #[test]
    fn identical_symlink_targets_are_equal() {
        let left = temp_root("same-link-left");
        let right = temp_root("same-link-right");
        write_file(&left, "target.txt", "content");
        write_file(&right, "target.txt", "content");
        std::os::unix::fs::symlink("target.txt", left.join("current")).expect("create link");
        std::os::unix::fs::symlink("target.txt", right.join("current")).expect("create link");
        assert!(compare_dirs(options(&left, &right)).expect("compare").is_equal());
        cleanup(&left, &right);
    }

    #[cfg(unix)]
    #[test]
    fn following_symlinks_descends_directories() {
        let left = temp_root("follow-left");
        let right = temp_root("follow-right");
        write_file(&left, "real/inside.txt", "content");
        std::os::unix::fs::symlink("real", left.join("alias")).expect("create link");
        write_file(&right, "real/inside.txt", "content");
        write_file(&right, "alias/inside.txt", "content");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(
            report.differences[0].kind,
            DirChangeKind::TypeChanged {
                left: DirEntryKind::Symlink,
                right: DirEntryKind::Directory,
            }
        );
        let report = compare_dirs(DirDiffOptions {
            follow_symlinks: true,
            ..options(&left, &right)
        })
        .expect("compare");
        assert!(report.is_equal());
        cleanup(&left, &right);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycles_terminate() {
        let left = temp_root("cycle-left");
        let right = temp_root("cycle-right");
        for root in [&left, &right] {
            fs::create_dir_all(root.join("a")).expect("create dir");
            std::os::unix::fs::symlink(root, root.join("a/back")).expect("create link");
        }
        let report = compare_dirs(DirDiffOptions {
            follow_symlinks: true,
            ..options(&left, &right)
        })
        .expect("compare");
        assert!(report.is_equal());
        cleanup(&left, &right);
    }

    #[test]
    fn ignore_patterns_prune_paths() {
        let left = temp_root("ignore-left");
        let right = temp_root("ignore-right");
        write_file(&left, "keep.txt", "same");
        write_file(&right, "keep.txt", "same");
        write_file(&left, "cache.tmp", "left temp");
        write_file(&right, "cache.tmp", "right temp");
        let report = compare_dirs(DirDiffOptions {
            ignore_globs: vec!["*.tmp".to_string()],
            ..options(&left, &right)
        })
        .expect("compare");
        assert!(report.is_equal());
        cleanup(&left, &right);
    }

    #[test]
    fn ignore_star_crosses_path_separators() {
        let left = temp_root("ignore-deep-left");
        let right = temp_root("ignore-deep-right");
        write_file(&left, "target/sub/x.txt", "left");
        write_file(&right, "target/sub/y.txt", "right");
        let report = compare_dirs(DirDiffOptions {
            ignore_globs: vec!["target/*".to_string()],
            ..options(&left, &right)
        })
        .expect("compare");
        assert!(report.is_equal());
        cleanup(&left, &right);
    }

    #[test]
    fn glob_exact_match() {
        assert!(glob_matches("a.txt", "a.txt"));
        assert!(!glob_matches("a.txt", "b.txt"));
    }

    #[test]
    fn glob_star_suffix() {
        assert!(glob_matches("*.tmp", "cache.tmp"));
        assert!(glob_matches("*.tmp", "deep/cache.tmp"));
        assert!(!glob_matches("*.tmp", "cache.tmp.bak"));
    }

    #[test]
    fn glob_star_prefix_and_middle() {
        assert!(glob_matches("log/*.log", "log/app.log"));
        assert!(glob_matches("a*c", "abc"));
        assert!(!glob_matches("a*c", "axb"));
    }

    #[test]
    fn glob_star_matches_empty() {
        assert!(glob_matches("file*", "file"));
    }

    #[test]
    fn glob_unicode() {
        assert!(glob_matches("*.rs", "lib.rs"));
        assert!(glob_matches("na*e", "naïve"));
    }

    #[test]
    fn glob_empty_pattern_matches_only_empty_text() {
        assert!(glob_matches("", ""));
        assert!(!glob_matches("", "x"));
    }

    #[test]
    fn missing_roots_error_per_side() {
        let left = temp_root("missing-left");
        let right = temp_root("missing-right");
        let nonexistent = left.join("no-such-dir");
        let err = compare_dirs(DirDiffOptions {
            left_root: nonexistent.clone(),
            ..options(&left, &right)
        })
        .expect_err("missing root must fail");
        assert!(err.to_string().contains("left root directory"));
        let err = compare_dirs(DirDiffOptions {
            right_root: nonexistent,
            ..options(&left, &right)
        })
        .expect_err("missing root must fail");
        assert!(err.to_string().contains("right root directory"));
        cleanup(&left, &right);
    }

    #[test]
    fn file_roots_error() {
        let left = temp_root("file-root-left");
        let right = temp_root("file-root-right");
        let file = left.join("plain.txt");
        fs::write(&file, "not a directory").expect("write file");
        let err = compare_dirs(DirDiffOptions {
            left_root: file,
            ..options(&left, &right)
        })
        .expect_err("file root must fail");
        assert!(err.to_string().contains("is not a directory"));
        cleanup(&left, &right);
    }

    #[test]
    fn differences_follow_path_order() {
        let left = temp_root("order-left");
        let right = temp_root("order-right");
        write_file(&left, "z.txt", "old");
        write_file(&right, "a.txt", "new");
        write_file(&right, "m.txt", "new");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(report.to_text(), "+ a.txt\n+ m.txt\n- z.txt");
        cleanup(&left, &right);
    }

    #[test]
    fn report_json_renders_differences() {
        let left = temp_root("json-left");
        let right = temp_root("json-right");
        write_file(&left, "sub/a.txt", "one");
        write_file(&right, "sub/a.txt", "two!");
        write_file(&right, "extra.txt", "x");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        let parsed: Value = serde_json::from_str(&report.to_json()).expect("valid JSON");
        let array = parsed.as_array().expect("array");
        assert_eq!(array.len(), 2);
        assert_eq!(array[0]["path"], "extra.txt");
        assert_eq!(array[0]["kind"], "added");
        assert_eq!(array[1]["path"], "sub/a.txt");
        assert_eq!(array[1]["kind"], "modified");
        assert_eq!(array[1]["left_size"], 3);
        assert_eq!(array[1]["right_size"], 4);
        cleanup(&left, &right);
    }

    #[test]
    fn report_text_renders_prefixed_lines() {
        let left = temp_root("text-left");
        let right = temp_root("text-right");
        write_file(&left, "change.txt", "one");
        write_file(&right, "change.txt", "two!!");
        write_file(&right, "new.txt", "fresh");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        let text = report.to_text();
        assert!(text.contains("~ change.txt: 3 bytes -> 5 bytes"));
        assert!(text.contains("+ new.txt"));
        cleanup(&left, &right);
    }

    #[test]
    fn unicode_paths_are_reported() {
        let left = temp_root("unicode-left");
        let right = temp_root("unicode-right");
        write_file(&right, "über cool/naïve.txt", "content");
        let report = compare_dirs(options(&left, &right)).expect("compare");
        assert_eq!(report.differences.len(), 2);
        assert_eq!(report.differences[0].relative_path, "über cool");
        assert_eq!(report.differences[1].relative_path, "über cool/naïve.txt");
        cleanup(&left, &right);
    }

    #[test]
    fn large_trees_stay_accurate() {
        let left = temp_root("scale-left");
        let right = temp_root("scale-right");
        for index in 0..300 {
            let relative = format!("dir{}/file{index}.txt", index % 10);
            write_file(&left, &relative, "content");
            write_file(&right, &relative, "content");
        }
        write_file(&right, "extra/only-here.txt", "extra");
        let report = compare_dirs(DirDiffOptions {
            compare_contents: true,
            ..options(&left, &right)
        })
        .expect("compare");
        assert_eq!(report.differences.len(), 2);
        cleanup(&left, &right);
    }

    #[test]
    fn options_defaults_are_sane() {
        let default = DirDiffOptions::default();
        assert_eq!(default.left_root, PathBuf::new());
        assert_eq!(default.right_root, PathBuf::new());
        assert!(!default.compare_contents);
        assert!(!default.follow_symlinks);
        assert!(default.ignore_globs.is_empty());
    }
}

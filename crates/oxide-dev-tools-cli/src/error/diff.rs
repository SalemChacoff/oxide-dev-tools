use std::fmt;

use oxide_dev_tools_core::{JsonDiffError, TextDiffError};

use super::GenericError;

/// Errors from `oxide diff` tools.
#[derive(Debug)]
pub enum DiffError {
    Text(TextDiffError),
    Json(JsonDiffError),
    /// A `--check` comparison found differences; the CLI exits nonzero
    /// without printing a message (mirrors `git diff --quiet`).
    CheckFailed,
    /// An argument or I/O failure shared with the other categories.
    Generic(GenericError),
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::Text(e) => write!(f, "{e}"),
            DiffError::Json(e) => write!(f, "{e}"),
            DiffError::CheckFailed => write!(f, "documents differ"),
            DiffError::Generic(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DiffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DiffError::Text(e) => Some(e),
            DiffError::Json(e) => Some(e),
            DiffError::CheckFailed => None,
            DiffError::Generic(e) => Some(e),
        }
    }
}

impl From<TextDiffError> for DiffError {
    fn from(e: TextDiffError) -> Self {
        DiffError::Text(e)
    }
}

impl From<JsonDiffError> for DiffError {
    fn from(e: JsonDiffError) -> Self {
        DiffError::Json(e)
    }
}

impl From<GenericError> for DiffError {
    fn from(e: GenericError) -> Self {
        DiffError::Generic(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn from_core_error_is_transparent() {
        let err = DiffError::from(TextDiffError::InputTooLong { limit: 4 });
        assert_eq!(err.to_string(), "character diff inputs must be at most 4 characters");
        assert!(err.source().is_some());
    }

    #[test]
    fn from_json_error_is_transparent() {
        let err = DiffError::from(JsonDiffError::InputTooLong {
            side: oxide_dev_tools_core::JsonSide::Right,
            limit: 4,
        });
        assert_eq!(err.to_string(), "right JSON input must be at most 4 characters");
        assert!(err.source().is_some());
    }

    #[test]
    fn check_failed_has_no_source() {
        let err = DiffError::CheckFailed;
        assert_eq!(err.to_string(), "documents differ");
        assert!(err.source().is_none());
    }

    #[test]
    fn wraps_generic_error() {
        let err = DiffError::from(GenericError::Argument("bad flag".into()));
        assert_eq!(err.to_string(), "bad flag");
        assert!(err.source().is_some());
    }
}

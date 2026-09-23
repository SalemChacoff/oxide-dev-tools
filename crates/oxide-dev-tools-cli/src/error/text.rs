use std::fmt;

use oxide_dev_tools_core::{CaseError, HtmlError, TruncateError, UnicodeError};

use super::GenericError;

/// Errors from `oxide text` tools.
#[derive(Debug)]
pub enum TextError {
    Case(CaseError),
    Html(HtmlError),
    Truncate(TruncateError),
    Unicode(UnicodeError),
    /// An argument or I/O failure shared with the other categories.
    Generic(GenericError),
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextError::Case(e) => write!(f, "{e}"),
            TextError::Html(e) => write!(f, "{e}"),
            TextError::Truncate(e) => write!(f, "{e}"),
            TextError::Unicode(e) => write!(f, "{e}"),
            TextError::Generic(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TextError::Case(e) => Some(e),
            TextError::Html(e) => Some(e),
            TextError::Truncate(e) => Some(e),
            TextError::Unicode(e) => Some(e),
            TextError::Generic(e) => Some(e),
        }
    }
}

impl From<CaseError> for TextError {
    fn from(e: CaseError) -> Self {
        TextError::Case(e)
    }
}

impl From<HtmlError> for TextError {
    fn from(e: HtmlError) -> Self {
        TextError::Html(e)
    }
}

impl From<TruncateError> for TextError {
    fn from(e: TruncateError) -> Self {
        TextError::Truncate(e)
    }
}

impl From<UnicodeError> for TextError {
    fn from(e: UnicodeError) -> Self {
        TextError::Unicode(e)
    }
}

impl From<GenericError> for TextError {
    fn from(e: GenericError) -> Self {
        TextError::Generic(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn from_core_error_is_transparent() {
        let err = TextError::from(CaseError::EmptyInput);
        assert_eq!(err.to_string(), "input contains no word characters");
        assert!(err.source().is_some());
    }

    #[test]
    fn from_html_error_is_transparent() {
        let err = TextError::from(HtmlError::UnknownEntity);
        assert_eq!(err.to_string(), "unknown HTML entity (supported names are case-sensitive)");
        assert!(err.source().is_some());
    }

    #[test]
    fn from_truncate_error_is_transparent() {
        let err = TextError::from(TruncateError::EllipsisTooLong);
        assert_eq!(err.to_string(), "ellipsis is longer than the maximum length");
        assert!(err.source().is_some());
    }

    #[test]
    fn from_unicode_error_is_transparent() {
        let err = TextError::from(UnicodeError::InvalidEscape);
        assert_eq!(err.to_string(), "malformed unicode escape sequence");
        assert!(err.source().is_some());
    }

    #[test]
    fn wraps_generic_error() {
        let err = TextError::from(GenericError::Argument("bad flag".into()));
        assert_eq!(err.to_string(), "bad flag");
        assert!(err.source().is_some());
    }
}

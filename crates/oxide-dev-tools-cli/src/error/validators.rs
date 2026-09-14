use std::fmt;

/// Errors from `oxide validate` tools.
#[derive(Debug)]
pub enum ValidError {
    /// The validated value is invalid; carries the joined issue list.
    InvalidEmail(String),
    /// The validated URL is invalid; carries the joined issue list.
    InvalidUrl(String),
}

impl fmt::Display for ValidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidError::InvalidEmail(msg) => write!(f, "invalid email: {msg}"),
            ValidError::InvalidUrl(msg) => write!(f, "invalid url: {msg}"),
        }
    }
}

impl std::error::Error for ValidError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn invalid_email_displays_issues() {
        let err = ValidError::InvalidEmail("missing '@' separator".to_string());
        assert_eq!(err.to_string(), "invalid email: missing '@' separator");
        assert!(err.source().is_none());
    }

    #[test]
    fn invalid_url_displays_issues() {
        let err = ValidError::InvalidUrl("missing scheme (relative URL)".to_string());
        assert_eq!(err.to_string(), "invalid url: missing scheme (relative URL)");
        assert!(err.source().is_none());
    }
}

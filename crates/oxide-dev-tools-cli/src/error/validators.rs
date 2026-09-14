use std::fmt;

/// Errors from `oxide validate` tools.
#[derive(Debug)]
pub enum ValidError {
    /// The validated email address is invalid; carries the joined issue list.
    Email(String),
    /// The validated IP address is invalid; carries the joined issue list.
    Ip(String),
    /// The validated URL is invalid; carries the joined issue list.
    Url(String),
}

impl fmt::Display for ValidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidError::Email(msg) => write!(f, "invalid email: {msg}"),
            ValidError::Ip(msg) => write!(f, "invalid ip: {msg}"),
            ValidError::Url(msg) => write!(f, "invalid url: {msg}"),
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
        let err = ValidError::Email("missing '@' separator".to_string());
        assert_eq!(err.to_string(), "invalid email: missing '@' separator");
        assert!(err.source().is_none());
    }

    #[test]
    fn invalid_url_displays_issues() {
        let err = ValidError::Url("missing scheme (relative URL)".to_string());
        assert_eq!(err.to_string(), "invalid url: missing scheme (relative URL)");
        assert!(err.source().is_none());
    }

    #[test]
    fn invalid_ip_displays_issues() {
        let err = ValidError::Ip("invalid IPv4 address syntax".to_string());
        assert_eq!(err.to_string(), "invalid ip: invalid IPv4 address syntax");
        assert!(err.source().is_none());
    }
}

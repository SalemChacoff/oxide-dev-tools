use std::fmt;

use super::GenericError;

/// Errors from `oxide validate` tools.
#[derive(Debug)]
pub enum ValidError {
    /// The validated card number is invalid; carries the joined issue list.
    Card(String),
    /// The validated document (JSON/YAML/XML) is invalid; carries the joined
    /// issue list.
    Doc(String),
    /// The validated email address is invalid; carries the joined issue list.
    Email(String),
    /// An argument or I/O failure shared with the other categories.
    Generic(GenericError),
    /// The validated IP address is invalid; carries the joined issue list.
    Ip(String),
    /// The analyzed password fails the configured requirements; carries the
    /// joined issue list.
    Password(String),
    /// The validated UUID is invalid; carries the joined issue list.
    Uuid(String),
    /// The validated URL is invalid; carries the joined issue list.
    Url(String),
}

impl fmt::Display for ValidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidError::Card(msg) => write!(f, "invalid card: {msg}"),
            ValidError::Doc(msg) => write!(f, "invalid document: {msg}"),
            ValidError::Email(msg) => write!(f, "invalid email: {msg}"),
            ValidError::Generic(e) => write!(f, "{e}"),
            ValidError::Ip(msg) => write!(f, "invalid ip: {msg}"),
            ValidError::Password(msg) => write!(f, "invalid password: {msg}"),
            ValidError::Uuid(msg) => write!(f, "invalid uuid: {msg}"),
            ValidError::Url(msg) => write!(f, "invalid url: {msg}"),
        }
    }
}

impl std::error::Error for ValidError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidError::Generic(e) => Some(e),
            ValidError::Card(_)
            | ValidError::Doc(_)
            | ValidError::Email(_)
            | ValidError::Ip(_)
            | ValidError::Password(_)
            | ValidError::Uuid(_)
            | ValidError::Url(_) => None,
        }
    }
}

impl From<GenericError> for ValidError {
    fn from(e: GenericError) -> Self {
        ValidError::Generic(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn invalid_document_displays_issues() {
        let err = ValidError::Doc("expected value at line 1 column 2".to_string());
        assert_eq!(err.to_string(), "invalid document: expected value at line 1 column 2");
        assert!(err.source().is_none());
    }

    #[test]
    fn invalid_card_displays_issues() {
        let err = ValidError::Card("fails the Luhn checksum".to_string());
        assert_eq!(err.to_string(), "invalid card: fails the Luhn checksum");
        assert!(err.source().is_none());
    }

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

    #[test]
    fn invalid_password_displays_issues() {
        let err = ValidError::Password("score 1 is below the required minimum of 3".to_string());
        assert_eq!(err.to_string(), "invalid password: score 1 is below the required minimum of 3");
        assert!(err.source().is_none());
    }

    #[test]
    fn invalid_uuid_displays_issues() {
        let err = ValidError::Uuid("invalid UUID syntax".to_string());
        assert_eq!(err.to_string(), "invalid uuid: invalid UUID syntax");
        assert!(err.source().is_none());
    }

    #[test]
    fn wraps_generic_error() {
        let err = ValidError::from(GenericError::Io("read failed".into()));
        assert_eq!(err.to_string(), "read failed");
        assert!(err.source().is_some());
    }
}

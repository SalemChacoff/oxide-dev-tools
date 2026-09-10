use std::fmt;

/// Errors shared by every tool category: invalid CLI arguments and I/O failures.
///
/// Tool-specific errors live in their category enum (e.g. [`super::CodecError`]);
/// helpers that parse arguments or touch the filesystem return this leaf type,
/// which every category error can wrap via its `Generic` variant.
#[derive(Debug)]
pub enum GenericError {
    /// An I/O operation failed (reading input or writing output).
    Io(String),
    /// A CLI argument is missing or invalid.
    Argument(String),
}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenericError::Io(msg) | GenericError::Argument(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for GenericError {}

impl From<String> for GenericError {
    fn from(msg: String) -> Self {
        GenericError::Argument(msg)
    }
}

impl From<&str> for GenericError {
    fn from(msg: &str) -> Self {
        GenericError::Argument(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn io_displays_message() {
        assert_eq!(GenericError::Io("disk full".into()).to_string(), "disk full");
    }

    #[test]
    fn argument_displays_message() {
        assert_eq!(GenericError::Argument("bad flag".into()).to_string(), "bad flag");
    }

    #[test]
    fn from_str_maps_to_argument() {
        let err: GenericError = "missing value".into();
        assert!(matches!(err, GenericError::Argument(_)));
        assert_eq!(err.to_string(), "missing value");
    }

    #[test]
    fn from_string_maps_to_argument() {
        let err = GenericError::from("missing value".to_string());
        assert!(matches!(err, GenericError::Argument(_)));
    }

    #[test]
    fn generic_error_has_no_source() {
        assert!(GenericError::Io("disk full".into()).source().is_none());
    }
}

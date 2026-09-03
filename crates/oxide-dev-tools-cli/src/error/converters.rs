use std::fmt;

use oxide_dev_tools_core::{DocError, TimestampError, UnitError};

use super::GenericError;

/// Errors from `oxide convert` tools.
#[derive(Debug)]
pub enum ConvertError {
    Doc(DocError),
    Timestamp(TimestampError),
    Unit(UnitError),
    /// An argument or I/O failure shared with the other categories.
    Generic(GenericError),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::Doc(e) => write!(f, "{e}"),
            ConvertError::Timestamp(e) => write!(f, "{e}"),
            ConvertError::Unit(e) => write!(f, "{e}"),
            ConvertError::Generic(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConvertError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConvertError::Doc(e) => Some(e),
            ConvertError::Timestamp(e) => Some(e),
            ConvertError::Unit(e) => Some(e),
            ConvertError::Generic(e) => Some(e),
        }
    }
}

impl From<DocError> for ConvertError {
    fn from(e: DocError) -> Self {
        ConvertError::Doc(e)
    }
}

impl From<TimestampError> for ConvertError {
    fn from(e: TimestampError) -> Self {
        ConvertError::Timestamp(e)
    }
}

impl From<UnitError> for ConvertError {
    fn from(e: UnitError) -> Self {
        ConvertError::Unit(e)
    }
}

impl From<GenericError> for ConvertError {
    fn from(e: GenericError) -> Self {
        ConvertError::Generic(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn from_core_error_is_transparent() {
        let err = ConvertError::from(DocError::NestingTooDeep(3));
        assert_eq!(err.to_string(), "document nesting exceeds the maximum depth of 3");
        assert!(err.source().is_some());
    }

    #[test]
    fn wraps_generic_error() {
        let err = ConvertError::from(GenericError::Io("read failed".into()));
        assert_eq!(err.to_string(), "read failed");
        assert!(err.source().is_some());
    }
}

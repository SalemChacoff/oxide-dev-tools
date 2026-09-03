use std::fmt;

use oxide_dev_tools_core::{Base64Error, HexError, UrlError};

use super::GenericError;

/// Errors from `oxide codec` tools.
#[derive(Debug)]
pub enum CodecError {
    Base64(Base64Error),
    Hex(HexError),
    Url(UrlError),
    /// An argument or I/O failure shared with the other categories.
    Generic(GenericError),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::Base64(e) => write!(f, "{e}"),
            CodecError::Hex(e) => write!(f, "{e}"),
            CodecError::Url(e) => write!(f, "{e}"),
            CodecError::Generic(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CodecError::Base64(e) => Some(e),
            CodecError::Hex(e) => Some(e),
            CodecError::Url(e) => Some(e),
            CodecError::Generic(e) => Some(e),
        }
    }
}

impl From<Base64Error> for CodecError {
    fn from(e: Base64Error) -> Self {
        CodecError::Base64(e)
    }
}

impl From<HexError> for CodecError {
    fn from(e: HexError) -> Self {
        CodecError::Hex(e)
    }
}

impl From<UrlError> for CodecError {
    fn from(e: UrlError) -> Self {
        CodecError::Url(e)
    }
}

impl From<GenericError> for CodecError {
    fn from(e: GenericError) -> Self {
        CodecError::Generic(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn from_core_error_is_transparent() {
        let err = CodecError::from(Base64Error::InvalidInput);
        assert_eq!(err.to_string(), "input is not valid base64");
        assert!(err.source().is_some());
    }

    #[test]
    fn wraps_generic_error() {
        let err = CodecError::from(GenericError::Argument("bad flag".into()));
        assert_eq!(err.to_string(), "bad flag");
        assert!(err.source().is_some());
    }
}

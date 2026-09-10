use std::fmt;

use oxide_dev_tools_core::{FakeError, IdError, JwtError, KeyError, LoremError, SampleError};

use super::GenericError;

/// Errors from `oxide gen` tools.
#[derive(Debug)]
pub enum GenError {
    Fake(FakeError),
    Id(IdError),
    Jwt(JwtError),
    Key(KeyError),
    Lorem(LoremError),
    Sample(SampleError),
    /// An argument or I/O failure shared with the other categories.
    Generic(GenericError),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenError::Fake(e) => write!(f, "{e}"),
            GenError::Id(e) => write!(f, "{e}"),
            GenError::Jwt(e) => write!(f, "{e}"),
            GenError::Key(e) => write!(f, "{e}"),
            GenError::Lorem(e) => write!(f, "{e}"),
            GenError::Sample(e) => write!(f, "{e}"),
            GenError::Generic(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for GenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GenError::Fake(e) => Some(e),
            GenError::Id(e) => Some(e),
            GenError::Jwt(e) => Some(e),
            GenError::Key(e) => Some(e),
            GenError::Lorem(e) => Some(e),
            GenError::Sample(e) => Some(e),
            GenError::Generic(e) => Some(e),
        }
    }
}

impl From<FakeError> for GenError {
    fn from(e: FakeError) -> Self {
        GenError::Fake(e)
    }
}

impl From<IdError> for GenError {
    fn from(e: IdError) -> Self {
        GenError::Id(e)
    }
}

impl From<JwtError> for GenError {
    fn from(e: JwtError) -> Self {
        GenError::Jwt(e)
    }
}

impl From<KeyError> for GenError {
    fn from(e: KeyError) -> Self {
        GenError::Key(e)
    }
}

impl From<LoremError> for GenError {
    fn from(e: LoremError) -> Self {
        GenError::Lorem(e)
    }
}

impl From<SampleError> for GenError {
    fn from(e: SampleError) -> Self {
        GenError::Sample(e)
    }
}

impl From<GenericError> for GenError {
    fn from(e: GenericError) -> Self {
        GenError::Generic(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn from_core_error_is_transparent() {
        let err = GenError::from(FakeError::ZeroCount);
        assert_eq!(err.to_string(), "count must be at least 1");
        assert!(err.source().is_some());
    }

    #[test]
    fn wraps_generic_error() {
        let err = GenError::from(GenericError::Argument("bad flag".into()));
        assert_eq!(err.to_string(), "bad flag");
        assert!(err.source().is_some());
    }
}

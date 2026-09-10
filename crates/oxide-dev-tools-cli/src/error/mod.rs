mod codecs;
mod converters;
mod generators;
mod generic;

pub use codecs::CodecError;
pub use converters::ConvertError;
pub use generators::GenError;
pub use generic::GenericError;

use std::fmt;

/// Unified CLI error type — one variant per tool category.
///
/// Each category owns its own error enum (`CodecError`, `ConvertError`,
/// `GenError`) so new tools only touch their category file; this root enum
/// grows only when a whole new category (e.g. validators) is added.
#[derive(Debug)]
pub enum CliError {
    Codec(CodecError),
    Convert(ConvertError),
    Gen(GenError),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Codec(e) => write!(f, "{e}"),
            CliError::Convert(e) => write!(f, "{e}"),
            CliError::Gen(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Codec(e) => Some(e),
            CliError::Convert(e) => Some(e),
            CliError::Gen(e) => Some(e),
        }
    }
}

impl From<CodecError> for CliError {
    fn from(e: CodecError) -> Self {
        CliError::Codec(e)
    }
}

impl From<ConvertError> for CliError {
    fn from(e: ConvertError) -> Self {
        CliError::Convert(e)
    }
}

impl From<GenError> for CliError {
    fn from(e: GenError) -> Self {
        CliError::Gen(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn root_displays_category_error() {
        let err = CliError::from(GenError::from(GenericError::Io("disk full".into())));
        assert_eq!(err.to_string(), "disk full");
        assert!(err.source().is_some());
    }

    #[test]
    fn root_displays_codec_error() {
        let err = CliError::from(CodecError::from(oxide_dev_tools_core::Base64Error::InvalidInput));
        assert_eq!(err.to_string(), "input is not valid base64");
    }

    #[test]
    fn root_displays_convert_error() {
        let err = CliError::from(ConvertError::from(oxide_dev_tools_core::DocError::MissingRoot));
        assert_eq!(err.to_string(), "XML document has no root element");
    }
}

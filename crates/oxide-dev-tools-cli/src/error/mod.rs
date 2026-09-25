mod codecs;
mod converters;
mod diff;
mod generators;
mod generic;
mod text;
mod validators;

pub use codecs::CodecError;
pub use converters::ConvertError;
pub use diff::DiffError;
pub use generators::GenError;
pub use generic::GenericError;
pub use text::TextError;
pub use validators::ValidError;

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
    Diff(DiffError),
    Gen(GenError),
    Text(TextError),
    Valid(ValidError),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Codec(e) => write!(f, "{e}"),
            CliError::Convert(e) => write!(f, "{e}"),
            CliError::Diff(e) => write!(f, "{e}"),
            CliError::Gen(e) => write!(f, "{e}"),
            CliError::Text(e) => write!(f, "{e}"),
            CliError::Valid(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Codec(e) => Some(e),
            CliError::Convert(e) => Some(e),
            CliError::Diff(e) => Some(e),
            CliError::Gen(e) => Some(e),
            CliError::Text(e) => Some(e),
            CliError::Valid(e) => Some(e),
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

impl From<DiffError> for CliError {
    fn from(e: DiffError) -> Self {
        CliError::Diff(e)
    }
}

impl From<GenError> for CliError {
    fn from(e: GenError) -> Self {
        CliError::Gen(e)
    }
}

impl From<TextError> for CliError {
    fn from(e: TextError) -> Self {
        CliError::Text(e)
    }
}

impl From<ValidError> for CliError {
    fn from(e: ValidError) -> Self {
        CliError::Valid(e)
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

    #[test]
    fn root_displays_diff_error() {
        let err = CliError::from(DiffError::from(oxide_dev_tools_core::TextDiffError::InputTooLong { limit: 8 }));
        assert_eq!(err.to_string(), "character diff inputs must be at most 8 characters");
        assert!(err.source().is_some());
    }

    #[test]
    fn root_displays_valid_error() {
        let err = CliError::from(ValidError::Email("local part is empty".into()));
        assert_eq!(err.to_string(), "invalid email: local part is empty");
        assert!(err.source().is_some_and(|source| source.source().is_none()));
    }

    #[test]
    fn root_displays_text_error() {
        let err = CliError::from(TextError::from(oxide_dev_tools_core::CaseError::EmptyInput));
        assert_eq!(err.to_string(), "input contains no word characters");
        assert!(err.source().is_some());
    }
}

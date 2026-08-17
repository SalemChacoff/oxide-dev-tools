use std::fmt;

/// Unified CLI error type.
#[derive(Debug)]
pub enum CliError {
    Id(oxide_dev_tools_core::IdError),
    Key(oxide_dev_tools_core::KeyError),
    Argument(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Id(e) => write!(f, "{e}"),
            CliError::Key(e) => write!(f, "{e}"),
            CliError::Argument(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Id(e) => Some(e),
            CliError::Key(e) => Some(e),
            CliError::Argument(_) => None,
        }
    }
}

impl From<oxide_dev_tools_core::IdError> for CliError {
    fn from(e: oxide_dev_tools_core::IdError) -> Self {
        CliError::Id(e)
    }
}

impl From<oxide_dev_tools_core::KeyError> for CliError {
    fn from(e: oxide_dev_tools_core::KeyError) -> Self {
        CliError::Key(e)
    }
}

impl From<String> for CliError {
    fn from(msg: String) -> Self {
        CliError::Argument(msg)
    }
}

impl From<&str> for CliError {
    fn from(msg: &str) -> Self {
        CliError::Argument(msg.to_string())
    }
}

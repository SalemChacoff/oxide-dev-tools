use std::fmt;

/// Unified CLI error type.
#[derive(Debug)]
pub enum CliError {
    Fake(oxide_dev_tools_core::FakeError),
    Id(oxide_dev_tools_core::IdError),
    Jwt(oxide_dev_tools_core::JwtError),
    Key(oxide_dev_tools_core::KeyError),
    Lorem(oxide_dev_tools_core::LoremError),
    Argument(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Fake(e) => write!(f, "{e}"),
            CliError::Id(e) => write!(f, "{e}"),
            CliError::Jwt(e) => write!(f, "{e}"),
            CliError::Key(e) => write!(f, "{e}"),
            CliError::Lorem(e) => write!(f, "{e}"),
            CliError::Argument(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Fake(e) => Some(e),
            CliError::Id(e) => Some(e),
            CliError::Jwt(e) => Some(e),
            CliError::Key(e) => Some(e),
            CliError::Lorem(e) => Some(e),
            CliError::Argument(_) => None,
        }
    }
}

impl From<oxide_dev_tools_core::FakeError> for CliError {
    fn from(e: oxide_dev_tools_core::FakeError) -> Self {
        CliError::Fake(e)
    }
}

impl From<oxide_dev_tools_core::IdError> for CliError {
    fn from(e: oxide_dev_tools_core::IdError) -> Self {
        CliError::Id(e)
    }
}

impl From<oxide_dev_tools_core::JwtError> for CliError {
    fn from(e: oxide_dev_tools_core::JwtError) -> Self {
        CliError::Jwt(e)
    }
}

impl From<oxide_dev_tools_core::KeyError> for CliError {
    fn from(e: oxide_dev_tools_core::KeyError) -> Self {
        CliError::Key(e)
    }
}

impl From<oxide_dev_tools_core::LoremError> for CliError {
    fn from(e: oxide_dev_tools_core::LoremError) -> Self {
        CliError::Lorem(e)
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

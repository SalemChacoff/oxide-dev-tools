use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::{GenericError, TextError};

/// `oxide text truncate <INPUT> [flags]` — cut text to a maximum length with an ellipsis
///
/// Lengths count characters (Unicode scalar values); the ellipsis counts
/// toward `--max-length`. Text that already fits is printed unchanged.
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text truncate \"hello world\" --max-length 8\n  oxide text truncate \"hello world\" --max-length 8 --position start\n  oxide text truncate \"hello world\" --max-length 9 --position middle\n  oxide text truncate \"hello world\" --max-length 8 --ellipsis ...\n  oxide text truncate \"hello world\" --max-length 5 --ellipsis \"\"\n  oxide text truncate draft.md --max-length 40 --input-file\n  cat draft.md | oxide text truncate - --max-length 40"
)]
pub struct TruncateArgs {
    /// Text to truncate, a path to a file, or "-" to read from stdin
    pub input: Option<String>,

    /// Treat <INPUT> as a file path (a missing file is an error)
    #[arg(long)]
    pub input_file: bool,

    /// Maximum output length in characters, ellipsis included
    #[arg(long)]
    pub max_length: Option<usize>,

    /// Ellipsis marker added when truncation occurs (empty = hard truncation)
    #[arg(long)]
    pub ellipsis: Option<String>,

    /// Where the ellipsis sits in the truncated output
    #[arg(long, value_enum)]
    pub position: Option<TruncatePositionCli>,
}

/// Where the ellipsis sits in the truncated output.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TruncatePositionCli {
    /// Keep the head of the text (`hello w…`)
    End,
    /// Keep the tail of the text (`…o world`)
    Start,
    /// Keep both ends (`hell…orld`)
    Middle,
}

impl From<TruncatePositionCli> for TruncatePosition {
    fn from(position: TruncatePositionCli) -> Self {
        match position {
            TruncatePositionCli::End => TruncatePosition::End,
            TruncatePositionCli::Start => TruncatePosition::Start,
            TruncatePositionCli::Middle => TruncatePosition::Middle,
        }
    }
}

pub fn exec(args: TruncateArgs) -> Result<(), TextError> {
    let input = super::resolve_text_input(&args.input, args.input_file)?;
    let max_length = args
        .max_length
        .ok_or_else(|| GenericError::Argument("missing --max-length (maximum output length)".into()))?;
    let options = TruncateOptions {
        input,
        max_length,
        ellipsis: args.ellipsis.unwrap_or_else(|| "…".to_string()),
        position: args.position.unwrap_or(TruncatePositionCli::End).into(),
    };
    println!("{}", truncate_text(options)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(max_length: usize) -> TruncateArgs {
        TruncateArgs {
            input: Some("hello world".into()),
            input_file: false,
            max_length: Some(max_length),
            ellipsis: None,
            position: None,
        }
    }

    #[test]
    fn exec_defaults_succeed() {
        assert!(exec(args(8)).is_ok());
    }

    #[test]
    fn exec_every_position_succeeds() {
        for position in [
            TruncatePositionCli::End,
            TruncatePositionCli::Start,
            TruncatePositionCli::Middle,
        ] {
            let result = exec(TruncateArgs {
                position: Some(position),
                ..args(8)
            });
            assert!(result.is_ok(), "{position:?}");
        }
    }

    #[test]
    fn exec_custom_and_empty_ellipsis_succeed() {
        for ellipsis in [Some("...".to_string()), Some(String::new()), None] {
            let result = exec(TruncateArgs { ellipsis, ..args(8) });
            assert!(result.is_ok());
        }
    }

    #[test]
    fn exec_missing_max_length_errors() {
        let result = exec(TruncateArgs {
            max_length: None,
            ..args(8)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing --max-length"));
    }

    #[test]
    fn exec_missing_input_errors() {
        let result = exec(TruncateArgs { input: None, ..args(8) });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing <INPUT>"));
    }

    #[test]
    fn exec_ellipsis_too_long_errors() {
        let result = exec(TruncateArgs {
            max_length: Some(2),
            ellipsis: Some("...".into()),
            ..args(8)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ellipsis is longer"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(TruncateArgs {
            input: Some("no-such-file-oxide.txt".into()),
            input_file: true,
            ..args(8)
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn position_maps_to_core() {
        assert_eq!(TruncatePosition::from(TruncatePositionCli::End), TruncatePosition::End);
        assert_eq!(TruncatePosition::from(TruncatePositionCli::Start), TruncatePosition::Start);
        assert_eq!(TruncatePosition::from(TruncatePositionCli::Middle), TruncatePosition::Middle);
    }
}

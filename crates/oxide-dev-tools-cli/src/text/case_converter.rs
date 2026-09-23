use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::TextError;

/// `oxide text case <target> <input>` — convert text to another letter case
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide text case camel \"hello world\"\n  oxide text case pascal \"hello world\"\n  oxide text case snake \"helloWorld\"\n  oxide text case screaming-snake \"helloWorld\"\n  oxide text case kebab \"helloWorld\"\n  oxide text case screaming-kebab \"helloWorld\"\n  oxide text case dot \"hello world\"\n  oxide text case title \"helloWorld\"\n  oxide text case lower \"HelloWorld\"\n  oxide text case upper \"hello world\""
)]
pub struct CaseArgs {
    /// Target letter case
    #[arg(value_enum)]
    target: CaseTargetCli,

    /// Text to convert
    input: String,
}

/// Letter cases selectable from the command line.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CaseTargetCli {
    /// `helloWorld`
    Camel,
    /// `HelloWorld`
    Pascal,
    /// `hello_world`
    Snake,
    /// `HELLO_WORLD`
    ScreamingSnake,
    /// `hello-world`
    Kebab,
    /// `HELLO-WORLD`
    ScreamingKebab,
    /// `hello.world`
    Dot,
    /// `Hello World`
    Title,
    /// `hello world`
    Lower,
    /// `HELLO WORLD`
    Upper,
}

impl From<CaseTargetCli> for CaseTarget {
    fn from(target: CaseTargetCli) -> Self {
        match target {
            CaseTargetCli::Camel => CaseTarget::Camel,
            CaseTargetCli::Pascal => CaseTarget::Pascal,
            CaseTargetCli::Snake => CaseTarget::Snake,
            CaseTargetCli::ScreamingSnake => CaseTarget::ScreamingSnake,
            CaseTargetCli::Kebab => CaseTarget::Kebab,
            CaseTargetCli::ScreamingKebab => CaseTarget::ScreamingKebab,
            CaseTargetCli::Dot => CaseTarget::Dot,
            CaseTargetCli::Title => CaseTarget::Title,
            CaseTargetCli::Lower => CaseTarget::Lower,
            CaseTargetCli::Upper => CaseTarget::Upper,
        }
    }
}

pub fn exec(args: CaseArgs) -> Result<(), TextError> {
    let opts = CaseOptions {
        input: args.input,
        target: args.target.into(),
    };
    println!("{}", convert_case(CaseKind::Convert(opts))?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec_target(target: CaseTargetCli) -> Result<(), TextError> {
        exec(CaseArgs {
            target,
            input: "hello world".into(),
        })
    }

    #[test]
    fn exec_every_target_succeeds() {
        for target in [
            CaseTargetCli::Camel,
            CaseTargetCli::Pascal,
            CaseTargetCli::Snake,
            CaseTargetCli::ScreamingSnake,
            CaseTargetCli::Kebab,
            CaseTargetCli::ScreamingKebab,
            CaseTargetCli::Dot,
            CaseTargetCli::Title,
            CaseTargetCli::Lower,
            CaseTargetCli::Upper,
        ] {
            assert!(exec_target(target).is_ok(), "{target:?}");
        }
    }

    #[test]
    fn exec_empty_input_errors() {
        let result = exec(CaseArgs {
            target: CaseTargetCli::Camel,
            input: "___".into(),
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no word characters"));
    }

    #[test]
    fn target_maps_to_core() {
        assert_eq!(CaseTarget::from(CaseTargetCli::Snake), CaseTarget::Snake);
        assert_eq!(CaseTarget::from(CaseTargetCli::ScreamingKebab), CaseTarget::ScreamingKebab);
    }
}

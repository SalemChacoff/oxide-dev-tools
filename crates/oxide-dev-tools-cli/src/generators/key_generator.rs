use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;

/// `oxide gen id [subcommand]` — ID generator dispatch
#[derive(Args)]
pub struct KeyArgs {
    #[command(subcommand)]
    pub kind: KeyCmd,
}

#[derive(Subcommand)]
pub enum KeyCmd {
    /// Generate a password
    #[command(name = "pass")]
    Pass,

    /// Generate a token
    #[command(name = "token")]
    Token,
}

pub fn exec(args: KeyArgs) {
    match args.kind {
        KeyCmd::Pass => {
            println!("{}", generate_key(KeyKind::Password));
        }
        KeyCmd::Token => println!("{}", generate_key(KeyKind::Token)),
    }
}

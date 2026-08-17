use clap::{Parser, Subcommand};

mod error;
mod generators;

#[derive(Parser)]
#[command(name = "oxide", version)]
struct Cli {
    #[command(subcommand)]
    tool: Tool,
}

#[derive(Subcommand)]
enum Tool {
    /// Generate IDs, ULIDs, NanoIDs, passwords, tokens, etc.
    Gen(generators::GenArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.tool {
        Tool::Gen(args) => generators::exec(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

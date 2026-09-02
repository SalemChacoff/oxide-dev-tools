use clap::{Parser, Subcommand};

mod codecs;
mod converters;
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
    /// Encode and decode data (base64, hex, URL, etc.).
    Codec(codecs::CodecArgs),
    /// Convert values between formats (timestamps, units, etc.).
    Convert(converters::ConvertArgs),
    /// Generate IDs, ULIDs, NanoIDs, passwords, tokens, etc.
    Gen(generators::GenArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.tool {
        Tool::Codec(args) => codecs::exec(args),
        Tool::Convert(args) => converters::exec(args),
        Tool::Gen(args) => generators::exec(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

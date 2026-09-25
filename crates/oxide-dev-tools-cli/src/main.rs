use clap::{Parser, Subcommand};

mod codecs;
mod converters;
mod diff;
mod error;
mod generators;
mod text;
mod validators;

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
    /// Compare texts and files, git-style.
    Diff(diff::DiffArgs),
    /// Generate IDs, ULIDs, NanoIDs, passwords, tokens, etc.
    Gen(generators::GenArgs),
    /// Transform text (case conversion, etc.).
    Text(text::TextArgs),
    /// Validate values against standards (emails, URLs, IPs, etc.).
    Validate(validators::ValidateArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.tool {
        Tool::Codec(args) => codecs::exec(args),
        Tool::Convert(args) => converters::exec(args),
        Tool::Diff(args) => diff::exec(args),
        Tool::Gen(args) => generators::exec(args),
        Tool::Text(args) => text::exec(args),
        Tool::Validate(args) => validators::exec(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

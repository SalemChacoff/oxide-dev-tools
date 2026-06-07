use clap::{Parser, Subcommand};

mod generators;

#[derive(Parser)]
#[command(name = "oxide", version)]
struct Cli {
    #[command(subcommand)]
    tool: Tool,
}

#[derive(Subcommand)]
enum Tool {
    /// Generate IDs, ULIDs, NanoIDs, etc.
    Gen(generators::GenArgs),
    // Future tools:
    // Compare(comparators::CompareArgs),
    // Codec(codec::CodecArgs), This is for encoding/decoding base64, zip, pem, pfx, etc
    // Converter(converters::ConverterArgs)
    // Text(text::TextArgs)
    // Validator(validators::ValidatorArgs)
}

fn main() {
    let cli = Cli::parse();

    match cli.tool {
        Tool::Gen(args) => generators::exec(args),
    }
}

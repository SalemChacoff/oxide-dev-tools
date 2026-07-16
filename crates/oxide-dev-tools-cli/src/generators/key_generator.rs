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
    Pass {
        /// Length of the password
        #[arg(short = 'l', long = "length", default_value_t = 16)]
        length: usize,

        /// Exclude lowercase letters (enabled by default)
        #[arg(short = 'w', long = "no-lowercase")]
        no_lowercase: bool,

        /// Exclude uppercase letters (enabled by default)
        #[arg(short = 'u', long = "no-uppercase")]
        no_uppercase: bool,

        /// Exclude digits (enabled by default)
        #[arg(short = 'd', long = "no-digits")]
        no_digits: bool,

        /// Include special characters (!@#$%^&*)
        /// Change boolean value to String value to pass the SpecialChars
        #[arg(short = 's', long = "special")]
        special: bool,
    },

    /// Generate a token
    #[command(name = "token")]
    Token,
}

pub fn exec(args: KeyArgs) {
    match args.kind {
        KeyCmd::Pass {
            length,
            no_lowercase,
            no_uppercase,
            no_digits,
            special,
        } => {
            let opts = PasswordOptions {
                length,
                lowercase: !no_lowercase,
                uppercase: !no_uppercase,
                digits: !no_digits,
                special,
            };

            // Validate if has at least one character type
            if !opts.lowercase && !opts.uppercase && !opts.digits && !opts.special {
                eprintln!("Error: At least one character type must be enabled");
                return;
            }
            println!("{}", generate_key(KeyKind::Password(opts)));
        }
        KeyCmd::Token => println!("{}", generate_key(KeyKind::Token)),
    }
}

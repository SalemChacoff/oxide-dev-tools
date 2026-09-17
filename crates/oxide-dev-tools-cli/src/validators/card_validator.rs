use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::ValidError;

/// `oxide validate card <NUMBER> [options]` — Luhn validation and issuer detection
#[derive(Args)]
pub struct CardArgs {
    /// Card number to validate
    pub value: String,

    /// Required issuer network
    #[arg(long, value_enum, default_value_t = CardNetworkArg::Any)]
    pub network: CardNetworkArg,

    /// Reject numbers whose issuer network is not recognized
    #[arg(long)]
    pub require_network: bool,

    /// Reject spaces and hyphens in the number
    #[arg(long)]
    pub no_separators: bool,

    /// Print the full validation report instead of the bare digits
    #[arg(long)]
    pub verbose: bool,
}

/// Issuer network choices for the `--network` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CardNetworkArg {
    /// Accept any network.
    Any,
    /// Require a Visa card.
    Visa,
    /// Require a Mastercard.
    Mastercard,
    /// Require an American Express card.
    AmericanExpress,
    /// Require a Discover card.
    Discover,
    /// Require a Diners Club card.
    DinersClub,
    /// Require a JCB card.
    Jcb,
    /// Require a UnionPay card.
    UnionPay,
    /// Require a Maestro card.
    Maestro,
    /// Require a Mir card.
    Mir,
    /// Require a RuPay card.
    RuPay,
    /// Require an Elo card.
    Elo,
    /// Require a Hipercard card.
    Hipercard,
    /// Require a Verve card.
    Verve,
    /// Require a UATP card.
    Uatp,
}

impl From<CardNetworkArg> for CardNetwork {
    fn from(arg: CardNetworkArg) -> Self {
        match arg {
            CardNetworkArg::Any => CardNetwork::Any,
            CardNetworkArg::Visa => CardNetwork::Visa,
            CardNetworkArg::Mastercard => CardNetwork::Mastercard,
            CardNetworkArg::AmericanExpress => CardNetwork::AmericanExpress,
            CardNetworkArg::Discover => CardNetwork::Discover,
            CardNetworkArg::DinersClub => CardNetwork::DinersClub,
            CardNetworkArg::Jcb => CardNetwork::Jcb,
            CardNetworkArg::UnionPay => CardNetwork::UnionPay,
            CardNetworkArg::Maestro => CardNetwork::Maestro,
            CardNetworkArg::Mir => CardNetwork::Mir,
            CardNetworkArg::RuPay => CardNetwork::RuPay,
            CardNetworkArg::Elo => CardNetwork::Elo,
            CardNetworkArg::Hipercard => CardNetwork::Hipercard,
            CardNetworkArg::Verve => CardNetwork::Verve,
            CardNetworkArg::Uatp => CardNetwork::Uatp,
        }
    }
}

pub fn exec(args: CardArgs) -> Result<(), ValidError> {
    let options = CardOptions {
        input: args.value,
        network: args.network.into(),
        require_network: args.require_network,
        no_separators: args.no_separators,
    };
    let report = validate_card(options);
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if args.verbose {
        print_report(&report);
    }
    if report.valid {
        if !args.verbose {
            println!("{}", report.normalized);
        }
        Ok(())
    } else {
        Err(ValidError::Card(report.issues.join("; ")))
    }
}

// -------- Report output --------

fn print_report(report: &CardReport) {
    println!("valid:    {}", yes_no(report.valid));
    println!("input:    {}", report.input);
    println!("digits:   {}", report.normalized);
    println!("network:  {}", network_name(report.network));
    println!("luhn:     {}", yes_no(report.luhn));
    println!("length:   {}", report.length);
    if !report.issues.is_empty() {
        println!("issues:");
        for issue in &report.issues {
            println!("  - {issue}");
        }
    }
    if !report.warnings.is_empty() {
        println!("warnings:");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
}

fn yes_no(valid: bool) -> &'static str {
    if valid { "yes" } else { "no" }
}

fn network_name(network: CardNetwork) -> &'static str {
    match network {
        CardNetwork::Any => "any",
        CardNetwork::Visa => "Visa",
        CardNetwork::Mastercard => "Mastercard",
        CardNetwork::AmericanExpress => "American Express",
        CardNetwork::Discover => "Discover",
        CardNetwork::DinersClub => "Diners Club",
        CardNetwork::Jcb => "JCB",
        CardNetwork::UnionPay => "UnionPay",
        CardNetwork::Maestro => "Maestro",
        CardNetwork::Mir => "Mir",
        CardNetwork::RuPay => "RuPay",
        CardNetwork::Elo => "Elo",
        CardNetwork::Hipercard => "Hipercard",
        CardNetwork::Verve => "Verve",
        CardNetwork::Uatp => "UATP",
        CardNetwork::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(value: &str) -> CardArgs {
        CardArgs {
            value: value.to_string(),
            network: CardNetworkArg::Any,
            require_network: false,
            no_separators: false,
            verbose: false,
        }
    }

    #[test]
    fn exec_valid_card_forms() {
        assert!(exec(args("4111111111111111")).is_ok());
        assert!(exec(args("5555 5555 5555 4444")).is_ok());
        assert!(exec(args("3782-822463-10005")).is_ok());
        assert!(exec(args("79927398713")).is_ok());
    }

    #[test]
    fn exec_invalid_card() {
        let result = exec(args("4111111111111112"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid card"));
    }

    #[test]
    fn exec_flags() {
        let result = exec(CardArgs {
            network: CardNetworkArg::Mastercard,
            ..args("4111111111111111")
        });
        assert!(result.unwrap_err().to_string().contains("expected a Mastercard card"));

        let result = exec(CardArgs {
            require_network: true,
            ..args("79927398713")
        });
        assert!(result.unwrap_err().to_string().contains("not recognized"));

        let result = exec(CardArgs {
            no_separators: true,
            ..args("4111 1111 1111 1111")
        });
        assert!(result.unwrap_err().to_string().contains("separators are not allowed"));

        assert!(
            exec(CardArgs {
                network: CardNetworkArg::Visa,
                ..args("4111111111111111")
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_verbose_valid_and_invalid() {
        assert!(
            exec(CardArgs {
                verbose: true,
                ..args("4111111111111111")
            })
            .is_ok()
        );
        assert!(
            exec(CardArgs {
                verbose: true,
                ..args("not-a-card")
            })
            .is_err()
        );
    }
}

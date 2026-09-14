use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::ValidError;

/// `oxide validate ip <ADDRESS> [options]` — IPv4/IPv6 validation and classification
#[derive(Args)]
pub struct IpArgs {
    /// IP address to validate
    pub address: String,

    /// Required address family
    #[arg(long, value_enum, default_value_t = IpModeArg::Auto)]
    pub mode: IpModeArg,

    /// Reject addresses that are not globally routable
    #[arg(long)]
    pub require_global: bool,

    /// Reject IPv4 octets with leading zeros (e.g. 192.168.001.1)
    #[arg(long)]
    pub no_leading_zeros: bool,

    /// Reject IPv6 zone identifiers (e.g. fe80::1%eth0)
    #[arg(long)]
    pub no_zone_id: bool,

    /// Print the full validation report instead of the bare canonical address
    #[arg(long)]
    pub verbose: bool,
}

/// Address family choices for the `--mode` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum IpModeArg {
    /// Accept IPv4 or IPv6, detected from the input.
    Auto,
    /// Require an IPv4 address.
    Ipv4,
    /// Require an IPv6 address.
    Ipv6,
}

impl From<IpModeArg> for IpMode {
    fn from(arg: IpModeArg) -> Self {
        match arg {
            IpModeArg::Auto => IpMode::Auto,
            IpModeArg::Ipv4 => IpMode::Ipv4,
            IpModeArg::Ipv6 => IpMode::Ipv6,
        }
    }
}

pub fn exec(args: IpArgs) -> Result<(), ValidError> {
    let options = IpOptions {
        input: args.address,
        mode: args.mode.into(),
        require_global: args.require_global,
        allow_leading_zeros: !args.no_leading_zeros,
        allow_zone_id: !args.no_zone_id,
    };
    let report = validate_ip(options);
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
        Err(ValidError::Ip(report.issues.join("; ")))
    }
}

// -------- Report output --------

fn print_report(report: &IpReport) {
    println!("valid:      {}", yes_no(report.valid));
    println!("input:      {}", report.input);
    println!("normalized: {}", report.normalized);
    println!("version:    {}", version_name(report.version));
    println!("class:      {}", class_name(report.class));
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

fn version_name(version: IpVersion) -> &'static str {
    match version {
        IpVersion::Ipv4 => "ipv4",
        IpVersion::Ipv6 => "ipv6",
    }
}

fn class_name(class: IpClass) -> &'static str {
    match class {
        IpClass::Unspecified => "unspecified",
        IpClass::Loopback => "loopback",
        IpClass::Private => "private",
        IpClass::LinkLocal => "link-local",
        IpClass::Multicast => "multicast",
        IpClass::Broadcast => "broadcast",
        IpClass::Documentation => "documentation",
        IpClass::Benchmark => "benchmark",
        IpClass::Shared => "shared",
        IpClass::UniqueLocal => "unique-local",
        IpClass::Ipv4Mapped => "ipv4-mapped",
        IpClass::Discard => "discard",
        IpClass::Teredo => "teredo",
        IpClass::SixToFour => "6to4",
        IpClass::Global => "global",
        IpClass::Reserved => "reserved",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(address: &str) -> IpArgs {
        IpArgs {
            address: address.to_string(),
            mode: IpModeArg::Auto,
            require_global: false,
            no_leading_zeros: false,
            no_zone_id: false,
            verbose: false,
        }
    }

    #[test]
    fn exec_valid_addresses() {
        assert!(exec(args("192.168.1.1")).is_ok());
        assert!(exec(args("2001:db8::1")).is_ok());
        assert!(exec(args("fe80::1%eth0")).is_ok());
    }

    #[test]
    fn exec_invalid_address() {
        let result = exec(args("999.1.1.1"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid ip"));
    }

    #[test]
    fn exec_flags() {
        let result = exec(IpArgs {
            require_global: true,
            ..args("10.0.0.1")
        });
        assert!(result.unwrap_err().to_string().contains("not globally routable"));

        let result = exec(IpArgs {
            no_leading_zeros: true,
            ..args("192.168.001.1")
        });
        assert!(result.unwrap_err().to_string().contains("leading zero"));

        let result = exec(IpArgs {
            no_zone_id: true,
            ..args("fe80::1%eth0")
        });
        assert!(result.unwrap_err().to_string().contains("zone"));

        let result = exec(IpArgs {
            mode: IpModeArg::Ipv4,
            ..args("::1")
        });
        assert!(result.unwrap_err().to_string().contains("expected an IPv4"));

        let result = exec(IpArgs {
            require_global: true,
            ..args("8.8.8.8")
        });
        assert!(result.is_ok());
    }

    #[test]
    fn exec_verbose_valid_and_invalid() {
        assert!(
            exec(IpArgs {
                verbose: true,
                ..args("8.8.8.8")
            })
            .is_ok()
        );
        assert!(
            exec(IpArgs {
                verbose: true,
                ..args("nope")
            })
            .is_err()
        );
    }
}

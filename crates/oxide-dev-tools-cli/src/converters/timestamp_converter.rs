use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::CliError;

/// `oxide convert timestamp <INPUT> [options]` — timestamp conversion
#[derive(Args)]
pub struct TimestampArgs {
    /// Value to convert
    pub input: String,

    /// Force the input format instead of auto-detection
    #[arg(long, value_enum, default_value_t = InputFormatArg::Auto)]
    pub from: InputFormatArg,

    /// Output format (default: iso for unix input, unix for date input)
    #[arg(long, value_enum)]
    pub to: Option<OutputFormatArg>,

    /// Unit for unix timestamps; the input unit is auto-detected by digit count
    #[arg(long, value_enum)]
    pub unit: Option<UnitArg>,

    /// Fractional digits (0-9) for ISO 8601 and unix output; truncated, not rounded
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=9))]
    pub precision: Option<u8>,

    /// Output timezone: utc, local, ±HH:MM offset, or an IANA name (e.g. Europe/Berlin)
    #[arg(long, default_value = "utc", allow_hyphen_values = true)]
    pub zone: String,
}

/// Input format choices for the `--from` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InputFormatArg {
    /// Detect the format automatically.
    Auto,
    /// Unix timestamp in seconds, milliseconds, microseconds, or nanoseconds.
    Unix,
    /// ISO 8601 / RFC 3339 date.
    Iso,
    /// RFC 2822 date (email style).
    Rfc2822,
    /// Long-form human-readable date.
    Human,
}

impl From<InputFormatArg> for TimestampInputFormat {
    fn from(arg: InputFormatArg) -> Self {
        match arg {
            InputFormatArg::Auto => TimestampInputFormat::Auto,
            InputFormatArg::Unix => TimestampInputFormat::Unix,
            InputFormatArg::Iso => TimestampInputFormat::Iso8601,
            InputFormatArg::Rfc2822 => TimestampInputFormat::Rfc2822,
            InputFormatArg::Human => TimestampInputFormat::Human,
        }
    }
}

/// Output format choices for the `--to` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormatArg {
    /// Unix timestamp.
    Unix,
    /// ISO 8601 / RFC 3339 date.
    Iso,
    /// RFC 2822 date (email style).
    Rfc2822,
    /// Long-form human-readable date.
    Human,
}

impl From<OutputFormatArg> for TimestampOutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Unix => TimestampOutputFormat::Unix,
            OutputFormatArg::Iso => TimestampOutputFormat::Iso8601,
            OutputFormatArg::Rfc2822 => TimestampOutputFormat::Rfc2822,
            OutputFormatArg::Human => TimestampOutputFormat::Human,
        }
    }
}

/// Unit choices for the `--unit` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UnitArg {
    /// Seconds.
    S,
    /// Milliseconds.
    Ms,
    /// Microseconds.
    Us,
    /// Nanoseconds.
    Ns,
}

impl From<UnitArg> for TimestampUnit {
    fn from(arg: UnitArg) -> Self {
        match arg {
            UnitArg::S => TimestampUnit::Seconds,
            UnitArg::Ms => TimestampUnit::Milliseconds,
            UnitArg::Us => TimestampUnit::Microseconds,
            UnitArg::Ns => TimestampUnit::Nanoseconds,
        }
    }
}

pub fn exec(args: TimestampArgs) -> Result<(), CliError> {
    let options = TimestampOptions {
        input: args.input,
        input_format: args.from.into(),
        output_format: args.to.map(Into::into),
        unit: args.unit.map(Into::into),
        precision: args.precision,
        zone: parse_timestamp_zone(&args.zone)?,
    };
    println!("{}", convert_timestamp(TimestampKind::Convert(options))?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &str) -> TimestampArgs {
        TimestampArgs {
            input: input.into(),
            from: InputFormatArg::Auto,
            to: None,
            unit: None,
            precision: None,
            zone: "utc".into(),
        }
    }

    #[test]
    fn exec_unix_to_iso() {
        assert!(exec(args("1750000000")).is_ok());
    }

    #[test]
    fn exec_iso_to_unix() {
        assert!(exec(args("2026-06-07T12:34:56Z")).is_ok());
    }

    #[test]
    fn exec_every_output_format() {
        let input = "2026-06-07T12:34:56Z".to_string();
        for to in [
            OutputFormatArg::Unix,
            OutputFormatArg::Iso,
            OutputFormatArg::Rfc2822,
            OutputFormatArg::Human,
        ] {
            assert!(
                exec(TimestampArgs {
                    input: input.clone(),
                    to: Some(to),
                    ..args("")
                })
                .is_ok()
            );
        }
    }

    #[test]
    fn exec_every_input_format() {
        let inputs = [
            (InputFormatArg::Unix, "1750000000"),
            (InputFormatArg::Iso, "2026-06-07T12:34:56Z"),
            (InputFormatArg::Rfc2822, "Sun, 07 Jun 2026 12:34:56 +0000"),
            (InputFormatArg::Human, "June 07, 2026 12:34:56 PM +0000"),
        ];
        for (from, input) in inputs {
            assert!(
                exec(TimestampArgs {
                    input: input.into(),
                    from,
                    ..args("")
                })
                .is_ok()
            );
        }
    }

    #[test]
    fn exec_zones() {
        let input = "2026-06-07T12:34:56Z".to_string();
        for zone in ["utc", "local", "+05:30", "Europe/Berlin"] {
            assert!(
                exec(TimestampArgs {
                    input: input.clone(),
                    zone: zone.into(),
                    ..args("")
                })
                .is_ok(),
                "zone {zone:?}"
            );
        }
    }

    #[test]
    fn exec_precision_and_unit() {
        assert!(
            exec(TimestampArgs {
                input: "1750000000.123456789".into(),
                to: Some(OutputFormatArg::Iso),
                precision: Some(3),
                ..args("")
            })
            .is_ok()
        );
        assert!(
            exec(TimestampArgs {
                input: "1750000000".into(),
                to: Some(OutputFormatArg::Unix),
                unit: Some(UnitArg::Ms),
                ..args("")
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_invalid_date_errors() {
        let result = exec(args("2026-02-30"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("day 30 out of range"));
    }

    #[test]
    fn exec_unknown_zone_errors() {
        let result = exec(TimestampArgs {
            zone: "Not/AZone".into(),
            ..args("2026-06-07T12:34:56Z")
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown IANA timezone"));
    }

    #[test]
    fn exec_invalid_offset_errors() {
        let result = exec(TimestampArgs {
            zone: "+25:00".into(),
            ..args("2026-06-07T12:34:56Z")
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("offset"));
    }
}

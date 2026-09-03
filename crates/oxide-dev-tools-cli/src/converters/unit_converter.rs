use clap::Args;
use oxide_dev_tools_core::*;

use crate::error::CliError;

/// Shared arguments for `oxide convert storage|rate|length|time|mass`
#[derive(Args)]
pub struct UnitConvertArgs {
    /// Numeric value, optionally with the unit attached (e.g. 1.5gB or 5km)
    pub value: Option<String>,

    /// Source unit when it is not attached to the value (e.g. gB, km, h)
    pub from: Option<String>,

    /// Target unit to convert into
    #[arg(long)]
    pub to: Option<String>,

    /// Fractional digits in the output (0-18, truncated)
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=18))]
    pub precision: Option<u8>,

    /// Calendar anchor for month/year conversions (YYYY-MM-DD, default 1970-01-01; time only)
    #[arg(long)]
    pub anchor: Option<String>,

    /// List every unit available for this category
    #[arg(long)]
    pub list: bool,
}

pub fn exec(args: UnitConvertArgs, category: UnitCategory) -> Result<(), CliError> {
    if args.list {
        println!("{}", unit_catalog(category));
        return Ok(());
    }
    let value = match args.value {
        Some(value) => value,
        None => return Err("missing <VALUE> (the number to convert, e.g. 1.5, 1.5gB, or 5km)".into()),
    };
    let to = match args.to {
        Some(to) => to,
        None => return Err("missing --to <UNIT> (the target unit, e.g. --to mib)".into()),
    };
    let options = UnitOptions {
        value,
        from: args.from,
        to,
        precision: args.precision,
        anchor: args.anchor,
    };
    let kind = match category {
        UnitCategory::Storage => UnitKind::Storage(options),
        UnitCategory::DataRate => UnitKind::DataRate(options),
        UnitCategory::Length => UnitKind::Length(options),
        UnitCategory::Time => UnitKind::Time(options),
        UnitCategory::Mass => UnitKind::Mass(options),
    };
    println!("{}", convert_unit(kind)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(value: &str) -> UnitConvertArgs {
        UnitConvertArgs {
            value: Some(value.into()),
            from: None,
            to: None,
            precision: None,
            anchor: None,
            list: false,
        }
    }

    #[test]
    fn exec_every_category() {
        let cases = [
            (UnitCategory::Storage, "1", Some("gB"), "mib"),
            (UnitCategory::DataRate, "8", Some("mbps"), "mB/s"),
            (UnitCategory::Length, "5", Some("km"), "mi"),
            (UnitCategory::Time, "90", Some("min"), "h"),
            (UnitCategory::Mass, "200", Some("lb"), "kg"),
        ];
        for (category, value, from, to) in cases {
            assert!(
                exec(
                    UnitConvertArgs {
                        value: Some(value.into()),
                        from: from.map(str::to_string),
                        to: Some(to.into()),
                        ..args("")
                    },
                    category
                )
                .is_ok(),
                "{category:?} {value} {from:?} -> {to}"
            );
        }
    }

    #[test]
    fn exec_glued_value() {
        let result = exec(
            UnitConvertArgs {
                to: Some("mi".into()),
                ..args("5km")
            },
            UnitCategory::Length,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn exec_list_prints_catalog() {
        let result = exec(UnitConvertArgs { list: true, ..args("") }, UnitCategory::Storage);
        assert!(result.is_ok());
    }

    #[test]
    fn exec_time_with_anchor() {
        let result = exec(
            UnitConvertArgs {
                value: Some("1".into()),
                from: Some("y".into()),
                to: Some("d".into()),
                anchor: Some("2020-01-01".into()),
                ..args("")
            },
            UnitCategory::Time,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn exec_missing_target_errors() {
        let result = exec(args("5km"), UnitCategory::Length);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing --to"));
    }

    #[test]
    fn exec_unknown_unit_errors() {
        let result = exec(
            UnitConvertArgs {
                value: Some("1".into()),
                from: Some("furlong".into()),
                to: Some("m".into()),
                ..args("")
            },
            UnitCategory::Length,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown unit"));
    }

    #[test]
    fn exec_ambiguous_value_errors() {
        let result = exec(
            UnitConvertArgs {
                value: Some("5km".into()),
                from: Some("km".into()),
                to: Some("mi".into()),
                ..args("")
            },
            UnitCategory::Length,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already includes a unit"));
    }

    #[test]
    fn exec_precision_out_of_range_errors() {
        let result = exec(
            UnitConvertArgs {
                value: Some("1".into()),
                from: Some("lb".into()),
                to: Some("kg".into()),
                precision: Some(19),
                ..args("")
            },
            UnitCategory::Mass,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of range"));
    }
}

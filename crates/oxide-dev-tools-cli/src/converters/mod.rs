pub mod timestamp_converter;
pub mod unit_converter;

use clap::{Args, Subcommand};
use oxide_dev_tools_core::UnitCategory;

use crate::error::CliError;

/// `oxide convert ...` — entry point for all converters
#[derive(Args)]
pub struct ConvertArgs {
    #[command(subcommand)]
    pub kind: ConvertKind,
}

#[derive(Subcommand)]
pub enum ConvertKind {
    /// Convert timestamps between Unix, ISO 8601, and human-readable formats.
    Timestamp(timestamp_converter::TimestampArgs),
    /// Convert data storage sizes between bits and bytes (SI and binary prefixes).
    Storage(unit_converter::UnitConvertArgs),
    /// Convert data rates between bit/s and byte/s units.
    Rate(unit_converter::UnitConvertArgs),
    /// Convert lengths between metric and imperial units.
    Length(unit_converter::UnitConvertArgs),
    /// Convert time durations (calendar-aware months and years).
    Time(unit_converter::UnitConvertArgs),
    /// Convert masses between metric and imperial units.
    Mass(unit_converter::UnitConvertArgs),
}

pub fn exec(args: ConvertArgs) -> Result<(), CliError> {
    match args.kind {
        ConvertKind::Timestamp(args) => timestamp_converter::exec(args),
        ConvertKind::Storage(args) => unit_converter::exec(args, UnitCategory::Storage),
        ConvertKind::Rate(args) => unit_converter::exec(args, UnitCategory::DataRate),
        ConvertKind::Length(args) => unit_converter::exec(args, UnitCategory::Length),
        ConvertKind::Time(args) => unit_converter::exec(args, UnitCategory::Time),
        ConvertKind::Mass(args) => unit_converter::exec(args, UnitCategory::Mass),
    }
}

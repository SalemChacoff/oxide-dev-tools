pub mod doc_converter;
pub mod timestamp_converter;
pub mod unit_converter;

use clap::{Args, Subcommand};
use oxide_dev_tools_core::{DocKind, UnitCategory};

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
    /// Convert JSON to YAML.
    #[command(name = "json2yaml")]
    Json2Yaml(doc_converter::DocConvertArgs),
    /// Convert YAML to JSON.
    #[command(name = "yaml2json")]
    Yaml2Json(doc_converter::DocConvertArgs),
    /// Convert JSON to XML.
    #[command(name = "json2xml")]
    Json2Xml(doc_converter::DocConvertArgs),
    /// Convert XML to JSON.
    #[command(name = "xml2json")]
    Xml2Json(doc_converter::DocConvertArgs),
    /// Convert YAML to XML.
    #[command(name = "yaml2xml")]
    Yaml2Xml(doc_converter::DocConvertArgs),
    /// Convert XML to YAML.
    #[command(name = "xml2yaml")]
    Xml2Yaml(doc_converter::DocConvertArgs),
}

pub fn exec(args: ConvertArgs) -> Result<(), CliError> {
    match args.kind {
        ConvertKind::Timestamp(args) => timestamp_converter::exec(args).map_err(Into::into),
        ConvertKind::Storage(args) => unit_converter::exec(args, UnitCategory::Storage).map_err(Into::into),
        ConvertKind::Rate(args) => unit_converter::exec(args, UnitCategory::DataRate).map_err(Into::into),
        ConvertKind::Length(args) => unit_converter::exec(args, UnitCategory::Length).map_err(Into::into),
        ConvertKind::Time(args) => unit_converter::exec(args, UnitCategory::Time).map_err(Into::into),
        ConvertKind::Mass(args) => unit_converter::exec(args, UnitCategory::Mass).map_err(Into::into),
        ConvertKind::Json2Yaml(args) => doc_converter::exec(args, DocKind::Json2Yaml).map_err(Into::into),
        ConvertKind::Yaml2Json(args) => doc_converter::exec(args, DocKind::Yaml2Json).map_err(Into::into),
        ConvertKind::Json2Xml(args) => doc_converter::exec(args, DocKind::Json2Xml).map_err(Into::into),
        ConvertKind::Xml2Json(args) => doc_converter::exec(args, DocKind::Xml2Json).map_err(Into::into),
        ConvertKind::Yaml2Xml(args) => doc_converter::exec(args, DocKind::Yaml2Xml).map_err(Into::into),
        ConvertKind::Xml2Yaml(args) => doc_converter::exec(args, DocKind::Xml2Yaml).map_err(Into::into),
    }
}

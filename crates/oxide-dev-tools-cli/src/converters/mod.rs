pub mod doc_converter;
pub mod timestamp_converter;
pub mod unit_converter;

use clap::{Args, Subcommand};
use oxide_dev_tools_core::{DocKind, UnitCategory};

use crate::error::CliError;

/// `oxide convert ...` — entry point for all converters
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide convert timestamp 1750000000\n  oxide convert storage 1.5 gB --to mib\n  oxide convert json2yaml '{\"a\":1}'"
)]
pub struct ConvertArgs {
    #[command(subcommand)]
    pub kind: ConvertKind,
}

#[derive(Subcommand)]
pub enum ConvertKind {
    /// Convert timestamps between Unix, ISO 8601, and human-readable formats.
    #[command(
        after_help = "Examples:\n  oxide convert timestamp 1750000000\n  oxide convert timestamp 2026-06-07T12:34:56Z --to human\n  oxide convert timestamp 2026-06-07T12:34:56.123456789Z --to unix --unit ms --precision 3\n  oxide convert timestamp 1750000000 --zone Europe/Berlin"
    )]
    Timestamp(timestamp_converter::TimestampArgs),
    /// Convert data storage sizes between bits and bytes (SI and binary prefixes).
    #[command(
        after_help = "Examples:\n  oxide convert storage 1.5 gB --to mib\n  oxide convert storage 1 kb --to KiB\n  oxide convert storage --list"
    )]
    Storage(unit_converter::UnitConvertArgs),
    /// Convert data rates between bit/s and byte/s units.
    #[command(
        after_help = "Examples:\n  oxide convert rate 100 mbit/s --to mb/s\n  oxide convert rate 8 mbps --to mB/s\n  oxide convert rate --list"
    )]
    Rate(unit_converter::UnitConvertArgs),
    /// Convert lengths between metric and imperial units.
    #[command(
        after_help = "Examples:\n  oxide convert length 5 km --to mi\n  oxide convert length 12 in --to cm\n  oxide convert length --list"
    )]
    Length(unit_converter::UnitConvertArgs),
    /// Convert time durations (calendar-aware months and years).
    #[command(
        after_help = "Examples:\n  oxide convert time 90 min --to h\n  oxide convert time 1 y --to d --anchor 2020-01-01\n  oxide convert time --list"
    )]
    Time(unit_converter::UnitConvertArgs),
    /// Convert masses between metric and imperial units.
    #[command(
        after_help = "Examples:\n  oxide convert mass 200 lb --to kg\n  oxide convert mass 1 oz --to g\n  oxide convert mass --list"
    )]
    Mass(unit_converter::UnitConvertArgs),
    /// Convert JSON to YAML.
    #[command(
        name = "json2yaml",
        after_help = "Examples:\n  oxide convert json2yaml '{\"name\":\"oxide\"}'\n  oxide convert json2yaml ./config.json --output ./config.yaml"
    )]
    Json2Yaml(doc_converter::DocConvertArgs),
    /// Convert YAML to JSON.
    #[command(
        name = "yaml2json",
        after_help = "Examples:\n  oxide convert yaml2json 'name: oxide'\n  oxide convert yaml2json ./config.yaml --input-file --pretty"
    )]
    Yaml2Json(doc_converter::DocConvertArgs),
    /// Convert JSON to XML.
    #[command(
        name = "json2xml",
        after_help = "Examples:\n  oxide convert json2xml '{\"a\":1}'\n  oxide convert json2xml '{\"a\":1,\"b\":[true,null]}' --root-name data --pretty"
    )]
    Json2Xml(doc_converter::DocConvertArgs),
    /// Convert XML to JSON.
    #[command(
        name = "xml2json",
        after_help = "Examples:\n  oxide convert xml2json '<root><a>1</a></root>'\n  oxide convert xml2json ./report.xml --input-file --pretty"
    )]
    Xml2Json(doc_converter::DocConvertArgs),
    /// Convert YAML to XML.
    #[command(
        name = "yaml2xml",
        after_help = "Examples:\n  oxide convert yaml2xml 'items: [one, two]'\n  oxide convert yaml2xml ./items.yaml --output ./items.xml"
    )]
    Yaml2Xml(doc_converter::DocConvertArgs),
    /// Convert XML to YAML.
    #[command(
        name = "xml2yaml",
        after_help = "Examples:\n  oxide convert xml2yaml '<root><items>one</items><items>two</items></root>'\n  oxide convert xml2yaml ./report.xml --output ./report.yaml"
    )]
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

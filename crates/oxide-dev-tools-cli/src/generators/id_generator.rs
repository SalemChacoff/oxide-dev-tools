use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `oxide gen id [subcommand]` — ID generator dispatch
#[derive(Args)]
pub struct IdArgs {
    #[command(subcommand)]
    pub kind: IdCmd,
}

#[derive(Subcommand)]
pub enum IdCmd {
    /// UUID v1 (timestamp + MAC)
    #[command(name = "uuidv1")]
    V1 {
        /// Date in ISO 8601 format (e.g., 2026-05-12). Defaults to now.
        date: Option<String>,
    },
    /// UUID v3 (MD5 namespace, deterministic)
    #[command(name = "uuidv3")]
    V3,
    /// UUID v4 (random)
    #[command(name = "uuidv4")]
    V4,
    /// UUID v5 (SHA-1 namespace, deterministic)
    #[command(name = "uuidv5")]
    V5,
    /// UUID v6 (reordered timestamp + MAC)
    #[command(name = "uuidv6")]
    V6 {
        /// Date in ISO 8601 format (e.g., 2026-05-12). Defaults to now.
        date: Option<String>,
    },
    /// UUID v7 (Unix timestamp + random)
    #[command(name = "uuidv7")]
    V7 {
        /// Date in ISO 8601 format (e.g., 2026-05-12). Defaults to now.
        date: Option<String>,
    },
    /// UUID v8 (custom / experimental)
    #[command(name = "uuidv8")]
    V8,
    /// ULID (26-char Crockford base32)
    Ulid,
    /// NanoID (21-char URL-safe)
    #[command(name = "nanoid")]
    NanoId,
}

// Refactor to call generate_id with IdKind enum
pub fn exec(args: IdArgs) {
    match args.kind {
        IdCmd::V1 { date } => {
            let time = parse_date(date);
            println!("{}", generate_id(IdKind::UuidV1(time)));
        }
        IdCmd::V3 => println!("{}", generate_id(IdKind::UuidV3)),
        IdCmd::V4 => println!("{}", generate_id(IdKind::UuidV4)),
        IdCmd::V5 => println!("{}", generate_id(IdKind::UuidV5)),
        IdCmd::V6 { date } => {
            let time = parse_date(date);
            println!("{}", generate_id(IdKind::UuidV6(time)));
        }
        IdCmd::V7 { date } => {
            let time = parse_date(date);
            println!("{}", generate_id(IdKind::UuidV7(time)));
        }
        IdCmd::V8 => println!("{}", generate_id(IdKind::UuidV8)),
        IdCmd::Ulid => println!("{}", generate_id(IdKind::Ulid)),
        IdCmd::NanoId => println!("{}", generate_id(IdKind::NanoId)),
    }
}

/// Parse an ISO 8601 date string (`YYYY-MM-DD`) into [`SystemTime`].
///
/// Returns `None` when no date is given — the caller (core generator)
/// will then use the current time as its default.
fn parse_date(date: Option<String>) -> Option<SystemTime> {
    date.map(|s| {
        let naive = s
            .parse::<chrono::NaiveDate>()
            .unwrap_or_else(|_| panic!("invalid date \"{s}\", expected YYYY-MM-DD"));
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let secs = naive.signed_duration_since(epoch).num_seconds();
        UNIX_EPOCH + Duration::from_secs(secs.max(0) as u64)
    })
}

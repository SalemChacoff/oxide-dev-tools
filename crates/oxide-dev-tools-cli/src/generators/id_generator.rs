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
    V3 {
        /// Namespace UUID (e.g., 6ba7b810-9dad-11d1-80b4-00c04fd430c8)
        #[arg(long)]
        namespace: Option<String>,
        /// Name to hash
        #[arg(long)]
        name: Option<String>,
    },
    /// UUID v4 (random)
    #[command(name = "uuidv4")]
    V4,
    /// UUID v5 (SHA-1 namespace, deterministic)
    #[command(name = "uuidv5")]
    V5 {
        /// Namespace UUID (e.g., 6ba7b810-9dad-11d1-80b4-00c04fd430c8)
        #[arg(long)]
        namespace: Option<String>,
        /// Name to hash
        #[arg(long)]
        name: Option<String>,
    },
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
        IdCmd::V3 { namespace, name } => {
            let params = parse_uuid_params(namespace, name);
            println!("{}", generate_id(IdKind::UuidV3(params)));
        }
        IdCmd::V4 => println!("{}", generate_id(IdKind::UuidV4)),
        IdCmd::V5 { namespace, name } => {
            let params = parse_uuid_params(namespace, name);
            println!("{}", generate_id(IdKind::UuidV5(params)));
        }
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

/// Parse optional `--namespace` and `--name` into an optional tuple for
/// UUID v3/v5 custom generation.
///
/// Returns `None` when neither argument is provided (defaults are used).
/// Panics if only one of them is given.
fn parse_uuid_params(namespace: Option<String>, name: Option<String>) -> Option<(uuid::Uuid, Vec<u8>)> {
    match (namespace, name) {
        (Some(ns), Some(n)) => {
            let uuid = uuid::Uuid::parse_str(&ns).unwrap_or_else(|_| panic!("invalid namespace UUID \"{ns}\""));
            Some((uuid, n.into_bytes()))
        }
        (None, None) => None,
        _ => panic!("--namespace and --name must be provided together"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    // ------------------------------------------------------------------
    // parse_date
    // ------------------------------------------------------------------

    #[test]
    fn parse_date_none_returns_none() {
        assert!(parse_date(None).is_none());
    }

    #[test]
    fn parse_date_valid_date() {
        let t = parse_date(Some("2026-06-01".into())).unwrap();
        let dur = t.duration_since(UNIX_EPOCH).unwrap();
        // 2026-06-01 = days since epoch * 86400
        // Quick calculation: 2026-06-01 is about 56 years after 1970
        assert!(dur.as_secs() > 1_700_000_000, "unexpected timestamp: {}", dur.as_secs());
        assert!(dur.as_secs() < 2_000_000_000, "unexpected timestamp: {}", dur.as_secs());
    }

    #[test]
    #[should_panic(expected = "invalid date")]
    fn parse_date_invalid_format() {
        parse_date(Some("not-a-date".into()));
    }

    #[test]
    #[should_panic(expected = "invalid date")]
    fn parse_date_wrong_order() {
        parse_date(Some("01-06-2026".into()));
    }

    #[test]
    fn parse_date_unix_epoch() {
        let t = parse_date(Some("1970-01-01".into())).unwrap();
        let dur = t.duration_since(UNIX_EPOCH).unwrap();
        assert_eq!(dur.as_secs(), 0);
    }

    // ------------------------------------------------------------------
    // parse_uuid_params
    // ------------------------------------------------------------------

    #[test]
    fn parse_uuid_params_none_returns_none() {
        assert!(parse_uuid_params(None, None).is_none());
    }

    #[test]
    fn parse_uuid_params_valid() {
        let ns = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
        let name = "hello";
        let result = parse_uuid_params(Some(ns.into()), Some(name.into()));
        let (uuid, bytes) = result.expect("expected Some");
        assert_eq!(uuid.to_string(), ns);
        assert_eq!(bytes, name.as_bytes());
    }

    #[test]
    fn parse_uuid_params_unicode_name() {
        let ns = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
        let name = "héllo wörld";
        let result = parse_uuid_params(Some(ns.into()), Some(name.into()));
        let (_, bytes) = result.expect("expected Some");
        assert_eq!(bytes, name.as_bytes());
    }

    #[test]
    #[should_panic(expected = "invalid namespace UUID")]
    fn parse_uuid_params_invalid_uuid() {
        parse_uuid_params(Some("not-a-uuid".into()), Some("x".into()));
    }

    #[test]
    #[should_panic(expected = "must be provided together")]
    fn parse_uuid_params_only_namespace() {
        parse_uuid_params(Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".into()), None);
    }

    #[test]
    #[should_panic(expected = "must be provided together")]
    fn parse_uuid_params_only_name() {
        parse_uuid_params(None, Some("x".into()));
    }

    // ------------------------------------------------------------------
    // exec — smoke tests (verify no panics)
    // ------------------------------------------------------------------

    #[test]
    fn exec_v1_default() {
        exec(IdArgs {
            kind: IdCmd::V1 { date: None },
        });
    }

    #[test]
    fn exec_v1_with_date() {
        exec(IdArgs {
            kind: IdCmd::V1 {
                date: Some("2026-06-01".into()),
            },
        });
    }

    #[test]
    fn exec_v3_default() {
        exec(IdArgs {
            kind: IdCmd::V3 {
                namespace: None,
                name: None,
            },
        });
    }

    #[test]
    fn exec_v3_with_custom() {
        exec(IdArgs {
            kind: IdCmd::V3 {
                namespace: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".into()),
                name: Some("test".into()),
            },
        });
    }

    #[test]
    fn exec_v4() {
        exec(IdArgs { kind: IdCmd::V4 });
    }

    #[test]
    fn exec_v5_default() {
        exec(IdArgs {
            kind: IdCmd::V5 {
                namespace: None,
                name: None,
            },
        });
    }

    #[test]
    fn exec_v5_with_custom() {
        exec(IdArgs {
            kind: IdCmd::V5 {
                namespace: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".into()),
                name: Some("test".into()),
            },
        });
    }

    #[test]
    fn exec_v6_default() {
        exec(IdArgs {
            kind: IdCmd::V6 { date: None },
        });
    }

    #[test]
    fn exec_v6_with_date() {
        exec(IdArgs {
            kind: IdCmd::V6 {
                date: Some("2026-06-01".into()),
            },
        });
    }

    #[test]
    fn exec_v7_default() {
        exec(IdArgs {
            kind: IdCmd::V7 { date: None },
        });
    }

    #[test]
    fn exec_v7_with_date() {
        exec(IdArgs {
            kind: IdCmd::V7 {
                date: Some("2026-06-01".into()),
            },
        });
    }

    #[test]
    fn exec_v8() {
        exec(IdArgs { kind: IdCmd::V8 });
    }

    #[test]
    fn exec_ulid() {
        exec(IdArgs { kind: IdCmd::Ulid });
    }

    #[test]
    fn exec_nanoid() {
        exec(IdArgs { kind: IdCmd::NanoId });
    }
}

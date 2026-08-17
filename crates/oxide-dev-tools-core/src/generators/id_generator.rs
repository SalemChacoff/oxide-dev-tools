use std::fmt;
use std::time::SystemTime;
use uuid::{Uuid, timestamp::Timestamp};

/// Errors that can occur when generating an ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// The requested ID kind is not yet implemented.
    Unsupported(String),
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::Unsupported(kind) => write!(f, "unsupported ID kind: {kind}"),
        }
    }
}

impl std::error::Error for IdError {}

/// Kinds of identifiers that can be generated.
#[derive(Debug)]
pub enum IdKind {
    /// UUID v1 (timestamp + node).
    UuidV1(Option<SystemTime>),
    /// UUID v3 (MD5 namespace + name, deterministic).
    UuidV3(Option<(uuid::Uuid, Vec<u8>)>),
    /// UUID v4 (random).
    UuidV4,
    /// UUID v5 (SHA-1 namespace + name, deterministic).
    UuidV5(Option<(uuid::Uuid, Vec<u8>)>),
    /// UUID v6 (reordered timestamp + node).
    UuidV6(Option<SystemTime>),
    /// UUID v7 (Unix timestamp + random).
    UuidV7(Option<SystemTime>),
    /// UUID v8 (custom / experimental).
    UuidV8,
    /// ULID (26-char Crockford base32).
    Ulid,
    /// NanoID (21-char URL-safe).
    NanoId,
}

/// Generate an identifier according to `kind`.
pub fn generate_id(kind: IdKind) -> Result<String, IdError> {
    match kind {
        IdKind::UuidV1(time) => Ok(gen_uuid_v1(time)),
        IdKind::UuidV3(params) => Ok(match params {
            Some((ns, name)) => gen_uuid_v3_with(&ns, &name),
            None => gen_uuid_v3(),
        }),
        IdKind::UuidV4 => Ok(gen_uuid_v4()),
        IdKind::UuidV5(params) => Ok(match params {
            Some((ns, name)) => gen_uuid_v5_with(&ns, &name),
            None => gen_uuid_v5(),
        }),
        IdKind::UuidV6(time) => Ok(gen_uuid_v6(time)),
        IdKind::UuidV7(time) => Ok(gen_uuid_v7(time)),
        IdKind::UuidV8 => Ok(gen_uuid_v8()),
        IdKind::Ulid => Ok(gen_ulid()),
        IdKind::NanoId => Ok(gen_nanoid()),
    }
}

// -------- UUID v1 (timestamp + MAC/node) --------

const NODE_ID: [u8; 6] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

fn gen_uuid_v1(time: Option<SystemTime>) -> String {
    let ts = time.map_or_else(now_timestamp, system_time_to_timestamp);
    Uuid::new_v1(ts, &NODE_ID).to_string()
}

// -------- UUID v3/v5 (namespace + name) --------

fn gen_uuid_v3() -> String {
    Uuid::new_v3(&uuid::Uuid::NAMESPACE_DNS, b"example.com").to_string()
}

fn gen_uuid_v3_with(ns: &uuid::Uuid, name: &[u8]) -> String {
    Uuid::new_v3(ns, name).to_string()
}

fn gen_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn gen_uuid_v5() -> String {
    Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, b"example.com").to_string()
}

fn gen_uuid_v5_with(ns: &uuid::Uuid, name: &[u8]) -> String {
    Uuid::new_v5(ns, name).to_string()
}

// -------- UUID v6 (reordered timestamp + node) --------

fn gen_uuid_v6(time: Option<SystemTime>) -> String {
    let ts = time.map_or_else(now_timestamp, system_time_to_timestamp);
    Uuid::new_v6(ts, &NODE_ID).to_string()
}

// -------- UUID v7 (unix timestamp + random) --------

fn gen_uuid_v7(time: Option<SystemTime>) -> String {
    let ts = time.map_or_else(now_timestamp, system_time_to_timestamp);
    Uuid::new_v7(ts).to_string()
}

// -------- UUID v8 (custom) --------

fn gen_uuid_v8() -> String {
    let bytes: [u8; 16] = rand::random();
    Uuid::new_v8(bytes).to_string()
}

// -------- Other ID types --------

fn gen_ulid() -> String {
    ulid::Ulid::new().to_string()
}

fn gen_nanoid() -> String {
    nanoid::nanoid!()
}

// -------- Helpers --------

fn now_timestamp() -> Timestamp {
    system_time_to_timestamp(SystemTime::now())
}

fn system_time_to_timestamp(t: SystemTime) -> Timestamp {
    let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap();
    Timestamp::from_unix(uuid::NoContext, dur.as_secs(), dur.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};
    use uuid::Version;

    fn parse(s: &str) -> Uuid {
        Uuid::parse_str(s).expect("not a valid UUID")
    }

    fn known_time() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_234_567_890)
    }

    #[test]
    fn v1_default_is_rfc4122() {
        let id = gen_uuid_v1(None);
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::Mac));
        assert_eq!(u.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn v1_with_fixed_time() {
        let id = gen_uuid_v1(Some(known_time()));
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::Mac));
    }

    #[test]
    fn v3_default_is_rfc4122() {
        let id = gen_uuid_v3();
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::Md5));
        assert_eq!(u.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn v3_is_deterministic() {
        let ns = uuid::Uuid::NAMESPACE_DNS;
        let a = gen_uuid_v3_with(&ns, b"example.com");
        let b = gen_uuid_v3_with(&ns, b"example.com");
        assert_eq!(a, b);
    }

    #[test]
    fn v3_different_inputs_differ() {
        let ns = uuid::Uuid::NAMESPACE_DNS;
        let a = gen_uuid_v3_with(&ns, b"alpha");
        let b = gen_uuid_v3_with(&ns, b"beta");
        assert_ne!(a, b);
    }

    #[test]
    fn v4_is_rfc4122() {
        let id = gen_uuid_v4();
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::Random));
        assert_eq!(u.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn v4_unique_across_1000_samples() {
        let mut set = std::collections::HashSet::new();
        for _ in 0..1_000 {
            assert!(set.insert(gen_uuid_v4()), "collision detected");
        }
    }

    #[test]
    fn v5_default_is_rfc4122() {
        let id = gen_uuid_v5();
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::Sha1));
        assert_eq!(u.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn v5_is_deterministic() {
        let ns = uuid::Uuid::NAMESPACE_DNS;
        let a = gen_uuid_v5_with(&ns, b"example.com");
        let b = gen_uuid_v5_with(&ns, b"example.com");
        assert_eq!(a, b);
    }

    #[test]
    fn v6_default_is_rfc4122() {
        let id = gen_uuid_v6(None);
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::SortMac));
        assert_eq!(u.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn v6_with_fixed_time() {
        let id = gen_uuid_v6(Some(known_time()));
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::SortMac));
    }

    #[test]
    fn v7_default_is_rfc4122() {
        let id = gen_uuid_v7(None);
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::SortRand));
        assert_eq!(u.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn v7_with_fixed_time() {
        let id = gen_uuid_v7(Some(known_time()));
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::SortRand));
    }

    #[test]
    fn v7_unique_across_1000_samples() {
        let mut set = std::collections::HashSet::new();
        for _ in 0..1_000 {
            assert!(set.insert(gen_uuid_v7(None)), "collision detected");
        }
    }

    #[test]
    fn v8_is_valid_uuid() {
        let id = gen_uuid_v8();
        let u = parse(&id);
        assert_eq!(u.get_version(), Some(Version::Custom));
    }

    #[test]
    fn ulid_has_correct_length() {
        let id = gen_ulid();
        assert_eq!(id.len(), 26);
    }

    #[test]
    fn ulid_uses_crockford_base32() {
        let id = gen_ulid();
        assert!(
            id.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "ULID contains invalid characters: {id}"
        );
    }

    #[test]
    fn nanoid_has_default_length() {
        let id = gen_nanoid();
        assert_eq!(id.len(), 21);
    }

    #[test]
    fn nanoid_uses_url_safe_alphabet() {
        let id = gen_nanoid();
        assert!(
            id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "NanoID contains invalid characters: {id}"
        );
    }

    #[test]
    fn generate_returns_non_empty_for_all_kinds() {
        let kinds = vec![
            IdKind::UuidV1(None),
            IdKind::UuidV3(None),
            IdKind::UuidV4,
            IdKind::UuidV5(None),
            IdKind::UuidV6(None),
            IdKind::UuidV7(None),
            IdKind::UuidV8,
            IdKind::Ulid,
            IdKind::NanoId,
        ];
        for kind in kinds {
            let id = generate_id(kind).expect("should generate id");
            assert!(!id.is_empty());
        }
    }
}

use std::time::SystemTime;
use uuid::{Uuid, timestamp::Timestamp};

#[derive(Debug)]
pub enum IdKind {
    UuidV1,
    UuidV3,
    UuidV4,
    UuidV5,
    UuidV6,
    UuidV7,
    UuidV8,
    Ulid,
    NanoId,
    Cuid2,
    Snowflake,
    ObjectId,
    KsuId,
}

pub fn generate(kind: IdKind) -> String {
    match kind {
        IdKind::UuidV1 => gen_uuid_v1(None),
        IdKind::UuidV3 => gen_uuid_v3(),
        IdKind::UuidV4 => gen_uuid_v4(),
        IdKind::UuidV5 => gen_uuid_v5(),
        IdKind::UuidV6 => gen_uuid_v6(None),
        IdKind::UuidV7 => gen_uuid_v7(None),
        IdKind::UuidV8 => gen_uuid_v8(),
        IdKind::Ulid => gen_ulid(),
        IdKind::NanoId => gen_nanoid(),
        _ => todo!(),
    }
}

// -------- UUID v1 (timestamp + MAC/node) --------

const NODE_ID: [u8; 6] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

pub fn gen_uuid_v1(time: Option<SystemTime>) -> String {
    let ts = time.map_or_else(now_timestamp, system_time_to_timestamp);
    Uuid::new_v1(ts, &NODE_ID).to_string()
}

// -------- UUID v3/v5 (namespace + name) --------

pub fn gen_uuid_v3() -> String {
    Uuid::new_v3(&uuid::Uuid::NAMESPACE_DNS, b"example.com").to_string()
}

// UUID v3 with custom namespace
pub fn gen_uuid_v3_with(ns: &uuid::Uuid, name: &[u8]) -> String {
    Uuid::new_v3(ns, name).to_string()
}

// -------- UUID v4 --------

pub fn gen_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

// -------- UUID v5 --------

pub fn gen_uuid_v5() -> String {
    Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, b"example.com").to_string()
}

pub fn gen_uuid_v5_with(ns: &uuid::Uuid, name: &[u8]) -> String {
    Uuid::new_v5(ns, name).to_string()
}

// -------- UUID v6 (reordered timestamp + node) --------

pub fn gen_uuid_v6(time: Option<SystemTime>) -> String {
    let ts = time.map_or_else(now_timestamp, system_time_to_timestamp);
    Uuid::new_v6(ts, &NODE_ID).to_string()
}

// -------- UUID v7 (unix timestamp + random) --------

pub fn gen_uuid_v7(time: Option<SystemTime>) -> String {
    let ts = time.map_or_else(now_timestamp, system_time_to_timestamp);
    Uuid::new_v7(ts).to_string()
}

// -------- UUID v8 (custom) --------

pub fn gen_uuid_v8() -> String {
    // Example: fill 16 bytes with your own scheme
    let bytes: [u8; 16] = rand::random();
    Uuid::new_v8(bytes).to_string()
}

// -------- Other ID types --------

pub fn gen_ulid() -> String {
    ulid::Ulid::new().to_string()
}
pub fn gen_nanoid() -> String {
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

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn parse(s: &str) -> Uuid {
        Uuid::parse_str(s).expect("not a valid UUID")
    }

    fn known_time() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_234_567_890)
    }

    // ------------------------------------------------------------------
    // UUID v1 — timestamp + MAC
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // UUID v3 — MD5 namespace, deterministic
    // ------------------------------------------------------------------

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
        let a = gen_uuid_v3_with(&ns, b"foo");
        let b = gen_uuid_v3_with(&ns, b"bar");
        assert_ne!(a, b);
    }

    // ------------------------------------------------------------------
    // UUID v4 — random
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // UUID v5 — SHA-1 namespace, deterministic
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // UUID v6 — reordered timestamp + MAC
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // UUID v7 — unix timestamp + random
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // UUID v8 — custom
    // ------------------------------------------------------------------

    #[test]
    fn v8_is_valid_uuid() {
        let id = gen_uuid_v8();
        let u = parse(&id); // panics if invalid format
        assert_eq!(u.get_version(), Some(Version::Custom));
    }

    // ------------------------------------------------------------------
    // ULID — 26-char Crockford base32
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // NanoID — 21-char URL-safe alphabet
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // generate() dispatch — all implemented kinds produce something
    // ------------------------------------------------------------------

    #[test]
    fn generate_returns_non_empty_for_all_kinds() {
        for kind in &[
            IdKind::UuidV1,
            IdKind::UuidV3,
            IdKind::UuidV4,
            IdKind::UuidV5,
            IdKind::UuidV6,
            IdKind::UuidV7,
            IdKind::UuidV8,
            IdKind::Ulid,
            IdKind::NanoId,
        ] {
            let id = generate(match kind {
                IdKind::UuidV1 => IdKind::UuidV1,
                IdKind::UuidV3 => IdKind::UuidV3,
                IdKind::UuidV4 => IdKind::UuidV4,
                IdKind::UuidV5 => IdKind::UuidV5,
                IdKind::UuidV6 => IdKind::UuidV6,
                IdKind::UuidV7 => IdKind::UuidV7,
                IdKind::UuidV8 => IdKind::UuidV8,
                IdKind::Ulid => IdKind::Ulid,
                IdKind::NanoId => IdKind::NanoId,
                _ => unreachable!(),
            });
            assert!(!id.is_empty(), "{kind:?} produced an empty string");
        }
    }
}

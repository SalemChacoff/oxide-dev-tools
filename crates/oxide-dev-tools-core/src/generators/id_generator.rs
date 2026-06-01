use std::time::SystemTime;
use uuid::{Uuid, timestamp::Timestamp};

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

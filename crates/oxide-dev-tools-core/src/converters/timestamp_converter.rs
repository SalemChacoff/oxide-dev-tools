//! Timestamp conversion between Unix timestamps, ISO 8601 / RFC 3339 dates,
//! and human-readable formats, in any direction.
//!
//! Parsing is allocation-free on the success path and uses exact integer
//! arithmetic (no floating point), so nanosecond precision is preserved.
//! Dates are converted with O(1) civil-date math instead of calendar
//! iteration.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, FixedOffset, Local, Offset, TimeZone, Utc};

/// Nanoseconds in one second.
const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// Format used for long-form human-readable timestamps,
/// e.g. `June 07, 2026 12:34:56 PM +0000`.
const HUMAN_FORMAT: &str = "%B %d, %Y %I:%M:%S%.f %p %z";

/// Errors that can occur when converting timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimestampError {
    /// The input is not a valid Unix timestamp.
    InvalidUnix(String),
    /// The input is not a valid ISO 8601 / RFC 3339 timestamp.
    InvalidIso8601(String),
    /// The input is not a valid RFC 2822 timestamp.
    InvalidRfc2822(String),
    /// The input is not a valid long-form human-readable timestamp.
    InvalidHuman(String),
    /// The input format could not be detected automatically.
    UnrecognizedFormat(String),
    /// A Unix timestamp or date is outside the supported range.
    UnitOutOfRange(String),
    /// The requested fractional-digit precision is not usable.
    InvalidPrecision(String),
    /// A fixed timezone offset is not valid.
    InvalidOffset(String),
    /// An IANA timezone name is unknown.
    UnknownTimeZone(String),
}

impl fmt::Display for TimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimestampError::InvalidUnix(message) => write!(f, "invalid unix timestamp: {message}"),
            TimestampError::InvalidIso8601(message) => write!(f, "invalid ISO 8601 timestamp: {message}"),
            TimestampError::InvalidRfc2822(message) => write!(f, "invalid RFC 2822 timestamp: {message}"),
            TimestampError::InvalidHuman(message) => write!(f, "invalid human-readable timestamp: {message}"),
            TimestampError::UnrecognizedFormat(message) => write!(f, "{message}"),
            TimestampError::UnitOutOfRange(message) => write!(f, "timestamp out of range: {message}"),
            TimestampError::InvalidPrecision(message) => write!(f, "{message}"),
            TimestampError::InvalidOffset(message) => write!(f, "{message}"),
            TimestampError::UnknownTimeZone(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for TimestampError {}

/// Format of the input value.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampInputFormat {
    /// Detect the format automatically.
    #[default]
    Auto,
    /// Unix timestamp in seconds, milliseconds, microseconds, or nanoseconds.
    Unix,
    /// ISO 8601 / RFC 3339 date.
    Iso8601,
    /// RFC 2822 date (email style, e.g. `Sun, 07 Jun 2026 12:34:56 +0000`).
    Rfc2822,
    /// Long-form human-readable date (e.g. `June 07, 2026 12:34:56 PM +0000`).
    Human,
}

/// Format of the output value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampOutputFormat {
    /// Unix timestamp.
    Unix,
    /// ISO 8601 / RFC 3339 date.
    Iso8601,
    /// RFC 2822 date (email style).
    Rfc2822,
    /// Long-form human-readable date.
    Human,
}

/// Unit used for Unix timestamps.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampUnit {
    /// Whole or fractional seconds.
    #[default]
    Seconds,
    /// Milliseconds.
    Milliseconds,
    /// Microseconds.
    Microseconds,
    /// Nanoseconds.
    Nanoseconds,
}

impl TimestampUnit {
    /// Nanoseconds in one unit of this kind.
    fn scale(self) -> i128 {
        match self {
            TimestampUnit::Seconds => 1_000_000_000,
            TimestampUnit::Milliseconds => 1_000_000,
            TimestampUnit::Microseconds => 1_000,
            TimestampUnit::Nanoseconds => 1,
        }
    }

    /// Maximum meaningful fractional digits for this unit.
    fn fraction_digits(self) -> u32 {
        match self {
            TimestampUnit::Seconds => 9,
            TimestampUnit::Milliseconds => 6,
            TimestampUnit::Microseconds => 3,
            TimestampUnit::Nanoseconds => 0,
        }
    }
}

/// Timezone used when rendering date output.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum TimestampZone {
    /// UTC (output rendered with a `Z` suffix).
    #[default]
    Utc,
    /// The system's local timezone.
    Local,
    /// A fixed offset east of UTC, in seconds (e.g. `+05:30` is 19_800).
    FixedOffset(i32),
    /// An IANA timezone name (e.g. `Europe/Berlin`).
    Iana(String),
}

/// Options for a timestamp conversion.
#[derive(Default, Debug, Clone)]
pub struct TimestampOptions {
    /// Value to convert.
    pub input: String,
    /// Format of the input value.
    pub input_format: TimestampInputFormat,
    /// Format of the output value; `None` picks a smart default
    /// (ISO 8601 for unix input, unix for date input).
    pub output_format: Option<TimestampOutputFormat>,
    /// Unit for unix input and output. For input, `None` auto-detects the
    /// unit by digit count; for output, `None` means seconds.
    pub unit: Option<TimestampUnit>,
    /// Fractional digits (0..=9) for ISO 8601 and unix output, truncated
    /// (not rounded). `None` trims trailing zeros. Human formats always
    /// render whole seconds.
    pub precision: Option<u8>,
    /// Timezone for date output (ignored for unix output).
    pub zone: TimestampZone,
}

/// Operations available on timestamps.
#[derive(Debug)]
pub enum TimestampKind {
    /// Convert a value between timestamp formats.
    Convert(TimestampOptions),
}

/// Convert a timestamp according to `kind`.
pub fn convert_timestamp(kind: TimestampKind) -> Result<String, TimestampError> {
    let TimestampKind::Convert(options) = kind;
    if let Some(digits) = options.precision {
        if digits > 9 {
            return Err(TimestampError::InvalidPrecision(format!("precision {digits} is out of range (0..=9)")));
        }
    }
    let input = options.input.trim();
    if input.is_empty() {
        return Err(TimestampError::UnrecognizedFormat(
            "input is empty; pass a unix timestamp, an ISO 8601 date, an RFC 2822 date, or a human-readable date"
                .to_string(),
        ));
    }
    let (instant, detected) = parse_input(input, options.input_format, options.unit)?;
    let output = options.output_format.unwrap_or_else(|| default_output(detected));
    format_output(instant, output, &options.zone, options.unit, options.precision)
}

/// Parse a timezone argument into a [`TimestampZone`].
///
/// Accepts `utc`, `local`, a fixed offset (`+05:30`, `-0800`), or an IANA
/// timezone name such as `Europe/Berlin` (case-insensitive).
pub fn parse_timestamp_zone(value: &str) -> Result<TimestampZone, TimestampError> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("utc") {
        return Ok(TimestampZone::Utc);
    }
    if value.eq_ignore_ascii_case("local") {
        return Ok(TimestampZone::Local);
    }
    if matches!(value.as_bytes().first(), Some(b'+') | Some(b'-')) {
        return Ok(TimestampZone::FixedOffset(parse_fixed_offset(value)?));
    }
    let zone = find_tz(value)?;
    Ok(TimestampZone::Iana(zone.name().to_string()))
}

// -------- Input parsing --------

/// Format detected for an input value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectedFormat {
    Unix,
    Iso8601,
    Rfc2822,
    Human,
}

fn parse_input(
    input: &str,
    input_format: TimestampInputFormat,
    unit: Option<TimestampUnit>,
) -> Result<(DateTime<Utc>, DetectedFormat), TimestampError> {
    match input_format {
        TimestampInputFormat::Auto => detect(input, unit),
        TimestampInputFormat::Unix => Ok((parse_unix(input, unit)?, DetectedFormat::Unix)),
        TimestampInputFormat::Iso8601 => Ok((parse_iso(input)?, DetectedFormat::Iso8601)),
        TimestampInputFormat::Rfc2822 => Ok((parse_rfc2822(input)?, DetectedFormat::Rfc2822)),
        TimestampInputFormat::Human => Ok((parse_human(input)?, DetectedFormat::Human)),
    }
}

fn detect(input: &str, unit: Option<TimestampUnit>) -> Result<(DateTime<Utc>, DetectedFormat), TimestampError> {
    if looks_like_unix(input) {
        return Ok((parse_unix(input, unit)?, DetectedFormat::Unix));
    }
    if looks_like_rfc2822(input) {
        return Ok((parse_rfc2822(input)?, DetectedFormat::Rfc2822));
    }
    if looks_like_human(input) {
        return Ok((parse_human(input)?, DetectedFormat::Human));
    }
    match parse_iso(input) {
        Ok(instant) => Ok((instant, DetectedFormat::Iso8601)),
        Err(error) if looks_like_iso(input) => Err(error),
        Err(_) => Err(TimestampError::UnrecognizedFormat(format!(
            "could not detect the format of input {input:?}; use --from to force unix, iso, rfc2822, or human"
        ))),
    }
}

fn default_output(detected: DetectedFormat) -> TimestampOutputFormat {
    match detected {
        DetectedFormat::Unix => TimestampOutputFormat::Iso8601,
        DetectedFormat::Iso8601 | DetectedFormat::Rfc2822 | DetectedFormat::Human => TimestampOutputFormat::Unix,
    }
}

/// Cheap probe: four digits, a dash, and at least ten characters — the
/// shape of an ISO 8601 date.
fn looks_like_iso(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() >= 10 && bytes[..4].iter().all(u8::is_ascii_digit) && bytes[4] == b'-'
}

const WEEKDAY_NAMES: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
const MONTH_NAMES: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// Cheap probe: an optional sign followed only by digits and at most one dot.
fn looks_like_unix(input: &str) -> bool {
    let bytes = input.as_bytes();
    let start = usize::from(bytes.first() == Some(&b'-'));
    if start >= bytes.len() || !bytes[start].is_ascii_digit() {
        return false;
    }
    let mut seen_dot = false;
    for byte in &bytes[start..] {
        match byte {
            b'0'..=b'9' => {}
            b'.' if !seen_dot => seen_dot = true,
            _ => return false,
        }
    }
    true
}

/// Cheap probe: a three-letter weekday followed by a comma.
fn looks_like_rfc2822(input: &str) -> bool {
    input.len() >= 4
        && WEEKDAY_NAMES.iter().any(|name| input[..3].eq_ignore_ascii_case(name))
        && input.as_bytes()[3] == b','
}

/// Cheap probe: a full month name followed by a space.
fn looks_like_human(input: &str) -> bool {
    MONTH_NAMES.iter().any(|month| {
        input.len() > month.len()
            && input[..month.len()].eq_ignore_ascii_case(month)
            && input.as_bytes()[month.len()] == b' '
    })
}

// -------- Unix timestamps --------

fn parse_unix(input: &str, unit: Option<TimestampUnit>) -> Result<DateTime<Utc>, TimestampError> {
    let (negative, int_part, frac_part) = split_unix(input)?;
    let value = int_part
        .bytes()
        .fold(0i128, |total, byte| total * 10 + i128::from(byte - b'0'));
    let unit = unit.unwrap_or_else(|| unit_for_digits(int_part.len()));
    let mut total_nanos = value * unit.scale() + fraction_nanos(frac_part, unit);
    if negative {
        total_nanos = -total_nanos;
    }
    let seconds = i64::try_from(total_nanos.div_euclid(NANOS_PER_SECOND))
        .map_err(|_| TimestampError::UnitOutOfRange(format!("input {input:?} exceeds the supported range")))?;
    let nanos = total_nanos.rem_euclid(NANOS_PER_SECOND) as u32;
    DateTime::from_timestamp(seconds, nanos)
        .ok_or_else(|| TimestampError::UnitOutOfRange(format!("input {input:?} is outside ±262144 years")))
}

/// Split a unix value into sign, integer digits, and fractional digits,
/// validating its shape along the way.
fn split_unix(input: &str) -> Result<(bool, &str, &str), TimestampError> {
    let bytes = input.as_bytes();
    let negative = bytes.first() == Some(&b'-');
    let body = &bytes[usize::from(negative)..];
    if body.is_empty() || !body[0].is_ascii_digit() {
        return Err(TimestampError::InvalidUnix(format!("input {input:?} does not start with a digit")));
    }
    let mut dot = None;
    for (index, byte) in body.iter().enumerate() {
        match byte {
            b'0'..=b'9' => {}
            b'.' if dot.is_none() && index > 0 => dot = Some(index),
            _ => {
                return Err(TimestampError::InvalidUnix(format!(
                    "input {input:?} contains unexpected character {:?}",
                    *byte as char
                )));
            }
        }
    }
    let int_len = dot.unwrap_or(body.len());
    if int_len > 19 {
        return Err(TimestampError::UnitOutOfRange(format!("input {input:?} has more than 19 digits")));
    }
    let base = usize::from(negative);
    let int_part = &input[base..base + int_len];
    let frac_part = match dot {
        Some(index) => &input[base + index + 1..],
        None => "",
    };
    if frac_part.len() > 9 {
        return Err(TimestampError::InvalidUnix(format!("input {input:?} has more than 9 fractional digits")));
    }
    Ok((negative, int_part, frac_part))
}

/// Pick a unit by integer digit count: ≤10 seconds, 11–13 milliseconds,
/// 14–16 microseconds, 17–19 nanoseconds.
fn unit_for_digits(digits: usize) -> TimestampUnit {
    match digits {
        1..=10 => TimestampUnit::Seconds,
        11..=13 => TimestampUnit::Milliseconds,
        14..=16 => TimestampUnit::Microseconds,
        _ => TimestampUnit::Nanoseconds,
    }
}

/// Convert a fractional part (digits only) into nanoseconds, truncating
/// beyond the unit's resolution.
fn fraction_nanos(digits: &str, unit: TimestampUnit) -> i128 {
    if digits.is_empty() {
        return 0;
    }
    let value = digits
        .bytes()
        .fold(0i128, |total, byte| total * 10 + i128::from(byte - b'0'));
    value * unit.scale() / 10i128.pow(digits.len() as u32)
}

// -------- ISO 8601 / RFC 3339 --------

/// Parsed fields of a validated ISO 8601 timestamp.
#[derive(Debug, Clone, Copy)]
struct IsoParts {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    fraction_nanos: u32,
    offset_secs: i32,
}

/// Parse and strictly validate an ISO 8601 timestamp with single-pass,
/// allocation-free scanning. Accepted shapes:
/// `YYYY-MM-DD`, `YYYY-MM-DDT HH:MM:SS[.f{1,9}][Z|±HH:MM]` (also lowercase
/// `t`/`z` and a space separator; a missing offset means UTC).
fn parse_iso(input: &str) -> Result<DateTime<Utc>, TimestampError> {
    let parts = iso_parts(input).map_err(|reason| {
        TimestampError::InvalidIso8601(format!("input {input:?} is not a valid ISO 8601 timestamp: {reason}"))
    })?;
    let days = days_from_civil(i64::from(parts.year), parts.month, parts.day);
    let seconds =
        days * 86_400 + i64::from(parts.hour) * 3_600 + i64::from(parts.minute) * 60 + i64::from(parts.second)
            - i64::from(parts.offset_secs);
    DateTime::from_timestamp(seconds, parts.fraction_nanos)
        .ok_or_else(|| TimestampError::InvalidIso8601(format!("input {input:?} is outside ±262144 years")))
}

fn iso_parts(input: &str) -> Result<IsoParts, String> {
    let bytes = input.as_bytes();
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err("expected a YYYY-MM-DD date".to_string());
    }
    let year = four_digits(&bytes[0..4], "year")?;
    let month = two_digits(&bytes[5..7], "month")?;
    let day = two_digits(&bytes[8..10], "day")?;
    validate_month_day(year, month, day)?;
    let rest = &bytes[10..];
    if rest.is_empty() {
        return Ok(IsoParts {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
            fraction_nanos: 0,
            offset_secs: 0,
        });
    }
    if !matches!(rest[0], b'T' | b't' | b' ') {
        return Err(format!("expected a 'T' separator between date and time, found {:?}", rest[0] as char));
    }
    let time = &rest[1..];
    if time.len() < 8 || time[2] != b':' || time[5] != b':' {
        return Err("expected a HH:MM:SS time after the date".to_string());
    }
    let hour = two_digits(&time[0..2], "hour")?;
    let minute = two_digits(&time[3..5], "minute")?;
    let second = two_digits(&time[6..8], "second")?;
    if hour > 23 {
        return Err(format!("hour {hour} out of range (0..=23)"));
    }
    if minute > 59 {
        return Err(format!("minute {minute} out of range (0..=59)"));
    }
    if second > 59 {
        return Err(format!("second {second} out of range (0..=59); leap seconds are not supported"));
    }
    let (fraction_nanos, offset_start) = parse_fraction(time)?;
    let offset_secs = parse_offset_fields(&time[offset_start..])?;
    Ok(IsoParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
        fraction_nanos,
        offset_secs,
    })
}

fn validate_month_day(year: i32, month: u32, day: u32) -> Result<(), String> {
    if !(1..=12).contains(&month) {
        return Err(format!("month {month} out of range (1..=12)"));
    }
    let max_day = days_in_month(year, month);
    if !(1..=max_day).contains(&day) {
        return Err(format!("day {day} out of range for month {month} of year {year} (1..={max_day})"));
    }
    Ok(())
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Parse an optional fractional-second part, returning its nanosecond value
/// and the byte index where any timezone offset starts.
fn parse_fraction(time: &[u8]) -> Result<(u32, usize), String> {
    let Some(after_seconds) = time.get(8..) else {
        return Ok((0, 8));
    };
    if after_seconds.first() != Some(&b'.') {
        return Ok((0, 8));
    }
    let digits = &after_seconds[1..];
    let count = digits.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if count == 0 {
        return Err("expected at least one digit after the decimal point".to_string());
    }
    if count > 9 {
        return Err("at most 9 fractional digits are supported".to_string());
    }
    let value = digits[..count]
        .iter()
        .fold(0u32, |total, byte| total * 10 + u32::from(byte - b'0'));
    Ok((value * 10u32.pow(9 - count as u32), 9 + count))
}

/// Parse a timezone offset: empty, `Z`/`z`, or a single space followed by
/// `Z`/`z` means UTC, otherwise `±HH:MM` (a single leading space is allowed).
fn parse_offset_fields(bytes: &[u8]) -> Result<i32, String> {
    let bytes = match bytes.first() {
        Some(b' ') => &bytes[1..],
        _ => bytes,
    };
    if bytes.is_empty() || bytes == b"Z" || bytes == b"z" {
        return Ok(0);
    }
    if bytes.len() == 6 && matches!(bytes[0], b'+' | b'-') && bytes[3] == b':' {
        let hour = two_digits(&bytes[1..3], "offset hour")?;
        let minute = two_digits(&bytes[4..6], "offset minute")?;
        if hour > 23 {
            return Err(format!("offset hour {hour} out of range (0..=23)"));
        }
        if minute > 59 {
            return Err(format!("offset minute {minute} out of range (0..=59)"));
        }
        let seconds = hour as i32 * 3_600 + minute as i32 * 60;
        return Ok(if bytes[0] == b'-' { -seconds } else { seconds });
    }
    Err(format!("invalid timezone offset {:?}", String::from_utf8_lossy(bytes)))
}

fn four_digits(bytes: &[u8], what: &str) -> Result<i32, String> {
    if !bytes.iter().all(u8::is_ascii_digit) {
        return Err(format!("expected a 4-digit {what}"));
    }
    Ok(bytes
        .iter()
        .fold(0i32, |total, byte| total * 10 + i32::from(byte - b'0')))
}

fn two_digits(bytes: &[u8], what: &str) -> Result<u32, String> {
    if !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return Err(format!("expected a 2-digit {what}"));
    }
    Ok(u32::from(bytes[0] - b'0') * 10 + u32::from(bytes[1] - b'0'))
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let month_prime = (i64::from(month) + 9) % 12;
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

// -------- RFC 2822 --------

fn parse_rfc2822(input: &str) -> Result<DateTime<Utc>, TimestampError> {
    let has_weekday = input.len() >= 4
        && WEEKDAY_NAMES.iter().any(|name| input[..3].eq_ignore_ascii_case(name))
        && input.as_bytes()[3] == b',';
    if !has_weekday {
        return Err(TimestampError::InvalidRfc2822(format!(
            "input {input:?} must start with a weekday followed by a comma (e.g. \"Sun, 07 Jun 2026 12:34:56 +0000\")"
        )));
    }
    let parsed =
        DateTime::parse_from_rfc2822(input).or_else(|_| DateTime::parse_from_rfc2822(&input.to_ascii_lowercase()));
    match parsed {
        Ok(instant) => Ok(instant.with_timezone(&Utc)),
        Err(error) => Err(TimestampError::InvalidRfc2822(format!("input {input:?}: {error}"))),
    }
}

// -------- Long-form human-readable --------

fn parse_human(input: &str) -> Result<DateTime<Utc>, TimestampError> {
    let parsed = DateTime::parse_from_str(input, HUMAN_FORMAT).or_else(|first_error| {
        let mut chars = input.chars();
        match chars.next() {
            Some(first) if first.is_ascii_lowercase() => {
                let capitalized = format!("{}{}", first.to_ascii_uppercase(), chars.as_str());
                DateTime::parse_from_str(&capitalized, HUMAN_FORMAT).map_err(|_| first_error)
            }
            _ => Err(first_error),
        }
    });
    match parsed {
        Ok(instant) => Ok(instant.with_timezone(&Utc)),
        Err(error) => Err(TimestampError::InvalidHuman(format!(
            "input {input:?} (expected e.g. \"June 07, 2026 12:34:56 PM +0000\"): {error}"
        ))),
    }
}

// -------- Output formatting --------

fn format_output(
    instant: DateTime<Utc>,
    output: TimestampOutputFormat,
    zone: &TimestampZone,
    unit: Option<TimestampUnit>,
    precision: Option<u8>,
) -> Result<String, TimestampError> {
    if output == TimestampOutputFormat::Unix {
        return format_unix(&instant, unit.unwrap_or_default(), precision);
    }
    let formatter = ZonedFormatter { output, precision };
    match zone {
        TimestampZone::Utc => Ok(formatter.render(instant)),
        TimestampZone::Local => Ok(formatter.render(instant.with_timezone(&Local))),
        TimestampZone::FixedOffset(seconds) => {
            let offset = FixedOffset::east_opt(*seconds)
                .ok_or_else(|| TimestampError::InvalidOffset(format!("offset {seconds}s is outside ±24 hours")))?;
            Ok(formatter.render(instant.with_timezone(&offset)))
        }
        TimestampZone::Iana(name) => {
            let zone = find_tz(name)?;
            Ok(formatter.render(instant.with_timezone(&zone)))
        }
    }
}

/// Renders a date in a specific zone for one output format.
#[derive(Debug, Clone, Copy)]
struct ZonedFormatter {
    output: TimestampOutputFormat,
    precision: Option<u8>,
}

impl ZonedFormatter {
    fn render<Tz: TimeZone>(&self, instant: DateTime<Tz>) -> String
    where
        Tz::Offset: fmt::Display,
    {
        match self.output {
            TimestampOutputFormat::Iso8601 => format_iso(&instant, self.precision),
            TimestampOutputFormat::Rfc2822 => instant.format("%a, %d %b %Y %H:%M:%S %z").to_string(),
            TimestampOutputFormat::Human => instant.format("%B %d, %Y %I:%M:%S %p %z").to_string(),
            TimestampOutputFormat::Unix => unreachable!("unix output is zone-independent"),
        }
    }
}

/// Render a date as ISO 8601 with exact fractional-digit control
/// (`Z` for UTC, numeric offsets otherwise).
fn format_iso<Tz: TimeZone>(instant: &DateTime<Tz>, precision: Option<u8>) -> String
where
    Tz::Offset: fmt::Display,
{
    let mut out = instant.format("%Y-%m-%dT%H:%M:%S").to_string();
    let nanos = instant.timestamp_subsec_nanos();
    match precision {
        Some(0) => {}
        Some(digits) => {
            let divisor = 10u32.pow(9 - u32::from(digits));
            out.push('.');
            out.push_str(&format!("{:0width$}", nanos / divisor, width = usize::from(digits)));
        }
        None => {
            if nanos != 0 {
                let mut fraction = format!("{nanos:09}");
                while fraction.ends_with('0') {
                    fraction.pop();
                }
                out.push('.');
                out.push_str(&fraction);
            }
        }
    }
    if instant.offset().fix().local_minus_utc() == 0 {
        out.push('Z');
    } else {
        out.push_str(&instant.format("%:z").to_string());
    }
    out
}

/// Render an instant as a unix value in the given unit, with exact
/// fractional-digit control and no floating point.
fn format_unix(instant: &DateTime<Utc>, unit: TimestampUnit, precision: Option<u8>) -> Result<String, TimestampError> {
    let capacity = unit.fraction_digits();
    if let Some(digits) = precision {
        if u32::from(digits) > capacity {
            return Err(TimestampError::InvalidPrecision(format!(
                "precision {digits} exceeds the {capacity} fractional digits available with {unit:?} units"
            )));
        }
    }
    let total_nanos = i128::from(instant.timestamp()) * NANOS_PER_SECOND + i128::from(instant.timestamp_subsec_nanos());
    let magnitude = total_nanos.abs();
    let whole = magnitude / unit.scale();
    let remainder = magnitude % unit.scale();
    let mut out = String::new();
    if total_nanos < 0 {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    match precision {
        Some(0) => {}
        Some(digits) => {
            let shift = capacity - u32::from(digits);
            let fraction = remainder / 10i128.pow(shift);
            out.push('.');
            out.push_str(&format!("{fraction:0width$}", width = digits as usize));
        }
        None => {
            if remainder != 0 {
                let mut fraction = format!("{remainder:0width$}", width = capacity as usize);
                while fraction.ends_with('0') {
                    fraction.pop();
                }
                out.push('.');
                out.push_str(&fraction);
            }
        }
    }
    Ok(out)
}

// -------- Timezones --------

/// Look up an IANA timezone by name, case-insensitively.
fn find_tz(name: &str) -> Result<chrono_tz::Tz, TimestampError> {
    let matched = chrono_tz::Tz::from_str(name).ok().or_else(|| {
        chrono_tz::TZ_VARIANTS
            .iter()
            .copied()
            .find(|zone| zone.name().eq_ignore_ascii_case(name))
    });
    matched.ok_or_else(|| {
        TimestampError::UnknownTimeZone(format!(
            "unknown IANA timezone {name:?} (expected e.g. \"Europe/Berlin\", \"America/New_York\")"
        ))
    })
}

/// Parse a fixed offset in `±HH:MM` or `±HHMM` form into seconds east of UTC.
fn parse_fixed_offset(value: &str) -> Result<i32, TimestampError> {
    let (sign, digits) = match value.as_bytes().first() {
        Some(b'-') => (-1, &value[1..]),
        Some(b'+') => (1, &value[1..]),
        _ => return Err(TimestampError::InvalidOffset(format!("offset {value:?} is not in ±HH:MM format"))),
    };
    let (hour_part, minute_part) = match digits.len() {
        4 => (&digits[..2], &digits[2..]),
        5 if digits.as_bytes()[2] == b':' => (&digits[..2], &digits[3..]),
        _ => return Err(TimestampError::InvalidOffset(format!("offset {value:?} is not in ±HH:MM format"))),
    };
    let malformed = || TimestampError::InvalidOffset(format!("offset {value:?} is not in ±HH:MM format"));
    let hour = hour_part.parse::<i32>().map_err(|_| malformed())?;
    let minute = minute_part.parse::<i32>().map_err(|_| malformed())?;
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) {
        return Err(TimestampError::InvalidOffset(format!("offset {value:?} is outside the ±23:59 range")));
    }
    Ok(sign * (hour * 3_600 + minute * 60))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn convert(input: &str) -> Result<String, TimestampError> {
        convert_timestamp(TimestampKind::Convert(TimestampOptions {
            input: input.to_string(),
            ..TimestampOptions::default()
        }))
    }

    fn with_options(options: TimestampOptions) -> Result<String, TimestampError> {
        convert_timestamp(TimestampKind::Convert(options))
    }

    fn unix_output(input: &str) -> Result<String, TimestampError> {
        with_options(TimestampOptions {
            input: input.to_string(),
            output_format: Some(TimestampOutputFormat::Unix),
            ..TimestampOptions::default()
        })
    }

    // -------- Unix input --------

    #[test]
    fn unix_known_vectors_to_iso() {
        for (input, expected) in [
            ("0", "1970-01-01T00:00:00Z"),
            ("1750000000", "2025-06-15T15:06:40Z"),
            ("-86400", "1969-12-31T00:00:00Z"),
            ("1750000000.5", "2025-06-15T15:06:40.5Z"),
            ("1750000000.123456789", "2025-06-15T15:06:40.123456789Z"),
            ("-0.5", "1969-12-31T23:59:59.5Z"),
        ] {
            assert_eq!(convert(input).unwrap(), expected, "converting {input:?}");
        }
    }

    #[test]
    fn unix_unit_auto_detection_by_digit_count() {
        for input in ["1750000000", "1750000000000", "1750000000000000", "1750000000000000000"] {
            assert_eq!(convert(input).unwrap(), "2025-06-15T15:06:40Z", "converting {input:?}");
        }
    }

    #[test]
    fn unix_unit_digit_boundaries_roundtrip_to_seconds() {
        for (input, expected_seconds) in [
            ("9999999999", "9999999999"),          // 10 digits: seconds
            ("10000000000", "10000000"),           // 11 digits: milliseconds
            ("1000000000000", "1000000000"),       // 13 digits: milliseconds
            ("10000000000000", "10000000"),        // 14 digits: microseconds
            ("1000000000000000", "1000000000"),    // 16 digits: microseconds
            ("10000000000000000", "10000000"),     // 17 digits: nanoseconds
            ("1000000000000000000", "1000000000"), // 19 digits: nanoseconds
        ] {
            assert_eq!(unix_output(input).unwrap(), expected_seconds, "converting {input:?}");
        }
    }

    #[test]
    fn unix_explicit_unit_overrides_detection() {
        let options = TimestampOptions {
            input: "1750000000".to_string(),
            unit: Some(TimestampUnit::Milliseconds),
            ..TimestampOptions::default()
        };
        assert_eq!(with_options(options).unwrap(), "1970-01-21T06:06:40Z");
    }

    #[test]
    fn unix_negative_values_roundtrip() {
        for input in ["-86400", "-0.5", "-1.5", "-1750000000.123"] {
            assert_eq!(unix_output(input).unwrap(), input, "roundtripping {input:?}");
        }
    }

    #[test]
    fn unix_millisecond_fraction_is_scaled() {
        let options = TimestampOptions {
            input: "1750000000000.5".to_string(),
            output_format: Some(TimestampOutputFormat::Unix),
            unit: Some(TimestampUnit::Milliseconds),
            ..TimestampOptions::default()
        };
        assert_eq!(with_options(options).unwrap(), "1750000000000.5");
        assert_eq!(convert("1750000000000.5").unwrap(), "2025-06-15T15:06:40.0005Z");
    }

    #[test]
    fn unix_invalid_inputs_error() {
        let convert_unix = |input: &str| {
            with_options(TimestampOptions {
                input: input.to_string(),
                input_format: TimestampInputFormat::Unix,
                ..TimestampOptions::default()
            })
        };
        assert!(
            convert_unix("123abc")
                .unwrap_err()
                .to_string()
                .contains("unexpected character 'a'")
        );
        assert!(
            convert_unix("1.2.3")
                .unwrap_err()
                .to_string()
                .contains("unexpected character '.'")
        );
        assert!(
            convert_unix(".5")
                .unwrap_err()
                .to_string()
                .contains("does not start with a digit")
        );
        assert!(
            convert_unix("-")
                .unwrap_err()
                .to_string()
                .contains("does not start with a digit")
        );
        assert!(
            convert_unix("12345678901234567890")
                .unwrap_err()
                .to_string()
                .contains("more than 19 digits")
        );
        assert!(
            convert_unix("1.1234567890")
                .unwrap_err()
                .to_string()
                .contains("more than 9 fractional digits")
        );
        let error = with_options(TimestampOptions {
            input: "9223372036854775807".to_string(),
            input_format: TimestampInputFormat::Unix,
            unit: Some(TimestampUnit::Seconds),
            ..TimestampOptions::default()
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("outside"));
    }

    // -------- ISO 8601 input --------

    #[test]
    fn iso_known_vectors_to_unix() {
        for (input, expected) in [
            ("2026-06-07T12:34:56Z", "1780835696"),
            ("2026-06-07T12:34:56+02:00", "1780828496"),
            ("2026-06-07", "1780790400"),
            ("2026-06-07 12:34:56", "1780835696"),
            ("2026-06-07t12:34:56z", "1780835696"),
            ("2026-06-07T12:34:56.5Z", "1780835696.5"),
            ("9999-12-31T23:59:59Z", "253402300799"),
        ] {
            assert_eq!(convert(input).unwrap(), expected, "converting {input:?}");
        }
    }

    #[test]
    fn iso_offsets_shift_the_instant() {
        assert_eq!(convert("2026-06-07T12:34:56-05:30").unwrap(), "1780855496");
        assert_eq!(convert("2026-06-07T12:34:56 +02:00").unwrap(), "1780828496");
    }

    #[test]
    fn iso_date_only_is_midnight_utc() {
        assert_eq!(convert("0000-01-01").unwrap(), "-62167219200");
    }

    #[test]
    fn iso_validation_errors() {
        for (input, expected) in [
            ("2026-02-30", "day 30 out of range for month 2 of year 2026 (1..=28)"),
            ("2026-13-01", "month 13 out of range (1..=12)"),
            ("2026-00-10", "month 0 out of range (1..=12)"),
            ("2026-06-00", "day 0 out of range"),
            ("1900-02-29", "day 29 out of range"), // 1900 is not a leap year
            ("2026-06-07T25:00:00Z", "hour 25 out of range (0..=23)"),
            ("2026-06-07T12:60:00Z", "minute 60 out of range (0..=59)"),
            ("2026-06-07T12:34:60Z", "second 60 out of range"),
            ("2026-06-07T12:34:56+24:00", "offset hour 24 out of range (0..=23)"),
            ("2026-06-07T12:34:56+02:60", "offset minute 60 out of range (0..=59)"),
            ("2026-06-07T12:34:56+02", "invalid timezone offset"),
            ("2026-06-07 12:34", "expected a HH:MM:SS time"),
            ("2026-06-07T12:34:56.1234567890Z", "at most 9 fractional digits"),
            ("2026-06-07T12:34:56.Z", "at least one digit after the decimal point"),
            ("2026-06-07T12:34:56x", "invalid timezone offset"),
        ] {
            let error = convert(input).unwrap_err();
            let message = error.to_string();
            assert!(message.contains(expected), "for {input:?} expected {expected:?}, got {message:?}");
            assert!(message.contains("invalid ISO 8601"), "for {input:?}");
        }
        // "20260607" auto-detects as a unix timestamp; forcing ISO input must reject it.
        let error = with_options(TimestampOptions {
            input: "20260607".to_string(),
            input_format: TimestampInputFormat::Iso8601,
            ..TimestampOptions::default()
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("expected a YYYY-MM-DD date"));
    }

    #[test]
    fn iso_valid_leap_years() {
        assert_eq!(convert("2000-02-29").unwrap(), "951782400");
        assert_eq!(convert("2024-02-29").unwrap(), "1709164800");
    }

    #[test]
    fn iso_date_cross_check_with_chrono() {
        for year in [0, 1, 4, 100, 400, 1582, 1900, 1970, 2000, 2026, 2400, 9999] {
            for month in 1..=12 {
                for day in [1, 15, 28] {
                    if let Some(naive) = NaiveDate::from_ymd_opt(year, month, day) {
                        let input = format!("{year:04}-{month:02}-{day:02}T06:07:08Z");
                        let expected = naive.and_hms_opt(6, 7, 8).unwrap().and_utc().timestamp();
                        assert_eq!(unix_output(&input).unwrap(), expected.to_string(), "for {input:?}");
                    }
                }
            }
        }
    }

    // -------- RFC 2822 input --------

    #[test]
    fn rfc2822_known_vectors() {
        for (input, expected) in [
            ("Sun, 07 Jun 2026 12:34:56 +0000", "1780835696"),
            ("Sun, 07 Jun 2026 12:34:56 GMT", "1780835696"),
            ("Sun, 07 Jun 2026 12:34:56 +0200", "1780828496"),
            ("Thu, 01 Jan 1970 00:00:00 +0000", "0"),
        ] {
            assert_eq!(convert(input).unwrap(), expected, "converting {input:?}");
        }
    }

    #[test]
    fn rfc2822_lowercase_is_tolerated() {
        assert_eq!(convert("sun, 07 jun 2026 12:34:56 +0000").unwrap(), "1780835696");
    }

    #[test]
    fn rfc2822_invalid_inputs_error() {
        let error = with_options(TimestampOptions {
            input: "07 Jun 2026 12:34:56 +0000".to_string(),
            input_format: TimestampInputFormat::Rfc2822,
            ..TimestampOptions::default()
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("invalid RFC 2822"));
    }

    // -------- Human-readable input --------

    #[test]
    fn human_known_vectors() {
        for (input, expected) in [
            ("June 07, 2026 12:34:56 PM +0000", "1780835696"),
            ("June 07, 2026 08:30:00 AM +0000", "1780821000"),
            ("June 07, 2026 12:34:56.25 PM +0000", "1780835696.25"),
            ("January 01, 1970 12:00:00 AM +0000", "0"),
        ] {
            assert_eq!(convert(input).unwrap(), expected, "converting {input:?}");
        }
    }

    #[test]
    fn human_lowercase_month_is_tolerated() {
        assert_eq!(convert("june 07, 2026 12:34:56 PM +0000").unwrap(), "1780835696");
    }

    #[test]
    fn human_invalid_inputs_error() {
        let error = convert("June 07, 2026").unwrap_err().to_string();
        assert!(error.contains("invalid human-readable"));
    }

    // -------- Output formats --------

    #[test]
    fn output_rfc2822_and_human() {
        let to = |output| {
            with_options(TimestampOptions {
                input: "2026-06-07T12:34:56Z".to_string(),
                output_format: Some(output),
                ..TimestampOptions::default()
            })
        };
        assert_eq!(to(TimestampOutputFormat::Rfc2822).unwrap(), "Sun, 07 Jun 2026 12:34:56 +0000");
        assert_eq!(to(TimestampOutputFormat::Human).unwrap(), "June 07, 2026 12:34:56 PM +0000");
    }

    #[test]
    fn output_human_am_and_midnight() {
        let to = |input: &str| {
            with_options(TimestampOptions {
                input: input.to_string(),
                output_format: Some(TimestampOutputFormat::Human),
                ..TimestampOptions::default()
            })
        };
        assert_eq!(to("2026-06-07T08:30:00Z").unwrap(), "June 07, 2026 08:30:00 AM +0000");
        assert_eq!(to("2026-06-07T00:05:00Z").unwrap(), "June 07, 2026 12:05:00 AM +0000");
        assert_eq!(to("2026-06-07T12:05:00Z").unwrap(), "June 07, 2026 12:05:00 PM +0000");
    }

    #[test]
    fn output_human_respects_zone() {
        let options = TimestampOptions {
            input: "2026-06-07T12:34:56Z".to_string(),
            output_format: Some(TimestampOutputFormat::Human),
            zone: TimestampZone::Iana("Europe/Berlin".to_string()),
            ..TimestampOptions::default()
        };
        assert_eq!(with_options(options).unwrap(), "June 07, 2026 02:34:56 PM +0200");
    }

    #[test]
    fn output_defaults_are_smart() {
        assert_eq!(convert("1750000000").unwrap(), "2025-06-15T15:06:40Z");
        assert_eq!(convert("2026-06-07T12:34:56Z").unwrap(), "1780835696");
        assert_eq!(convert("Sun, 07 Jun 2026 12:34:56 +0000").unwrap(), "1780835696");
        assert_eq!(convert("June 07, 2026 12:34:56 PM +0000").unwrap(), "1780835696");
    }

    // -------- Precision --------

    #[test]
    fn precision_controls_iso_fraction() {
        let to_iso = |input: &str, precision: Option<u8>| {
            with_options(TimestampOptions {
                input: input.to_string(),
                output_format: Some(TimestampOutputFormat::Iso8601),
                precision,
                ..TimestampOptions::default()
            })
        };
        let value = "1750000000.123456789";
        assert_eq!(to_iso(value, None).unwrap(), "2025-06-15T15:06:40.123456789Z");
        assert_eq!(to_iso(value, Some(0)).unwrap(), "2025-06-15T15:06:40Z");
        assert_eq!(to_iso(value, Some(3)).unwrap(), "2025-06-15T15:06:40.123Z");
        assert_eq!(to_iso(value, Some(6)).unwrap(), "2025-06-15T15:06:40.123456Z");
        assert_eq!(to_iso(value, Some(9)).unwrap(), "2025-06-15T15:06:40.123456789Z");
        assert_eq!(to_iso("1750000000.5", None).unwrap(), "2025-06-15T15:06:40.5Z");
        assert_eq!(to_iso("1750000000", Some(3)).unwrap(), "2025-06-15T15:06:40.000Z");
    }

    #[test]
    fn precision_controls_unix_fraction() {
        let to_unix = |input: &str, precision: Option<u8>| {
            with_options(TimestampOptions {
                input: input.to_string(),
                input_format: TimestampInputFormat::Unix,
                output_format: Some(TimestampOutputFormat::Unix),
                precision,
                ..TimestampOptions::default()
            })
        };
        assert_eq!(to_unix("1750000000.5", None).unwrap(), "1750000000.5");
        assert_eq!(to_unix("1750000000.5", Some(0)).unwrap(), "1750000000");
        assert_eq!(to_unix("1750000000.5", Some(3)).unwrap(), "1750000000.500");
        assert_eq!(to_unix("1750000000", Some(3)).unwrap(), "1750000000.000");
    }

    #[test]
    fn precision_beyond_unit_resolution_errors() {
        let options = TimestampOptions {
            input: "1750000000000".to_string(),
            input_format: TimestampInputFormat::Unix,
            output_format: Some(TimestampOutputFormat::Unix),
            unit: Some(TimestampUnit::Milliseconds),
            precision: Some(7),
            ..TimestampOptions::default()
        };
        let error = with_options(options).unwrap_err().to_string();
        assert!(error.contains("precision 7 exceeds the 6 fractional digits"));
    }

    #[test]
    fn precision_out_of_range_errors() {
        let error = with_options(TimestampOptions {
            input: "1750000000".to_string(),
            precision: Some(10),
            ..TimestampOptions::default()
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("precision 10 is out of range"));
    }

    // -------- Zones --------

    #[test]
    fn zone_fixed_offset_renders_iso() {
        let options = TimestampOptions {
            input: "2026-06-07T12:34:56Z".to_string(),
            output_format: Some(TimestampOutputFormat::Iso8601),
            zone: TimestampZone::FixedOffset(19_800),
            ..TimestampOptions::default()
        };
        assert_eq!(with_options(options).unwrap(), "2026-06-07T18:04:56+05:30");
    }

    #[test]
    fn zone_iana_handles_dst() {
        let to_iso = |input: &str, zone: &str| {
            with_options(TimestampOptions {
                input: input.to_string(),
                output_format: Some(TimestampOutputFormat::Iso8601),
                zone: parse_timestamp_zone(zone).unwrap(),
                ..TimestampOptions::default()
            })
        };
        assert_eq!(to_iso("2026-06-07T12:34:56Z", "Europe/Berlin").unwrap(), "2026-06-07T14:34:56+02:00");
        assert_eq!(to_iso("2026-01-15T12:00:00Z", "Europe/Berlin").unwrap(), "2026-01-15T13:00:00+01:00");
        assert_eq!(to_iso("2026-06-07T12:34:56Z", "America/New_York").unwrap(), "2026-06-07T08:34:56-04:00");
    }

    #[test]
    fn zone_local_roundtrips_the_instant() {
        let options = TimestampOptions {
            input: "2026-06-07T12:34:56Z".to_string(),
            output_format: Some(TimestampOutputFormat::Iso8601),
            zone: TimestampZone::Local,
            ..TimestampOptions::default()
        };
        let out = with_options(options).unwrap();
        let reparsed = DateTime::parse_from_rfc3339(&out).unwrap().with_timezone(&Utc);
        assert_eq!(reparsed.timestamp(), 1780835696);
    }

    #[test]
    fn zone_is_ignored_for_unix_output() {
        for zone in [
            TimestampZone::Local,
            TimestampZone::FixedOffset(19_800),
            TimestampZone::Iana("Europe/Berlin".to_string()),
        ] {
            let options = TimestampOptions {
                input: "2026-06-07T12:34:56Z".to_string(),
                output_format: Some(TimestampOutputFormat::Unix),
                zone,
                ..TimestampOptions::default()
            };
            assert_eq!(with_options(options).unwrap(), "1780835696");
        }
    }

    #[test]
    fn parse_zone_variants() {
        assert_eq!(parse_timestamp_zone("utc").unwrap(), TimestampZone::Utc);
        assert_eq!(parse_timestamp_zone("UTC").unwrap(), TimestampZone::Utc);
        assert_eq!(parse_timestamp_zone("local").unwrap(), TimestampZone::Local);
        assert_eq!(parse_timestamp_zone("+05:30").unwrap(), TimestampZone::FixedOffset(19_800));
        assert_eq!(parse_timestamp_zone("-0800").unwrap(), TimestampZone::FixedOffset(-28_800));
        assert_eq!(parse_timestamp_zone("Europe/Berlin").unwrap(), TimestampZone::Iana("Europe/Berlin".to_string()));
        assert_eq!(parse_timestamp_zone("europe/berlin").unwrap(), TimestampZone::Iana("Europe/Berlin".to_string()));
    }

    #[test]
    fn parse_zone_errors() {
        assert!(matches!(parse_timestamp_zone("Not/AZone"), Err(TimestampError::UnknownTimeZone(_))));
        assert!(matches!(parse_timestamp_zone("+25:00"), Err(TimestampError::InvalidOffset(_))));
        assert!(matches!(parse_timestamp_zone("+05:60"), Err(TimestampError::InvalidOffset(_))));
        assert!(matches!(parse_timestamp_zone("+5:00"), Err(TimestampError::InvalidOffset(_))));
    }

    // -------- Detection and misc --------

    #[test]
    fn auto_detection_handles_each_format() {
        assert_eq!(convert("1750000000").unwrap(), "2025-06-15T15:06:40Z");
        assert_eq!(convert("2026-06-07T12:34:56Z").unwrap(), "1780835696");
        assert_eq!(convert("Sun, 07 Jun 2026 12:34:56 +0000").unwrap(), "1780835696");
        assert_eq!(convert("June 07, 2026 12:34:56 PM +0000").unwrap(), "1780835696");
    }

    #[test]
    fn input_whitespace_is_trimmed() {
        assert_eq!(convert("  1750000000  ").unwrap(), "2025-06-15T15:06:40Z");
    }

    #[test]
    fn unrecognized_inputs_error() {
        for input in ["", "hello world", "123abc", "2026-06-07T12:34:56.1234567890Z"] {
            let message = convert(input).unwrap_err().to_string();
            assert!(
                message.contains("could not detect")
                    || message.contains("invalid ISO 8601")
                    || message.contains("empty"),
                "unexpected message for {input:?}: {message:?}"
            );
        }
    }

    #[test]
    fn empty_input_mentions_formats() {
        let message = convert("").unwrap_err().to_string();
        assert!(message.contains("unix timestamp"));
        assert!(message.contains("ISO 8601"));
    }

    #[test]
    fn roundtrip_random_instants() {
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..20_000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let seconds = (state as i64) % 6_000_000_000;
            let nanos = ((state >> 32) % 1_000_000_000) as u32;
            let instant = DateTime::from_timestamp(seconds, nanos).unwrap();
            let iso = format!("{}.{nanos:09}Z", instant.format("%Y-%m-%dT%H:%M:%S"));
            let unix = convert(&iso).unwrap();
            let back = with_options(TimestampOptions {
                input: unix,
                output_format: Some(TimestampOutputFormat::Iso8601),
                precision: Some(9),
                ..TimestampOptions::default()
            })
            .unwrap();
            assert_eq!(back, iso);
        }
    }

    #[test]
    fn roundtrip_human_formats() {
        for (input, output) in [
            ("Sun, 07 Jun 2026 12:34:56 +0000", TimestampOutputFormat::Rfc2822),
            ("June 07, 2026 08:30:00 AM +0000", TimestampOutputFormat::Human),
        ] {
            let unix = convert(input).unwrap();
            let back = with_options(TimestampOptions {
                input: unix,
                output_format: Some(output),
                ..TimestampOptions::default()
            })
            .unwrap();
            assert_eq!(back, input);
        }
    }

    #[test]
    fn options_defaults() {
        let options = TimestampOptions::default();
        assert_eq!(options.input, "");
        assert_eq!(options.input_format, TimestampInputFormat::Auto);
        assert_eq!(options.output_format, None);
        assert_eq!(options.unit, None);
        assert_eq!(options.precision, None);
        assert_eq!(options.zone, TimestampZone::Utc);
    }

    #[test]
    fn error_display_is_human_readable() {
        assert_eq!(
            convert("2026-02-30").unwrap_err().to_string(),
            "invalid ISO 8601 timestamp: input \"2026-02-30\" is not a valid ISO 8601 timestamp: day 30 out of range for month 2 of year 2026 (1..=28)"
        );
        assert!(convert("hello").unwrap_err().to_string().contains("use --from"));
    }
}

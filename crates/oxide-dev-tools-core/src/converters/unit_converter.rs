//! Unit conversion across five categories: data storage, data rate, length,
//! time, and mass.
//!
//! All conversions use exact rational arithmetic (no floating point), so
//! values such as `1 in = 2.54 cm` or `1 lb = 0.45359237 kg` come out exact.
//! Storage and rate units support both SI (decimal, ×1000) and IEC (binary,
//! ×1024) prefixes; a trailing lowercase `b` means bit and an uppercase `B`
//! means byte.
//!
//! Time supports calendar-aware months and years: `1 y` is anchored to a
//! reference date (default `1970-01-01`, overridable per conversion) and
//! resolves to 365 or 366 days depending on leap years, exactly like date
//! arithmetic in programming language libraries.

use std::cmp::Ordering;
use std::fmt;

use chrono::{Months, NaiveDate};

/// Maximum number of fractional digits a caller can request.
const MAX_PRECISION: u8 = 18;

/// Default number of fractional digits when the result does not terminate.
const DEFAULT_FRACTION_DIGITS: usize = 12;

/// Upper bound for emitting an exactly terminating decimal expansion.
const MAX_EXACT_DIGITS: usize = 96;

/// Seconds in one day.
const SECONDS_PER_DAY: i128 = 86_400;

/// Default anchor date for calendar-aware time units.
const DEFAULT_ANCHOR: &str = "1970-01-01";

/// Errors that can occur when converting units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitError {
    /// The value is not a plain decimal number.
    InvalidNumber(String),
    /// The source unit is missing and could not be taken from the value.
    MissingSourceUnit(String),
    /// The target unit is missing.
    MissingTargetUnit(String),
    /// The value already carries a unit and a source unit was also given.
    AmbiguousValue(String),
    /// A unit symbol is unknown for the category.
    UnknownUnit {
        /// Human-readable category name.
        category: &'static str,
        /// The unit as typed by the caller.
        unit: String,
        /// Close matches, if any.
        suggestions: Vec<String>,
    },
    /// The requested fractional precision is out of range.
    InvalidPrecision(String),
    /// The calendar anchor is not a valid date.
    InvalidAnchor(String),
    /// A value or intermediate result exceeds the supported range.
    ConversionOverflow(String),
}

impl fmt::Display for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitError::InvalidNumber(message) => write!(f, "invalid number: {message}"),
            UnitError::MissingSourceUnit(message) => write!(f, "{message}"),
            UnitError::MissingTargetUnit(message) => write!(f, "{message}"),
            UnitError::AmbiguousValue(message) => write!(f, "{message}"),
            UnitError::UnknownUnit {
                category,
                unit,
                suggestions,
            } => {
                if suggestions.is_empty() {
                    write!(f, "unknown unit {unit:?} for {category}")
                } else {
                    write!(f, "unknown unit {unit:?} for {category}; did you mean {}?", suggestions.join(", "))
                }
            }
            UnitError::InvalidPrecision(message) => write!(f, "{message}"),
            UnitError::InvalidAnchor(message) => write!(f, "invalid anchor date: {message}"),
            UnitError::ConversionOverflow(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for UnitError {}

/// Category of units a conversion operates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitCategory {
    /// Data storage sizes (bits and bytes).
    Storage,
    /// Data rates (per second).
    DataRate,
    /// Lengths (metric and imperial).
    Length,
    /// Time durations (calendar-aware months and years).
    Time,
    /// Masses (metric and imperial).
    Mass,
}

impl UnitCategory {
    fn name(self) -> &'static str {
        match self {
            UnitCategory::Storage => "data storage",
            UnitCategory::DataRate => "data rate",
            UnitCategory::Length => "length",
            UnitCategory::Time => "time",
            UnitCategory::Mass => "mass",
        }
    }

    fn base(self) -> &'static str {
        match self {
            UnitCategory::Storage => "byte",
            UnitCategory::DataRate => "bit/s",
            UnitCategory::Length => "meter",
            UnitCategory::Time => "second",
            UnitCategory::Mass => "gram",
        }
    }

    fn units(self) -> &'static [UnitDef] {
        match self {
            UnitCategory::Storage => STORAGE_UNITS,
            UnitCategory::DataRate => RATE_UNITS,
            UnitCategory::Length => LENGTH_UNITS,
            UnitCategory::Time => TIME_UNITS,
            UnitCategory::Mass => MASS_UNITS,
        }
    }
}

/// Options for a unit conversion.
#[derive(Default, Debug, Clone)]
pub struct UnitOptions {
    /// Numeric value to convert, optionally with the source unit attached
    /// (e.g. `1.5gB` or `5km`).
    pub value: String,
    /// Source unit when it is not attached to the value.
    pub from: Option<String>,
    /// Target unit to convert into.
    pub to: String,
    /// Fractional digits in the output (0..=18, truncated). `None` emits the
    /// exact terminating expansion when short, otherwise 12 digits with
    /// trailing zeros trimmed.
    pub precision: Option<u8>,
    /// Calendar anchor for month/year time conversions (`YYYY-MM-DD`,
    /// default `1970-01-01`). Ignored for non-time categories.
    pub anchor: Option<String>,
}

/// Operations available on the unit converter.
#[derive(Debug)]
pub enum UnitKind {
    /// Convert a data storage value.
    Storage(UnitOptions),
    /// Convert a data rate value.
    DataRate(UnitOptions),
    /// Convert a length value.
    Length(UnitOptions),
    /// Convert a time value.
    Time(UnitOptions),
    /// Convert a mass value.
    Mass(UnitOptions),
}

/// Convert a unit value according to `kind`.
pub fn convert_unit(kind: UnitKind) -> Result<String, UnitError> {
    let (category, options) = match kind {
        UnitKind::Storage(options) => (UnitCategory::Storage, options),
        UnitKind::DataRate(options) => (UnitCategory::DataRate, options),
        UnitKind::Length(options) => (UnitCategory::Length, options),
        UnitKind::Time(options) => (UnitCategory::Time, options),
        UnitKind::Mass(options) => (UnitCategory::Mass, options),
    };
    if let Some(precision) = options.precision {
        if precision > MAX_PRECISION {
            return Err(UnitError::InvalidPrecision(format!(
                "precision {precision} is out of range (0..={MAX_PRECISION})"
            )));
        }
    }
    let anchor = parse_anchor(options.anchor.as_deref())?;
    let (value, from, to) = resolve_input(&options, category)?;
    let from_def = find_unit(category, &from).ok_or_else(|| unknown_unit(category, &from))?;
    let to_def = find_unit(category, &to).ok_or_else(|| unknown_unit(category, &to))?;
    let value = parse_decimal(&value)?;
    let base = scale_from(value, from_def, anchor)?;
    let result = scale_to(base, to_def, anchor)?;
    Ok(format_ratio(&result, options.precision))
}

/// Render a human-readable listing of every unit in a category.
pub fn unit_catalog(category: UnitCategory) -> String {
    let mut output = format!("{} units (base unit: {}):", category.name(), category.base());
    for def in category.units() {
        output.push_str(&format!("\n  {:<8} {}", def.symbol, def.description));
    }
    output
}

// -------- Unit registry --------

/// Static definition of one unit within a category.
#[derive(Debug)]
struct UnitDef {
    /// Canonical symbol (prefix lowercase, `b`/`B` case-sensitive).
    symbol: &'static str,
    /// Alternative spellings accepted when parsing.
    aliases: &'static [&'static str],
    /// Factor relative to the category base unit as a rational.
    factor_num: i128,
    /// Factor denominator relative to the category base unit.
    factor_den: i128,
    /// Calendar months per unit for calendar-aware time units.
    calendar_months: Option<i64>,
    /// Human-readable description for the unit catalog.
    description: &'static str,
}

macro_rules! unit {
    ($symbol:literal, $num:literal, $den:literal, $desc:literal) => {
        UnitDef {
            symbol: $symbol,
            aliases: &[],
            factor_num: $num,
            factor_den: $den,
            calendar_months: None,
            description: $desc,
        }
    };
    ($symbol:literal, $num:literal, $den:literal, $desc:literal, aliases [$($alias:literal),*]) => {
        UnitDef {
            symbol: $symbol,
            aliases: &[$($alias),*],
            factor_num: $num,
            factor_den: $den,
            calendar_months: None,
            description: $desc,
        }
    };
    ($symbol:literal, $desc:literal, calendar $months:literal) => {
        UnitDef {
            symbol: $symbol,
            aliases: &[],
            factor_num: 0,
            factor_den: 0,
            calendar_months: Some($months),
            description: $desc,
        }
    };
}

static STORAGE_UNITS: &[UnitDef] = &[
    // Bits (SI, decimal)
    unit!("b", 1, 8, "bit (1/8 byte)"),
    unit!("kb", 125, 1, "kilobit (1,000 bits)", aliases["kbit"]),
    unit!("mb", 125_000, 1, "megabit (1,000,000 bits)", aliases["mbit"]),
    unit!("gb", 125_000_000, 1, "gigabit (10^9 bits)", aliases["gbit"]),
    unit!("tb", 125_000_000_000, 1, "terabit (10^12 bits)", aliases["tbit"]),
    unit!("pb", 125_000_000_000_000, 1, "petabit (10^15 bits)", aliases["pbit"]),
    unit!("eb", 125_000_000_000_000_000, 1, "exabit (10^18 bits)", aliases["ebit"]),
    unit!("zb", 125_000_000_000_000_000_000, 1, "zettabit (10^21 bits)", aliases["zbit"]),
    unit!("yb", 125_000_000_000_000_000_000_000, 1, "yottabit (10^24 bits)", aliases["ybit"]),
    // Bytes (SI, decimal)
    unit!("B", 1, 1, "byte", aliases["byte"]),
    unit!("kB", 1_000, 1, "kilobyte (1,000 bytes)", aliases["kbyte"]),
    unit!("mB", 1_000_000, 1, "megabyte (1,000,000 bytes)", aliases["mbyte"]),
    unit!("gB", 1_000_000_000, 1, "gigabyte (10^9 bytes)", aliases["gbyte"]),
    unit!("tB", 1_000_000_000_000, 1, "terabyte (10^12 bytes)", aliases["tbyte"]),
    unit!("pB", 1_000_000_000_000_000, 1, "petabyte (10^15 bytes)", aliases["pbyte"]),
    unit!("eB", 1_000_000_000_000_000_000, 1, "exabyte (10^18 bytes)", aliases["ebyte"]),
    unit!("zB", 1_000_000_000_000_000_000_000, 1, "zettabyte (10^21 bytes)", aliases["zbyte"]),
    unit!("yB", 1_000_000_000_000_000_000_000_000, 1, "yottabyte (10^24 bytes)", aliases["ybyte"]),
    // Bits (IEC, binary)
    unit!("kib", 128, 1, "kibibit (1,024 bits)"),
    unit!("mib", 131_072, 1, "mebibit (1,048,576 bits)"),
    unit!("gib", 134_217_728, 1, "gibibit (2^30 bits)"),
    unit!("tib", 137_438_953_472, 1, "tebibit (2^40 bits)"),
    unit!("pib", 140_737_488_355_328, 1, "pebibit (2^50 bits)"),
    unit!("eib", 144_115_188_075_855_872, 1, "exbibit (2^60 bits)"),
    unit!("zib", 147_573_952_589_676_412_928, 1, "zebibit (2^70 bits)"),
    unit!("yib", 151_115_727_451_828_646_838_272, 1, "yobibit (2^80 bits)"),
    // Bytes (IEC, binary)
    unit!("kiB", 1_024, 1, "kibibyte (1,024 bytes)"),
    unit!("miB", 1_048_576, 1, "mebibyte (2^20 bytes)"),
    unit!("giB", 1_073_741_824, 1, "gibibyte (2^30 bytes)"),
    unit!("tiB", 1_099_511_627_776, 1, "tebibyte (2^40 bytes)"),
    unit!("piB", 1_125_899_906_842_624, 1, "pebibyte (2^50 bytes)"),
    unit!("eiB", 1_152_921_504_606_846_976, 1, "exbibyte (2^60 bytes)"),
    unit!("ziB", 1_180_591_620_717_411_303_424, 1, "zebibyte (2^70 bytes)"),
    unit!("yiB", 1_208_925_819_614_629_174_706_176, 1, "yobibyte (2^80 bytes)"),
];

static RATE_UNITS: &[UnitDef] = &[
    // Bits per second (SI, decimal)
    unit!("b/s", 1, 1, "bit per second", aliases ["bps", "bit/s"]),
    unit!("kb/s", 1_000, 1, "kilobit per second", aliases ["kbps", "kbit/s"]),
    unit!("mb/s", 1_000_000, 1, "megabit per second", aliases ["Mbps", "Mbit/s"]),
    unit!("gb/s", 1_000_000_000, 1, "gigabit per second", aliases ["Gbps", "Gbit/s"]),
    unit!("tb/s", 1_000_000_000_000, 1, "terabit per second", aliases ["Tbps", "Tbit/s"]),
    unit!("pb/s", 1_000_000_000_000_000, 1, "petabit per second", aliases ["Pbps", "Pbit/s"]),
    unit!("eb/s", 1_000_000_000_000_000_000, 1, "exabit per second", aliases ["Ebps", "Ebit/s"]),
    // Bytes per second (SI, decimal)
    unit!("B/s", 8, 1, "byte per second", aliases ["Bps", "byte/s"]),
    unit!("kB/s", 8_000, 1, "kilobyte per second", aliases ["kBps", "kbyte/s"]),
    unit!("mB/s", 8_000_000, 1, "megabyte per second", aliases ["MBps", "Mbyte/s"]),
    unit!("gB/s", 8_000_000_000, 1, "gigabyte per second", aliases ["GBps", "Gbyte/s"]),
    unit!("tB/s", 8_000_000_000_000, 1, "terabyte per second", aliases ["TBps", "Tbyte/s"]),
    unit!("pB/s", 8_000_000_000_000_000, 1, "petabyte per second", aliases ["PBps", "Pbyte/s"]),
    unit!("eB/s", 8_000_000_000_000_000_000, 1, "exabyte per second", aliases ["EBps", "Ebyte/s"]),
    // Bits per second (IEC, binary)
    unit!("kib/s", 1_024, 1, "kibibit per second", aliases ["Kibps", "Kibit/s"]),
    unit!("mib/s", 1_048_576, 1, "mebibit per second", aliases ["Mibps", "Mibit/s"]),
    unit!("gib/s", 1_073_741_824, 1, "gibibit per second", aliases ["Gibps", "Gibit/s"]),
    unit!("tib/s", 1_099_511_627_776, 1, "tebibit per second", aliases ["Tibps", "Tibit/s"]),
    unit!("pib/s", 1_125_899_906_842_624, 1, "pebibit per second", aliases ["Pibps", "Pibit/s"]),
    unit!("eib/s", 1_152_921_504_606_846_976, 1, "exbibit per second", aliases ["Eibps", "Eibit/s"]),
    // Bytes per second (IEC, binary)
    unit!("kiB/s", 8_192, 1, "kibibyte per second", aliases["KiBps"]),
    unit!("miB/s", 8_388_608, 1, "mebibyte per second", aliases["MiBps"]),
    unit!("giB/s", 8_589_934_592, 1, "gibibyte per second", aliases["GiBps"]),
    unit!("tiB/s", 8_796_093_022_208, 1, "tebibyte per second", aliases["TiBps"]),
    unit!("piB/s", 9_007_199_254_740_992, 1, "pebibyte per second", aliases["PiBps"]),
    unit!("eiB/s", 9_223_372_036_854_775_808, 1, "exbibyte per second", aliases["EiBps"]),
];

static LENGTH_UNITS: &[UnitDef] = &[
    unit!("nm", 1, 1_000_000_000, "nanometer"),
    unit!("um", 1, 1_000_000, "micrometer (µm)", aliases ["µm", "micron"]),
    unit!("mm", 1, 1_000, "millimeter"),
    unit!("cm", 1, 100, "centimeter"),
    unit!("dm", 1, 10, "decimeter"),
    unit!("m", 1, 1, "meter", aliases ["meter", "metre"]),
    unit!("dam", 10, 1, "decameter"),
    unit!("hm", 100, 1, "hectometer"),
    unit!("km", 1_000, 1, "kilometer"),
    unit!("in", 254, 10_000, "inch (2.54 cm)", aliases["inch"]),
    unit!("ft", 3_048, 10_000, "foot (12 in)", aliases ["foot", "feet"]),
    unit!("yd", 9_144, 10_000, "yard (3 ft)", aliases["yard"]),
    unit!("mi", 1_609_344, 1_000, "mile (5,280 ft)", aliases["mile"]),
    unit!("nmi", 1_852, 1, "nautical mile"),
];

static TIME_UNITS: &[UnitDef] = &[
    unit!("ns", 1, 1_000_000_000, "nanosecond"),
    unit!("us", 1, 1_000_000, "microsecond (µs)", aliases ["µs", "μs"]),
    unit!("ms", 1, 1_000, "millisecond"),
    unit!("s", 1, 1, "second", aliases ["sec", "second"]),
    unit!("min", 60, 1, "minute", aliases["minute"]),
    unit!("h", 3_600, 1, "hour", aliases ["hr", "hour"]),
    unit!("d", 86_400, 1, "day", aliases["day"]),
    unit!("wk", 604_800, 1, "week", aliases["week"]),
    unit!("mo", "month (calendar, anchor-dependent)", calendar 1),
    unit!("y", "year (calendar, anchor-dependent)", calendar 12),
];

static MASS_UNITS: &[UnitDef] = &[
    unit!("ug", 1, 1_000_000, "microgram (µg)", aliases ["µg", "μg"]),
    unit!("mg", 1, 1_000, "milligram"),
    unit!("g", 1, 1, "gram", aliases["gram"]),
    unit!("kg", 1_000, 1, "kilogram"),
    unit!("t", 1_000_000, 1, "tonne (metric ton, 1,000 kg)", aliases["tonne"]),
    unit!("oz", 28_349_523_125, 1_000_000_000, "ounce (avoirdupois)", aliases["ounce"]),
    unit!("lb", 45_359_237, 100_000, "pound (avoirdupois)", aliases ["lbs", "pound"]),
    unit!("st", 635_029_318, 100_000, "stone (14 lb)", aliases["stone"]),
    unit!("ct", 1, 5, "carat (0.2 g)", aliases["carat"]),
    unit!("tn", 90_718_474, 100, "short ton (2,000 lb)", aliases["shortton"]),
    unit!("lt", 635_029_318, 625, "long ton (2,240 lb)", aliases["longton"]),
];

// -------- Rational arithmetic --------

/// An exact rational number with a positive denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ratio {
    num: i128,
    den: i128,
}

impl Ratio {
    fn reduced(mut self) -> Self {
        let divisor = gcd_u128(self.num.unsigned_abs(), self.den.unsigned_abs());
        if divisor > 1 {
            self.num /= divisor as i128;
            self.den /= divisor as i128;
        }
        self
    }
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn overflow_error() -> UnitError {
    UnitError::ConversionOverflow("value or result is too large".to_string())
}

fn mul_ratio(value: Ratio, num: i128, den: i128) -> Result<Ratio, UnitError> {
    let num = value.num.checked_mul(num).ok_or_else(overflow_error)?;
    let den = value.den.checked_mul(den).ok_or_else(overflow_error)?;
    Ok(Ratio { num, den }.reduced())
}

fn div_ratio(value: Ratio, num: i128, den: i128) -> Result<Ratio, UnitError> {
    let product_num = value.num.checked_mul(den).ok_or_else(overflow_error)?;
    let product_den = value.den.checked_mul(num).ok_or_else(overflow_error)?;
    Ok(Ratio {
        num: product_num,
        den: product_den,
    }
    .reduced())
}

fn add_ratio(left: Ratio, right: Ratio) -> Result<Ratio, UnitError> {
    let left_num = left.num.checked_mul(right.den).ok_or_else(overflow_error)?;
    let right_num = right.num.checked_mul(left.den).ok_or_else(overflow_error)?;
    let num = left_num.checked_add(right_num).ok_or_else(overflow_error)?;
    let den = left.den.checked_mul(right.den).ok_or_else(overflow_error)?;
    Ok(Ratio { num, den }.reduced())
}

fn sub_ratio(left: Ratio, right: Ratio) -> Result<Ratio, UnitError> {
    let left_num = left.num.checked_mul(right.den).ok_or_else(overflow_error)?;
    let right_num = right.num.checked_mul(left.den).ok_or_else(overflow_error)?;
    let num = left_num.checked_sub(right_num).ok_or_else(overflow_error)?;
    let den = left.den.checked_mul(right.den).ok_or_else(overflow_error)?;
    Ok(Ratio { num, den }.reduced())
}

fn compare_ratio_i128(value: &Ratio, other: i128) -> Ordering {
    value.num.cmp(&(other * value.den))
}

// -------- Input parsing --------

/// Parse a plain decimal number (optional sign, optional fraction) as a ratio.
fn parse_decimal(input: &str) -> Result<Ratio, UnitError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(UnitError::InvalidNumber("number is empty".to_string()));
    }
    let (negative, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => match trimmed.strip_prefix('+') {
            Some(rest) => (false, rest),
            None => (false, trimmed),
        },
    };
    let mut num: i128 = 0;
    let mut fraction_digits: u32 = 0;
    let mut seen_point = false;
    let mut seen_digit = false;
    for character in digits.chars() {
        if character == '.' {
            if seen_point {
                return Err(UnitError::InvalidNumber(format!("{input:?} has more than one decimal point")));
            }
            seen_point = true;
        } else if let Some(digit) = character.to_digit(10) {
            seen_digit = true;
            num = num
                .checked_mul(10)
                .and_then(|part| part.checked_add(i128::from(digit)))
                .ok_or_else(|| UnitError::InvalidNumber(format!("{input:?} is too large")))?;
            if seen_point {
                fraction_digits += 1;
            }
        } else {
            return Err(UnitError::InvalidNumber(format!("{input:?} is not a plain decimal number")));
        }
    }
    if !seen_digit {
        return Err(UnitError::InvalidNumber(format!("{input:?} is not a plain decimal number")));
    }
    let den = 10i128
        .checked_pow(fraction_digits)
        .ok_or_else(|| UnitError::InvalidNumber(format!("{input:?} has too many fractional digits")))?;
    let num = if negative {
        num.checked_neg()
            .ok_or_else(|| UnitError::InvalidNumber(format!("{input:?} is too large")))?
    } else {
        num
    };
    Ok(Ratio { num, den }.reduced())
}

/// Normalize a symbol for case-insensitive comparison, folding `µ` into `u`.
fn normalize_text(input: &str) -> String {
    input.to_lowercase().replace(['µ', 'μ'], "u")
}

/// Normalize a bit/byte symbol to canonical form (lowercase prefix,
/// case-sensitive `b`/`B` suffix).
fn normalize_bit_byte(input: &str) -> Option<String> {
    let lower = normalize_text(input);
    if lower == "bit" {
        return Some("b".to_string());
    }
    if lower == "byte" {
        return Some("B".to_string());
    }
    if let Some(prefix) = lower.strip_suffix("bit") {
        return Some(format!("{prefix}b"));
    }
    if let Some(prefix) = lower.strip_suffix("byte") {
        return Some(format!("{prefix}B"));
    }
    let suffix = input.chars().last()?;
    if suffix != 'b' && suffix != 'B' {
        return None;
    }
    let prefix = &lower[..lower.len() - 1];
    if prefix.is_empty() {
        Some(suffix.to_string())
    } else {
        Some(format!("{prefix}{suffix}"))
    }
}

/// Normalize a data-rate symbol to canonical `x/s` form. Only the `/s` and
/// `ps` suffixes are stripped here; spelled `bit`/`byte` forms are resolved
/// by [`normalize_bit_byte`], which preserves the `b`/`B` case.
fn normalize_rate(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let lower = normalize_text(trimmed);
    let core = if lower.ends_with("/s") || lower.ends_with("ps") {
        &trimmed[..trimmed.len() - 2]
    } else {
        return None;
    };
    let base = normalize_bit_byte(core)?;
    Some(format!("{base}/s"))
}

fn normalize_symbol(category: UnitCategory, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    match category {
        UnitCategory::Storage => normalize_bit_byte(trimmed),
        UnitCategory::DataRate => normalize_rate(trimmed),
        UnitCategory::Length | UnitCategory::Time | UnitCategory::Mass => Some(normalize_text(trimmed)),
    }
}

fn find_unit(category: UnitCategory, raw: &str) -> Option<&'static UnitDef> {
    let normalized = normalize_symbol(category, raw)?;
    category
        .units()
        .iter()
        .find(|def| def.symbol == normalized || def.aliases.iter().any(|alias| normalize_text(alias) == normalized))
}

/// Whether `value` ends with `candidate`, honoring the bit/byte case rule.
fn suffix_matches(category: UnitCategory, value: &str, candidate: &str) -> bool {
    let Some(start) = value.len().checked_sub(candidate.len()) else {
        return false;
    };
    let suffix = &value[start..];
    match category {
        UnitCategory::Storage | UnitCategory::DataRate => {
            let candidate_chars = candidate.chars();
            let value_chars = suffix.chars();
            for (expected, actual) in candidate_chars.zip(value_chars) {
                if expected == 'b' || expected == 'B' {
                    if expected != actual {
                        return false;
                    }
                } else if !expected.eq_ignore_ascii_case(&actual) {
                    return false;
                }
            }
            true
        }
        UnitCategory::Length | UnitCategory::Time | UnitCategory::Mass => {
            normalize_text(candidate) == normalize_text(suffix)
        }
    }
}

/// Split a glued value such as `1.5gB` into its number and unit parts.
fn glued_match(category: UnitCategory, value: &str) -> Option<(String, String)> {
    let mut best: Option<(&'static str, &'static UnitDef, usize)> = None;
    for def in category.units() {
        let candidates = std::iter::once(def.symbol).chain(def.aliases.iter().copied());
        for candidate in candidates {
            if suffix_matches(category, value, candidate) && best.is_none_or(|(_, _, len)| candidate.len() > len) {
                best = Some((candidate, def, candidate.len()));
            }
        }
    }
    let (_, def, len) = best?;
    let number = &value[..value.len() - len];
    if parse_decimal(number).is_err() {
        return None;
    }
    Some((number.to_string(), def.symbol.to_string()))
}

/// Split the value plus optional source unit into (number, from, to).
fn resolve_input(options: &UnitOptions, category: UnitCategory) -> Result<(String, String, String), UnitError> {
    let to = options.to.trim();
    if to.is_empty() {
        return Err(UnitError::MissingTargetUnit("missing target unit; pass --to <UNIT>".to_string()));
    }
    let value = options.value.trim();
    if value.is_empty() {
        return Err(UnitError::InvalidNumber("value is empty".to_string()));
    }
    match options.from.as_deref().map(str::trim).filter(|from| !from.is_empty()) {
        Some(from) => {
            if let Err(parse_error) = parse_decimal(value) {
                if glued_match(category, value).is_some() {
                    return Err(UnitError::AmbiguousValue(format!(
                        "value {value:?} already includes a unit; omit the separate unit argument"
                    )));
                }
                return Err(parse_error);
            }
            Ok((value.to_string(), from.to_string(), to.to_string()))
        }
        None => {
            if let Err(parse_error) = parse_decimal(value) {
                match glued_match(category, value) {
                    Some((number, unit)) => return Ok((number, unit, to.to_string())),
                    None => return Err(parse_error),
                }
            }
            Err(UnitError::MissingSourceUnit(format!(
                "value {value:?} has no unit; attach it to the value (e.g. '5km') or pass it as an argument"
            )))
        }
    }
}

// -------- Calendar-aware time units --------

fn parse_anchor(raw: Option<&str>) -> Result<NaiveDate, UnitError> {
    let input = raw
        .map(str::trim)
        .filter(|anchor| !anchor.is_empty())
        .unwrap_or(DEFAULT_ANCHOR);
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .map_err(|_| UnitError::InvalidAnchor(format!("{input:?} is not a valid date (use YYYY-MM-DD)")))
}

fn shift_months(date: NaiveDate, months: i128) -> Result<NaiveDate, UnitError> {
    if months == 0 {
        return Ok(date);
    }
    let count = if months > 0 {
        u32::try_from(months).map_err(|_| overflow_error())?
    } else {
        let magnitude = months.checked_neg().ok_or_else(overflow_error)?;
        u32::try_from(magnitude).map_err(|_| overflow_error())?
    };
    let shifted = if months > 0 {
        date.checked_add_months(Months::new(count))
    } else {
        date.checked_sub_months(Months::new(count))
    };
    shifted.ok_or_else(overflow_error)
}

fn month_span_days(date: NaiveDate, months: i128) -> Result<i64, UnitError> {
    let end = shift_months(date, months)?;
    Ok((end - date).num_days())
}

fn span_days_from_anchor(anchor: NaiveDate, whole: i128, months_per_unit: i64) -> Result<i64, UnitError> {
    let months = whole
        .checked_mul(i128::from(months_per_unit))
        .ok_or_else(overflow_error)?;
    let end = shift_months(anchor, months)?;
    Ok((end - anchor).num_days())
}

/// Resolve a calendar unit value (months/years) into base seconds, anchored
/// at `anchor`. Whole units use true calendar arithmetic; the fractional part
/// scales against the length of the next calendar unit.
fn calendar_to_seconds(value: Ratio, anchor: NaiveDate, months_per_unit: i64) -> Result<Ratio, UnitError> {
    let whole = value.num / value.den;
    let remainder = Ratio {
        num: value.num - whole * value.den,
        den: value.den,
    };
    let months = whole
        .checked_mul(i128::from(months_per_unit))
        .ok_or_else(overflow_error)?;
    let shifted = shift_months(anchor, months)?;
    let whole_days = i128::from((shifted - anchor).num_days());
    let next_days = month_span_days(shifted, i128::from(months_per_unit))?;
    let whole_part = whole_days.checked_mul(value.den).ok_or_else(overflow_error)?;
    let fraction_part = remainder
        .num
        .checked_mul(i128::from(next_days))
        .ok_or_else(overflow_error)?;
    let num = whole_part.checked_add(fraction_part).ok_or_else(overflow_error)?;
    let days = Ratio { num, den: value.den }.reduced();
    mul_ratio(days, SECONDS_PER_DAY, 1)
}

/// Resolve base seconds into a calendar unit value (months/years), anchored
/// at `anchor`: whole units plus a fraction of the following unit.
fn seconds_to_calendar(seconds: Ratio, anchor: NaiveDate, months_per_unit: i64) -> Result<Ratio, UnitError> {
    let days = div_ratio(seconds, SECONDS_PER_DAY, 1)?;
    let whole_days = days.num.checked_div(days.den).ok_or_else(overflow_error)?;
    let estimate = whole_days
        .checked_mul(16)
        .and_then(|part| part.checked_div(487))
        .ok_or_else(overflow_error)?;
    let mut whole = estimate;
    for _ in 0..16 {
        let span = span_days_from_anchor(anchor, whole, months_per_unit)?;
        let next = span_days_from_anchor(anchor, whole + 1, months_per_unit)?;
        let in_unit = compare_ratio_i128(&days, i128::from(span)) != Ordering::Less
            && compare_ratio_i128(&days, i128::from(next)) == Ordering::Less;
        if in_unit {
            let span_ratio = Ratio {
                num: i128::from(span) * days.den,
                den: days.den,
            };
            let remainder = sub_ratio(days, span_ratio)?;
            let fraction = div_ratio(remainder, i128::from(next - span), 1)?;
            return add_ratio(Ratio { num: whole, den: 1 }, fraction);
        }
        if compare_ratio_i128(&days, i128::from(span)) == Ordering::Less {
            whole -= 1;
        } else {
            whole += 1;
        }
    }
    Err(overflow_error())
}

// -------- Conversion and output --------

fn scale_from(value: Ratio, def: &UnitDef, anchor: NaiveDate) -> Result<Ratio, UnitError> {
    match def.calendar_months {
        Some(months_per_unit) => calendar_to_seconds(value, anchor, months_per_unit),
        None => mul_ratio(value, def.factor_num, def.factor_den),
    }
}

fn scale_to(base: Ratio, def: &UnitDef, anchor: NaiveDate) -> Result<Ratio, UnitError> {
    match def.calendar_months {
        Some(months_per_unit) => seconds_to_calendar(base, anchor, months_per_unit),
        None => div_ratio(base, def.factor_num, def.factor_den),
    }
}

/// Render a ratio as a decimal string. Explicit precision truncates; the
/// default emits the exact expansion when it terminates within
/// [`MAX_EXACT_DIGITS`] digits, otherwise 12 digits with trailing zeros
/// trimmed.
fn format_ratio(value: &Ratio, precision: Option<u8>) -> String {
    let negative = value.num < 0;
    let absolute = value.num.unsigned_abs();
    let den = value.den as u128;
    let integer = absolute / den;
    let mut remainder = absolute % den;
    let mut output = integer.to_string();
    if remainder != 0 {
        let mut fraction = String::new();
        match precision {
            Some(precision) => {
                for _ in 0..precision {
                    remainder *= 10;
                    fraction.push(char::from(b'0' + (remainder / den) as u8));
                    remainder %= den;
                }
            }
            None => {
                let mut exact = false;
                for _ in 0..MAX_EXACT_DIGITS {
                    remainder *= 10;
                    fraction.push(char::from(b'0' + (remainder / den) as u8));
                    remainder %= den;
                    if remainder == 0 {
                        exact = true;
                        break;
                    }
                }
                if !exact {
                    fraction.truncate(DEFAULT_FRACTION_DIGITS);
                    while fraction.ends_with('0') {
                        fraction.pop();
                    }
                }
            }
        }
        if !fraction.is_empty() {
            output.push('.');
            output.push_str(&fraction);
        }
    }
    if negative { format!("-{output}") } else { output }
}

// -------- Error helpers --------

fn unknown_unit(category: UnitCategory, raw: &str) -> UnitError {
    UnitError::UnknownUnit {
        category: category.name(),
        unit: raw.to_string(),
        suggestions: suggestions(category, raw),
    }
}

/// Collect close matches for an unknown unit, limited to a handful.
fn suggestions(category: UnitCategory, raw: &str) -> Vec<String> {
    let query = normalize_text(raw);
    let mut scored: Vec<(usize, usize, usize, String)> = Vec::new();
    for def in category.units() {
        for candidate in std::iter::once(def.symbol).chain(def.aliases.iter().copied()) {
            let normalized = normalize_text(candidate);
            let distance = edit_distance(&query, &normalized);
            if distance <= 2 {
                let prefix = common_prefix_len(&query, &normalized);
                scored.push((distance, usize::MAX - prefix, candidate.len(), candidate.to_string()));
            }
        }
    }
    scored.sort();
    scored.dedup_by(|left, right| left.3 == right.3);
    scored.truncate(4);
    scored.into_iter().map(|(_, _, _, candidate)| candidate).collect()
}

/// Length of the shared character prefix of two strings.
fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left_char, right_char)| left_char == right_char)
        .count()
}

/// Levenshtein distance between two small strings.
fn edit_distance(left: &str, right: &str) -> usize {
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];
    for (index, left_char) in left_chars.iter().enumerate() {
        current[0] = index + 1;
        for (offset, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[offset] + usize::from(left_char != right_char);
            current[offset + 1] = substitution.min(previous[offset + 1] + 1).min(current[offset] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(
        category: UnitCategory,
        value: &str,
        from: Option<&str>,
        to: &str,
        precision: Option<u8>,
        anchor: Option<&str>,
    ) -> Result<String, UnitError> {
        let options = UnitOptions {
            value: value.to_string(),
            from: from.map(str::to_string),
            to: to.to_string(),
            precision,
            anchor: anchor.map(str::to_string),
        };
        let kind = match category {
            UnitCategory::Storage => UnitKind::Storage(options),
            UnitCategory::DataRate => UnitKind::DataRate(options),
            UnitCategory::Length => UnitKind::Length(options),
            UnitCategory::Time => UnitKind::Time(options),
            UnitCategory::Mass => UnitKind::Mass(options),
        };
        convert_unit(kind)
    }

    fn storage(value: &str, from: &str, to: &str) -> Result<String, UnitError> {
        convert(UnitCategory::Storage, value, Some(from), to, None, None)
    }

    fn rate(value: &str, from: &str, to: &str) -> Result<String, UnitError> {
        convert(UnitCategory::DataRate, value, Some(from), to, None, None)
    }

    fn length(value: &str, from: &str, to: &str) -> Result<String, UnitError> {
        convert(UnitCategory::Length, value, Some(from), to, None, None)
    }

    fn time(value: &str, from: &str, to: &str) -> Result<String, UnitError> {
        convert(UnitCategory::Time, value, Some(from), to, None, None)
    }

    fn mass(value: &str, from: &str, to: &str) -> Result<String, UnitError> {
        convert(UnitCategory::Mass, value, Some(from), to, None, None)
    }

    // -------- Data storage --------

    #[test]
    fn storage_decimal_and_binary_prefixes() {
        assert_eq!(storage("1", "GB", "MB").unwrap(), "1000");
        assert_eq!(storage("1", "GiB", "MiB").unwrap(), "1024");
        assert_eq!(storage("1", "KiB", "b").unwrap(), "8192");
        assert_eq!(storage("1", "kB", "KiB").unwrap(), "0.9765625");
        assert_eq!(storage("1", "kib", "kiB").unwrap(), "0.125");
        assert_eq!(storage("2", "KiB", "kb").unwrap(), "16.384");
    }

    #[test]
    fn storage_bits_and_bytes_case_rule() {
        assert_eq!(storage("1", "b", "B").unwrap(), "0.125");
        assert_eq!(storage("1", "KB", "kB").unwrap(), "1");
        assert_eq!(storage("1", "kb", "KB").unwrap(), "0.125");
        assert_eq!(storage("1", "Kb", "Kib").unwrap(), "0.9765625");
        assert_eq!(storage("1", "kB", "kiB").unwrap(), "0.9765625");
    }

    #[test]
    fn storage_fractional_input() {
        assert_eq!(storage("1.5", "gB", "mib").unwrap(), "11444.091796875");
        assert_eq!(storage("1.5", "gB", "miB").unwrap(), "1430.511474609375");
        assert_eq!(storage("1.5", "gb", "miB").unwrap(), "178.813934326171875");
    }

    #[test]
    fn storage_extreme_ratio_terminates_exactly() {
        let result = storage("1", "B", "yiB").unwrap();
        assert!(result.starts_with("0.000000000000000000000000827180612553"));
        assert_eq!(result.len(), 2 + 80);
        let back = storage("1", "B", "kiB").unwrap();
        assert_eq!(back, "0.0009765625");
        assert_eq!(storage(&back, "kiB", "B").unwrap(), "1");
    }

    // -------- Data rate --------

    #[test]
    fn rate_bit_and_byte_per_second() {
        assert_eq!(rate("100", "mbit/s", "mb/s").unwrap(), "100");
        assert_eq!(rate("8", "mbps", "mB/s").unwrap(), "1");
        assert_eq!(rate("100", "kbit/s", "mb/s").unwrap(), "0.1");
        assert_eq!(rate("1", "kB/s", "kbps").unwrap(), "8");
        assert_eq!(rate("1", "KiB/s", "kbps").unwrap(), "8.192");
        assert_eq!(rate("1", "B/s", "bps").unwrap(), "8");
    }

    // -------- Length --------

    #[test]
    fn length_metric_vectors() {
        assert_eq!(length("1", "m", "cm").unwrap(), "100");
        assert_eq!(length("1", "um", "nm").unwrap(), "1000");
        assert_eq!(length("1", "m", "um").unwrap(), "1000000");
        assert_eq!(length("1", "km", "m").unwrap(), "1000");
    }

    #[test]
    fn length_imperial_vectors() {
        assert_eq!(length("1", "in", "cm").unwrap(), "2.54");
        assert_eq!(length("1", "ft", "in").unwrap(), "12");
        assert_eq!(length("1", "yd", "ft").unwrap(), "3");
        assert_eq!(length("1", "mi", "km").unwrap(), "1.609344");
        assert_eq!(length("1", "nmi", "km").unwrap(), "1.852");
        assert_eq!(length("1", "km", "mi").unwrap(), "0.621371192237");
        assert_eq!(length("5", "km", "mi").unwrap(), "3.106855961186");
        assert_eq!(length("1", "mi", "ft").unwrap(), "5280");
    }

    // -------- Time --------

    #[test]
    fn time_fixed_unit_vectors() {
        assert_eq!(time("90", "min", "h").unwrap(), "1.5");
        assert_eq!(time("1", "d", "s").unwrap(), "86400");
        assert_eq!(time("1", "h", "min").unwrap(), "60");
        assert_eq!(time("2", "wk", "d").unwrap(), "14");
        assert_eq!(time("1", "us", "ns").unwrap(), "1000");
        assert_eq!(time("0.5", "h", "min").unwrap(), "30");
        assert_eq!(time("-2", "h", "min").unwrap(), "-120");
    }

    #[test]
    fn time_calendar_year_respects_leap_years() {
        assert_eq!(time("1", "y", "d").unwrap(), "365");
        let leap = convert(UnitCategory::Time, "1", Some("y"), "d", None, Some("2020-01-01")).unwrap();
        assert_eq!(leap, "366");
        let leap_half = convert(UnitCategory::Time, "1.5", Some("y"), "d", None, Some("2020-01-01")).unwrap();
        assert_eq!(leap_half, "548.5");
        assert_eq!(time("1", "y", "mo").unwrap(), "12");
        assert_eq!(time("13", "mo", "y").unwrap(), "1.084931506849");
    }

    #[test]
    fn time_calendar_month_uses_true_month_lengths() {
        let january = convert(UnitCategory::Time, "1", Some("mo"), "d", None, Some("2000-01-01")).unwrap();
        assert_eq!(january, "31");
        let two_months = convert(UnitCategory::Time, "2", Some("mo"), "d", None, Some("2000-01-01")).unwrap();
        assert_eq!(two_months, "60");
        let sixty_days = convert(UnitCategory::Time, "60", Some("d"), "mo", None, Some("2000-01-01")).unwrap();
        assert_eq!(sixty_days, "2");
        let thirty_days = convert(UnitCategory::Time, "30", Some("d"), "mo", None, Some("2000-01-01")).unwrap();
        assert_eq!(thirty_days, "0.967741935483");
    }

    // -------- Mass --------

    #[test]
    fn mass_metric_vectors() {
        assert_eq!(mass("1", "t", "kg").unwrap(), "1000");
        assert_eq!(mass("1", "ug", "g").unwrap(), "0.000001");
        assert_eq!(mass("1", "kg", "g").unwrap(), "1000");
    }

    #[test]
    fn mass_imperial_vectors() {
        assert_eq!(mass("1", "lb", "kg").unwrap(), "0.45359237");
        assert_eq!(mass("1", "oz", "g").unwrap(), "28.349523125");
        assert_eq!(mass("1", "kg", "lb").unwrap(), "2.204622621848");
        assert_eq!(mass("200", "lb", "kg").unwrap(), "90.718474");
        assert_eq!(mass("1", "st", "lb").unwrap(), "14");
        assert_eq!(mass("1", "ct", "g").unwrap(), "0.2");
        assert_eq!(mass("1", "tn", "lb").unwrap(), "2000");
        assert_eq!(mass("1", "lt", "kg").unwrap(), "1016.0469088");
    }

    // -------- Glued values and aliases --------

    #[test]
    fn glued_units_are_extracted_from_the_value() {
        assert_eq!(convert(UnitCategory::Storage, "1.5gB", None, "mib", None, None).unwrap(), "11444.091796875");
        assert_eq!(convert(UnitCategory::Storage, "1.5gB", None, "miB", None, None).unwrap(), "1430.511474609375");
        assert_eq!(convert(UnitCategory::Storage, "1.5GB", None, "MB", None, None).unwrap(), "1500");
        assert_eq!(convert(UnitCategory::Length, "5km", None, "mi", None, None).unwrap(), "3.106855961186");
        assert_eq!(convert(UnitCategory::Mass, "200lb", None, "kg", None, None).unwrap(), "90.718474");
        assert_eq!(convert(UnitCategory::Time, "90min", None, "h", None, None).unwrap(), "1.5");
        assert_eq!(convert(UnitCategory::DataRate, "100mbps", None, "mB/s", None, None).unwrap(), "12.5");
    }

    #[test]
    fn unicode_micro_and_imperial_aliases() {
        assert_eq!(length("1", "µm", "nm").unwrap(), "1000");
        assert_eq!(length("1", "feet", "in").unwrap(), "12");
        assert_eq!(mass("2", "lbs", "oz").unwrap(), "32");
        assert_eq!(mass("1", "µg", "mg").unwrap(), "0.001");
        assert_eq!(time("1", "hr", "min").unwrap(), "60");
        assert_eq!(time("1", "μs", "ns").unwrap(), "1000");
    }

    // -------- Precision and rounding behavior --------

    #[test]
    fn precision_truncates_without_rounding() {
        let result = convert(UnitCategory::Mass, "1", Some("lb"), "kg", Some(4), None).unwrap();
        assert_eq!(result, "0.4535");
        let result = convert(UnitCategory::Mass, "1", Some("lb"), "kg", Some(18), None).unwrap();
        assert_eq!(result, "0.453592370000000000");
        let result = convert(UnitCategory::Length, "1", Some("m"), "mi", Some(0), None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn non_terminating_results_use_default_precision() {
        assert_eq!(storage("1", "kB", "kiB").unwrap(), "0.9765625");
        let third = convert(UnitCategory::Time, "1", Some("min"), "wk", None, None).unwrap();
        assert_eq!(third, "0.000099206349");
    }

    // -------- Errors --------

    #[test]
    fn unknown_units_offer_suggestions() {
        let error = storage("1", "kbx", "kB").unwrap_err();
        assert!(error.to_string().contains("unknown unit \"kbx\""));
        assert!(error.to_string().contains("did you mean"));
        assert!(error.to_string().contains("kb"));
        let error = length("1", "furlong", "m").unwrap_err();
        assert!(error.to_string().contains("unknown unit \"furlong\" for length"));
    }

    #[test]
    fn invalid_numbers_error() {
        let error = length("abc", "km", "m").unwrap_err();
        assert!(error.to_string().contains("not a plain decimal number"));
        let error = length("1.2.3", "km", "m").unwrap_err();
        assert!(error.to_string().contains("decimal point"));
        let error = storage("99999999999999999999999999999999999999999999", "kb", "mb").unwrap_err();
        assert!(error.to_string().contains("too large"));
    }

    #[test]
    fn missing_or_ambiguous_units_error() {
        let error = convert(UnitCategory::Length, "5", None, "mi", None, None).unwrap_err();
        assert!(error.to_string().contains("has no unit"));
        let error = convert(UnitCategory::Length, "5km", Some("km"), "mi", None, None).unwrap_err();
        assert!(error.to_string().contains("already includes a unit"));
        let error = convert(UnitCategory::Length, "5", Some("km"), "", None, None).unwrap_err();
        assert!(error.to_string().contains("missing target unit"));
    }

    #[test]
    fn precision_and_anchor_errors() {
        let error = convert(UnitCategory::Mass, "1", Some("lb"), "kg", Some(19), None).unwrap_err();
        assert!(error.to_string().contains("out of range"));
        let error = convert(UnitCategory::Time, "1", Some("y"), "d", None, Some("2020-13-01")).unwrap_err();
        assert!(error.to_string().contains("invalid anchor date"));
    }

    #[test]
    fn zero_converts_to_zero() {
        assert_eq!(length("0", "km", "mi").unwrap(), "0");
        assert_eq!(mass("0", "lb", "kg").unwrap(), "0");
    }

    // -------- Registry-wide coverage --------

    #[test]
    fn every_unit_roundtrips_through_the_base() {
        let cases = [
            (UnitCategory::Storage, "B"),
            (UnitCategory::DataRate, "b/s"),
            (UnitCategory::Length, "m"),
            (UnitCategory::Time, "s"),
            (UnitCategory::Mass, "g"),
        ];
        for (category, base) in cases {
            for def in category.units() {
                let to_base = convert(category, "1", Some(def.symbol), base, None, None).unwrap();
                let back = convert(category, &to_base, Some(base), def.symbol, None, None).unwrap();
                let delta = (back.parse::<f64>().unwrap() - 1.0).abs();
                assert!(delta < 1e-9, "roundtrip drifted for {} {}: {to_base} -> {back}", category.name(), def.symbol);
            }
        }
    }

    #[test]
    fn calendar_units_roundtrip_exactly() {
        for def in UnitCategory::Time
            .units()
            .iter()
            .filter(|def| def.calendar_months.is_some())
        {
            let to_base = convert(UnitCategory::Time, "1", Some(def.symbol), "s", None, None).unwrap();
            let back = convert(UnitCategory::Time, &to_base, Some("s"), def.symbol, None, None).unwrap();
            assert_eq!(back, "1", "calendar roundtrip failed for {}", def.symbol);
        }
    }

    #[test]
    fn catalog_lists_every_category() {
        let storage = unit_catalog(UnitCategory::Storage);
        assert!(storage.contains("data storage units"));
        assert!(storage.contains("kiB"));
        assert!(storage.contains("kib"));
        let length = unit_catalog(UnitCategory::Length);
        assert!(length.contains("nmi"));
        let mass = unit_catalog(UnitCategory::Mass);
        assert!(mass.contains("tonne"));
        let time = unit_catalog(UnitCategory::Time);
        assert!(time.contains("calendar"));
    }
}

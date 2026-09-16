//! Password strength analysis (zxcvbn-based scoring, pattern detection, and
//! crack-time estimates).
//!
//! [`validate_password`] analyzes a password with the
//! [`zxcvbn`](https://crates.io/crates/zxcvbn) estimator: dictionary and
//! pattern matching (keyboard walks, sequences, repeats, dates, l33t
//! substitutions), a guess-count estimate, and crack-time projections for
//! four attack scenarios. A minimum acceptable score can be required with
//! [`PasswordStrengthOptions::min_score`], and personalized words (names,
//! usernames, email addresses) can be supplied with
//! [`PasswordStrengthOptions::user_inputs`] so they are treated as
//! easy-to-guess dictionary entries.
//!
//! The analysis is heuristic: a low score means "easy to guess by the
//! methods zxcvbn models", not "actually compromised". For confidentiality,
//! the report deliberately never echoes the password — it only carries the
//! length, character-class counts, and derived metrics.
//!
//! Note: zxcvbn analyzes at most the first 100 characters; longer inputs
//! are flagged with a warning.

// -------- Public API --------

/// Strength band for a zxcvbn score from 0 to 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasswordStrength {
    /// Score 0: crackable with 10^3 guesses or fewer.
    #[default]
    VeryWeak,
    /// Score 1: crackable with 10^6 guesses or fewer.
    Weak,
    /// Score 2: crackable with 10^8 guesses or fewer.
    Fair,
    /// Score 3: crackable with 10^10 guesses or fewer.
    Strong,
    /// Score 4: requires more than 10^10 guesses.
    VeryStrong,
}

impl From<u8> for PasswordStrength {
    fn from(score: u8) -> Self {
        match score {
            0 => PasswordStrength::VeryWeak,
            1 => PasswordStrength::Weak,
            2 => PasswordStrength::Fair,
            3 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }
}

impl PasswordStrength {
    /// Human-readable label for the strength band.
    pub fn label(self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "very weak",
            PasswordStrength::Weak => "weak",
            PasswordStrength::Fair => "fair",
            PasswordStrength::Strong => "strong",
            PasswordStrength::VeryStrong => "very strong",
        }
    }
}

/// Options for [`validate_password`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PasswordStrengthOptions {
    /// The password to analyze.
    pub input: String,
    /// Minimum acceptable score (0-4); 0 means "analysis only". Values above
    /// 4 are reported as an issue.
    pub min_score: u8,
    /// Words the password should not be based on (names, usernames, email
    /// addresses); they are scored like dictionary entries.
    pub user_inputs: Vec<String>,
}

/// Crack-time projections under four attack scenarios.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrackTimeReport {
    /// Online attack on a rate-limited service (100 guesses per hour).
    pub online_throttled: String,
    /// Online attack without rate limiting (10 guesses per second).
    pub online_unthrottled: String,
    /// Offline attack against a slow hash function (10,000 guesses/second).
    pub offline_slow: String,
    /// Offline attack against a fast hash function (10 billion guesses/second).
    pub offline_fast: String,
}

/// The result of analyzing a password.
///
/// The raw password is never stored: only its length, character-class
/// counts, and derived metrics are reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordReport {
    /// Whether the password meets the configured requirements.
    pub valid: bool,
    /// Overall strength score from 0 to 4 (zxcvbn scale).
    pub score: u8,
    /// Strength band corresponding to [`PasswordReport::score`].
    pub strength: PasswordStrength,
    /// Number of characters in the password.
    pub input_len: usize,
    /// Total Shannon entropy of the character distribution, in bits
    /// (rounded).
    pub entropy_bits: u32,
    /// Order of magnitude of the estimated guess count (rounded log10).
    pub guesses_log10: u32,
    /// Number of ASCII lowercase characters.
    pub lowercase: usize,
    /// Number of ASCII uppercase characters.
    pub uppercase: usize,
    /// Number of ASCII digits.
    pub digits: usize,
    /// Number of ASCII symbols (printable, non-alphanumeric).
    pub symbols: usize,
    /// Number of characters outside the classes above (e.g. spaces, accented
    /// letters).
    pub other: usize,
    /// Human-readable descriptions of the patterns zxcvbn matched.
    pub patterns: Vec<String>,
    /// Estimated time to crack under the four attack scenarios.
    pub crack_times: CrackTimeReport,
    /// Reasons the password fails the configured requirements.
    pub issues: Vec<String>,
    /// Advisory findings that do not fail validation.
    pub warnings: Vec<String>,
    /// Suggestions for choosing a stronger password.
    pub suggestions: Vec<String>,
}

/// Analyze a password and produce a detailed strength report.
///
/// Analysis never fails with an error and never panics: an unacceptable
/// password is reported via [`PasswordReport::valid`] and
/// [`PasswordReport::issues`].
pub fn validate_password(options: PasswordStrengthOptions) -> PasswordReport {
    let mut report = PasswordReport {
        valid: false,
        score: 0,
        strength: PasswordStrength::VeryWeak,
        input_len: 0,
        entropy_bits: 0,
        guesses_log10: 0,
        lowercase: 0,
        uppercase: 0,
        digits: 0,
        symbols: 0,
        other: 0,
        patterns: Vec::new(),
        crack_times: CrackTimeReport::default(),
        issues: Vec::new(),
        warnings: Vec::new(),
        suggestions: Vec::new(),
    };

    if options.input.is_empty() {
        report.issues.push("input is empty".to_string());
        return report;
    }
    if options.min_score > 4 {
        report
            .issues
            .push(format!("minimum score {} is outside the supported range (0-4)", options.min_score));
    }

    report.input_len = options.input.chars().count();
    classify_chars(&options.input, &mut report);
    report.entropy_bits = shannon_entropy_bits(&options.input);

    let user_inputs: Vec<&str> = options.user_inputs.iter().map(String::as_str).collect();
    let entropy = zxcvbn::zxcvbn(&options.input, &user_inputs);

    report.score = u8::from(entropy.score());
    report.strength = PasswordStrength::from(report.score);
    report.guesses_log10 = entropy.guesses_log10().round() as u32;
    report.patterns = describe_patterns(entropy.sequence());

    let crack = entropy.crack_times();
    report.crack_times = CrackTimeReport {
        online_throttled: crack.online_throttling_100_per_hour().to_string(),
        online_unthrottled: crack.online_no_throttling_10_per_second().to_string(),
        offline_slow: crack.offline_slow_hashing_1e4_per_second().to_string(),
        offline_fast: crack.offline_fast_hashing_1e10_per_second().to_string(),
    };

    if let Some(feedback) = entropy.feedback() {
        if let Some(warning) = feedback.warning() {
            report.warnings.push(warning.to_string());
        }
        report.suggestions = feedback.suggestions().iter().map(ToString::to_string).collect();
    }
    if report.input_len > 100 {
        report
            .warnings
            .push("analysis covers the first 100 characters only".to_string());
    }

    if report.score < options.min_score {
        report
            .issues
            .push(format!("score {} is below the required minimum of {}", report.score, options.min_score));
    }
    report.valid = report.issues.is_empty();
    report
}

// -------- Character classes --------

/// Count the characters of each class in `input` and store the counts in
/// `report`.
fn classify_chars(input: &str, report: &mut PasswordReport) {
    for ch in input.chars() {
        if ch.is_ascii_lowercase() {
            report.lowercase += 1;
        } else if ch.is_ascii_uppercase() {
            report.uppercase += 1;
        } else if ch.is_ascii_digit() {
            report.digits += 1;
        } else if ch.is_ascii_graphic() {
            report.symbols += 1;
        } else {
            report.other += 1;
        }
    }
}

/// Total Shannon entropy of the character distribution (per-character
/// entropy times the character count), rounded to whole bits.
fn shannon_entropy_bits(input: &str) -> u32 {
    let mut counts = std::collections::HashMap::<char, f64>::new();
    let mut total = 0.0;
    for ch in input.chars() {
        *counts.entry(ch).or_default() += 1.0;
        total += 1.0;
    }
    if total == 0.0 {
        return 0;
    }
    let mut per_character = 0.0;
    for count in counts.values() {
        let probability = count / total;
        per_character -= probability * probability.log2();
    }
    (per_character * total).round() as u32
}

// -------- Pattern descriptions --------

/// Describe the matched patterns in human-readable form, skipping
/// bruteforce regions (they carry no structural information) and dropping
/// duplicate descriptions.
fn describe_patterns(sequence: &[zxcvbn::Match]) -> Vec<String> {
    use zxcvbn::matching::patterns::MatchPattern;

    let mut descriptions = Vec::new();
    for item in sequence {
        let description = match &item.pattern {
            MatchPattern::Dictionary(pattern) => {
                let mut text = format!(
                    "dictionary: {} ({}, rank {})",
                    pattern.matched_word,
                    dictionary_label(&pattern.dictionary_name),
                    pattern.rank
                );
                if pattern.l33t {
                    text.push_str(" (l33t");
                    if let Some(substitution) = &pattern.sub_display {
                        text.push_str(&format!(": {substitution}"));
                    }
                    text.push(')');
                }
                if pattern.reversed {
                    text.push_str(" (reversed)");
                }
                text
            }
            MatchPattern::Spatial(pattern) => {
                format!("keyboard: {} ({} graph, {} turns)", item.token, pattern.graph, pattern.turns)
            }
            MatchPattern::Repeat(pattern) => {
                format!("repeat: {} ({} repeated {} times)", item.token, pattern.base_token, pattern.repeat_count)
            }
            MatchPattern::Sequence(pattern) => {
                let direction = if pattern.ascending { "ascending" } else { "descending" };
                format!("sequence: {} ({}, {direction})", item.token, pattern.sequence_name)
            }
            MatchPattern::Regex(pattern) => format!("regex: {} ({})", item.token, pattern.regex_name),
            MatchPattern::Date(pattern) => {
                format!("date: {} ({:04}-{:02}-{:02})", item.token, pattern.year, pattern.month, pattern.day)
            }
            MatchPattern::BruteForce => continue,
        };
        if !descriptions.contains(&description) {
            descriptions.push(description);
        }
    }
    descriptions
}

/// Map zxcvbn's (crate-private) dictionary names to human-readable labels.
///
/// The name is matched through its `Debug` representation because
/// `DictionaryType` is not re-exported by the zxcvbn crate; unknown variants
/// fall back to the debug name itself.
fn dictionary_label(dictionary: &impl std::fmt::Debug) -> String {
    match format!("{dictionary:?}").as_str() {
        "Passwords" => "common passwords",
        "English" => "English words",
        "FemaleNames" => "female names",
        "MaleNames" => "male names",
        "Surnames" => "surnames",
        "UsTvAndFilm" => "US TV and film",
        "UserInputs" => "user input",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(input: &str) -> PasswordReport {
        validate_password(PasswordStrengthOptions {
            input: input.to_string(),
            ..PasswordStrengthOptions::default()
        })
    }

    fn analyze_with(input: &str, min_score: u8, user_inputs: &[&str]) -> PasswordReport {
        validate_password(PasswordStrengthOptions {
            input: input.to_string(),
            min_score,
            user_inputs: user_inputs.iter().map(|word| (*word).to_string()).collect(),
        })
    }

    #[test]
    fn empty_input_is_rejected() {
        let report = analyze("");
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("input is empty")));
    }

    #[test]
    fn common_password_scores_zero() {
        let report = analyze("password");
        assert_eq!(report.score, 0);
        assert_eq!(report.strength, PasswordStrength::VeryWeak);
        assert!(report.valid);
        assert!(report.patterns.iter().any(|pattern| pattern.starts_with("dictionary:")));
        assert!(report.patterns.iter().any(|pattern| pattern.contains("rank 2")));
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("common password"))
        );
        assert!(!report.suggestions.is_empty());
    }

    #[test]
    fn keyboard_pattern_is_detected() {
        let report = analyze("!QAZ@WSX");
        assert!(report.score <= 2);
        assert!(report.patterns.iter().any(|pattern| pattern.starts_with("keyboard:")));
    }

    #[test]
    fn sequence_is_detected() {
        let report = analyze("abcdefghij");
        assert!(report.score <= 1);
        assert!(report.patterns.iter().any(|pattern| pattern.starts_with("sequence:")));
    }

    #[test]
    fn repeat_is_detected() {
        let report = analyze("aaaaaaaaaa");
        assert!(report.score <= 1);
        assert!(report.patterns.iter().any(|pattern| pattern.starts_with("repeat:")));
    }

    #[test]
    fn date_is_detected() {
        let report = analyze("alice12/25/1990");
        assert!(report.score <= 2);
        assert!(report.patterns.iter().any(|pattern| pattern.starts_with("date:")));
    }

    #[test]
    fn recent_years_are_detected() {
        let report = analyze("alice1990");
        assert!(report.score <= 2);
        assert!(report.patterns.iter().any(|pattern| pattern.contains("recent_year")));
    }

    #[test]
    fn l33t_substitutions_are_detected() {
        let report = analyze("p4ssw0rd");
        assert_eq!(report.score, 0);
        assert!(report.patterns.iter().any(|pattern| pattern.contains("l33t")));
    }

    #[test]
    fn passphrase_scores_high() {
        let report = analyze("correct horse battery staple");
        assert!(report.score >= 3);
        assert!(report.valid);
    }

    #[test]
    fn random_password_scores_full() {
        let report = analyze("v8#Kq2$mL9@pR4!zT7^wN5&sF1*cD3%");
        assert_eq!(report.score, 4);
        assert_eq!(report.strength, PasswordStrength::VeryStrong);
    }

    #[test]
    fn user_inputs_collapse_the_score() {
        let report = analyze_with("alice1990", 0, &["alice"]);
        assert!(report.score <= 1);
        assert!(report.patterns.iter().any(|pattern| pattern.contains("user input")));
    }

    #[test]
    fn min_score_gate_is_enforced() {
        let strong = analyze_with("v8#Kq2$mL9@pR4!zT7^wN5&sF1*cD3%", 4, &[]);
        assert!(strong.valid);

        let weak = analyze_with("password", 1, &[]);
        assert!(!weak.valid);
        assert!(
            weak.issues
                .iter()
                .any(|issue| issue.contains("below the required minimum of 1"))
        );
    }

    #[test]
    fn min_score_out_of_range_is_rejected() {
        let report = analyze_with("n0tMyRe4lPw", 5, &[]);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("outside the supported range"))
        );
    }

    #[test]
    fn character_classes_are_counted() {
        let report = analyze("Ab1!é");
        assert_eq!(report.input_len, 5);
        assert_eq!(report.lowercase, 1);
        assert_eq!(report.uppercase, 1);
        assert_eq!(report.digits, 1);
        assert_eq!(report.symbols, 1);
        assert_eq!(report.other, 1);
    }

    #[test]
    fn reversed_dictionary_words_are_detected() {
        let report = analyze("poiuytrewq");
        assert_eq!(report.score, 0);
        assert!(report.patterns.iter().any(|pattern| pattern.contains("reversed")));
    }

    #[test]
    fn entropy_and_crack_times_are_reported() {
        let report = analyze("correct horse battery staple");
        assert!(report.entropy_bits > 50);
        assert!(report.guesses_log10 > 5);
        assert!(!report.crack_times.online_throttled.is_empty());
        assert!(!report.crack_times.online_unthrottled.is_empty());
        assert!(!report.crack_times.offline_slow.is_empty());
        assert!(!report.crack_times.offline_fast.is_empty());
    }

    #[test]
    fn long_inputs_note_the_100_character_cap() {
        let report = analyze(&"x".repeat(120));
        assert_eq!(report.input_len, 120);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("first 100 characters"))
        );
    }

    #[test]
    fn strength_bands_cover_all_scores() {
        assert_eq!(PasswordStrength::from(0), PasswordStrength::VeryWeak);
        assert_eq!(PasswordStrength::from(1), PasswordStrength::Weak);
        assert_eq!(PasswordStrength::from(2), PasswordStrength::Fair);
        assert_eq!(PasswordStrength::from(3), PasswordStrength::Strong);
        assert_eq!(PasswordStrength::from(4), PasswordStrength::VeryStrong);
        assert_eq!(PasswordStrength::from(200), PasswordStrength::VeryStrong);
        assert_eq!(PasswordStrength::VeryWeak.label(), "very weak");
        assert_eq!(PasswordStrength::VeryStrong.label(), "very strong");
    }
}

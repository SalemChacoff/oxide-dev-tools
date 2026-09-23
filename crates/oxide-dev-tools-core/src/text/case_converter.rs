use std::fmt;

/// Errors that can occur when converting text between letter cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseError {
    /// The input contains no word characters to convert.
    EmptyInput,
}

impl fmt::Display for CaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaseError::EmptyInput => write!(f, "input contains no word characters"),
        }
    }
}

impl std::error::Error for CaseError {}

/// Target letter cases supported by the case converter.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseTarget {
    /// `helloWorld`
    #[default]
    Camel,
    /// `HelloWorld`
    Pascal,
    /// `hello_world`
    Snake,
    /// `HELLO_WORLD`
    ScreamingSnake,
    /// `hello-world`
    Kebab,
    /// `HELLO-WORLD`
    ScreamingKebab,
    /// `hello.world`
    Dot,
    /// `Hello World`
    Title,
    /// `hello world`
    Lower,
    /// `HELLO WORLD`
    Upper,
}

/// Options for a case conversion operation.
#[derive(Debug, Clone)]
pub struct CaseOptions {
    /// Text to convert.
    pub input: String,
    /// Letter case of the converted output.
    pub target: CaseTarget,
}

impl Default for CaseOptions {
    fn default() -> Self {
        Self {
            input: String::new(),
            target: CaseTarget::Camel,
        }
    }
}

/// Operations available on text case conversion.
#[derive(Debug)]
pub enum CaseKind {
    Convert(CaseOptions),
}

/// Convert `kind`'s input text to the requested letter case.
pub fn convert_case(kind: CaseKind) -> Result<String, CaseError> {
    match kind {
        CaseKind::Convert(opts) => convert(&opts),
    }
}

// -------- Conversion --------

fn convert(opts: &CaseOptions) -> Result<String, CaseError> {
    let words = segment_words(&opts.input);
    if words.is_empty() {
        return Err(CaseError::EmptyInput);
    }
    Ok(join_words(&words, opts.target))
}

// -------- Word segmentation --------

/// Split `input` into canonical lowercased word tokens.
///
/// Word boundaries are non-alphanumeric characters, lower→upper case
/// transitions, the tail of acronym runs (`HTTPServer` → `http`, `server`),
/// and letter↔digit transitions (`sha256` → `sha`, `256`). Unicode letters
/// are preserved; everything else acts as a separator.
fn segment_words(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (index, &ch) in chars.iter().enumerate() {
        if !ch.is_alphanumeric() {
            flush(&mut current, &mut words);
            continue;
        }
        let prev = index.checked_sub(1).map(|i| chars[i]);
        let next = chars.get(index + 1).copied();
        if starts_new_word(ch, prev, next) {
            flush(&mut current, &mut words);
        }
        current.extend(ch.to_lowercase());
    }
    flush(&mut current, &mut words);
    words
}

/// Move `current` into `words` unless it is empty.
fn flush(current: &mut String, words: &mut Vec<String>) {
    if !current.is_empty() {
        words.push(std::mem::take(current));
    }
}

/// Whether `ch` starts a new word given the surrounding characters.
fn starts_new_word(ch: char, prev: Option<char>, next: Option<char>) -> bool {
    let Some(prev) = prev else {
        return false;
    };
    if ch.is_uppercase() {
        // `lowerUpper` and `digitUpper` always split.
        if prev.is_lowercase() || prev.is_numeric() {
            return true;
        }
        // `UPPERUpper` splits before the tail of an acronym run.
        return prev.is_uppercase() && next.is_some_and(|n| n.is_lowercase());
    }
    // Letter↔digit transitions split.
    prev.is_alphanumeric() && ch.is_numeric() != prev.is_numeric()
}

// -------- Joining --------

/// Join canonical words with the separator and casing of `target`.
fn join_words(words: &[String], target: CaseTarget) -> String {
    match target {
        CaseTarget::Camel => words
            .iter()
            .enumerate()
            .map(|(index, word)| if index == 0 { word.clone() } else { capitalize(word) })
            .collect(),
        CaseTarget::Pascal => words.iter().map(|word| capitalize(word)).collect(),
        CaseTarget::Snake => words.join("_"),
        CaseTarget::ScreamingSnake => words
            .iter()
            .map(|word| word.to_uppercase())
            .collect::<Vec<_>>()
            .join("_"),
        CaseTarget::Kebab => words.join("-"),
        CaseTarget::ScreamingKebab => words
            .iter()
            .map(|word| word.to_uppercase())
            .collect::<Vec<_>>()
            .join("-"),
        CaseTarget::Dot => words.join("."),
        CaseTarget::Title => words.iter().map(|word| capitalize(word)).collect::<Vec<_>>().join(" "),
        CaseTarget::Lower => words.join(" "),
        CaseTarget::Upper => words
            .iter()
            .map(|word| word.to_uppercase())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Uppercase the first character of `word`, leaving the rest untouched.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(input: &str, target: CaseTarget) -> CaseOptions {
        CaseOptions {
            input: input.to_string(),
            target,
        }
    }

    fn convert_to(input: &str, target: CaseTarget) -> String {
        convert_case(CaseKind::Convert(opts(input, target))).unwrap()
    }

    #[test]
    fn camel_known_vectors() {
        for (input, expected) in [
            ("hello world", "helloWorld"),
            ("hello_world", "helloWorld"),
            ("hello-world", "helloWorld"),
            ("hello.world", "helloWorld"),
            ("HELLO_WORLD", "helloWorld"),
            ("HelloWorld", "helloWorld"),
            ("XMLHttpRequest", "xmlHttpRequest"),
            ("sha256 hash", "sha256Hash"),
        ] {
            assert_eq!(convert_to(input, CaseTarget::Camel), expected, "converting {input:?}");
        }
    }

    #[test]
    fn pascal_known_vectors() {
        for (input, expected) in [
            ("hello world", "HelloWorld"),
            ("hello_world", "HelloWorld"),
            ("hello-world", "HelloWorld"),
            ("helloWorld", "HelloWorld"),
            ("XMLHttpRequest", "XmlHttpRequest"),
        ] {
            assert_eq!(convert_to(input, CaseTarget::Pascal), expected, "converting {input:?}");
        }
    }

    #[test]
    fn snake_known_vectors() {
        for (input, expected) in [
            ("helloWorld", "hello_world"),
            ("HelloWorld", "hello_world"),
            ("hello world", "hello_world"),
            ("hello-world", "hello_world"),
            ("hello.world", "hello_world"),
            ("HTTPServer", "http_server"),
            ("myXMLParser", "my_xml_parser"),
            ("sha256Hash", "sha_256_hash"),
            ("version2", "version_2"),
            ("hello2world", "hello_2_world"),
            ("foo__bar baz", "foo_bar_baz"),
        ] {
            assert_eq!(convert_to(input, CaseTarget::Snake), expected, "converting {input:?}");
        }
    }

    #[test]
    fn screaming_snake_known_vectors() {
        assert_eq!(convert_to("helloWorld", CaseTarget::ScreamingSnake), "HELLO_WORLD");
        assert_eq!(convert_to("héllo wörld", CaseTarget::ScreamingSnake), "HÉLLO_WÖRLD");
    }

    #[test]
    fn kebab_known_vectors() {
        assert_eq!(convert_to("helloWorld", CaseTarget::Kebab), "hello-world");
        assert_eq!(convert_to("hello_world", CaseTarget::Kebab), "hello-world");
        assert_eq!(convert_to("HTTPServer", CaseTarget::Kebab), "http-server");
    }

    #[test]
    fn screaming_kebab_known_vectors() {
        assert_eq!(convert_to("helloWorld", CaseTarget::ScreamingKebab), "HELLO-WORLD");
        assert_eq!(convert_to("hello_world", CaseTarget::ScreamingKebab), "HELLO-WORLD");
    }

    #[test]
    fn dot_known_vectors() {
        assert_eq!(convert_to("helloWorld", CaseTarget::Dot), "hello.world");
        assert_eq!(convert_to("hello world", CaseTarget::Dot), "hello.world");
    }

    #[test]
    fn title_known_vectors() {
        assert_eq!(convert_to("helloWorld", CaseTarget::Title), "Hello World");
        assert_eq!(convert_to("hello_world", CaseTarget::Title), "Hello World");
        assert_eq!(convert_to("héllo wörld", CaseTarget::Title), "Héllo Wörld");
    }

    #[test]
    fn lower_known_vectors() {
        assert_eq!(convert_to("HelloWorld", CaseTarget::Lower), "hello world");
        assert_eq!(convert_to("hello_world", CaseTarget::Lower), "hello world");
    }

    #[test]
    fn upper_known_vectors() {
        assert_eq!(convert_to("helloWorld", CaseTarget::Upper), "HELLO WORLD");
        assert_eq!(convert_to("héllo wörld", CaseTarget::Upper), "HÉLLO WÖRLD");
    }

    #[test]
    fn digits_form_words() {
        assert_eq!(convert_to("2 factor auth", CaseTarget::Camel), "2FactorAuth");
        assert_eq!(convert_to("2 factor auth", CaseTarget::Pascal), "2FactorAuth");
        assert_eq!(convert_to("2 factor auth", CaseTarget::Snake), "2_factor_auth");
        assert_eq!(convert_to("123!@#", CaseTarget::Snake), "123");
    }

    #[test]
    fn empty_input_errors() {
        for input in ["", "   ", "___", "-._-"] {
            assert_eq!(convert(&opts(input, CaseTarget::Snake)), Err(CaseError::EmptyInput), "converting {input:?}");
        }
        let err = convert_case(CaseKind::Convert(opts("___", CaseTarget::Camel))).unwrap_err();
        assert!(err.to_string().contains("no word characters"));
    }

    #[test]
    fn options_defaults() {
        let opts = CaseOptions::default();
        assert_eq!(opts.input, "");
        assert_eq!(opts.target, CaseTarget::Camel);
    }

    #[test]
    fn dispatch_through_kind() {
        let kind = CaseKind::Convert(opts("hello world", CaseTarget::Kebab));
        assert_eq!(convert_case(kind).unwrap(), "hello-world");
    }

    #[test]
    fn leading_trailing_separators_are_ignored() {
        assert_eq!(convert_to("  hello-world  ", CaseTarget::Snake), "hello_world");
    }
}

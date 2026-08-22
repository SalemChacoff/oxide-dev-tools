use std::fmt;

use rand::RngExt;
use rand::rngs::ThreadRng;
use rand::seq::IndexedRandom;

/// Errors that can occur when generating lorem ipsum text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoremError {
    /// A requested count (words, sentences, or per-unit limits) was zero.
    ZeroCount,
    /// The sampler was given an empty word list.
    EmptyWordList,
    /// The minimum words per sentence exceeded the maximum.
    InvalidRange,
}

impl fmt::Display for LoremError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoremError::ZeroCount => write!(f, "count must be at least 1"),
            LoremError::EmptyWordList => write!(f, "word list must not be empty"),
            LoremError::InvalidRange => write!(f, "min_words must not exceed max_words"),
        }
    }
}

impl std::error::Error for LoremError {}

/// Kinds of lorem ipsum text that can be generated.
#[derive(Debug)]
pub enum LoremKind {
    Words(WordOptions),
    Sentences(SentenceOptions),
    Paragraphs(ParagraphOptions),
}

/// Generate lorem ipsum text according to `kind`.
///
/// Words are drawn uniformly from a fixed classical vocabulary, so repeated
/// calls produce different text while every token stays on-vocabulary.
pub fn generate_lorem(kind: LoremKind) -> Result<String, LoremError> {
    match kind {
        LoremKind::Words(opts) => {
            ensure_nonzero(opts.count)?;
            gen_words(LOREM_WORDS, opts.count, opts.start_with_lorem)
        }
        LoremKind::Sentences(opts) => {
            ensure_nonzero(opts.count)?;
            ensure_nonzero(opts.min_words)?;
            ensure_nonzero(opts.max_words)?;
            if opts.min_words > opts.max_words {
                return Err(LoremError::InvalidRange);
            }
            gen_sentences(LOREM_WORDS, opts.count, opts.min_words, opts.max_words, opts.start_with_lorem)
        }
        LoremKind::Paragraphs(opts) => {
            ensure_nonzero(opts.count)?;
            ensure_nonzero(opts.sentences_per_paragraph)?;
            gen_paragraphs(LOREM_WORDS, opts.count, opts.sentences_per_paragraph, opts.start_with_lorem)
        }
    }
}

/// Reject zero counts, which would otherwise silently produce empty output.
fn ensure_nonzero(count: usize) -> Result<(), LoremError> {
    if count == 0 { Err(LoremError::ZeroCount) } else { Ok(()) }
}

/// Classical vocabulary drawn from the canonical Cicero-based passage.
const LOREM_WORDS: &[&str] = &[
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
    "sed",
    "do",
    "eiusmod",
    "tempor",
    "incididunt",
    "ut",
    "labore",
    "et",
    "dolore",
    "magna",
    "aliqua",
    "enim",
    "ad",
    "minim",
    "veniam",
    "quis",
    "nostrud",
    "exercitation",
    "ullamco",
    "laboris",
    "nisi",
    "aliquip",
    "ex",
    "ea",
    "commodo",
    "consequat",
    "duis",
    "aute",
    "irure",
    "in",
    "reprehenderit",
    "voluptate",
    "velit",
    "esse",
    "cillum",
    "eu",
    "fugiat",
    "nulla",
    "pariatur",
    "excepteur",
    "sint",
    "occaecat",
    "cupidatat",
    "non",
    "proident",
    "sunt",
    "culpa",
    "qui",
    "officia",
    "deserunt",
    "mollit",
    "anim",
    "id",
    "est",
    "laborum",
];

/// The classical opening phrase, prepended when `start_with_lorem` is set.
const OPENING_WORDS: &[&str] = &["Lorem", "ipsum", "dolor", "sit", "amet"];

/// Default minimum words per generated sentence.
pub const DEFAULT_MIN_WORDS: usize = 4;

/// Default maximum words per generated sentence.
pub const DEFAULT_MAX_WORDS: usize = 12;

// -------- Words --------

/// Options for bare word generation.
#[derive(Debug, Clone)]
pub struct WordOptions {
    /// Number of words to generate.
    pub count: usize,
    /// Begin with the classical "Lorem ipsum dolor sit amet" opening.
    pub start_with_lorem: bool,
}

impl Default for WordOptions {
    fn default() -> Self {
        Self {
            count: 10,
            start_with_lorem: false,
        }
    }
}

fn gen_words(words: &[&str], count: usize, start_with_lorem: bool) -> Result<String, LoremError> {
    if words.is_empty() {
        return Err(LoremError::EmptyWordList);
    }
    let mut rng = rand::rng();
    let mut tokens: Vec<&str> = Vec::with_capacity(count);
    if start_with_lorem {
        tokens.extend_from_slice(&OPENING_WORDS[..count.min(OPENING_WORDS.len())]);
    }
    while tokens.len() < count {
        tokens.push(*words.choose(&mut rng).unwrap());
    }
    Ok(tokens.join(" "))
}

// -------- Sentences --------

/// Options for sentence generation.
#[derive(Debug, Clone)]
pub struct SentenceOptions {
    /// Number of sentences to generate.
    pub count: usize,
    /// Minimum words per sentence (inclusive).
    pub min_words: usize,
    /// Maximum words per sentence (inclusive).
    pub max_words: usize,
    /// Begin the first sentence with the classical opening.
    pub start_with_lorem: bool,
}

impl Default for SentenceOptions {
    fn default() -> Self {
        Self {
            count: 3,
            min_words: DEFAULT_MIN_WORDS,
            max_words: DEFAULT_MAX_WORDS,
            start_with_lorem: false,
        }
    }
}

fn gen_sentences(
    words: &[&str],
    count: usize,
    min_words: usize,
    max_words: usize,
    start_with_lorem: bool,
) -> Result<String, LoremError> {
    if words.is_empty() {
        return Err(LoremError::EmptyWordList);
    }
    let mut rng = rand::rng();
    let sentences: Vec<String> = (0..count)
        .map(|index| {
            let with_opening = start_with_lorem && index == 0;
            gen_sentence(words, &mut rng, min_words, max_words, with_opening)
        })
        .collect();
    Ok(sentences.join(" "))
}

fn gen_sentence(
    words: &[&str],
    rng: &mut ThreadRng,
    min_words: usize,
    max_words: usize,
    start_with_lorem: bool,
) -> String {
    // The classical opening is five words long, so the first sentence is at
    // least that long even when the caller's minimum is smaller.
    let (lower, upper) = if start_with_lorem {
        (min_words.max(OPENING_WORDS.len()), max_words.max(OPENING_WORDS.len()))
    } else {
        (min_words, max_words)
    };
    let target = rng.random_range(lower..=upper);
    let mut tokens: Vec<&str> = if start_with_lorem {
        OPENING_WORDS.to_vec()
    } else {
        Vec::with_capacity(target)
    };
    while tokens.len() < target {
        tokens.push(*words.choose(rng).unwrap());
    }
    let mut text = String::new();
    text.push_str(&capitalize_first(tokens[0]));
    for token in &tokens[1..] {
        text.push(' ');
        text.push_str(token);
    }
    text.push('.');
    text
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

// -------- Paragraphs --------

/// Options for paragraph generation.
#[derive(Debug, Clone)]
pub struct ParagraphOptions {
    /// Number of paragraphs to generate.
    pub count: usize,
    /// Sentences per paragraph.
    pub sentences_per_paragraph: usize,
    /// Begin the first sentence with the classical opening.
    pub start_with_lorem: bool,
}

impl Default for ParagraphOptions {
    fn default() -> Self {
        Self {
            count: 3,
            sentences_per_paragraph: 4,
            start_with_lorem: false,
        }
    }
}

fn gen_paragraphs(
    words: &[&str],
    count: usize,
    sentences_per_paragraph: usize,
    start_with_lorem: bool,
) -> Result<String, LoremError> {
    if words.is_empty() {
        return Err(LoremError::EmptyWordList);
    }
    let mut rng = rand::rng();
    let mut paragraphs = Vec::with_capacity(count);
    for paragraph_index in 0..count {
        let mut sentences = Vec::with_capacity(sentences_per_paragraph);
        for sentence_index in 0..sentences_per_paragraph {
            let with_opening = start_with_lorem && paragraph_index == 0 && sentence_index == 0;
            sentences.push(gen_sentence(words, &mut rng, DEFAULT_MIN_WORDS, DEFAULT_MAX_WORDS, with_opening));
        }
        paragraphs.push(sentences.join(" "));
    }
    Ok(paragraphs.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn words_default_count() {
        let text = generate_lorem(LoremKind::Words(WordOptions::default())).unwrap();
        assert_eq!(text.split_whitespace().count(), 10);
    }

    #[test]
    fn words_custom_count() {
        let opts = WordOptions {
            count: 25,
            ..Default::default()
        };
        let text = generate_lorem(LoremKind::Words(opts)).unwrap();
        assert_eq!(text.split_whitespace().count(), 25);
    }

    #[test]
    fn words_start_with_lorem_is_deterministic_opening() {
        let opts = WordOptions {
            count: 5,
            start_with_lorem: true,
        };
        let text = generate_lorem(LoremKind::Words(opts)).unwrap();
        assert_eq!(text, "Lorem ipsum dolor sit amet");
    }

    #[test]
    fn words_start_with_lorem_extends_past_opening() {
        let opts = WordOptions {
            count: 9,
            start_with_lorem: true,
        };
        let text = generate_lorem(LoremKind::Words(opts)).unwrap();
        assert!(text.starts_with("Lorem ipsum dolor sit amet "));
        assert_eq!(text.split_whitespace().count(), 9);
    }

    #[test]
    fn words_zero_count_errors() {
        let opts = WordOptions {
            count: 0,
            ..Default::default()
        };
        let err = generate_lorem(LoremKind::Words(opts)).unwrap_err();
        assert!(err.to_string().contains("at least 1"));
    }

    #[test]
    fn words_different_each_call() {
        let opts = WordOptions::default();
        let a = generate_lorem(LoremKind::Words(opts.clone())).unwrap();
        let b = generate_lorem(LoremKind::Words(opts)).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn words_empty_word_list_errors() {
        let err = gen_words(&[], 5, false).unwrap_err();
        assert!(err.to_string().contains("word list"));
    }

    #[test]
    fn sentences_default_count() {
        let text = generate_lorem(LoremKind::Sentences(SentenceOptions::default())).unwrap();
        assert_eq!(text.matches('.').count(), 3);
    }

    #[test]
    fn sentences_capitalized_and_terminated() {
        let opts = SentenceOptions {
            count: 6,
            ..Default::default()
        };
        let text = generate_lorem(LoremKind::Sentences(opts)).unwrap();
        let parts: Vec<&str> = text.split('.').filter(|part| !part.is_empty()).collect();
        assert_eq!(parts.len(), 6);
        for part in parts {
            let first = part.trim_start().chars().next().unwrap();
            assert!(first.is_uppercase(), "sentence must start capitalized: {part}");
        }
    }

    #[test]
    fn sentences_respect_word_bounds() {
        let opts = SentenceOptions {
            count: 40,
            min_words: 5,
            max_words: 8,
            ..Default::default()
        };
        let text = generate_lorem(LoremKind::Sentences(opts)).unwrap();
        for part in text.split('.').filter(|part| !part.is_empty()) {
            let words = part.split_whitespace().count();
            assert!((5..=8).contains(&words), "got {words} words in: {part}");
        }
    }

    #[test]
    fn sentences_start_with_lorem_opening() {
        let opts = SentenceOptions {
            count: 2,
            start_with_lorem: true,
            ..Default::default()
        };
        let text = generate_lorem(LoremKind::Sentences(opts)).unwrap();
        assert!(text.starts_with("Lorem ipsum dolor sit amet "));
        let first = text.split('.').next().unwrap();
        assert!(first.split_whitespace().count() >= 5);
    }

    #[test]
    fn sentences_zero_count_errors() {
        let opts = SentenceOptions {
            count: 0,
            ..Default::default()
        };
        let err = generate_lorem(LoremKind::Sentences(opts)).unwrap_err();
        assert!(err.to_string().contains("at least 1"));
    }

    #[test]
    fn sentences_zero_min_words_errors() {
        let opts = SentenceOptions {
            min_words: 0,
            ..Default::default()
        };
        let err = generate_lorem(LoremKind::Sentences(opts)).unwrap_err();
        assert!(err.to_string().contains("at least 1"));
    }

    #[test]
    fn sentences_invalid_range_errors() {
        let opts = SentenceOptions {
            count: 1,
            min_words: 10,
            max_words: 4,
            ..Default::default()
        };
        let err = generate_lorem(LoremKind::Sentences(opts)).unwrap_err();
        assert!(err.to_string().contains("min_words"));
    }

    #[test]
    fn sentences_empty_word_list_errors() {
        let err = gen_sentences(&[], 3, 4, 12, false).unwrap_err();
        assert!(err.to_string().contains("word list"));
    }

    #[test]
    fn sentences_unique_across_1000_samples() {
        let opts = SentenceOptions::default();
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            let text = generate_lorem(LoremKind::Sentences(opts.clone())).unwrap();
            assert!(seen.insert(text), "duplicate lorem ipsum text across 1,000 samples");
        }
        assert_eq!(seen.len(), 1_000);
    }

    #[test]
    fn paragraphs_default_shape() {
        let text = generate_lorem(LoremKind::Paragraphs(ParagraphOptions::default())).unwrap();
        assert_eq!(text.match_indices("\n\n").count(), 2);
        assert_eq!(text.matches('.').count(), 12);
    }

    #[test]
    fn paragraphs_custom_shape() {
        let opts = ParagraphOptions {
            count: 2,
            sentences_per_paragraph: 5,
            ..Default::default()
        };
        let text = generate_lorem(LoremKind::Paragraphs(opts)).unwrap();
        assert_eq!(text.match_indices("\n\n").count(), 1);
        assert_eq!(text.matches('.').count(), 10);
    }

    #[test]
    fn paragraphs_start_with_lorem_opening() {
        let opts = ParagraphOptions {
            count: 2,
            start_with_lorem: true,
            ..Default::default()
        };
        let text = generate_lorem(LoremKind::Paragraphs(opts)).unwrap();
        assert!(text.starts_with("Lorem ipsum dolor sit amet "));
    }

    #[test]
    fn paragraphs_zero_count_errors() {
        let opts = ParagraphOptions {
            count: 0,
            ..Default::default()
        };
        let err = generate_lorem(LoremKind::Paragraphs(opts)).unwrap_err();
        assert!(err.to_string().contains("at least 1"));
    }

    #[test]
    fn paragraphs_zero_sentences_per_paragraph_errors() {
        let opts = ParagraphOptions {
            sentences_per_paragraph: 0,
            ..Default::default()
        };
        let err = generate_lorem(LoremKind::Paragraphs(opts)).unwrap_err();
        assert!(err.to_string().contains("at least 1"));
    }

    #[test]
    fn paragraphs_empty_word_list_errors() {
        let err = gen_paragraphs(&[], 3, 4, false).unwrap_err();
        assert!(err.to_string().contains("word list"));
    }
}

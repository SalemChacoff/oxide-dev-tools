use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;

use crate::error::CliError;

/// `oxide gen lorem [subcommand]` — lorem ipsum text generator
#[derive(Args)]
pub struct LoremArgs {
    #[command(subcommand)]
    pub kind: LoremCmd,
}

#[derive(Subcommand)]
pub enum LoremCmd {
    /// Generate a run of random lorem ipsum words
    #[command(name = "words")]
    Words {
        /// Number of words to generate
        #[arg(short = 'l', long = "length", default_value_t = 10)]
        length: usize,

        /// Begin with the classic "Lorem ipsum dolor sit amet" opening
        #[arg(short = 's', long = "start")]
        start: bool,
    },

    /// Generate lorem ipsum sentences
    #[command(name = "sentences")]
    Sentences {
        /// Number of sentences to generate
        #[arg(short = 'l', long = "length", default_value_t = 3)]
        length: usize,

        /// Minimum words per sentence
        #[arg(long = "min-words", default_value_t = 4)]
        min_words: usize,

        /// Maximum words per sentence
        #[arg(long = "max-words", default_value_t = 12)]
        max_words: usize,

        /// Begin the first sentence with the classic "Lorem ipsum" opening
        #[arg(short = 's', long = "start")]
        start: bool,
    },

    /// Generate lorem ipsum paragraphs separated by blank lines
    #[command(name = "paragraphs")]
    Paragraphs {
        /// Number of paragraphs to generate
        #[arg(short = 'l', long = "length", default_value_t = 3)]
        length: usize,

        /// Sentences per paragraph
        #[arg(long = "sentences-per-paragraph", default_value_t = 4)]
        sentences_per_paragraph: usize,

        /// Begin the first sentence with the classic "Lorem ipsum" opening
        #[arg(short = 's', long = "start")]
        start: bool,
    },
}

pub fn exec(args: LoremArgs) -> Result<(), CliError> {
    match args.kind {
        LoremCmd::Words { length, start } => {
            let opts = WordOptions {
                count: length,
                start_with_lorem: start,
            };
            println!("{}", generate_lorem(LoremKind::Words(opts))?);
        }
        LoremCmd::Sentences {
            length,
            min_words,
            max_words,
            start,
        } => {
            let opts = SentenceOptions {
                count: length,
                min_words,
                max_words,
                start_with_lorem: start,
            };
            println!("{}", generate_lorem(LoremKind::Sentences(opts))?);
        }
        LoremCmd::Paragraphs {
            length,
            sentences_per_paragraph,
            start,
        } => {
            let opts = ParagraphOptions {
                count: length,
                sentences_per_paragraph,
                start_with_lorem: start,
            };
            println!("{}", generate_lorem(LoremKind::Paragraphs(opts))?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_words_default() {
        assert!(
            exec(LoremArgs {
                kind: LoremCmd::Words {
                    length: 10,
                    start: false,
                }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_words_with_start() {
        assert!(
            exec(LoremArgs {
                kind: LoremCmd::Words { length: 8, start: true }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_words_zero_length_errors() {
        let result = exec(LoremArgs {
            kind: LoremCmd::Words {
                length: 0,
                start: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 1"));
    }

    #[test]
    fn exec_sentences_default() {
        assert!(
            exec(LoremArgs {
                kind: LoremCmd::Sentences {
                    length: 3,
                    min_words: 4,
                    max_words: 12,
                    start: false,
                }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_sentences_invalid_range_errors() {
        let result = exec(LoremArgs {
            kind: LoremCmd::Sentences {
                length: 1,
                min_words: 12,
                max_words: 4,
                start: false,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("min_words"));
    }

    #[test]
    fn exec_paragraphs_default() {
        assert!(
            exec(LoremArgs {
                kind: LoremCmd::Paragraphs {
                    length: 3,
                    sentences_per_paragraph: 4,
                    start: false,
                }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_paragraphs_with_start() {
        assert!(
            exec(LoremArgs {
                kind: LoremCmd::Paragraphs {
                    length: 2,
                    sentences_per_paragraph: 5,
                    start: true,
                }
            })
            .is_ok()
        );
    }
}

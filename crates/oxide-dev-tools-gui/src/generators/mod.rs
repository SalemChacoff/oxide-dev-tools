//! Tool pages for the generators category (README Phase 7.5): `id`, `key`,
//! `lorem`, `fake`, and `sample` — one Pattern B (Generator) page each: an
//! options form on the left, the result pane on the right.
//!
//! Pages call `oxide-dev-tools-core` entry points directly and never
//! reimplement generation logic. A successful run is reported through the
//! [`RunReporter`] seam so the shell's recent-runs history stays in sync.

pub mod fake_view;
pub mod id_view;
pub mod key_view;
pub mod lorem_view;
pub mod sample_view;

pub use fake_view::FakeView;
pub use id_view::IdView;
pub use key_view::KeyView;
pub use lorem_view::LoremView;
pub use sample_view::SampleView;

// The run-reporting seam is shared with the other tool categories (codecs,
// Phase 7.6); re-exported here so the generator pages keep one import path.
pub use crate::shared::run_seam::{RunReporter, report_run, sentence_case, summarize};

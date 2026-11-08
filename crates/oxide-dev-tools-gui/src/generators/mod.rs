//! Tool pages for the generators category (README Phase 7.5): `id`, `key`,
//! `lorem`, `fake`, and `sample` — one Pattern B (Generator) page each: an
//! options form on the left, the result pane on the right.
//!
//! Pages call `oxide-dev-tools-core` entry points directly and never
//! reimplement generation logic. A successful run is reported through the
//! [`RunReporter`] seam so the shell's recent-runs history stays in sync.

use std::rc::Rc;

use gpui_kit::App;

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

/// Reports a finished run (tool id + one-line summary) so the shell can
/// record it in the recent-runs history.
pub type RunReporter = Rc<dyn Fn(&str, &str, &mut App)>;

/// Reports a successful run when a reporter is registered.
fn report_run(reporter: &Option<RunReporter>, tool_id: &str, summary: &str, cx: &mut App) {
    if let Some(reporter) = reporter {
        reporter(tool_id, summary, cx);
    }
}

/// Condenses a generated value into a one-line history summary.
fn summarize(value: &str) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= 40 {
        single_line
    } else {
        let head: String = single_line.chars().take(39).collect();
        format!("{head}…")
    }
}

/// Renders a core error message as sentence-case inline validation copy.
fn sentence_case(message: String) -> String {
    let mut chars = message.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_collapses_whitespace_and_caps_at_forty_characters() {
        assert_eq!(summarize("9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d"), "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d");
        assert_eq!(summarize("line one\nline two"), "line one line two");

        let long = "a".repeat(50);
        let summary = summarize(&long);
        assert_eq!(summary.chars().count(), 40);
        assert!(summary.ends_with('…'));
    }

    #[test]
    fn sentence_case_capitalizes_core_error_messages() {
        assert_eq!(sentence_case("count must be at least 1".into()), "Count must be at least 1");
        assert_eq!(sentence_case(String::new()), "");
    }
}

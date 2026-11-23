//! Tool pages for the codecs category (README Phase 7.6): `base64` first, then
//! `hex` and `url-codec`. Codec pages are Pattern A (Transform): inputs and
//! mode selectors on the left, the result pane on the right.
//!
//! Pages call `oxide-dev-tools-core` entry points directly and never
//! reimplement codec logic. A successful run is reported through the
//! [`RunReporter`](crate::shared::run_seam::RunReporter) seam so the shell's
//! recent-runs history stays in sync.

pub mod base64_view;
pub mod hex_view;

pub use base64_view::Base64View;
pub use hex_view::HexView;

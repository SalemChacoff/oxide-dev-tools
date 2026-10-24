//! `shell` — the window shell (README Phase 7.4): bootstrap, the shell view
//! with its title bar, resizable sidebar, detail area, global search header,
//! and command palette, plus the navigation, palette, seed, notification,
//! and action contracts those surfaces share.
//!
//! Tool-page modules (one per category, starting with `crate::generators`
//! in Phase 7.5) replace the placeholder pages with the real ones; the
//! registry and navigation here do not change.

pub mod actions;
pub mod bootstrap;
pub mod home;
pub mod navigation;
pub mod notifications;
pub mod palette;
pub mod search_header;
pub mod shell_view;
pub mod sidebar;
pub mod titlebar;
pub mod tool_seed;

//! Cross-category app components (README Phase 7.3): the reusable UI
//! substrate every tool page and the shell compose.
//!
//! Per the design contract, only `tool_field` and `tool_card` are wrapped app
//! components (they carry domain policy); the rest are compositions with a
//! real contract — the registry, the output pane, sidebar entries, history,
//! and settings — composed from gpui-kit components as-is, never
//! rename-wrapped.

use std::rc::Rc;

use gpui_kit::{App, ClickEvent, SharedString, Window};

/// A pointer-activation handler (card opens, exports, resets).
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A navigation handler that reopens the tool of a recent run.
pub type OpenToolHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

pub mod category_item;
pub mod output_panel;
pub mod recent_runs;
pub mod registry;
pub mod settings_panel;
pub mod tool_card;
pub mod tool_field;

pub use category_item::{sidebar_category_group, sidebar_tool_item};
pub use output_panel::OutputPanel;
pub use recent_runs::{RECENT_RUNS_CAP, RecentRun, RecentRuns, RecentRunsState, relative_time};
pub use registry::{Category, RegistryError, ToolMeta, ToolRegistry, ToolRegistryGlobal, registry};
pub use settings_panel::{SettingsPanel, SettingsState};
pub use tool_card::ToolCard;
pub use tool_field::ToolField;

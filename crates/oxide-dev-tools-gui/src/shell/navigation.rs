//! `navigation` — the shell's destination model: where the detail area
//! points, keyed by stable identity.
//!
//! Pure data and lookups only; views live in the shell modules. Tool ids are
//! the registry's stable ids, never translated labels or list positions.

use gpui_kit::SharedString;

/// The detail area's current destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    /// The home view: search header + tool-card grid.
    Home,
    /// One tool page, keyed by its registry id.
    Tool(SharedString),
    /// The settings page.
    Settings,
}

impl Destination {
    /// Stable identity segment for element ids and state keys.
    pub fn id(&self) -> &str {
        match self {
            Self::Home => "home",
            Self::Tool(id) => id.as_ref(),
            Self::Settings => "settings",
        }
    }

    /// Whether this destination is the tool with the given registry id.
    pub fn is_tool(&self, id: &str) -> bool {
        matches!(self, Self::Tool(tool) if tool.as_ref() == id)
    }

    /// The tool id, when this destination is a tool page.
    pub fn tool_id(&self) -> Option<&SharedString> {
        match self {
            Self::Tool(id) => Some(id),
            Self::Home | Self::Settings => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_identity_segments() {
        assert_eq!(Destination::Home.id(), "home");
        assert_eq!(Destination::Settings.id(), "settings");
        assert_eq!(Destination::Tool("base64".into()).id(), "base64");
    }

    #[test]
    fn tool_lookups_match_the_registry_id() {
        let tool = Destination::Tool("uuid".into());

        assert!(tool.is_tool("uuid"));
        assert!(!tool.is_tool("uuidv4"));
        assert_eq!(tool.tool_id(), Some(&SharedString::from("uuid")));

        assert!(!Destination::Home.is_tool("uuid"));
        assert_eq!(Destination::Home.tool_id(), None);
        assert_eq!(Destination::Settings.tool_id(), None);
    }

    #[test]
    fn destinations_are_comparable_values() {
        assert_eq!(Destination::Tool("base64".into()), Destination::Tool("base64".into()));
        assert_ne!(Destination::Tool("base64".into()), Destination::Tool("hex".into()));
        assert_ne!(Destination::Home, Destination::Settings);
    }
}

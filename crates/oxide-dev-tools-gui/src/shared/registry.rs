//! Stable tool identity: the registry that navigation, search, and home cards
//! all consume.
//!
//! Tool pages register here from Phase 7.5 on (see the `add-tool-ui` skill);
//! Phase 7.3 ships the data contract: the six categories, one `ToolMeta`
//! entry per tool (stable id, icon, i18n keys, search aliases), and the
//! lookup/search logic every navigation surface shares.

use std::fmt;

use gpui_kit::assets::IconName;
use gpui_kit::{App, Global, SharedString};

/// The six tool categories, mirroring the core/CLI structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Generators,
    Codecs,
    Text,
    Converters,
    Validators,
    Comparators,
}

impl Category {
    /// All categories in sidebar order.
    pub const ALL: [Self; 6] = [
        Self::Generators,
        Self::Codecs,
        Self::Text,
        Self::Converters,
        Self::Validators,
        Self::Comparators,
    ];

    /// Stable identity segment for element ids and persistence.
    pub fn id(self) -> &'static str {
        match self {
            Self::Generators => "generators",
            Self::Codecs => "codecs",
            Self::Text => "text",
            Self::Converters => "converters",
            Self::Validators => "validators",
            Self::Comparators => "comparators",
        }
    }

    /// i18n key under `app.categories.*` in `locales/ui.yml`.
    pub fn title_key(self) -> &'static str {
        match self {
            Self::Generators => "app.categories.generators",
            Self::Codecs => "app.categories.codecs",
            Self::Text => "app.categories.text",
            Self::Converters => "app.categories.converters",
            Self::Validators => "app.categories.validators",
            Self::Comparators => "app.categories.comparators",
        }
    }

    /// Localized category name for the current locale.
    pub fn title(self) -> String {
        rust_i18n::t!(self.title_key()).into_owned()
    }
}

/// One registered tool: the sidebar entry, search entry, and home card.
///
/// The id is the stable identity every surface keys on — never a translated
/// label or a list index.
#[derive(Clone, Debug)]
pub struct ToolMeta {
    id: SharedString,
    category: Category,
    icon: IconName,
    title_key: SharedString,
    description_key: SharedString,
    aliases: Vec<SharedString>,
}

impl ToolMeta {
    /// Registers a tool. `title_key` / `description_key` point into
    /// `locales/ui.yml` under `app.<category>.<tool>.*` (English default).
    pub fn new(
        id: impl Into<SharedString>,
        category: Category,
        icon: IconName,
        title_key: impl Into<SharedString>,
        description_key: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            category,
            icon,
            title_key: title_key.into(),
            description_key: description_key.into(),
            aliases: Vec::new(),
        }
    }

    /// Adds one lowercase search alias (e.g. "base 64", "encode decode").
    pub fn with_alias(mut self, alias: impl Into<SharedString>) -> Self {
        self.aliases.push(alias.into().to_lowercase().into());
        self
    }

    /// Stable tool identity.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// The category the tool belongs to.
    pub fn category(&self) -> Category {
        self.category
    }

    /// Home-card and sidebar icon (Lucide).
    pub fn icon(&self) -> IconName {
        self.icon
    }

    /// i18n key of the tool title.
    pub fn title_key(&self) -> &SharedString {
        &self.title_key
    }

    /// i18n key of the one-line description.
    pub fn description_key(&self) -> &SharedString {
        &self.description_key
    }

    /// Lowercase search aliases.
    pub fn aliases(&self) -> &[SharedString] {
        &self.aliases
    }

    /// Localized tool title for the current locale.
    pub fn title(&self) -> String {
        rust_i18n::t!(self.title_key.as_ref()).into_owned()
    }

    /// Localized one-line description for the current locale.
    pub fn description(&self) -> String {
        rust_i18n::t!(self.description_key.as_ref()).into_owned()
    }
}

/// Error while registering a tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// Another tool already owns this id.
    DuplicateId(SharedString),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "a tool with id {id} is already registered"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// The registered tool set. Tools land here as their pages ship (7.5–7.10);
/// the shell, search index, and home cards all read from this one source.
#[derive(Clone, Debug, Default)]
pub struct ToolRegistry {
    tools: Vec<ToolMeta>,
}

impl ToolRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a tool; fails when the id is already taken.
    pub fn register(&mut self, tool: ToolMeta) -> Result<(), RegistryError> {
        if self.tools.iter().any(|existing| existing.id() == tool.id()) {
            return Err(RegistryError::DuplicateId(tool.id().clone()));
        }
        self.tools.push(tool);
        Ok(())
    }

    /// Every registered tool, in registration order.
    pub fn tools(&self) -> &[ToolMeta] {
        &self.tools
    }

    /// Tools of one category, in registration order.
    pub fn by_category(&self, category: Category) -> impl Iterator<Item = &ToolMeta> {
        self.tools.iter().filter(move |tool| tool.category() == category)
    }

    /// Looks a tool up by its stable id.
    pub fn find(&self, id: &str) -> Option<&ToolMeta> {
        self.tools.iter().find(|tool| tool.id().as_ref() == id)
    }

    /// Ranks tools by how well they match a search query, best first.
    ///
    /// Matching runs against the id, the aliases, and the localized title.
    /// Id prefix matches rank above id/alias matches, which rank above title
    /// matches; an empty query matches nothing (the caller renders the full
    /// catalog instead).
    pub fn search(&self, query: &str) -> Vec<&ToolMeta> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(u8, &ToolMeta)> = self
            .tools
            .iter()
            .filter_map(|tool| {
                let id = tool.id().to_lowercase();
                let title = tool.title().to_lowercase();
                let score = if id.starts_with(&query) {
                    3
                } else if id.contains(&query) || tool.aliases().iter().any(|a| a.contains(&query)) {
                    2
                } else if title.starts_with(&query) || title.contains(&query) {
                    1
                } else {
                    return None;
                };
                Some((score, tool))
            })
            .collect();
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        scored.into_iter().map(|(_, tool)| tool).collect()
    }
}

/// Global hosting the application's registry.
///
/// Installed by the shell at bootstrap (Phase 7.4); reads go through
/// [`registry`] and panic on a missing global — that means wiring broke.
pub struct ToolRegistryGlobal(ToolRegistry);

impl ToolRegistryGlobal {
    /// Wraps a registry for installation as a global.
    pub fn new(registry: ToolRegistry) -> Self {
        Self(registry)
    }
}

impl Global for ToolRegistryGlobal {}

/// Returns the application's tool registry.
pub fn registry(cx: &App) -> &ToolRegistry {
    &cx.global::<ToolRegistryGlobal>().0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_tool(id: &str, category: Category) -> ToolMeta {
        ToolMeta::new(
            id,
            category,
            IconName::KeyRound,
            format!("app.{}.{id}.title", category.id()),
            format!("app.{}.{id}.description", category.id()),
        )
        .with_alias(format!("alias-{id}"))
    }

    fn fixture_registry() -> ToolRegistry {
        let mut tools = ToolRegistry::new();
        tools.register(fixture_tool("uuid", Category::Generators)).unwrap();
        tools.register(fixture_tool("password", Category::Generators)).unwrap();
        tools.register(fixture_tool("base64", Category::Codecs)).unwrap();
        tools
    }

    #[test]
    fn register_rejects_duplicate_ids_and_keeps_order() {
        let mut tools = ToolRegistry::new();
        tools.register(fixture_tool("uuid", Category::Generators)).unwrap();

        assert_eq!(
            tools.register(fixture_tool("uuid", Category::Codecs)),
            Err(RegistryError::DuplicateId("uuid".into()))
        );
        assert_eq!(tools.tools().len(), 1);

        tools.register(fixture_tool("base64", Category::Codecs)).unwrap();
        let ids: Vec<&str> = tools.tools().iter().map(|t| t.id().as_ref()).collect();
        assert_eq!(ids, ["uuid", "base64"]);
    }

    #[test]
    fn by_category_filters_and_aliases_are_normalized() {
        let tools = fixture_registry();
        let generators: Vec<&str> = tools
            .by_category(Category::Generators)
            .map(|t| t.id().as_ref())
            .collect();
        assert_eq!(generators, ["uuid", "password"]);

        let base64 = tools.find("base64").expect("registered tool");
        assert_eq!(base64.aliases(), ["alias-base64"]);
    }

    #[test]
    fn search_ranks_id_prefixes_above_aliases_and_titles() {
        let tools = fixture_registry();
        // "base" is an id prefix for base64 and nothing else.
        assert_eq!(tools.search("base").iter().map(|t| t.id().as_ref()).collect::<Vec<_>>(), ["base64"]);
        // "alias-pass" matches only the password alias.
        assert_eq!(
            tools
                .search("alias-pass")
                .iter()
                .map(|t| t.id().as_ref())
                .collect::<Vec<_>>(),
            ["password"]
        );
        // A query matching several tools ranks them by score.
        let hits: Vec<&str> = tools.search("a").iter().map(|t| t.id().as_ref()).collect();
        assert!(hits.contains(&"uuid"));
        assert!(hits.contains(&"base64"));
        assert!(hits.contains(&"password"));
        // Empty queries match nothing — the caller renders the full catalog.
        assert!(tools.search("  ").is_empty());
    }
}

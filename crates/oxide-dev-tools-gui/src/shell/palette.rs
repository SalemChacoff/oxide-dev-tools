//! `palette` — the command palette over the tool registry: one group per
//! category and the index-path → tool resolution the confirm handler needs.

use gpui_kit::base::IndexPath;
use gpui_kit::component::Icon;
use gpui_kit::component::command::{CommandGroup, CommandItem};

use crate::shared::registry::{Category, ToolMeta, ToolRegistry};

/// Builds one palette group per category, in sidebar order.
pub fn build_groups(registry: &ToolRegistry) -> Vec<CommandGroup> {
    Category::ALL
        .into_iter()
        .map(|category| {
            let items = registry.by_category(category).map(|meta| {
                CommandItem::new()
                    .label(meta.title())
                    .icon(Icon::new(meta.icon()))
                    .keywords(meta.aliases().iter().cloned())
            });
            CommandGroup::new().label(category.title()).items(items)
        })
        .collect()
}

/// Resolves a confirmed palette index path to its tool.
///
/// The model installs exactly the six category groups and nothing ungrouped,
/// so the section names the category (the implicit ungrouped section never
/// exists) and the row names the position inside that category's item list.
pub fn resolve_tool<'a>(registry: &'a ToolRegistry, path: &IndexPath) -> Option<&'a ToolMeta> {
    let category = Category::ALL.get(path.section)?;
    registry.by_category(*category).nth(path.row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::tool_seed;

    fn seeded() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        tool_seed::register_all(&mut registry).expect("seed registers without duplicates");
        registry
    }

    #[test]
    fn builds_one_group_per_category_in_sidebar_order() {
        let registry = seeded();
        let groups = build_groups(&registry);

        assert_eq!(groups.len(), Category::ALL.len());
        assert_eq!(groups[0].heading().map(|heading| heading.as_ref()), Some("Generators"));
        assert_eq!(groups[5].heading().map(|heading| heading.as_ref()), Some("Comparators"));
    }

    #[test]
    fn resolves_index_paths_back_to_the_registered_tools() {
        let registry = seeded();

        // Section 0, row 0 is the first generator: the id tool.
        let path = IndexPath::new(0).section(0);
        assert_eq!(resolve_tool(&registry, &path).map(|tool| tool.id().as_ref()), Some("id"));

        // Section 2, row 1 is the second text tool: codec.
        let path = IndexPath::new(1).section(2);
        assert_eq!(resolve_tool(&registry, &path).map(|tool| tool.id().as_ref()), Some("codec"));

        // Section 5 (comparators), row 3: text diff.
        let path = IndexPath::new(3).section(5);
        assert_eq!(resolve_tool(&registry, &path).map(|tool| tool.id().as_ref()), Some("text"));
    }

    #[test]
    fn out_of_range_paths_resolve_to_nothing() {
        let registry = seeded();
        assert!(resolve_tool(&registry, &IndexPath::new(0).section(6)).is_none());
        assert!(resolve_tool(&registry, &IndexPath::new(99).section(0)).is_none());
    }

    #[test]
    fn every_resolvable_path_round_trips_with_the_registry() {
        let registry = seeded();
        let groups = build_groups(&registry);

        for (section, category) in Category::ALL.into_iter().enumerate() {
            for row in 0..registry.by_category(category).count() {
                let path = IndexPath::new(row).section(section);
                let resolved = resolve_tool(&registry, &path).expect("path in range");
                assert_eq!(resolved.category(), category);
            }
            let _ = &groups[section];
        }
    }
}

//! `assets` — the application's icon bundle.
//!
//! gpui-kit registers only its default component icons (~100 SVGs) when the
//! app runs `with_assets(gpui_kit::assets::Assets)`. Every other Lucide name
//! in the `IconName` catalog exists at compile time but renders blank unless
//! its SVG is embedded here. This module embeds exactly the icons the tool
//! catalog and shell surfaces use and composes them over the default bundle;
//! the bundling test fails when a seeded tool icon is missing from the list.

use std::borrow::Cow;

use gpui_kit::assets::{Assets, icon_assets};

// The tool-catalog icons beyond gpui-kit's default component bundle.
//
// One entry per distinct registry icon — aliases cost nothing extra, the SVG
// embeds once (`Barcode` serves both the id generator and the uuid
// validator). When a new tool ships an icon not already listed here, add it
// and run `assets::tests`.
icon_assets!(
    ToolIcons,
    [
        ArrowLeftRight,
        ArrowUpDown,
        Barcode,
        Binary,
        Braces,
        Clock,
        CreditCard,
        Diff,
        FolderTree,
        GitCompare,
        Hash,
        KeyRound,
        Link,
        Mail,
        Package,
        Ruler,
        ScanLine,
        Scale,
        Type,
        UserRound,
    ]
);

/// The composed asset source the window bootstrap registers: the selected
/// tool icons first, then gpui-kit's default component icons.
#[derive(Clone, Copy, Debug, Default)]
pub struct AppAssets;

impl gpui_kit::AssetSource for AppAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ToolIcons.load(path)? {
            return Ok(Some(bytes));
        }
        Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<gpui_kit::SharedString>> {
        let mut paths = Assets.list(path)?;
        paths.extend(ToolIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::registry::ToolRegistry;
    use crate::shell::tool_seed;
    use gpui_kit::AssetSource;

    #[test]
    fn every_seeded_tool_icon_resolves_from_the_app_bundle() {
        let mut registry = ToolRegistry::new();
        tool_seed::register_all(&mut registry).expect("seed registers without duplicates");
        for meta in registry.tools() {
            let path = meta.icon().path();
            let bytes = AppAssets
                .load(&path)
                .unwrap_or_else(|error| panic!("loading {path}: {error}"));
            assert!(bytes.is_some(), "icon {path} is not embedded in the app icon bundle");
        }
    }

    #[test]
    fn the_default_component_icons_still_resolve() {
        for name in [gpui_kit::assets::IconName::Settings, gpui_kit::assets::IconName::Copy] {
            let path = name.path();
            let bytes = AppAssets
                .load(&path)
                .unwrap_or_else(|error| panic!("loading {path}: {error}"));
            assert!(bytes.is_some(), "default component icon {path} was lost");
        }
    }

    #[test]
    fn the_extras_source_lists_exactly_the_embedded_tool_icons() {
        let paths = ToolIcons.list("icons/").expect("extras list");
        assert_eq!(paths.len(), 20);
        for path in &paths {
            let bytes = AppAssets
                .load(path)
                .unwrap_or_else(|error| panic!("loading {path}: {error}"));
            assert!(bytes.is_some(), "listed icon {path} does not load");
        }
    }
}

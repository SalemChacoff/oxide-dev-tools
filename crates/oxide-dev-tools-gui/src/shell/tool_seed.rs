//! `tool_seed` — the tool catalog the shell's navigation spine is built from.
//!
//! Tool pages land from Phase 7.5 on; the metadata (stable id, category,
//! icon, copy keys, search aliases) ships here so the sidebar, the palette,
//! and the home grid are fully exercisable against the real catalog.
//!
//! Id rule: the CLI subcommand name. When two categories share a name
//! (`url`, `json`), every tool with that name carries its category suffix —
//! `url-codec` / `url-validator`, `json-diff` / `json-validator` — so ids
//! stay globally unique and self-describing.

use gpui_kit::assets::IconName;

use crate::shared::registry::{Category, RegistryError, ToolMeta, ToolRegistry};

/// Registers the full planned tool catalog (Phases 7.5–7.10 ship the pages).
pub fn register_all(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    register_generators(registry)?;
    register_codecs(registry)?;
    register_text(registry)?;
    register_converters(registry)?;
    register_validators(registry)?;
    register_comparators(registry)?;
    Ok(())
}

/// One catalog entry: copy lives in `locales/ui.yml` under
/// `app.tools.<category>.<id>.*` (English default).
fn tool(id: &'static str, category: Category, icon: IconName) -> ToolMeta {
    ToolMeta::new(
        id,
        category,
        icon,
        format!("app.tools.{}.{id}.title", category.id()),
        format!("app.tools.{}.{id}.description", category.id()),
    )
}

fn register_generators(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    registry.register(
        tool("id", Category::Generators, IconName::Barcode)
            .with_alias("uuid")
            .with_alias("ulid")
            .with_alias("nanoid"),
    )?;
    registry.register(
        tool("key", Category::Generators, IconName::KeyRound)
            .with_alias("password")
            .with_alias("token")
            .with_alias("jwt"),
    )?;
    registry.register(
        tool("lorem", Category::Generators, IconName::Type)
            .with_alias("lipsum")
            .with_alias("paragraphs"),
    )?;
    registry.register(
        tool("fake", Category::Generators, IconName::UserRound)
            .with_alias("persona")
            .with_alias("person")
            .with_alias("profile"),
    )?;
    registry.register(
        tool("sample", Category::Generators, IconName::FileText)
            .with_alias("pdf")
            .with_alias("png")
            .with_alias("jpg"),
    )?;
    Ok(())
}

fn register_codecs(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    registry.register(
        tool("base64", Category::Codecs, IconName::Binary)
            .with_alias("encode")
            .with_alias("decode")
            .with_alias("b64"),
    )?;
    registry.register(tool("hex", Category::Codecs, IconName::Hash).with_alias("hexadecimal"))?;
    registry.register(
        tool("url-codec", Category::Codecs, IconName::Link)
            .with_alias("url")
            .with_alias("encode")
            .with_alias("decode")
            .with_alias("percent"),
    )?;
    Ok(())
}

fn register_text(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    registry.register(
        tool("case", Category::Text, IconName::CaseSensitive)
            .with_alias("lowercase")
            .with_alias("uppercase")
            .with_alias("camel")
            .with_alias("snake"),
    )?;
    registry.register(
        tool("codec", Category::Text, IconName::Braces)
            .with_alias("html")
            .with_alias("entities")
            .with_alias("unicode"),
    )?;
    registry.register(
        tool("count", Category::Text, IconName::Ruler)
            .with_alias("statistics")
            .with_alias("words")
            .with_alias("lines"),
    )?;
    registry.register(
        tool("scan", Category::Text, IconName::ScanLine)
            .with_alias("whitespace")
            .with_alias("hidden")
            .with_alias("non-ascii"),
    )?;
    registry.register(
        tool("truncate", Category::Text, IconName::Ellipsis)
            .with_alias("clip")
            .with_alias("pad"),
    )?;
    Ok(())
}

fn register_converters(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    registry.register(
        tool("doc", Category::Converters, IconName::ArrowLeftRight)
            .with_alias("json")
            .with_alias("yaml")
            .with_alias("xml")
            .with_alias("convert"),
    )?;
    registry.register(
        tool("timestamp", Category::Converters, IconName::Clock)
            .with_alias("unix")
            .with_alias("date")
            .with_alias("epoch"),
    )?;
    registry.register(
        tool("unit", Category::Converters, IconName::Scale)
            .with_alias("length")
            .with_alias("mass")
            .with_alias("storage")
            .with_alias("time"),
    )?;
    Ok(())
}

fn register_validators(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    registry.register(tool("card", Category::Validators, IconName::CreditCard).with_alias("credit"))?;
    registry.register(tool("email", Category::Validators, IconName::Mail).with_alias("mail"))?;
    registry.register(
        tool("file", Category::Validators, IconName::Package)
            .with_alias("filetype")
            .with_alias("magic")
            .with_alias("mime"),
    )?;
    registry.register(
        tool("ip", Category::Validators, IconName::Network)
            .with_alias("ipv4")
            .with_alias("ipv6")
            .with_alias("address"),
    )?;
    registry.register(
        tool("json-validator", Category::Validators, IconName::Braces)
            .with_alias("json")
            .with_alias("syntax"),
    )?;
    registry.register(
        tool("password", Category::Validators, IconName::Eye)
            .with_alias("strength")
            .with_alias("entropy"),
    )?;
    registry.register(
        tool("url-validator", Category::Validators, IconName::Link)
            .with_alias("url")
            .with_alias("uri"),
    )?;
    registry.register(tool("uuid", Category::Validators, IconName::Barcode).with_alias("uuid"))?;
    registry.register(tool("xml", Category::Validators, IconName::Braces).with_alias("syntax"))?;
    registry.register(tool("yaml", Category::Validators, IconName::Braces).with_alias("syntax"))?;
    Ok(())
}

fn register_comparators(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    registry.register(
        tool("dir", Category::Comparators, IconName::FolderTree)
            .with_alias("directory")
            .with_alias("folders"),
    )?;
    registry.register(
        tool("json-diff", Category::Comparators, IconName::GitCompare)
            .with_alias("json")
            .with_alias("compare"),
    )?;
    registry.register(
        tool("semver", Category::Comparators, IconName::ArrowUpDown)
            .with_alias("version")
            .with_alias("compare"),
    )?;
    registry.register(tool("text", Category::Comparators, IconName::Diff).with_alias("diff"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        register_all(&mut registry).expect("seed registers without duplicates");
        registry
    }

    #[test]
    fn seeds_the_full_catalog_without_duplicate_ids() {
        let registry = seeded();
        assert_eq!(registry.tools().len(), 30);

        // Every category has its tools, in the sidebar order.
        for category in Category::ALL {
            assert!(registry.by_category(category).next().is_some(), "category {} is empty", category.id());
        }
    }

    #[test]
    fn colliding_cli_names_carry_their_category_suffix() {
        let registry = seeded();
        let urls: Vec<&str> = registry.search("url").iter().map(|tool| tool.id().as_ref()).collect();
        assert!(urls.contains(&"url-codec"));
        assert!(urls.contains(&"url-validator"));
        assert_ne!(urls.len(), 0);

        let jsons: Vec<&str> = registry.search("json").iter().map(|tool| tool.id().as_ref()).collect();
        assert!(jsons.contains(&"json-diff"));
        assert!(jsons.contains(&"json-validator"));
    }

    #[test]
    fn every_tool_resolves_its_copy_keys() {
        let registry = seeded();
        for meta in registry.tools() {
            let title = meta.title();
            let description = meta.description();
            assert!(!title.is_empty());
            assert!(!description.is_empty());
            // The keys must exist in locales/ui.yml; a missing key would
            // render as the raw key path.
            assert!(!title.starts_with("app."), "missing locale key for {}", meta.id());
            assert!(!description.starts_with("app."), "missing locale key for {}", meta.id());
        }
    }
}

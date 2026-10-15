//! `category_item` — sidebar tool entries: registry metadata mapped onto the
//! gpui-kit sidebar menu items.
//!
//! The selection marker follows the design guides: the active item tints its
//! own surface (`sidebar_accent`) with the accent foreground — no leading-edge
//! bar. Element ids stay with the `SidebarMenu` (it derives item ids from the
//! menu id); the shell owns menu ids in Phase 7.4.

use gpui_kit::component::sidebar::{SidebarGroup, SidebarMenuItem};
use gpui_kit::{App, ClickEvent, Window};

use crate::shared::registry::{Category, ToolMeta};

/// Maps one registered tool onto a sidebar item.
///
/// `on_open` fires on pointer or keyboard activation; the label and icon come
/// from the registry entry, so the sidebar never re-declares tool copy.
pub fn sidebar_tool_item(
    meta: &ToolMeta,
    active: bool,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> SidebarMenuItem {
    SidebarMenuItem::new(meta.title())
        .icon(meta.icon())
        .active(active)
        .on_click(on_open)
}

/// Builds one collapsible sidebar group for a category.
pub fn sidebar_category_group(
    category: Category,
    items: impl IntoIterator<Item = SidebarMenuItem>,
) -> SidebarGroup<SidebarMenuItem> {
    SidebarGroup::new(category.title()).children(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::registry::ToolRegistry;
    use gpui_kit::assets::IconName;
    use gpui_kit::component::sidebar::{SidebarItem as _, SidebarMenu};
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, div,
    };
    use std::cell::Cell;
    use std::rc::Rc;

    fn fixture_meta() -> ToolMeta {
        ToolMeta::new(
            "uuid",
            Category::Generators,
            IconName::KeyRound,
            "app.generators.uuid.title",
            "app.generators.uuid.description",
        )
    }

    #[test]
    fn registry_metadata_drives_the_item_without_redeclared_copy() {
        let mut registry = ToolRegistry::new();
        registry.register(fixture_meta()).unwrap();
        let meta = registry.find("uuid").expect("registered");

        // The mapping consumes only registry data — the sidebar never holds
        // its own label or icon.
        let _item = sidebar_tool_item(meta, true, |_, _, _| {});
        let _group =
            sidebar_category_group(Category::Generators, std::iter::once(sidebar_tool_item(meta, false, |_, _, _| {})));
    }

    struct MenuTestView {
        opened: Rc<Cell<bool>>,
        active: bool,
    }

    impl Render for MenuTestView {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let opened = self.opened.clone();
            let menu = SidebarMenu::new()
                .child(sidebar_tool_item(&fixture_meta(), self.active, move |_, _, _| opened.set(true)));
            div().p_4().child(menu.render("menu", window, cx))
        }
    }

    #[gpui_kit::test]
    fn sidebar_item_activates_the_tool(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let opened = Rc::new(Cell::new(false));
        let (_, cx) = cx.add_window_view(|_, _| MenuTestView {
            opened: opened.clone(),
            active: false,
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            // SidebarMenu derives item ids from the menu id; the first item
            // is `menu-0`.
            assert!(window.try_find("menu-0").is_some());
            assert!(!opened.get());

            window.click("menu-0", cx);
            assert!(opened.get());
        });
    }
}

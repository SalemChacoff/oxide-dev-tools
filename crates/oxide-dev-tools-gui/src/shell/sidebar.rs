//! `sidebar` — the persistent category navigation: six collapsible groups
//! built from the tool registry, plus the Settings slot in the footer.
//!
//! Rows are app-owned focusable controls (`shared::category_item`) rather than
//! gpui-kit 0.7.1 `SidebarMenuItem`s: the library item renders a TreeItem
//! `div` without a focus handle, so pointer clicks worked while Tab could not
//! reach a tool. Each row here is a Button-role control with a stable ID, an
//! accessible name, selected/expanded state, a Tab stop, and Enter/Space
//! activation. The active tool tints its own surface (`sidebar_accent`) with
//! the accent foreground; category rows disclose their tool list with a
//! chevron and `aria_expanded`.

use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::h_flex;
use gpui_kit::component::sidebar::{Sidebar, SidebarCollapsible, SidebarItem as _};
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Role, StatefulInteractiveElement as _,
    Styled as _, Window, div,
};

use crate::shared::category_item::{SidebarNavGroup, SidebarNavItem};
use crate::shared::registry::{Category, registry};
use crate::shell::navigation::Destination;
use crate::shell::shell_view::ShellView;

impl ShellView {
    /// The window's navigation sidebar (width owned by the resizable panel).
    pub(crate) fn render_sidebar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = cx.entity();
        let active_tool = self.destination.tool_id().cloned();

        let groups = Category::ALL
            .into_iter()
            .map(|category| {
                let items = registry(cx)
                    .by_category(category)
                    .map(|meta| {
                        let id = meta.id().clone();
                        let is_active = active_tool.as_ref().is_some_and(|tool| tool == meta.id());
                        let shell = shell.clone();
                        SidebarNavItem::for_tool(meta, is_active, move |_, window, cx| {
                            shell.update(cx, |shell, cx| shell.open_tool(id.clone(), window, cx));
                        })
                    })
                    .collect::<Vec<_>>();
                let shell = shell.clone();
                SidebarNavGroup::new(category, items, self.is_category_expanded(category), move |_, _, cx| {
                    shell.update(cx, |shell, cx| shell.toggle_category(category, cx));
                })
            })
            .collect::<Vec<_>>();

        div()
            .id("shell-navigation")
            .role(Role::Navigation)
            .aria_label(rust_i18n::t!("app.shell.navigation"))
            .size_full()
            .test_support()
            .child(
                Sidebar::new("shell-sidebar")
                    .collapsible(SidebarCollapsible::None)
                    .w_full()
                    .children(groups)
                    .footer(h_flex().w_full().child(self.render_settings_slot(shell).render(
                        "sidebar-settings",
                        window,
                        cx,
                    ))),
            )
    }

    /// The Settings destination slot at the sidebar's bottom.
    fn render_settings_slot(&self, shell: Entity<Self>) -> SidebarNavItem {
        let selected = self.destination == Destination::Settings;

        SidebarNavItem::new(
            "sidebar-settings",
            IconName::Settings,
            rust_i18n::t!("app.settings.title"),
            selected,
            move |_, _, cx| {
                shell.update(cx, |shell, cx| shell.open_settings(cx));
            },
        )
    }
}

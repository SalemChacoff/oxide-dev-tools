//! `sidebar` — the persistent category navigation: six collapsible groups
//! built from the tool registry, plus the Settings slot in the footer.
//!
//! The active tool row carries the library's `active` treatment (tinted
//! surface, accent foreground). Group rows toggle their tool list on click,
//! like the mock's `<details>` groups; the caret stays visible as the
//! disclosure affordance. Element ids follow the `Sidebar` contract: groups
//! key on their position, tool rows on `{group}-{position}` — registration
//! order is a bootstrap constant, so these ids are stable.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::sidebar::{Sidebar, SidebarCollapsible, SidebarMenuItem};
use gpui_kit::component::{Icon, Selectable as _, h_flex};
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, Window};

use crate::shared::registry::{Category, registry};
use crate::shared::sidebar_tool_item;
use crate::shell::navigation::Destination;
use crate::shell::shell_view::ShellView;

impl ShellView {
    /// The window's navigation sidebar (width owned by the resizable panel).
    pub(crate) fn render_sidebar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = cx.entity();
        let active_tool = self.destination.tool_id();

        let groups = Category::ALL
            .into_iter()
            .map(|category| {
                let items = registry(cx)
                    .by_category(category)
                    .map(|meta| {
                        let id = meta.id().clone();
                        let is_active = active_tool.is_some_and(|tool| tool == meta.id());
                        sidebar_tool_item(meta, is_active, {
                            let shell = shell.clone();
                            move |_, window, cx| {
                                shell.update(cx, |shell, cx| shell.open_tool(id.clone(), window, cx));
                            }
                        })
                    })
                    .collect::<Vec<_>>();

                SidebarMenuItem::new(category.title())
                    .children(items)
                    .default_open(true)
                    .click_to_toggle(true)
            })
            .collect::<Vec<_>>();

        Sidebar::new("shell-sidebar")
            .collapsible(SidebarCollapsible::None)
            .w_full()
            .children(groups)
            .footer(h_flex().child(self.render_settings_slot(shell)))
    }

    /// The Settings destination slot at the sidebar's bottom.
    fn render_settings_slot(&self, shell: gpui_kit::Entity<Self>) -> impl IntoElement {
        Button::new("sidebar-settings")
            .ghost()
            .w_full()
            .icon(Icon::new(IconName::Settings).size_4())
            .label(rust_i18n::t!("app.settings.title").into_owned())
            .selected(self.destination == Destination::Settings)
            .on_click(move |_, _, cx| {
                shell.update(cx, |shell, cx| shell.open_settings(cx));
            })
    }
}

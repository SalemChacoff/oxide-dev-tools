//! `category_item` — accessible sidebar entries: one focusable row per tool,
//! one disclosure row per category, and the Settings footer slot.
//!
//! gpui-kit 0.7.1's `SidebarMenuItem` renders a `TreeItem` `div` with a click
//! handler but no focus handle, so pointer activation works while Tab cannot
//! reach a tool. These entries implement the custom-control recipe instead:
//! stable ID, Button role, accessible name, selected/expanded state, a tracked
//! Tab stop, a visible focus ring, and one `on_click` listener that serves
//! pointer, Enter/Space, and the AccessKit Click action.
//!
//! The caller (the shell) owns navigation and disclosure state; every row is
//! rendered from that model and calls back into it.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::sidebar::SidebarItem;
use gpui_kit::component::{ActiveTheme as _, Collapsible, Icon, StyledExt as _, v_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _,
};

use crate::shared::ClickHandler;
use crate::shared::registry::{Category, ToolMeta};
use crate::theme::OxideTheme;

/// One focusable navigation row (a tool, or the Settings footer slot).
#[derive(Clone)]
pub struct SidebarNavItem {
    id: ElementId,
    icon: IconName,
    label: SharedString,
    active: bool,
    on_click: ClickHandler,
    collapsed: bool,
}

impl SidebarNavItem {
    /// Creates a row keyed by a stable element id.
    pub fn new(
        id: impl Into<ElementId>,
        icon: IconName,
        label: impl Into<SharedString>,
        active: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            active,
            on_click: Rc::new(on_click),
            collapsed: false,
        }
    }

    /// Builds a tool row from registry metadata, so the sidebar never
    /// re-declares tool copy or icons.
    pub fn for_tool(
        meta: &ToolMeta,
        active: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::new((ElementId::from("sidebar-tool"), meta.id().clone()), meta.icon(), meta.title(), active, on_click)
    }

    /// The stable row identity.
    pub fn id(&self) -> &ElementId {
        &self.id
    }
}

impl Collapsible for SidebarNavItem {
    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    fn is_collapsed(&self) -> bool {
        self.collapsed
    }
}

impl SidebarItem for SidebarNavItem {
    fn render(self, _id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = nav_focus_handle(&self.id, window, cx);
        let tokens = OxideTheme::tokens(cx);
        let radius = cx.theme().radius;
        let row_id = self.id;
        let label = self.label;
        let icon = self.icon;
        let active = self.active;
        let on_click = self.on_click;
        let active_bg = cx.theme().sidebar_accent;
        let active_fg = cx.theme().sidebar_accent_foreground;
        let idle_fg = cx.theme().sidebar_foreground;
        let hover_bg = cx.theme().sidebar_accent.opacity(0.8);
        let border_transparent = tokens.background.opacity(0.);

        div()
            .id(row_id)
            .test_support()
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_selected(active)
            .track_focus(&focus)
            .focus_visible(|style| style.border_color(tokens.primary_ring))
            .w_full()
            .h_7()
            .px_2()
            .gap_2()
            .flex()
            .items_center()
            .rounded(radius)
            .border_1()
            .border_color(border_transparent)
            .text_sm()
            .when(active, |this| this.font_medium().bg(active_bg).text_color(active_fg))
            .when(!active, |this| {
                this.text_color(idle_fg)
                    .hover(move |style| style.bg(hover_bg).text_color(active_fg))
            })
            .on_click(move |event, window, cx| on_click(event, window, cx))
            .child(Icon::new(icon).size_4())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(label),
            )
    }
}

/// One category group: a disclosure row plus its tool rows while open.
#[derive(Clone)]
pub struct SidebarNavGroup {
    category: Category,
    items: Vec<SidebarNavItem>,
    open: bool,
    on_toggle: ClickHandler,
    collapsed: bool,
}

impl SidebarNavGroup {
    /// Creates a group whose disclosure state is owned by the caller.
    pub fn new(
        category: Category,
        items: impl IntoIterator<Item = SidebarNavItem>,
        open: bool,
        on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            category,
            items: items.into_iter().collect(),
            open,
            on_toggle: Rc::new(on_toggle),
            collapsed: false,
        }
    }
}

impl Collapsible for SidebarNavGroup {
    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    fn is_collapsed(&self) -> bool {
        self.collapsed
    }
}

impl SidebarItem for SidebarNavGroup {
    fn render(self, _id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let group_id: ElementId = (ElementId::from("sidebar-group"), self.category.id()).into();
        let focus = nav_focus_handle(&group_id, window, cx);
        let click_focus = focus.clone();
        let tokens = OxideTheme::tokens(cx);
        let radius = cx.theme().radius;
        let expanded = self.open && !self.collapsed;
        let title = self.category.title();
        let on_toggle = self.on_toggle;
        let hover_bg = cx.theme().sidebar_accent.opacity(0.8);
        let idle_fg = cx.theme().sidebar_foreground.opacity(0.7);
        let active_fg = cx.theme().sidebar_accent_foreground;
        let border_transparent = tokens.background.opacity(0.);

        let group_row = div()
            .id(group_id)
            .test_support()
            .role(Role::Button)
            .aria_label(title.clone())
            .aria_expanded(expanded)
            .track_focus(&focus)
            .focus_visible(|style| style.border_color(tokens.primary_ring))
            .w_full()
            .h_8()
            .px_2()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .rounded(radius)
            .border_1()
            .border_color(border_transparent)
            .text_xs()
            .text_color(idle_fg)
            .hover(move |style| style.bg(hover_bg).text_color(active_fg))
            .on_click(move |event, window, cx| {
                // Collapsing must not strand focus on a child that unmounts.
                click_focus.focus(window, cx);
                on_toggle(event, window, cx);
            })
            .child(div().flex_1().min_w_0().overflow_hidden().child(title))
            .child(
                Icon::new(if expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size_4(),
            );

        let mut rows: Vec<AnyElement> = Vec::new();
        if expanded {
            for item in self.items {
                let id = item.id().clone();
                rows.push(item.render(id, window, cx).into_any_element());
            }
        }

        v_flex().w_full().gap_1().child(group_row).when(expanded, |this| {
            this.child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .ml_3p5()
                    .pl_2p5()
                    .border_l_1()
                    .border_color(cx.theme().sidebar_border)
                    .children(rows),
            )
        })
    }
}

/// A focus handle that participates in Tab traversal and survives rerenders.
fn nav_focus_handle(id: &ElementId, window: &mut Window, cx: &mut App) -> FocusHandle {
    window
        .use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle().tab_stop(true))
        .read(cx)
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Context, Render, TestAppContext, VisualTestContext, px};
    use std::cell::Cell;

    struct ItemHarness {
        clicked: Rc<Cell<bool>>,
        active: bool,
    }

    impl Render for ItemHarness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let clicked = self.clicked.clone();
            v_flex().w(px(240.)).child(
                SidebarNavItem::new("test-tool", IconName::KeyRound, "UUID", self.active, move |_, _, _| {
                    clicked.set(true)
                })
                .render("test-tool", window, cx),
            )
        }
    }

    #[gpui_kit::test]
    fn tool_row_is_named_selectable_and_reachable_from_the_keyboard(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        let clicked = Rc::new(Cell::new(false));
        let (_view, cx) = cx.add_window_view(|_, _| ItemHarness {
            clicked: clicked.clone(),
            active: true,
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let row = window.try_find("test-tool").expect("the tool row renders");
            assert_eq!(row.role(), Some(Role::Button));
            assert_eq!(row.label(), Some("UUID"));
            assert_eq!(row.selected(), Some(true));
            assert!(!clicked.get());

            window.focus_next(cx);
            _ = window.draw(cx);
            let row = window.try_find("test-tool").expect("the tool row renders");
            assert_eq!(row.focused(), Some(true), "the row is a tab stop");

            window.press("enter", cx);
            assert!(clicked.get(), "Enter activates the row");

            clicked.set(false);
            window.press("space", cx);
            assert!(clicked.get(), "Space activates the row");
        });
    }

    struct GroupHarness {
        open: Rc<Cell<bool>>,
        toggled: Rc<Cell<usize>>,
    }

    impl Render for GroupHarness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let open = self.open.clone();
            let toggled = self.toggled.clone();
            let item = SidebarNavItem::new("group-child", IconName::KeyRound, "UUID", false, |_, _, _| {});
            let group = SidebarNavGroup::new(Category::Generators, [item], open.get(), move |_, _, cx| {
                open.set(!open.get());
                toggled.set(toggled.get() + 1);
                cx.refresh_windows();
            });

            v_flex().w(px(240.)).child(group.render("test-group", window, cx))
        }
    }

    #[gpui_kit::test]
    fn group_row_exposes_expanded_state_and_toggles_from_the_keyboard(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        let open = Rc::new(Cell::new(false));
        let toggled = Rc::new(Cell::new(0));
        let (_view, cx) = cx.add_window_view(|_, _| GroupHarness {
            open: open.clone(),
            toggled: toggled.clone(),
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let group = window
                .try_find((ElementId::from("sidebar-group"), "generators"))
                .expect("the group renders");
            assert_eq!(group.role(), Some(Role::Button));
            assert_eq!(group.label(), Some("Generators"));
            assert_eq!(group.expanded(), Some(false));
            assert!(window.try_find("group-child").is_none());

            window.focus_next(cx);
            _ = window.draw(cx);
            let group = window
                .try_find((ElementId::from("sidebar-group"), "generators"))
                .expect("the group renders");
            assert_eq!(group.focused(), Some(true));

            window.press("enter", cx);
            assert_eq!(toggled.get(), 1);
        });

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let group = window
                .try_find((ElementId::from("sidebar-group"), "generators"))
                .expect("the group renders");
            assert_eq!(group.expanded(), Some(true));
            assert!(window.try_find("group-child").is_some(), "opening reveals the tool rows");
        });
    }
}

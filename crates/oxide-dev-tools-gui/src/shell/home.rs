//! `home` — the home view: recent runs (when non-empty) above tool-card
//! sections grouped by category, with live filtering from the global search.
//!
//! The search query lives in the shell's `InputState`; this view only reads
//! it. Cards keep a fixed width and wrap, like the mock's
//! `minmax(236px, 1fr)` grid. Empty search results show the design spec's
//! no-match state with a `Clear search` action.

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyHeader, EmptyTitle};
use gpui_kit::component::{Sizable as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, Focusable as _, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _,
};

use crate::shared::ToolCard;
use crate::shared::registry::{Category, ToolMeta, registry};
use crate::shell::shell_view::ShellView;

impl ShellView {
    /// The home page: optional recent-runs section, then the catalog.
    pub(crate) fn render_home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search.read(cx).value();
        let show_recent = query.trim().is_empty() && !self.recent.read(cx).is_empty();

        v_flex()
            .id("home-page")
            .test_support()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .px_6()
            .pb_6()
            .when(show_recent, |this| this.child(self.render_recent_section()))
            .child(self.render_catalog(&query, cx))
    }

    /// The recent-runs history, rendered only when it has entries.
    fn render_recent_section(&self) -> impl IntoElement {
        v_flex()
            .gap_3()
            .pb_6()
            .child(div().text_sm().font_semibold().child(rust_i18n::t!("app.recent.title")))
            .child(self.recent_runs.clone())
    }

    /// The catalog: all categories, or the search hits grouped by category.
    fn render_catalog(&self, query: &str, cx: &mut Context<Self>) -> AnyElement {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            let shell = cx.entity();
            let sections = Category::ALL
                .into_iter()
                .map(|category| {
                    let tools = registry(cx).by_category(category).cloned().collect::<Vec<_>>();
                    self.render_category_section(shell.clone(), category, tools)
                })
                .collect::<Vec<_>>();
            v_flex().gap_6().children(sections).into_any_element()
        } else {
            let hits = registry(cx).search(trimmed);
            if hits.is_empty() {
                self.render_no_matches(trimmed, cx).into_any_element()
            } else {
                let shell = cx.entity();
                let sections = Category::ALL
                    .into_iter()
                    .filter_map(|category| {
                        let tools = hits
                            .iter()
                            .filter(|meta| meta.category() == category)
                            .map(|meta| (*meta).clone())
                            .collect::<Vec<_>>();
                        (!tools.is_empty()).then(|| self.render_category_section(shell.clone(), category, tools))
                    })
                    .collect::<Vec<_>>();
                v_flex().gap_6().children(sections).into_any_element()
            }
        }
    }

    /// One titled card section. The card keeps the library's focus and
    /// keyboard contract; the fixed-width wrapper owns the grid rhythm
    /// (14rem: the mock's ~236px cards on the nearest scale step). The
    /// section is `'static`: cards own their copy and their navigation
    /// closure, so nothing borrows the registry or the render context.
    fn render_category_section(
        &self,
        shell: gpui_kit::Entity<Self>,
        category: Category,
        tools: Vec<ToolMeta>,
    ) -> impl IntoElement + 'static {
        v_flex()
            .gap_3()
            .child(div().text_sm().font_semibold().child(category.title()))
            .child(h_flex().flex_wrap().gap_2().children(tools.iter().map(|meta| {
                let id = meta.id().clone();
                div().w_56().child(ToolCard::new(
                    format!("tool-card-{}", meta.id()),
                    meta.icon(),
                    meta.title(),
                    meta.description(),
                    {
                        let shell = shell.clone();
                        move |_, window, cx| {
                            shell.update(cx, |shell, cx| shell.open_tool(id.clone(), window, cx));
                        }
                    },
                ))
            })))
    }

    /// The no-match state: keep the query visible, offer `Clear search`.
    fn render_no_matches(&self, query: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let search = self.search.clone();
        let shell = cx.entity();

        div().id("home-empty").test_support().mx_auto().w_128().pt_10().child(
            Empty::new()
                .header(
                    EmptyHeader::new().title(
                        EmptyTitle::new().child(rust_i18n::t!("app.state.no_matches", query = query).into_owned()),
                    ),
                )
                .content(
                    EmptyContent::new().child(
                        Button::new("clear-search")
                            .outline()
                            .small()
                            .label(rust_i18n::t!("app.common.clear_search"))
                            .on_click(move |_, window, cx| {
                                search.update(cx, |input, cx| input.set_value("", window, cx));
                                search.read(cx).focus_handle(cx).focus(window, cx);
                                shell.update(cx, |_, cx| cx.notify());
                            }),
                    ),
                ),
        )
    }
}

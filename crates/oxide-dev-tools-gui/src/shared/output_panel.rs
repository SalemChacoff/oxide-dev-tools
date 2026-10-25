//! `output_panel` — the result surface every Pattern A/B tool page renders
//! into: a titled pane with `Copy` and an optional `Export…` action above a
//! scrollable, monospace data block.
//!
//! Output content is **data**, never interpreted markup, so it renders as a
//! plain monospace text run (a `TextView` would parse it as Markdown and
//! corrupt UUIDs, JSON, and diffs that happen to look like syntax). Whitespace
//! runs collapse under the default `Normal` whitespace; pages whose data must
//! keep exact spacing (diffs) revisit this in Phase 7.10.
//!
//! States: content, empty (`Nothing to show`), and loading (spinner row while
//! a run is actually in flight — previous content stays visible). The export
//! seam emits an event; the owning page opens the save dialog (Phase 7.4
//! window support).

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::clipboard::Clipboard;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, Size, StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    App, ClickEvent, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Window, div, prelude::FluentBuilder as _,
};

use crate::shared::ClickHandler;
use crate::theme::OxideTheme;

/// The result pane: title row with `Copy` / `Export…`, then the data surface.
#[derive(IntoElement)]
pub struct OutputPanel {
    id: ElementId,
    content: SharedString,
    binary: Option<Vec<u8>>,
    loading: bool,
    export_handler: Option<ClickHandler>,
}

impl OutputPanel {
    /// Creates an empty panel. The `id` namespaces every child marker id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            content: SharedString::default(),
            binary: None,
            loading: false,
            export_handler: None,
        }
    }

    /// Sets the data to display (user/tool data, never translated).
    pub fn content(mut self, content: impl Into<SharedString>) -> Self {
        self.content = content.into();
        self
    }

    /// Sets binary data to display (sample files). The panel renders a size
    /// summary instead of a text run and hides `Copy` — bytes are exported,
    /// not copied.
    pub fn binary(mut self, bytes: Vec<u8>) -> Self {
        self.binary = Some(bytes);
        self
    }

    /// Shows the loading row while a run is in flight.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Registers the export command; the panel shows `Export…` only when set.
    /// The handler opens the save dialog and writes the file.
    pub fn on_export(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.export_handler = Some(Rc::new(handler));
        self
    }

    /// Whether the panel currently displays data.
    pub fn has_content(&self) -> bool {
        !self.content.is_empty() || self.binary.is_some()
    }

    /// Whether the panel currently displays binary data.
    fn is_binary(&self) -> bool {
        self.binary.is_some()
    }
}

impl RenderOnce for OutputPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_content = self.has_content();
        let is_binary = self.is_binary();
        let mono_family = cx.theme().mono_font_family.clone();

        v_flex()
            .w_full()
            .h_full()
            .bg(OxideTheme::tokens(cx).surface)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_lg()
            .overflow_hidden()
            .child(
                h_flex()
                    .w_full()
                    .px_4()
                    .py_2p5()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(rust_i18n::t!("app.result.title")),
                    )
                    .child(self.render_actions(has_content, is_binary)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .when(self.loading, |this| {
                        this.child(
                            h_flex()
                                .p_3()
                                .gap_2()
                                .items_center()
                                .child(Spinner::new().small())
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(rust_i18n::t!("app.state.running")),
                                )
                                .id((self.id.clone(), "loading"))
                                .test_support(),
                        )
                    })
                    .when(is_binary, |this| {
                        let size = format_size(self.binary.as_ref().map_or(0, Vec::len));
                        this.child(
                            div()
                                .p_3()
                                .font_family(mono_family.clone())
                                .text_sm()
                                .child(size)
                                .id((self.id.clone(), "size"))
                                .test_support(),
                        )
                    })
                    .when(!is_binary && has_content, |this| {
                        this.child(
                            div()
                                .p_3()
                                .font_family(mono_family)
                                .text_sm()
                                .child(self.content.clone())
                                .id((self.id.clone(), "content"))
                                .test_support(),
                        )
                    })
                    .when(!self.loading && !has_content, |this| {
                        this.child(
                            div()
                                .p_3()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(rust_i18n::t!("app.state.nothing_to_show"))
                                .id((self.id.clone(), "empty"))
                                .test_support(),
                        )
                    }),
            )
            .id(self.id)
            .test_support()
    }
}

/// Formats a byte count with thousands separators: `1,234 bytes`.
fn format_size(bytes: usize) -> String {
    let digits = bytes.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (offset, ch) in digits.chars().enumerate() {
        if offset > 0 && (digits.len() - offset).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!("{grouped} bytes")
}

impl OutputPanel {
    /// The pane action row: `Copy` with copied-state feedback (text data
    /// only), `Export…` when the page registered it. Both are disabled while
    /// the panel is empty — the empty state itself names the reason.
    fn render_actions(&self, has_content: bool, is_binary: bool) -> impl IntoElement {
        let export_handler = self.export_handler.clone();
        h_flex()
            .gap_1()
            .when(has_content && !is_binary, |this| {
                this.child(
                    Clipboard::new((self.id.clone(), "copy"))
                        .value(self.content.clone())
                        .tooltip(rust_i18n::t!("app.result.copy_hint"))
                        .with_size(Size::Small)
                        .into_any_element(),
                )
            })
            .when(!has_content && !is_binary, |this| {
                this.child(
                    Button::new((self.id.clone(), "copy"))
                        .ghost()
                        .icon(IconName::Copy)
                        .tooltip(rust_i18n::t!("app.result.copy_hint"))
                        .disabled(true)
                        .small()
                        .into_any_element(),
                )
            })
            .when_some(export_handler, |this, handler| {
                this.child(
                    Button::new((self.id.clone(), "export"))
                        .outline()
                        .label(rust_i18n::t!("app.common.export"))
                        .disabled(!has_content)
                        .small()
                        .on_click(move |event, window, cx| handler(event, window, cx)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::v_flex;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Context, Render, TestAppContext, VisualTestContext};

    struct PanelTestView {
        content: SharedString,
        binary: bool,
        loading: bool,
        with_export: bool,
    }

    impl Render for PanelTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut panel = OutputPanel::new("panel")
                .content(self.content.clone())
                .loading(self.loading);
            if self.binary {
                panel = panel.binary(vec![0; 2_048]);
            }
            if self.with_export {
                panel = panel.on_export(|_, _, _| {});
            }
            v_flex().p_4().child(panel)
        }
    }

    fn open(cx: &mut TestAppContext, view: PanelTestView) -> &mut VisualTestContext {
        let (_, cx) = cx.add_window_view(|_, _| view);
        cx
    }

    #[gpui_kit::test]
    fn content_empty_and_loading_states_are_distinct(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        // Content state: data block and an enabled copy action.
        let cx = open(
            cx,
            PanelTestView {
                content: "9b1deb4d".into(),
                binary: false,
                loading: false,
                with_export: true,
            },
        );
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("panel"), "content")).is_some());
            assert!(window.try_find((ElementId::from("panel"), "empty")).is_none());
            assert!(window.try_find((ElementId::from("panel"), "copy")).is_some());
            assert!(window.try_find((ElementId::from("panel"), "export")).is_some());
        });
    }

    #[gpui_kit::test]
    fn empty_state_hides_export_and_loading_keeps_previous_content(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        // Empty: only the muted empty state, export hidden entirely.
        let cx = open(
            cx,
            PanelTestView {
                content: SharedString::default(),
                binary: false,
                loading: false,
                with_export: false,
            },
        );
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("panel"), "empty")).is_some());
            assert!(window.try_find((ElementId::from("panel"), "content")).is_none());
            assert!(window.try_find((ElementId::from("panel"), "export")).is_none());
        });
    }

    #[gpui_kit::test]
    fn loading_renders_the_spinner_row_beside_content(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        let cx = open(
            cx,
            PanelTestView {
                content: "previous result".into(),
                binary: false,
                loading: true,
                with_export: false,
            },
        );
        cx.update(|window, cx| {
            _ = window.draw(cx);
            // Loading keeps the previous result and adds the progress row.
            assert!(window.try_find((ElementId::from("panel"), "loading")).is_some());
            assert!(window.try_find((ElementId::from("panel"), "content")).is_some());
        });
    }

    #[gpui_kit::test]
    fn binary_content_shows_size_keeps_export_and_hides_copy(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        let cx = open(
            cx,
            PanelTestView {
                content: SharedString::default(),
                binary: true,
                loading: false,
                with_export: true,
            },
        );
        cx.update(|window, cx| {
            _ = window.draw(cx);
            // Binary data renders the size summary, never a text run or copy.
            assert!(window.try_find((ElementId::from("panel"), "size")).is_some());
            assert!(window.try_find((ElementId::from("panel"), "content")).is_none());
            assert!(window.try_find((ElementId::from("panel"), "copy")).is_none());
            assert!(window.try_find((ElementId::from("panel"), "empty")).is_none());
            assert!(window.try_find((ElementId::from("panel"), "export")).is_some());
        });
    }

    #[test]
    fn byte_counts_group_with_thousands_separators() {
        assert_eq!(format_size(0), "0 bytes");
        assert_eq!(format_size(999), "999 bytes");
        assert_eq!(format_size(1_000), "1,000 bytes");
        assert_eq!(format_size(1_234_567), "1,234,567 bytes");
    }

    #[test]
    fn export_seam_and_content_reader_are_stable() {
        let panel = OutputPanel::new("panel");
        assert!(!panel.has_content());
        assert!(!panel.on_export(|_, _, _| {}).has_content());

        let panel = OutputPanel::new("panel").content("data");
        assert!(panel.has_content());
    }
}

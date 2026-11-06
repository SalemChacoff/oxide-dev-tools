//! `output_panel` — the result surface every Pattern A/B tool page renders
//! into: a titled pane with `Copy` and an optional `Export…` action above a
//! scrollable, monospace data block.
//!
//! Output content is **data**, never interpreted markup, so it renders as a
//! plain monospace text run (a `TextView` would parse it as Markdown and
//! corrupt UUIDs, JSON, and diffs that happen to look like syntax). Whitespace
//! runs collapse under the default `Normal` whitespace; pages whose data must
//! keep exact spacing (diffs) revisit this in Phase 7.10. Long text wraps at
//! word boundaries to the pane width, so the result grows vertically and
//! scrolls instead of widening the pane.
//!
//! States: content, empty (`Nothing to show`), and loading (spinner row while
//! a run is actually in flight — previous content stays visible). The export
//! seam emits an event; the owning page opens the platform's native save
//! dialog through it.
//!
//! Two height modes: side-by-side (wide windows) fills the host pane via
//! `h_full`; stacked flow (narrow windows, see
//! `tool_page::stacked_result_cap`) bounds itself — the content area grows
//! with the data up to one viewport minus the page chrome, then scrolls
//! internally, so the panel hugs its content instead of collapsing or
//! stretching the page.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::clipboard::Clipboard;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, Size, StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    App, ClickEvent, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, Window, div, prelude::FluentBuilder as _,
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
    /// The handler opens the platform save dialog and writes the file.
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_content = self.has_content();
        let is_binary = self.is_binary();

        // Stacked flow: the page flows the panel under the form, so the panel
        // bounds itself. The outer column takes `max_h(cap)` and the inner
        // `overflow_hidden` wrapper is the scroll host, the gpui-kit recipe
        // for a max-height pane (`MaxHeightParentTest` in `gpui-component`'s
        // scrollable tests): `max_h` → `flex_1().overflow_hidden()` →
        // `size_full().overflow_y_scrollbar()`. Putting `max_h` on the
        // scrollable element itself copies the cap onto the scroll wrapper,
        // so the data area never gets a definite host height and the page
        // scroll swallows every wheel event. Side-by-side: the host gives the
        // panel a definite height and `h_full` fills it.
        let content_cap = crate::shared::tool_page::stacked_result_cap(window);

        let content_area = match content_cap {
            Some(cap) => v_flex()
                .w_full()
                .min_w_0()
                .max_h(cap)
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(self.content_states(v_flex().size_full(), cx).overflow_y_scrollbar()),
                )
                .into_any_element(),
            None => self
                .content_states(div().flex_1().w_full().min_w_0().min_h_0(), cx)
                .overflow_y_scrollbar()
                .into_any_element(),
        };

        v_flex()
            .w_full()
            .when(content_cap.is_none(), |this| this.h_full())
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
            .child(content_area)
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
    /// The data-surface states (loading / size / content / empty) applied to
    /// a pre-built content container, so the stacked-flow and side-by-side
    /// height modes share one state block.
    fn content_states(&self, base: Div, cx: &mut App) -> Div {
        let has_content = self.has_content();
        let is_binary = self.is_binary();
        let mono_family = cx.theme().mono_font_family.clone();
        let muted_fg = cx.theme().muted_foreground;
        let panel_id = self.id.clone();

        base.when(self.loading, |this| {
            this.child(
                h_flex()
                    .p_3()
                    .gap_2()
                    .items_center()
                    .child(Spinner::new().small())
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted_fg)
                            .child(rust_i18n::t!("app.state.running")),
                    )
                    .id((panel_id.clone(), "loading"))
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
                    .id((panel_id.clone(), "size"))
                    .test_support(),
            )
        })
        .when(!is_binary && has_content, |this| {
            this.child(
                div()
                    .w_full()
                    .p_3()
                    .font_family(mono_family)
                    .text_sm()
                    .whitespace_normal()
                    .child(self.content.clone())
                    .id((panel_id.clone(), "content"))
                    .test_support(),
            )
        })
        .when(!self.loading && !has_content, |this| {
            this.child(
                div()
                    .p_3()
                    .text_sm()
                    .text_color(muted_fg)
                    .child(rust_i18n::t!("app.state.nothing_to_show"))
                    .id((panel_id, "empty"))
                    .test_support(),
            )
        })
    }

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
    use gpui_kit::{
        AppContext as _, Context, Render, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px,
    };

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

    #[gpui_kit::test]
    fn stacked_flow_displays_content_and_stays_near_one_viewport(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        // A narrow window: the page flows the panel under the form, so the
        // panel must bound itself and still display the data (the collapse
        // regression: the content area must not shrink to the header).
        let bounds = gpui_kit::Bounds {
            origin: gpui_kit::Point::default(),
            size: gpui_kit::Size {
                width: gpui_kit::px(700.),
                height: gpui_kit::px(640.),
            },
        };
        let (window, _content) = cx.update(|cx| {
            gpui_kit::open_window(
                gpui_kit::WindowOptions {
                    window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|_| PanelTestView {
                        content: "line\n".repeat(500).into(),
                        binary: false,
                        loading: false,
                        with_export: false,
                    })
                },
            )
            .expect("open test window")
        });
        let cx: &mut VisualTestContext = VisualTestContext::from_window(window, cx).into_mut();

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let panel = window.try_find("panel").expect("panel renders").bounds();
            let content = window
                .try_find((ElementId::from("panel"), "content"))
                .expect("content renders")
                .bounds();

            // The content shows at a readable height (not collapsed to the
            // header) and the panel stays within roughly one viewport.
            assert!(
                content.size.height > gpui_kit::px(48.),
                "stacked content must render at a readable height: {content:?}"
            );
            assert!(
                panel.size.height <= window.viewport_size().height,
                "the stacked panel must stay near one viewport: {panel:?}"
            );
        });
    }

    #[gpui_kit::test]
    fn stacked_content_scrolls_internally_when_it_reaches_the_cap(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        // A narrow window with oversized data: the capped content area must
        // own the wheel event instead of staying pinned while the page scroll
        // moves the whole panel.
        let bounds = gpui_kit::Bounds {
            origin: gpui_kit::Point::default(),
            size: gpui_kit::Size {
                width: px(700.),
                height: px(640.),
            },
        };
        let (window, _content) = cx.update(|cx| {
            gpui_kit::open_window(
                gpui_kit::WindowOptions {
                    window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|_| PanelTestView {
                        content: "line\n".repeat(500).into(),
                        binary: false,
                        loading: false,
                        with_export: false,
                    })
                },
            )
            .expect("open test window")
        });
        let cx: &mut VisualTestContext = VisualTestContext::from_window(window, cx).into_mut();

        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        let before = cx.update(|window, _| {
            window
                .try_find((ElementId::from("panel"), "content"))
                .expect("content renders")
                .bounds()
                .origin
                .y
        });

        // Wheel inside the panel's data area.
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(350.), px(200.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-50.))),
            ..Default::default()
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        let after = cx.update(|window, _| {
            window
                .try_find((ElementId::from("panel"), "content"))
                .expect("content renders")
                .bounds()
                .origin
                .y
        });

        assert!(after < before, "the capped result must scroll internally: before {before:?}, after {after:?}");
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

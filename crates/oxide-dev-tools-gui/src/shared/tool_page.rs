//! `tool_page` — the responsive chrome every tool page renders into: the
//! title, then the options form and the result pane. Wide windows keep the
//! panes side by side (form 24rem, result fills the rest). Below the
//! stacking breakpoint the page becomes one vertical flow: the form keeps
//! its natural height (every input and the run action stay visible), the
//! result sits directly under it capped near the viewport height with its
//! own scroll for huge data, and the page itself scrolls vertically only
//! when the two together overflow the window.

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

/// Window width below which the result pane stacks under the form.
///
/// The design floor is a ~900×640 window: with the sidebar at its 180px
/// minimum the detail area keeps ~720px, exactly what the side-by-side
/// layout needs (24rem form + gap + a readable result pane). Below 960px
/// the panes stack into one scrolling page so every input stays reachable
/// at the minimum size.
pub const STACK_BREAKPOINT: Pixels = px(960.);

/// Fixed vertical chrome the stacked result cap reserves: the title row,
/// `pt_4`, `pb_6`, and slack. The result may grow to the viewport height
/// minus this before it scrolls internally; anything further out scrolls
/// with the page.
const STACKED_RESULT_CHROME_H: Pixels = px(96.);

/// The pattern cap from the design spec (§4): tool content is top-centered.
const CONTENT_MAX_W: Pixels = px(1060.);

/// Room each form pane keeps around its content for the focus ring.
///
/// gpui-kit paints a focused control's ring as a 3px band just outside its
/// 1px border, so the ring reaches 4px past the control's box. Both form
/// panes are scroll containers, whose mask covers both axes; without this
/// padding the ring's left and right edges are cut off at the pane (the
/// design contract is "focus ring, never clipped by overflow").
const FOCUS_RING_CLEARANCE: Pixels = px(4.);

/// The shared tool-page chrome. `form` is the option controls (labels,
/// fields, run action); `result` is the output surface. This wrapper owns
/// the pane layout and its responsive switch.
#[derive(IntoElement)]
pub struct ToolPage {
    title: SharedString,
    form: AnyElement,
    result: AnyElement,
    form_id: ElementId,
}

impl ToolPage {
    /// Creates the chrome for one tool page.
    pub fn new(title: impl Into<SharedString>, form: impl IntoElement, result: impl IntoElement) -> Self {
        Self {
            title: title.into(),
            form: form.into_any_element(),
            result: result.into_any_element(),
            form_id: ElementId::Name(SharedString::from("tool-form")),
        }
    }

    /// The form column's stable identity in side-by-side mode and the page
    /// scroll's identity in stacked mode (scroll position retention across
    /// page switches).
    pub fn form_id(mut self, id: impl Into<ElementId>) -> Self {
        self.form_id = id.into();
        self
    }

    /// Narrow-window panes: one scrolling page. The form keeps its natural
    /// height and the result flows directly under it — the output panel
    /// applies its own stacked cap via [`stacked_result_cap`] — and the page
    /// scrolls only when the two together overflow the window.
    fn stacked_panes(form: AnyElement, result: AnyElement, form_id: ElementId) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .pt_4()
            .w_full()
            .id((ElementId::from("tool-panes"), "stacked"))
            .test_support()
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .id(form_id)
                    .test_support()
                    .p(FOCUS_RING_CLEARANCE)
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .w_full()
                            .mx_auto()
                            .max_w(CONTENT_MAX_W)
                            .gap_4()
                            .child(form)
                            .child(result),
                    ),
            )
    }

    /// Wide-window panes: a 24rem form column with its own scrollbar beside
    /// a result pane that fills the rest.
    fn side_by_side_panes(form: AnyElement, result: AnyElement, form_id: ElementId) -> impl IntoElement {
        h_flex()
            .flex_1()
            .min_h_0()
            .pt_4()
            .gap_4()
            .items_stretch()
            .w_full()
            .mx_auto()
            .max_w(CONTENT_MAX_W)
            .child(
                v_flex()
                    .w_96()
                    .min_h_0()
                    .p(FOCUS_RING_CLEARANCE)
                    .overflow_y_scrollbar()
                    .id(form_id)
                    .child(form),
            )
            .child(div().flex_1().min_h_0().child(result))
            .id((ElementId::from("tool-panes"), "side"))
            .test_support()
    }
}

impl RenderOnce for ToolPage {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let ToolPage {
            title,
            form,
            result,
            form_id,
        } = self;
        let stacked = window.viewport_size().width < STACK_BREAKPOINT;

        v_flex()
            .id("tool-page")
            .test_support()
            .flex_1()
            .min_h_0()
            .w_full()
            .px_6()
            .pb_6()
            .child(
                h_flex()
                    .justify_center()
                    .child(div().text_lg().font_semibold().child(title)),
            )
            .child(if stacked {
                ToolPage::stacked_panes(form, result, form_id).into_any_element()
            } else {
                ToolPage::side_by_side_panes(form, result, form_id).into_any_element()
            })
    }
}

/// The result pane's height cap for narrow windows: `Some` when the panes
/// stack, `None` in side-by-side mode. The stacked page flows the result
/// under the form, so the output panel must bound itself — its content area
/// grows with the data up to one viewport minus the fixed page chrome, then
/// scrolls internally. `OutputPanel` reads this seam; page code must not
/// re-wrap the panel in a height-unbounded flex child.
pub(crate) fn stacked_result_cap(window: &Window) -> Option<Pixels> {
    (window.viewport_size().width < STACK_BREAKPOINT)
        .then(|| (window.viewport_size().height - STACKED_RESULT_CHROME_H).max(px(0.)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::OutputPanel;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AppContext as _, Bounds, Point, Render, Size, TestAppContext, VisualTestContext, WindowBounds, WindowOptions,
    };

    struct ChromeTestView;

    impl Render for ChromeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            // Sized window root: the page's `flex_1` layout needs a definite
            // parent height, which the shell provides in the real app.
            v_flex().size_full().child(
                ToolPage::new(
                    "Test tool",
                    v_flex()
                        .child(div().id("test-control").h_8().w_full().test_support())
                        .child(div().id("test-control-last").h_12().w_full().test_support()),
                    v_flex().id("test-result").test_support().child("result pane"),
                )
                .form_id("test-form"),
            )
        }
    }

    /// A tool page whose form and result both outgrow the minimum window, to
    /// exercise the stacked page scroll and the result cap.
    struct TallChromeTestView;

    impl Render for TallChromeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            v_flex().size_full().child(
                ToolPage::new(
                    "Test tool",
                    v_flex().gap_2().children(numbered_rows("form-row", 40)),
                    OutputPanel::new("tall-output").content("line\n".repeat(500)),
                )
                .form_id("tall-form"),
            )
        }
    }

    /// `count` rows of fixed height; the last one is findable as
    /// `<prefix>-last`.
    fn numbered_rows(prefix: &str, count: usize) -> Vec<AnyElement> {
        (0..count)
            .map(|ix| {
                let row = div().h_12().w_full();
                if ix + 1 == count {
                    row.id(SharedString::from(format!("{prefix}-last")))
                        .test_support()
                        .into_any_element()
                } else {
                    row.into_any_element()
                }
            })
            .collect()
    }

    fn open_view<V: Render + 'static>(
        view: V,
        width: f32,
        height: f32,
        cx: &mut TestAppContext,
    ) -> &mut VisualTestContext {
        let bounds = Bounds {
            origin: Point::default(),
            size: Size {
                width: px(width),
                height: px(height),
            },
        };
        let (window, _content) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |_, cx| cx.new(|_| view),
            )
            .expect("open test window")
        });
        VisualTestContext::from_window(window, cx).into_mut()
    }

    #[gpui_kit::test]
    fn wide_windows_keep_the_panes_side_by_side(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_view(ChromeTestView, 1200., 800., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("tool-panes"), "side")).is_some());
            assert!(window.try_find((ElementId::from("tool-panes"), "stacked")).is_none());
        });
    }

    #[gpui_kit::test]
    fn form_panes_reserve_room_for_the_focus_ring(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_view(ChromeTestView, 1200., 800., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let panes = window
                .try_find((ElementId::from("tool-panes"), "side"))
                .expect("side-by-side panes render")
                .bounds();
            let control = window.try_find("test-control").expect("form control renders").bounds();
            // The 24rem column starts at the pane's left edge (384px at the
            // default 16px base font); the ring band needs FOCUS_RING_CLEARANCE
            // on both sides or the scroll mask cuts it.
            assert!(control.left() - panes.left() >= FOCUS_RING_CLEARANCE);
            assert!(panes.left() + px(384.) - control.right() >= FOCUS_RING_CLEARANCE);
        });
    }

    #[gpui_kit::test]
    fn stacked_panes_reserve_room_for_the_focus_ring(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_view(ChromeTestView, 800., 640., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let form = window
                .try_find("test-form")
                .expect("stacked page scroll renders")
                .bounds();
            let control = window.try_find("test-control").expect("form control renders").bounds();
            // The stacked page scroll masks both axes and the controls sit
            // flush with its edges, so the outward ring band needs
            // FOCUS_RING_CLEARANCE on the left and right or the mask cuts it.
            assert!(control.left() - form.left() >= FOCUS_RING_CLEARANCE);
            assert!(form.right() - control.right() >= FOCUS_RING_CLEARANCE);
        });
    }

    #[gpui_kit::test]
    fn narrow_windows_stack_the_result_under_the_form(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_view(ChromeTestView, 800., 640., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("tool-panes"), "stacked")).is_some());
            assert!(window.try_find((ElementId::from("tool-panes"), "side")).is_none());
        });
    }

    #[gpui_kit::test]
    fn stacked_form_keeps_its_natural_height_and_the_result_sits_below(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_view(ChromeTestView, 800., 640., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let page = window
                .try_find("test-form")
                .expect("stacked page scroll renders")
                .bounds();
            let last = window
                .try_find("test-control-last")
                .expect("last form control renders")
                .bounds();
            let result = window.try_find("test-result").expect("result renders").bounds();

            // The form flows at its natural height: the last control (the run
            // action slot) sits inside the visible page instead of behind an
            // internal form scrollbar.
            assert!(
                last.bottom() <= page.bottom(),
                "the last form control must stay visible: control {last:?}, page {page:?}"
            );
            // The result is the next block, directly under the last input.
            assert!(
                result.top() >= last.bottom(),
                "the result must start below the last form control: result {result:?}, control {last:?}"
            );
        });
    }

    #[gpui_kit::test]
    fn stacked_page_scrolls_when_form_and_result_overflow_and_caps_the_result(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });
        let cx = open_view(TallChromeTestView, 800., 640., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let page = window
                .try_find("tall-form")
                .expect("stacked page scroll renders")
                .bounds();
            let last_form = window
                .try_find("form-row-last")
                .expect("last form row renders")
                .bounds();
            let panel = window.try_find("tall-output").expect("result panel renders").bounds();
            let content = window
                .try_find((ElementId::from("tall-output"), "content"))
                .expect("result content renders")
                .bounds();

            // The form outgrows the window: the page scroll owns the overflow
            // instead of the form hiding inputs behind a scrollbar, and the
            // result sits below the fold for the page scroll to reveal.
            assert!(last_form.bottom() > page.bottom());
            assert!(panel.top() > page.bottom());
            // The result caps near the viewport height: a 500-line output
            // stays within one viewport instead of stretching the page, and
            // the oversized text run overflows the panel as scroll content
            // rather than leaking out.
            let cap = window.viewport_size().height - STACKED_RESULT_CHROME_H;
            assert!(
                panel.size.height <= cap + px(64.),
                "the stacked panel must respect the cap plus its header: panel {panel:?}, cap {cap:?}"
            );
            assert!(
                content.size.height > panel.size.height,
                "the oversized content must overflow the panel and scroll: content {content:?}, panel {panel:?}"
            );
        });
    }
}

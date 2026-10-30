//! `tool_page` — the responsive chrome every tool page renders into: the
//! title, then the options form and the result pane. Wide windows keep the
//! panes side by side (form 24rem, result fills the rest); below the
//! stacking breakpoint the result pane moves under the form, so neither
//! pane collapses at the minimum window size.

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, RenderOnce,
    SharedString, Styled as _, Window, div, px,
};

/// Window width below which the result pane stacks under the form.
///
/// The design floor is a ~900×640 window: with the sidebar at its 180px
/// minimum the detail area keeps ~720px, exactly what the side-by-side
/// layout needs (24rem form + gap + a readable result pane). Below 960px
/// the panes stack so the result keeps its room at the minimum size.
pub const STACK_BREAKPOINT: Pixels = px(960.);

/// The form column's height cap in stacked mode: it keeps the result pane
/// visible at the minimum window height; the form scrolls internally beyond
/// it.
const STACKED_FORM_MAX_H: Pixels = px(384.);

/// The pattern cap from the design spec (§4): tool content is top-centered.
const CONTENT_MAX_W: Pixels = px(1060.);

/// Room the scrolling form pane keeps around its content for the focus ring.
///
/// gpui-kit paints a focused control's ring as a 3px band just outside its
/// 1px border, so the ring reaches 4px past the control's box. A GPUI scroll
/// container masks both axes with one rect; without this padding the ring's
/// left and right edges are cut off at the pane (the design contract is
/// "focus ring, never clipped by overflow").
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

    /// The form column's stable identity (scroll position retention across
    /// page switches).
    pub fn form_id(mut self, id: impl Into<ElementId>) -> Self {
        self.form_id = id.into();
        self
    }
}

impl RenderOnce for ToolPage {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
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
                    .child(div().text_lg().font_semibold().child(self.title)),
            )
            .child(if stacked {
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .pt_4()
                    .gap_4()
                    .w_full()
                    .mx_auto()
                    .max_w(CONTENT_MAX_W)
                    .child(
                        v_flex()
                            .w_full()
                            .flex_none()
                            .max_h(STACKED_FORM_MAX_H)
                            .p(FOCUS_RING_CLEARANCE)
                            .overflow_y_scrollbar()
                            .id(self.form_id.clone())
                            .child(self.form),
                    )
                    .child(div().flex_1().min_h_0().child(self.result))
                    .id((ElementId::from("tool-panes"), "stacked"))
                    .test_support()
                    .into_any_element()
            } else {
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
                            .id(self.form_id)
                            .child(self.form),
                    )
                    .child(div().flex_1().min_h_0().child(self.result))
                    .id((ElementId::from("tool-panes"), "side"))
                    .test_support()
                    .into_any_element()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AppContext as _, Bounds, Point, Render, Size, TestAppContext, VisualTestContext, WindowBounds, WindowOptions,
    };

    struct ChromeTestView;

    impl Render for ChromeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            ToolPage::new(
                "Test tool",
                v_flex().child(div().id("test-control").h_8().w_full().test_support()),
                v_flex().child("result pane"),
            )
            .form_id("test-form")
        }
    }

    fn open_with(width: f32, height: f32, cx: &mut TestAppContext) -> &mut VisualTestContext {
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
                |_, cx| cx.new(|_| ChromeTestView),
            )
            .expect("open test window")
        });
        VisualTestContext::from_window(window, cx).into_mut()
    }

    #[gpui_kit::test]
    fn wide_windows_keep_the_panes_side_by_side(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_with(1200., 800., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("tool-panes"), "side")).is_some());
            assert!(window.try_find((ElementId::from("tool-panes"), "stacked")).is_none());
        });
    }

    #[gpui_kit::test]
    fn form_panes_reserve_room_for_the_focus_ring(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_with(1200., 800., cx);

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
    fn narrow_windows_stack_the_result_under_the_form(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let cx = open_with(800., 640., cx);

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("tool-panes"), "stacked")).is_some());
            assert!(window.try_find((ElementId::from("tool-panes"), "side")).is_none());
        });
    }
}

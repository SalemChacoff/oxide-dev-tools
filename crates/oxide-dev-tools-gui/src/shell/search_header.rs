//! `search_header` — the global tool search: an isolated icon slot divided
//! by a hairline from the text field, with a platform `Ctrl K` chip at the
//! end. Sits at the very top center of every page (home today, tool pages
//! from Phase 7.5 on).
//!
//! The focus ring covers the whole component: the container tracks the
//! input's focus handle, so keyboard visibility is never clipped by the
//! inner input's chrome (which is off).

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex};
use gpui_kit::{
    App, ClickEvent, Entity, Focusable as _, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    Styled as _, Window,
};

use crate::shared::ClickHandler;
use crate::shell::actions::palette_keystroke;
use crate::theme::OxideTheme;

/// The global search box; `on_open_palette` fires from the `Ctrl K` chip.
#[derive(IntoElement)]
pub struct SearchHeader {
    state: Entity<InputState>,
    on_open_palette: ClickHandler,
}

impl SearchHeader {
    /// Creates the header over the shell's search input state.
    pub fn new(
        state: &Entity<InputState>,
        on_open_palette: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            state: state.clone(),
            on_open_palette: Rc::new(on_open_palette),
        }
    }
}

impl RenderOnce for SearchHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_open_palette = self.on_open_palette;
        let focus_handle = self.state.read(cx).focus_handle(cx);
        let tokens = OxideTheme::tokens(cx);

        h_flex()
            .id("global-search")
            .test_support()
            .w_112()
            .h_8()
            .items_center()
            .rounded(cx.theme().radius_lg)
            .border_1()
            .border_color(cx.theme().border)
            .bg(tokens.surface)
            .track_focus(&focus_handle)
            .focus_visible(|style| style.border_color(tokens.primary_ring))
            .child(
                h_flex()
                    .h_full()
                    .px_2p5()
                    .items_center()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(Icon::new(IconName::Search).size_4().text_color(cx.theme().muted)),
            )
            .child(Input::new(&self.state).appearance(false).flex_1())
            .child(
                Button::new("palette-trigger")
                    .ghost()
                    .xsmall()
                    .child(Kbd::new(palette_keystroke()))
                    .on_click(move |event, window, cx| on_open_palette(event, window, cx)),
            )
    }
}

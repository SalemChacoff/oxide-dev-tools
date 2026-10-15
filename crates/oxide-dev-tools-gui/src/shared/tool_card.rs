//! `tool_card` — the home-grid entry: icon, tool name, and one-line
//! description on a quiet card, keyboard-reachable, `Enter` opens the tool.
//!
//! Built on `Button` (Text variant) so the card keeps the library's focus,
//! keyboard, and accessibility contract instead of becoming a clickable
//! `div`. The Text variant leaves the frame to the card — surface fill,
//! hairline border, tinted hover — while the inner column carries the
//! icon (brand accent) and the two copy lines.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Icon, StyledExt as _, v_flex};
use gpui_kit::{
    App, ClickEvent, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Window, div,
};

use crate::shared::ClickHandler;
use crate::theme::OxideTheme;

/// One tool card on the home grid.
#[derive(IntoElement)]
pub struct ToolCard {
    id: ElementId,
    icon: IconName,
    title: SharedString,
    description: SharedString,
    on_open: ClickHandler,
}

impl ToolCard {
    /// Creates the card; `on_open` receives the pointer event that activated
    /// it (keyboard activation reuses Button's semantics).
    pub fn new(
        id: impl Into<ElementId>,
        icon: IconName,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            title: title.into(),
            description: description.into(),
            on_open: Rc::new(on_open),
        }
    }
}

impl RenderOnce for ToolCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_open = self.on_open;
        let tokens = OxideTheme::tokens(cx);

        // The `Button` (Text variant) keeps focus, keyboard, and a11y; the
        // card surface lives on the inner column because Button already owns
        // its variant hover style — a second `hover` refinement on the button
        // would panic (one hover style per element).
        Button::new(self.id)
            .text()
            .cursor_default()
            .rounded_lg()
            .on_click(move |event, window, cx| on_open(event, window, cx))
            .child(
                v_flex()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .items_start()
                    .bg(tokens.surface)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .hover(|style| style.bg(tokens.surface_2))
                    .child(Icon::new(self.icon).size_4().text_color(cx.theme().primary))
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(self.title),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_xs()
                            .text_color(cx.theme().muted)
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(self.description),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Context, Render, TestAppContext, VisualTestContext};
    use std::cell::Cell;

    struct CardTestView {
        opened: Rc<Cell<bool>>,
    }

    impl Render for CardTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let opened = self.opened.clone();
            v_flex().p_4().child(ToolCard::new(
                "card-uuid",
                IconName::KeyRound,
                "UUID",
                "Generate UUIDs (v1–v8), ULIDs, and NanoIDs.",
                move |_, _, _| opened.set(true),
            ))
        }
    }

    #[gpui_kit::test]
    fn renders_the_card_and_activates_on_click(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });

        let opened = Rc::new(Cell::new(false));
        let (_, cx) = cx.add_window_view(|_, _| CardTestView { opened: opened.clone() });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("card-uuid").is_some());
            assert!(!opened.get());

            window.click("card-uuid", cx);
            assert!(opened.get());
        });
    }
}

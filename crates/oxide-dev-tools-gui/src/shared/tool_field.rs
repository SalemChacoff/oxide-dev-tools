//! `tool_field` — one option control with its label and a single message
//! slot: help by default, inline validation while invalid.
//!
//! The message slot renders exactly one line so the form's vertical rhythm
//! stays stable through optional content: validation replaces help, it does
//! not push it aside. Validation is never color alone — a Lucide alert icon
//! accompanies the danger text.

use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::label::Label;
use gpui_kit::component::{ActiveTheme as _, Icon, h_flex, v_flex};
use gpui_kit::{
    App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _,
    Window, div, prelude::FluentBuilder as _,
};

/// Wraps one option control with its label and inline help/validation.
///
/// Stateless: the caller owns the field's values; the child control keeps its
/// own interaction contract (focus, keyboard, a11y) untouched.
#[derive(IntoElement)]
pub struct ToolField {
    id: ElementId,
    label: SharedString,
    help: Option<SharedString>,
    validation: Option<SharedString>,
    child: gpui_kit::AnyElement,
}

impl ToolField {
    /// Creates the field. The `id` namespaces every child marker id.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            help: None,
            validation: None,
            child: child.into_any_element(),
        }
    }

    /// Sets the help line shown while the field is valid.
    pub fn help(mut self, help: impl Into<SharedString>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Sets the validation line; it replaces the help line while set.
    pub fn validation(mut self, validation: impl Into<SharedString>) -> Self {
        self.validation = Some(validation.into());
        self
    }

    /// Whether the field currently carries a validation message.
    pub fn has_validation(&self) -> bool {
        self.validation.is_some()
    }
}

impl RenderOnce for ToolField {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let invalid = self.validation.is_some();
        let message = self.validation.or(self.help);

        v_flex()
            .w_full()
            .gap_1()
            .child(Label::new(self.label))
            .child(self.child)
            .when_some(message, |this, message| {
                let message = div().text_xs().child(message);
                this.child(if invalid {
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(
                            Icon::new(IconName::TriangleAlert)
                                .size_3()
                                .text_color(cx.theme().danger),
                        )
                        .child(message.text_color(cx.theme().danger))
                        .id((self.id.clone(), "alert-icon"))
                        .test_support()
                } else {
                    message
                        .text_color(cx.theme().muted_foreground)
                        .id((self.id.clone(), "help"))
                        .test_support()
                })
            })
            .id(self.id)
            .test_support()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::input::{Input, InputState};
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{AppContext as _, Context, Entity, Render, TestAppContext, VisualTestContext};

    struct FieldTestView {
        input: Entity<InputState>,
    }

    impl FieldTestView {
        fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
            Self {
                input: cx.new(|cx| InputState::new(window, cx)),
            }
        }
    }

    impl Render for FieldTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            v_flex()
                .p_4()
                .child(
                    ToolField::new("field-count", "Count", Input::new(&self.input))
                        .help("Between 1 and 1,000 values per run."),
                )
                .child(
                    ToolField::new("field-email", "Email", Input::new(&self.input))
                        .help("Used for the notification address.")
                        .validation("Enter a valid email address"),
                )
        }
    }

    #[gpui_kit::test]
    fn renders_label_control_and_one_message_slot(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (_, cx) = cx.add_window_view(FieldTestView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            // Both fields render their wrapper and child control.
            assert!(window.try_find("field-count").is_some());
            assert!(window.try_find("field-email").is_some());
            // The valid field shows the help slot, never the validation slot.
            assert!(window.try_find((ElementId::from("field-count"), "help")).is_some());
            assert!(
                window
                    .try_find((ElementId::from("field-count"), "alert-icon"))
                    .is_none()
            );
            // Validation replaces help while invalid.
            assert!(
                window
                    .try_find((ElementId::from("field-email"), "alert-icon"))
                    .is_some()
            );
            assert!(window.try_find((ElementId::from("field-email"), "help")).is_none());
        });
    }

    #[test]
    fn validation_state_is_readable_without_rendering() {
        let field = ToolField::new("field-x", "Label", div()).validation("Required");
        assert!(field.has_validation());

        let field = ToolField::new("field-x", "Label", div()).help("Optional hint");
        assert!(!field.has_validation());
    }
}

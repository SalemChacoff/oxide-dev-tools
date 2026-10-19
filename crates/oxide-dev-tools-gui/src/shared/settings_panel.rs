//! `settings_panel` — theme and defaults, the Phase 7.3 slice of the Settings
//! screen (the static mock's full group list ships later; Density and the
//! behavior switches stay open until the design review decides them).
//!
//! Theme is a `RadioGroup` — a small visible set, applied immediately through
//! `theme::apply` (the only allowed mutation path). Reset defaults is a
//! reversible command: no confirmation dialog, the radio flipping back to
//! System is the visible feedback. Tool pages listen to `on_reset_defaults`
//! from Phase 7.5 on.

use std::rc::Rc;

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, ClickEvent, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Styled as _, Window, div,
};

use crate::shared::ClickHandler;
use crate::theme::{self, ThemePreference};

/// The retained settings model. Tool-defaults state joins it when tool pages
/// ship (7.5+); the panel owns theme preference from Phase 7.3.
#[derive(Default)]
pub struct SettingsState {
    preference: ThemePreference,
}

impl SettingsState {
    /// Creates the default state: System preference.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current theme preference.
    pub fn preference(&self) -> ThemePreference {
        self.preference
    }

    /// Replaces the preference and notifies observers.
    pub fn set_preference(&mut self, preference: ThemePreference, cx: &mut Context<Self>) {
        self.preference = preference;
        cx.notify();
    }

    /// Maps a radio index to its preference (System, Light, Dark).
    pub fn preference_from_index(ix: usize) -> ThemePreference {
        match ix {
            0 => ThemePreference::System,
            1 => ThemePreference::Light,
            _ => ThemePreference::Dark,
        }
    }
}

/// The Settings screen panel: Appearance (theme) and Defaults groups.
pub struct SettingsPanel {
    state: Entity<SettingsState>,
    on_reset_defaults: Option<ClickHandler>,
}

impl SettingsPanel {
    /// Creates the panel with its own state.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            state: cx.new(|_| SettingsState::new()),
            on_reset_defaults: None,
        }
    }

    /// Reuses state owned elsewhere (the shell, Phase 7.4).
    pub fn with_state(state: Entity<SettingsState>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            state,
            on_reset_defaults: None,
        }
    }

    /// The retained settings model.
    pub fn state(&self) -> Entity<SettingsState> {
        self.state.clone()
    }

    /// Registers the reset command; tool pages restore their option defaults.
    pub fn on_reset_defaults(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_reset_defaults = Some(Rc::new(handler));
        self
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preference = self.state.read(cx).preference();
        let selected_index = match preference {
            ThemePreference::System => Some(0),
            ThemePreference::Light => Some(1),
            ThemePreference::Dark => Some(2),
        };

        v_flex()
            .w_full()
            .gap_4()
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .child(rust_i18n::t!("app.settings.appearance")),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .gap_4()
                            .child(
                                v_flex()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().foreground)
                                            .child(rust_i18n::t!("app.settings.theme")),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(rust_i18n::t!("app.settings.theme_help")),
                                    ),
                            )
                            .child(self.render_theme_radios(selected_index, window)),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .child(rust_i18n::t!("app.settings.defaults")),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(rust_i18n::t!("app.settings.defaults_help")),
                            )
                            .child(self.render_reset_button()),
                    ),
            )
            .id("settings-panel")
            .test_support()
    }
}

impl SettingsPanel {
    /// The System / Light / Dark radio group. Applying the theme routes
    /// through `theme::apply` — never a direct theme mutation — and switching
    /// never touches focus or input state.
    fn render_theme_radios(&self, selected_index: Option<usize>, _window: &mut Window) -> impl IntoElement {
        let state = self.state.clone();
        RadioGroup::horizontal("settings-theme")
            .flex_none()
            .selected_index(selected_index)
            .children([
                Radio::new("theme-system").label(rust_i18n::t!("app.settings.system").into_owned()),
                Radio::new("theme-light").label(rust_i18n::t!("app.settings.light").into_owned()),
                Radio::new("theme-dark").label(rust_i18n::t!("app.settings.dark").into_owned()),
            ])
            .on_change(move |ix, window, cx| {
                let preference = SettingsState::preference_from_index(*ix);
                state.update(cx, |state, cx| state.set_preference(preference, cx));
                theme::apply(cx, preference, window.appearance());
            })
    }

    /// Reset defaults: restores the System preference and notifies tool pages.
    /// Reversible, so no confirmation — the radio returning to System is the
    /// feedback.
    fn render_reset_button(&self) -> impl IntoElement {
        let state = self.state.clone();
        let reset_handler = self.on_reset_defaults.clone();
        Button::new("settings-reset-defaults")
            .outline()
            .label(rust_i18n::t!("app.settings.reset_defaults"))
            .small()
            .on_click(move |event, window, cx| {
                state.update(cx, |state, cx| state.set_preference(ThemePreference::System, cx));
                theme::apply(cx, ThemePreference::System, window.appearance());
                if let Some(handler) = reset_handler.as_ref() {
                    handler(event, window, cx);
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{TestAppContext, VisualTestContext};
    use std::cell::Cell;

    fn install(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| theme::apply(cx, ThemePreference::System, gpui_kit::WindowAppearance::Light));
    }

    #[test]
    fn preference_index_mapping_is_total() {
        assert_eq!(SettingsState::preference_from_index(0), ThemePreference::System);
        assert_eq!(SettingsState::preference_from_index(1), ThemePreference::Light);
        assert_eq!(SettingsState::preference_from_index(2), ThemePreference::Dark);
        assert_eq!(SettingsState::preference_from_index(7), ThemePreference::Dark);
    }

    struct SettingsTestView {
        panel: Entity<SettingsPanel>,
    }

    impl Render for SettingsTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            v_flex().p_4().child(self.panel.clone())
        }
    }

    #[gpui_kit::test]
    fn radio_change_updates_the_preference_and_reset_returns_to_system(cx: &mut TestAppContext) {
        install(cx);
        let state = cx.update(|cx| cx.new(|_| SettingsState::new()));
        let resets = Rc::new(Cell::new(0));

        let (_view, cx) = cx.add_window_view(|window, cx| {
            let panel = cx.new(|cx| {
                let resets = resets.clone();
                SettingsPanel::with_state(state.clone(), window, cx)
                    .on_reset_defaults(move |_, _, _| resets.set(resets.get() + 1))
            });
            SettingsTestView { panel }
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("settings-panel").is_some());
            assert_eq!(state.read(cx).preference(), ThemePreference::System);
        });

        // The dark radio is the third index inside the radio group.
        cx.update(|window, cx| {
            let mut scope = window.within("settings-theme");
            scope.click(2usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(state.read(cx).preference(), ThemePreference::Dark);
        });

        // Reset returns to System and notifies tool-page listeners.
        cx.update(|window, cx| {
            window.click("settings-reset-defaults", cx);
            assert_eq!(state.read(cx).preference(), ThemePreference::System);
            assert_eq!(resets.get(), 1);
        });
    }
}

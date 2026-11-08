//! `titlebar` — the window title-bar chrome: the brand (navigates Home) and
//! the theme control (System / Light / Dark), matching the mock.
//!
//! The theme control shares the shell's `SettingsState` with the settings
//! page — one source of truth — and every change routes through
//! `theme::apply`, the only allowed theme mutation path.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::DropdownMenu as _;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, TitleBar, h_flex};
use gpui_kit::{Anchor, Context, IntoElement, ParentElement as _, Styled as _, div};

use crate::shell::actions::{SetDarkTheme, SetLightTheme, SetSystemTheme};
use crate::shell::shell_view::ShellView;
use crate::theme::ThemePreference;

impl ShellView {
    /// The title bar: brand left (opens Home), theme control right; window
    /// controls stay platform-native.
    pub(crate) fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let preference = self.settings.read(cx).preference();

        TitleBar::new()
            .child(self.render_brand(cx))
            .child(self.render_theme_control(preference))
    }

    /// The brand mark; it is the Home destination's button.
    fn render_brand(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = cx.entity();
        Button::new("brand")
            .ghost()
            .small()
            .cursor_default()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().size_2().rounded_full().bg(cx.theme().primary))
                    .child(div().text_sm().font_medium().child(rust_i18n::t!("app.shell.brand"))),
            )
            .on_click(move |_, _, cx| {
                shell.update(cx, |shell, cx| shell.go_home(cx));
            })
    }

    /// The System / Light / Dark dropdown, backed by the shared settings state.
    fn render_theme_control(&self, preference: ThemePreference) -> impl IntoElement {
        let label = preference_label(preference);

        Button::new("titlebar-theme")
            .ghost()
            .small()
            .label(label)
            .icon(Icon::new(IconName::ChevronDown).size_3p5())
            .dropdown_menu_with_anchor(Anchor::BottomRight, move |menu, _, _| {
                menu.menu_with_check(
                    rust_i18n::t!("app.settings.system").into_owned(),
                    preference == ThemePreference::System,
                    Box::new(SetSystemTheme),
                )
                .menu_with_check(
                    rust_i18n::t!("app.settings.light").into_owned(),
                    preference == ThemePreference::Light,
                    Box::new(SetLightTheme),
                )
                .menu_with_check(
                    rust_i18n::t!("app.settings.dark").into_owned(),
                    preference == ThemePreference::Dark,
                    Box::new(SetDarkTheme),
                )
            })
    }
}

/// The localized label of one theme preference (reuses the settings copy).
fn preference_label(preference: ThemePreference) -> String {
    match preference {
        ThemePreference::System => rust_i18n::t!("app.settings.system").into_owned(),
        ThemePreference::Light => rust_i18n::t!("app.settings.light").into_owned(),
        ThemePreference::Dark => rust_i18n::t!("app.settings.dark").into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_labels_match_the_settings_copy() {
        assert_eq!(preference_label(ThemePreference::System), "System");
        assert_eq!(preference_label(ThemePreference::Light), "Light");
        assert_eq!(preference_label(ThemePreference::Dark), "Dark");
    }
}

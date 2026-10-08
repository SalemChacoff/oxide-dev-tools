//! Application i18n wiring.
//!
//! English is the default language: every user-visible string in the GUI
//! resolves through `rust_i18n::t!` against `locales/ui.yml`, and every key
//! in that file must define `en` (also the fallback for unknown locales).
//! The `gpui-kit-i18n` agent skill owns this contract — read it before
//! adding or changing any UI string.

use gpui_kit::App;

/// Register the application's locales with GPUI Component.
///
/// Must run once at startup, before `gpui_kit::init(cx)`, so component
/// lookups see the application's translations first (then the component
/// built-ins). The window bootstrap lands in Phase 7.4; `main` will call
/// this from inside `gpui_kit::application().run(...)`.
///
/// The alias is required: `extend!` derives the lookup namespace from the
/// identifier, which must be `gpui_component` (the crate name with hyphens
/// converted to underscores).
pub fn init(_cx: &mut App) {
    use gpui_kit::component as gpui_component;
    rust_i18n::extend!(gpui_component);
}

#[cfg(test)]
mod tests {
    // Locale is process-global state (`rust_i18n::set_locale`); the checks
    // run in one test so parallel tests cannot race on it. Always reset.
    #[test]
    fn english_is_the_default_and_the_fallback() {
        rust_i18n::set_locale("en");
        assert_eq!(rust_i18n::t!("app.common.copy"), "Copy");
        assert_eq!(rust_i18n::t!("app.settings.title"), "Settings");

        // Unknown locales fall back to English instead of returning the key.
        rust_i18n::set_locale("xx-YY");
        assert_eq!(rust_i18n::t!("app.common.copy"), "Copy");
        assert_eq!(rust_i18n::t!("app.state.running"), "Running…");

        // Interpolation works through the fallback as well.
        assert_eq!(rust_i18n::t!("app.state.no_matches", query = "uuid"), "No tools match “uuid”");

        rust_i18n::set_locale("en");
    }
}

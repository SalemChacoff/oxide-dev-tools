//! Desktop GUI for oxide-dev-tools (README Phase 7).
//!
//! Bootstrap order is a contract: `i18n::init` registers the application's
//! locales, then `gpui_kit::init` initializes every enabled library layer,
//! and only then does `shell::bootstrap::run` install the registry global
//! and open the shell window (`Root` at the top, `TitleBar` chrome, the
//! oxide-design theme applied from the window appearance).

pub mod generators;
pub mod i18n;
pub mod shared;
pub mod shell;
pub mod theme;

// The application's locale file (locales/ui.yml). English is the default
// language and the fallback; see the `gpui-kit-i18n` agent skill.
rust_i18n::i18n!("locales", fallback = "en");

fn main() {
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(|cx| {
        i18n::init(cx);
        gpui_kit::init(cx);
        shell::bootstrap::run(cx);
    });
}

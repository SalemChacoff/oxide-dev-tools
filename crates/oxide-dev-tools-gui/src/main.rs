//! Desktop GUI for oxide-dev-tools (README Phase 7).
//!
//! Bootstrap order is a contract: `i18n::init` registers the application's
//! locales, then `gpui_kit::init` initializes every enabled library layer,
//! and only then does `shell::bootstrap::run` install the registry global
//! and open the shell window (`Root` at the top, `TitleBar` chrome, the
//! oxide-design theme applied from the window appearance). The composed icon
//! bundle (`assets::AppAssets` — gpui-kit's default component icons plus the
//! tool catalog extras) registers with the platform application before
//! anything runs: icons not listed there exist in the catalog but render
//! blank.

pub mod assets;
pub mod generators;
pub mod i18n;
pub mod shared;
pub mod shell;
pub mod theme;

// The application's locale file (locales/ui.yml). English is the default
// language and the fallback; see the `gpui-kit-i18n` agent skill.
rust_i18n::i18n!("locales", fallback = "en");

fn main() {
    gpui_kit::application().with_assets(assets::AppAssets).run(|cx| {
        i18n::init(cx);
        gpui_kit::init(cx);
        shell::bootstrap::run(cx);
    });
}

//! Desktop GUI for oxide-dev-tools (README Phase 7).
//!
//! The window bootstrap lands in Phase 7.4; this stub keeps the crate
//! compiling while the Phase 7.2 design system (the `theme` module) and the
//! i18n wiring land. At bootstrap, call `i18n::init(cx)` inside
//! `gpui_kit::application().run(...)` before `gpui_kit::init(cx)`.

pub mod i18n;
pub mod theme;

// The application's locale file (locales/ui.yml). English is the default
// language and the fallback; see the `gpui-kit-i18n` agent skill.
rust_i18n::i18n!("locales", fallback = "en");

fn main() {
    println!("oxide-gui: window bootstrap lands in Phase 7.4");
}

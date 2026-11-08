//! `actions` — the shell's commands: one action per logical command, bound
//! and dispatched from every entry point (key binding, search chip, title-bar
//! menu).
//!
//! Handlers register as app-global action listeners in `ShellView::new`
//! (never focus-stack listeners), so commands reach the shell from any
//! overlay regardless of where focus sits.

use gpui_kit::{KeyBinding, Keystroke, Modifiers, actions};

actions!(shell, [TogglePalette, SetSystemTheme, SetLightTheme, SetDarkTheme]);

/// The platform palette binding: ⌘K on macOS, Ctrl+K elsewhere.
pub fn palette_key_binding() -> KeyBinding {
    #[cfg(target_os = "macos")]
    let keystroke = "cmd-k";
    #[cfg(not(target_os = "macos"))]
    let keystroke = "ctrl-k";

    KeyBinding::new(keystroke, TogglePalette, None)
}

/// The keystroke the search header's chip displays (`Kbd` formats it per
/// platform). Mirrors [`palette_key_binding`] exactly.
pub fn palette_keystroke() -> Keystroke {
    #[cfg(target_os = "macos")]
    {
        Keystroke {
            modifiers: Modifiers::command(),
            key: "k".into(),
            key_char: Some("k".into()),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Keystroke {
            modifiers: Modifiers::control(),
            key: "k".into(),
            key_char: Some("k".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_chip_describes_the_platform_binding() {
        let chip = palette_keystroke();

        #[cfg(target_os = "macos")]
        assert_eq!(chip.modifiers, Modifiers::command());
        #[cfg(not(target_os = "macos"))]
        assert_eq!(chip.modifiers, Modifiers::control());

        assert_eq!(chip.key, "k");

        // Constructing the binding proves the keystroke literal parses on
        // this platform (KeyBinding::new panics on an invalid literal).
        let _binding = palette_key_binding();
    }
}

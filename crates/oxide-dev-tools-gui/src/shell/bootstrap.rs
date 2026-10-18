//! `bootstrap` — window bootstrap: the single place that installs the app
//! globals and opens the shell window.
//!
//! Wiring order matters: `i18n::init` and `gpui_kit::init` ran in `main`
//! before this; here the registry global lands, then the window opens with
//! the enforced minimum size, the oxide-design theme applies, and the shell
//! goes under one gpui-kit `Root` (overlays, notifications, modal focus).

use gpui_kit::component::{Root, TitleBar};
use gpui_kit::{App, AppContext as _, Pixels, Size, WindowOptions, px, size};

use crate::shared::registry::{RegistryError, ToolRegistry, ToolRegistryGlobal};
use crate::shell::shell_view::ShellView;
use crate::shell::tool_seed;
use crate::theme::{self, ThemePreference};

/// The enforced minimum window size: every tool page displays correctly at
/// it (design spec §4; 900×640 from the mock).
pub const MIN_WINDOW_WIDTH: f32 = 900.;
pub const MIN_WINDOW_HEIGHT: f32 = 640.;

/// The minimum window size as a [`WindowOptions`] value.
pub fn min_window_size() -> Size<Pixels> {
    size(px(MIN_WINDOW_WIDTH), px(MIN_WINDOW_HEIGHT))
}

/// Installs the seeded tool registry as a global (pure app-level wiring).
pub fn install_registry(cx: &mut App) -> Result<(), RegistryError> {
    let mut registry = ToolRegistry::new();
    tool_seed::register_all(&mut registry)?;
    cx.set_global(ToolRegistryGlobal::new(registry));
    Ok(())
}

/// Opens the shell window on the running application.
pub fn run(cx: &mut App) {
    if let Err(error) = install_registry(cx) {
        eprintln!("oxide-gui: tool registry failed to initialize: {error}");
        return;
    }

    let options = WindowOptions {
        window_min_size: Some(min_window_size()),
        ..TitleBar::window_options()
    };

    if let Err(error) = cx.open_window(options, |window, cx| {
        let shell = cx.new(|cx| ShellView::new(window, cx));
        theme::apply(cx, ThemePreference::System, window.appearance());
        cx.new(|cx| Root::new(shell, window, cx))
    }) {
        eprintln!("oxide-gui: failed to open window: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;

    use crate::shared::registry::registry;

    #[test]
    fn min_window_size_matches_the_design_spec() {
        let min = min_window_size();
        assert_eq!(min.width, px(900.));
        assert_eq!(min.height, px(640.));
    }

    #[gpui_kit::test]
    fn install_registry_seeds_the_global(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            assert!(install_registry(cx).is_ok());
            assert_eq!(registry(cx).tools().len(), 30);
        });
    }
}

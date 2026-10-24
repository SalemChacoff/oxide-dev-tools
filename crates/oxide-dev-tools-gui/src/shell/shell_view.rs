//! `shell_view` — the window shell (README Phase 7.4): title bar, resizable
//! sidebar, detail area with the global search header, and the command
//! palette.
//!
//! One gpui-kit `Root` wraps this view at bootstrap (it coordinates
//! overlays, notifications, and modal focus restoration). State ownership:
//! the destination, the shared settings state, the palette and search input
//! states, the resizable-sidebar state, and the recent-runs history all live
//! here — nothing else owns them.
//!
//! Shell commands (`TogglePalette`, the theme actions) register as
//! app-global action listeners, so they reach the shell from any overlay
//! regardless of where focus sits; every entry point dispatches the same
//! command instead of copying mutations.

use gpui_kit::base::FocusTrapElement as _;
use gpui_kit::base::{ResizableState, TestSupportExt as _, h_resizable, resizable_panel};
use gpui_kit::component::command::{Command, CommandState};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::generators::{FakeView, IdView, KeyView, LoremView, RunReporter, SampleView};
use crate::shared::recent_runs::{RecentRuns, RecentRunsState};
use crate::shared::registry::registry;
use crate::shared::settings_panel::{SettingsPanel, SettingsState};
use crate::shell::actions::{SetDarkTheme, SetLightTheme, SetSystemTheme, TogglePalette, palette_key_binding};
use crate::shell::navigation::Destination;
use crate::shell::palette;
use crate::shell::search_header::SearchHeader;
use crate::theme::{self, ThemePreference};

/// The window's app content.
pub struct ShellView {
    pub(crate) destination: Destination,
    pub(crate) settings: Entity<SettingsState>,
    pub(crate) settings_panel: Entity<SettingsPanel>,
    pub(crate) command: Entity<CommandState>,
    pub(crate) search: Entity<InputState>,
    pub(crate) recent: Entity<RecentRunsState>,
    pub(crate) recent_runs: Entity<RecentRuns>,
    pub(crate) resizable: Entity<ResizableState>,
    pub(crate) palette_open: bool,
    id_page: Entity<IdView>,
    key_page: Entity<KeyView>,
    lorem_page: Entity<LoremView>,
    fake_page: Entity<FakeView>,
    sample_page: Entity<SampleView>,
    _subscriptions: Vec<Subscription>,
}

impl ShellView {
    /// Creates the shell and its retained state.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = cx.new(|_| SettingsState::new());
        let settings_panel = cx.new(|cx| SettingsPanel::with_state(settings.clone(), window, cx));
        let command = cx.new(|cx| CommandState::new(window, cx));
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(rust_i18n::t!("app.common.search").into_owned()));
        let recent = cx.new(|_| RecentRunsState::new());
        let resizable = cx.new(|_| ResizableState::default());

        // The history view reuses the shell's history and reopens runs
        // through the shell; the weak handle exists before the shell does.
        let weak_shell = cx.weak_entity();

        // Tool pages report finished runs through the reporter so the shell
        // history records them (Phase 7.5).
        let run_reporter: RunReporter = {
            let weak_shell = weak_shell.clone();
            std::rc::Rc::new(move |tool_id, summary, cx| {
                if let Some(shell) = weak_shell.upgrade() {
                    shell.update(cx, |shell, cx| shell.record_run(tool_id, summary, cx));
                }
            })
        };
        let id_page = cx.new(|cx| IdView::new(window, cx).on_run(run_reporter.clone()));
        let key_page = cx.new(|cx| KeyView::new(window, cx).on_run(run_reporter.clone()));
        let lorem_page = cx.new(|cx| LoremView::new(window, cx).on_run(run_reporter.clone()));
        let fake_page = cx.new(|cx| FakeView::new(window, cx).on_run(run_reporter.clone()));
        let sample_page = cx.new(|cx| SampleView::new(window, cx).on_run(run_reporter));

        let recent_runs = cx.new(|cx| {
            let weak_shell = weak_shell.clone();
            RecentRuns::with_history(recent.clone(), window, cx).on_open_tool(move |id, window, cx| {
                if let Some(shell) = weak_shell.upgrade() {
                    shell.update(cx, |shell, cx| shell.open_tool(id.clone(), window, cx));
                }
            })
        });

        let subscriptions = vec![
            cx.subscribe_in(&search, window, Self::on_search_input),
            cx.observe(&settings, |_this, _, cx| cx.notify()),
            cx.observe_window_appearance(window, Self::on_window_appearance),
        ];

        cx.bind_keys([palette_key_binding()]);

        let shell = cx.weak_entity();
        let window_handle = window.window_handle();
        register_global_action::<TogglePalette, _>(shell.clone(), window_handle, cx, |shell, window, cx| {
            shell.toggle_palette(window, cx);
        });
        let window_handle = window.window_handle();
        register_global_action::<SetSystemTheme, _>(shell.clone(), window_handle, cx, |shell, window, cx| {
            shell.set_theme_preference(ThemePreference::System, window, cx);
        });
        let window_handle = window.window_handle();
        register_global_action::<SetLightTheme, _>(shell.clone(), window_handle, cx, |shell, window, cx| {
            shell.set_theme_preference(ThemePreference::Light, window, cx);
        });
        let window_handle = window.window_handle();
        register_global_action::<SetDarkTheme, _>(shell, window_handle, cx, |shell, window, cx| {
            shell.set_theme_preference(ThemePreference::Dark, window, cx);
        });

        // First-frame focus lands in the global search: the palette trigger
        // and the filter are reachable without a mouse.
        cx.on_next_frame(window, |this, window, cx| {
            this.focus_search(window, cx);
        });

        Self {
            destination: Destination::Home,
            settings,
            settings_panel,
            command,
            search,
            recent,
            recent_runs,
            resizable,
            palette_open: false,
            id_page,
            key_page,
            lorem_page,
            fake_page,
            sample_page,
            _subscriptions: subscriptions,
        }
    }

    /// The shared settings model (theme preference, tool defaults later).
    pub fn settings(&self) -> Entity<SettingsState> {
        self.settings.clone()
    }

    /// The search input state the header and the filter share.
    pub fn search(&self) -> Entity<InputState> {
        self.search.clone()
    }

    /// Navigates to a tool page by its registry id.
    pub fn open_tool(&mut self, id: impl Into<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        self.destination = Destination::Tool(id.into());
        if self.palette_open {
            self.close_palette(window, cx);
        }
        cx.notify();
    }

    /// Records a finished tool run in the shared recent-runs history.
    fn record_run(&mut self, tool_id: &str, summary: &str, cx: &mut Context<Self>) {
        self.recent.update(cx, |state, cx| state.record(tool_id, summary, cx));
        cx.notify();
    }

    /// Navigates back to the home view.
    pub fn go_home(&mut self, cx: &mut Context<Self>) {
        self.destination = Destination::Home;
        cx.notify();
    }

    /// Navigates to the settings page.
    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.destination = Destination::Settings;
        cx.notify();
    }

    /// Applies a theme preference through the only allowed mutation path.
    pub fn set_theme_preference(&mut self, preference: ThemePreference, window: &mut Window, cx: &mut Context<Self>) {
        self.settings
            .update(cx, |state, cx| state.set_preference(preference, cx));
        theme::apply(cx, preference, window.appearance());
    }

    /// Opens or closes the command palette.
    pub fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette_open {
            self.close_palette(window, cx);
        } else {
            self.open_palette(window, cx);
        }
    }

    /// Closes the palette and returns focus to the search trigger.
    pub fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = false;
        cx.notify();

        let search = self.search.clone();
        window.defer(cx, move |window, cx| {
            search.read(cx).focus_handle(cx).focus(window, cx);
        });
    }

    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = true;
        cx.notify();

        let command = self.command.clone();
        window.defer(cx, move |window, cx| {
            command.update(cx, |state, cx| state.focus(window, cx));
        });
    }

    fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.read(cx).focus_handle(cx).focus(window, cx);
    }

    /// The search input's semantic events: live filtering and Enter.
    fn on_search_input(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => cx.notify(),
            InputEvent::PressEnter { .. } => {
                let query = self.search.read(cx).value();
                let first = registry(cx).search(&query).into_iter().next().cloned();
                if let Some(tool) = first {
                    self.open_tool(tool.id().clone(), window, cx);
                }
            }
            InputEvent::Focus | InputEvent::Blur => {}
        }
    }

    /// System appearance changes re-apply the theme when System is preferred.
    fn on_window_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let preference = self.settings.read(cx).preference();
        if preference == ThemePreference::System {
            theme::apply(cx, preference, window.appearance());
        }
    }

    /// The detail area: the global search above the current page.
    fn render_detail(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = cx.entity();

        v_flex()
            .id("detail")
            .test_support()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(h_flex().px_6().pt_5().pb_2().justify_center().child(SearchHeader::new(
                &self.search,
                move |_, window, cx| {
                    shell.update(cx, |shell, cx| shell.toggle_palette(window, cx));
                },
            )))
            .child(self.render_page(cx))
    }

    /// The page for the current destination.
    fn render_page(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.destination {
            Destination::Home => self.render_home(cx).into_any_element(),
            Destination::Settings => self.render_settings_page().into_any_element(),
            Destination::Tool(id) => self.render_tool_page(id.clone(), cx).into_any_element(),
        }
    }

    /// Tool pages from Phase 7.5 on replace the placeholder for shipped
    /// tools — an honest "not available yet" state stays for the rest.
    fn render_tool_page(&self, id: SharedString, cx: &mut Context<Self>) -> AnyElement {
        match id.as_ref() {
            "id" => self.id_page.clone().into_any_element(),
            "key" => self.key_page.clone().into_any_element(),
            "lorem" => self.lorem_page.clone().into_any_element(),
            "fake" => self.fake_page.clone().into_any_element(),
            "sample" => self.sample_page.clone().into_any_element(),
            _ => {
                let title = registry(cx)
                    .find(&id)
                    .map_or_else(|| id.to_string(), |meta| meta.title());

                v_flex()
                    .id("tool-page")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .px_6()
                    .pb_6()
                    .child(
                        h_flex()
                            .justify_center()
                            .child(div().text_lg().font_semibold().child(title)),
                    )
                    .child(
                        div()
                            .id("tool-placeholder")
                            .test_support()
                            .mx_auto()
                            .w_128()
                            .pt_10()
                            .child(
                                Empty::new().header(
                                    EmptyHeader::new()
                                        .title(
                                            EmptyTitle::new()
                                                .child(rust_i18n::t!("app.shell.placeholder_title").into_owned()),
                                        )
                                        .description(
                                            EmptyDescription::new()
                                                .child(rust_i18n::t!("app.shell.placeholder_description").into_owned()),
                                        ),
                                ),
                            ),
                    )
                    .into_any_element()
            }
        }
    }

    /// The settings page: the Phase 7.3 panel over the shared state.
    fn render_settings_page(&self) -> impl IntoElement {
        v_flex()
            .id("settings-page")
            .test_support()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .px_6()
            .pb_6()
            .child(
                h_flex().justify_center().child(
                    div()
                        .text_lg()
                        .font_semibold()
                        .child(rust_i18n::t!("app.settings.title")),
                ),
            )
            .child(div().mx_auto().w_128().pt_4().child(self.settings_panel.clone()))
    }

    /// The command palette overlay: top-centered, focus-trapped, Escape
    /// dismisses and returns focus to the search trigger (the library's
    /// palette owns the Escape binding).
    fn render_palette(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.command.read(cx).focus_handle(cx);
        let groups = palette::build_groups(registry(cx));

        let command = groups
            .into_iter()
            .fold(Command::new(&self.command), |command, group| command.group(group))
            .placeholder(rust_i18n::t!("app.shell.palette_placeholder").into_owned())
            .on_confirm({
                let shell = cx.weak_entity();
                move |path, window, cx| {
                    let Some(shell) = shell.upgrade() else {
                        return;
                    };
                    shell.update(cx, |shell, cx| {
                        let tool = palette::resolve_tool(registry(cx), &path).cloned();
                        if let Some(tool) = tool {
                            shell.open_tool(tool.id().clone(), window, cx);
                        }
                    });
                }
            })
            .on_cancel({
                let shell = cx.weak_entity();
                move |window, cx| {
                    let Some(shell) = shell.upgrade() else {
                        return;
                    };
                    shell.update(cx, |shell, cx| shell.close_palette(window, cx));
                }
            });

        // The palette is the shell's last child, so tree order paints it
        // above the title bar and the body without an explicit z-index.
        div().absolute().top_6().inset_0().child(
            h_flex()
                .justify_center()
                .child(div().w_112().child(command).focus_trap("shell-palette", &focus_handle)),
        )
    }
}

impl Render for ShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .relative()
            .child(self.render_title_bar(cx))
            .child(
                h_flex().flex_1().min_h_0().items_stretch().child(
                    h_resizable("shell-resizable").with_state(&self.resizable).children([
                        resizable_panel()
                            // Physical pane minima: the resizable API's
                            // contract, not rem-scale content spacing.
                            .size(px(240.))
                            .size_range(px(180.)..px(400.))
                            .child(self.render_sidebar(window, cx)),
                        resizable_panel().child(self.render_detail(window, cx)),
                    ]),
                ),
            )
            .when(self.palette_open, |this| this.child(self.render_palette(window, cx)))
    }
}

/// Registers one app-global action listener that forwards to the shell
/// window: overlays (menus, palette) dispatch actions outside the shell's
/// element ancestry, so focus-stack listeners would miss them. The window
/// handle is captured at construction — dispatch never depends on platform
/// focus, which keeps the path deterministic in headless tests too.
fn register_global_action<A, F>(
    shell: gpui_kit::WeakEntity<ShellView>,
    window_handle: gpui_kit::AnyWindowHandle,
    cx: &mut App,
    handle: F,
) where
    A: gpui_kit::Action,
    F: Fn(&mut ShellView, &mut Window, &mut Context<ShellView>) + Clone + 'static,
{
    App::on_action(cx, move |_: &A, cx| {
        // The dispatch may run inside this window's own update — its slot is
        // taken out of the app's window map for the duration — so defer once
        // more: by then the window is back and the nested update succeeds.
        let shell = shell.clone();
        let window_handle = window_handle;
        let handle = handle.clone();
        cx.defer(move |cx| {
            let _ = window_handle.update(cx, |_, window, cx| {
                if let Some(shell) = shell.upgrade() {
                    shell.update(cx, |shell, cx| handle(shell, window, cx));
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Keystroke, TestAppContext, VisualTestContext};

    use crate::shell::bootstrap;

    fn install(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| bootstrap::install_registry(cx).expect("seed registers"));
        cx.update(|cx| theme::apply(cx, ThemePreference::System, gpui_kit::WindowAppearance::Light));
    }

    #[gpui_kit::test]
    fn shell_starts_home_and_opens_a_tool_from_the_sidebar(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("home-page").is_some());
            assert!(window.try_find("tool-card-id").is_some());
            assert!(window.try_find("tool-page").is_none());

            // Generators' first tool row (submenu item "0-0") is the id tool.
            window.click("0-0", cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("tool-page").is_some());
            // The id tool shipped in Phase 7.5 — the placeholder is gone.
            assert!(window.try_find("id-mode").is_some());
            assert!(window.try_find("tool-placeholder").is_none());
            assert!(window.try_find("home-page").is_none());
        });
    }

    #[gpui_kit::test]
    fn brand_returns_home_and_the_footer_opens_settings(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click("sidebar-settings", cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("settings-page").is_some());
            assert!(window.try_find("settings-panel").is_some());

            window.click("brand", cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("home-page").is_some());
            assert!(window.try_find("settings-page").is_none());
        });
    }

    #[gpui_kit::test]
    fn search_filters_the_home_cards_and_clear_restores_the_catalog(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("tool-card-base64").is_some());

            let search = view.read(cx).search.clone();
            search.update(cx, |input, cx| input.set_value("base", window, cx));
            view.update(cx, |_, cx| cx.notify());
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("tool-card-base64").is_some());
            assert!(window.try_find("tool-card-id").is_none());
            assert!(window.try_find("tool-card-timestamp").is_none());
        });

        // A query matching nothing shows the no-match state with Clear search.
        cx.update(|window, cx| {
            let search = view.read(cx).search.clone();
            search.update(cx, |input, cx| input.set_value("zzzz", window, cx));
            view.update(cx, |_, cx| cx.notify());
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("home-empty").is_some());
            assert!(window.try_find("clear-search").is_some());

            window.click("clear-search", cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("home-empty").is_none());
            assert!(window.try_find("tool-card-id").is_some());
            assert_eq!(view.read(cx).search.read(cx).value(), SharedString::default());
        });
    }

    #[gpui_kit::test]
    fn search_enter_opens_the_first_match(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let search = view.read(cx).search.clone();
            search.update(cx, |input, cx| input.set_value("base", window, cx));
            view.update(cx, |_, cx| cx.notify());
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            view.update(cx, |shell, cx| shell.focus_search(window, cx));
            window.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("tool-page").is_some());
        });
    }

    #[gpui_kit::test]
    fn palette_opens_from_the_chip_and_enter_confirms_the_first_match(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click("palette-trigger", cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("command").is_some());

            // Empty query: the first match is the first generator (id).
            window.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("command").is_none());
            assert!(window.try_find("tool-page").is_some());
        });
    }

    #[gpui_kit::test]
    fn palette_escape_closes_it_and_returns_focus_to_the_search(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click("palette-trigger", cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("command").is_some());

            window.dispatch_keystroke(Keystroke::parse("escape").unwrap(), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("command").is_none());

            let search = view.read(cx).search.clone();
            assert!(search.read(cx).focus_handle(cx).contains_focused(window, cx));
        });
    }

    #[gpui_kit::test]
    fn theme_actions_reach_the_shell_from_a_global_dispatch(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.dispatch_action(Box::new(SetDarkTheme), cx);
        });
        // The dispatch defers into the effect cycle; park so the deferred
        // task runs before the assertions.
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(view.read(cx).settings.read(cx).preference(), ThemePreference::Dark);
            assert_eq!(theme::OxideTheme::tokens(cx).background, theme::OxideTokens::dark().background);
        });
    }

    #[gpui_kit::test]
    fn theme_preference_flows_through_the_shell_method(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            view.update(cx, |shell, cx| {
                shell.set_theme_preference(ThemePreference::Light, window, cx);
            });
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(view.read(cx).settings.read(cx).preference(), ThemePreference::Light);
            assert_eq!(theme::OxideTheme::tokens(cx).background, theme::OxideTokens::light().background);
        });
    }

    #[gpui_kit::test]
    fn home_cards_keep_the_mock_grid_width(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(ShellView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            // 14rem at the default 16px base font: the mock's ~236px cards
            // on the nearest rem-scale step.
            let bounds = window.try_find("tool-card-id").expect("id card is rendered").bounds();
            assert_eq!(bounds.size.width, px(224.));
        });
    }
}

//! `recent_runs` — the bounded history of tool runs shown on the home view.
//!
//! Domain policy lives in `RecentRunsState` (cap, newest-first, clear); the
//! view renders it as a titled list of quiet rows. Each run carries a ULID
//! from core's id generator, so rows keep stable, domain-derived element
//! identity. `Clear history` is visible only while entries exist and asks an
//! AlertDialog that names the object and the verb (`Clear recent runs?` +
//! `Clear`) — clearing history is irreversible. Persistence lands with the
//! shell (Phase 7.4+); the model keeps only serialization-ready fields.

use std::rc::Rc;

use chrono::{DateTime, Duration, Utc};
use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Window, div, prelude::FluentBuilder as _,
};
use oxide_dev_tools_core::{IdKind, generate_id};

use crate::shared::OpenToolHandler;
use crate::shared::registry::registry;
use crate::theme::OxideTheme;

/// History size; enough for a useful home view, small enough to stay
/// unbounded-free without virtualization.
pub const RECENT_RUNS_CAP: usize = 20;

/// One recorded run: stable ULID identity, the tool, a one-line summary, and
/// the wall-clock time.
#[derive(Clone, Debug)]
pub struct RecentRun {
    id: SharedString,
    tool_id: SharedString,
    summary: SharedString,
    at: DateTime<Utc>,
}

impl RecentRun {
    /// Records a finished run. The id comes from core's ULID generator —
    /// unique, sortable, and stable across renders.
    pub fn new(tool_id: impl Into<SharedString>, summary: impl Into<SharedString>) -> Self {
        let id = generate_id(IdKind::Ulid).expect("ULID generation cannot fail");
        Self {
            id: id.into(),
            tool_id: tool_id.into(),
            summary: summary.into(),
            at: Utc::now(),
        }
    }

    /// Stable run identity.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// The tool that produced the run.
    pub fn tool_id(&self) -> &SharedString {
        &self.tool_id
    }

    /// One-line result summary.
    pub fn summary(&self) -> &SharedString {
        &self.summary
    }

    /// When the run finished.
    pub fn at(&self) -> DateTime<Utc> {
        self.at
    }
}

/// The retained history model. Owned by the shell once it exists; Phase 7.3
/// proves the contract with focused tests.
#[derive(Default)]
pub struct RecentRunsState {
    runs: Vec<RecentRun>,
}

impl RecentRunsState {
    /// Creates an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Prepends a finished run and trims to [`RECENT_RUNS_CAP`], newest first.
    pub fn record(
        &mut self,
        tool_id: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.runs.insert(0, RecentRun::new(tool_id, summary));
        self.runs.truncate(RECENT_RUNS_CAP);
        cx.notify();
    }

    /// Clears the history.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.runs.clear();
        cx.notify();
    }

    /// Recorded runs, newest first.
    pub fn runs(&self) -> &[RecentRun] {
        &self.runs
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }
}

/// The home-view history section. `on_open_tool` reopens the tool a row
/// describes; without it, rows render inert.
pub struct RecentRuns {
    state: Entity<RecentRunsState>,
    on_open_tool: Option<OpenToolHandler>,
}

impl RecentRuns {
    /// Creates the view with its own history.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            state: cx.new(|_| RecentRunsState::new()),
            on_open_tool: None,
        }
    }

    /// Reuses a history owned elsewhere (the shell, Phase 7.4).
    pub fn with_history(state: Entity<RecentRunsState>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            state,
            on_open_tool: None,
        }
    }

    /// The retained history model.
    pub fn state(&self) -> Entity<RecentRunsState> {
        self.state.clone()
    }

    /// Registers the row-activation command: reopen the run's tool.
    pub fn on_open_tool(mut self, handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_open_tool = Some(Rc::new(handler));
        self
    }

    /// The activation callback, cloned for one row.
    fn open_handler(&self) -> Option<OpenToolHandler> {
        self.on_open_tool.clone()
    }
}

impl Render for RecentRuns {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (has_runs, runs) = {
            let state = self.state.read(cx);
            (!state.is_empty(), state.runs().to_vec())
        };
        let rows: Vec<gpui_kit::AnyElement> = runs
            .iter()
            .map(|run| render_run_row(run, self, cx).into_any_element())
            .collect();

        v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .child(div().text_sm().font_medium().child(rust_i18n::t!("app.recent.title")))
                    .when(has_runs, |this| {
                        let state = self.state.clone();
                        this.child(
                            Button::new((ElementId::from("recent-runs"), "clear"))
                                .ghost()
                                .label(rust_i18n::t!("app.recent.clear_history"))
                                .small()
                                .on_click(move |_, window, cx| {
                                    let state = state.clone();
                                    window.open_alert_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .confirm()
                                            .title(rust_i18n::t!("app.recent.clear_title"))
                                            .description(rust_i18n::t!("app.recent.clear_body"))
                                            .button_props(
                                                DialogButtonProps::default()
                                                    .ok_text(rust_i18n::t!("app.recent.clear_action"))
                                                    .ok_variant(gpui_kit::component::button::ButtonVariant::Danger)
                                                    .cancel_text(rust_i18n::t!("app.common.cancel"))
                                                    .show_cancel(true)
                                                    .on_ok({
                                                        let state = state.clone();
                                                        move |_, _, cx| {
                                                            state.update(cx, |state, cx| {
                                                                state.clear(cx);
                                                            });
                                                            true
                                                        }
                                                    }),
                                            )
                                    });
                                }),
                        )
                    }),
            )
            .when(!has_runs, |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted)
                        .child(rust_i18n::t!("app.recent.empty"))
                        .id((ElementId::from("recent-runs"), "empty"))
                        .test_support(),
                )
            })
            .when(has_runs, move |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .gap_1()
                        .children(rows)
                        .id((ElementId::from("recent-runs"), "list"))
                        .test_support(),
                )
            })
            .id("recent-runs")
            .test_support()
    }
}

/// One history row: tool icon, tool title, one-line summary, relative time.
/// Built on a quiet `Button` so keyboard activation stays free; the registry
/// supplies the icon and title — rows never re-declare tool copy.
fn render_run_row(run: &RecentRun, view: &RecentRuns, cx: &mut Context<RecentRuns>) -> impl IntoElement {
    let tokens = OxideTheme::tokens(cx);
    let meta = registry(cx).find(run.tool_id().as_ref());
    let title = meta
        .map(|meta| meta.title())
        .unwrap_or_else(|| run.tool_id().to_string());
    let icon = meta.map(|meta| meta.icon()).unwrap_or(IconName::Clock);
    let time = relative_time(run.at(), Utc::now());
    let open_handler = view.open_handler();
    let tool_id = run.tool_id().clone();
    let row_id: ElementId = (ElementId::from("recent-run"), run.id().clone()).into();

    // The `Button` (Text variant) keeps focus, keyboard, and a11y; the row
    // surface lives on the inner flex because Button already owns its variant
    // hover style — a second `hover` refinement would panic.
    Button::new(row_id)
        .text()
        .cursor_default()
        .w_full()
        .rounded(cx.theme().radius)
        .when_some(open_handler, |this, handler| this.on_click(move |_, window, cx| handler(&tool_id, window, cx)))
        .child(
            h_flex()
                .w_full()
                .p_2()
                .gap_2()
                .items_center()
                .rounded(cx.theme().radius)
                .hover(|style| style.bg(tokens.surface_2))
                .child(Icon::new(icon).size_4().text_color(cx.theme().muted))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .items_start()
                        .child(div().text_sm().text_color(cx.theme().foreground).child(title))
                        .child(
                            div()
                                .w_full()
                                .text_xs()
                                .text_color(cx.theme().muted)
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(run.summary().clone()),
                        ),
                )
                .child(div().text_xs().text_color(cx.theme().muted).child(time)),
        )
}

/// Formats a wall-clock age in whole messages: `just now`, `3m ago`, `2h ago`,
/// `5d ago`, `2w ago`.
pub fn relative_time(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let elapsed = now.signed_duration_since(at);
    if elapsed < Duration::minutes(1) {
        rust_i18n::t!("app.recent.just_now").into_owned()
    } else if elapsed < Duration::hours(1) {
        rust_i18n::t!("app.recent.minutes_ago", n = elapsed.num_minutes()).into_owned()
    } else if elapsed < Duration::days(1) {
        rust_i18n::t!("app.recent.hours_ago", n = elapsed.num_hours()).into_owned()
    } else if elapsed < Duration::weeks(1) {
        rust_i18n::t!("app.recent.days_ago", n = elapsed.num_days()).into_owned()
    } else {
        rust_i18n::t!("app.recent.weeks_ago", n = elapsed.num_weeks()).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::registry::{ToolRegistry, ToolRegistryGlobal};
    use gpui_kit::component::Root;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{TestAppContext, VisualTestContext};
    use std::cell::RefCell;

    #[gpui_kit::test]
    fn state_caps_at_twenty_newest_first(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            let state = cx.new(|_| RecentRunsState::new());
            for ix in 0..(RECENT_RUNS_CAP + 5) {
                state.update(cx, |state, cx| {
                    state.record(format!("tool-{ix}"), format!("run {ix}"), cx);
                });
            }
            state.update(cx, |state, cx| {
                assert_eq!(state.runs().len(), RECENT_RUNS_CAP);
                assert_eq!(state.runs()[0].tool_id(), "tool-24");
                assert_eq!(state.runs()[19].tool_id(), "tool-5");
                // Runs carry stable, unique ULID identities.
                let ids: std::collections::HashSet<&SharedString> = state.runs().iter().map(|run| run.id()).collect();
                assert_eq!(ids.len(), RECENT_RUNS_CAP);

                state.clear(cx);
                assert!(state.is_empty());
            });
        });
    }

    #[test]
    fn relative_time_uses_whole_messages_per_bucket() {
        let now = Utc::now();
        assert_eq!(relative_time(now, now), "just now");
        assert_eq!(relative_time(now - Duration::minutes(3), now), "3m ago");
        assert_eq!(relative_time(now - Duration::hours(2), now), "2h ago");
        assert_eq!(relative_time(now - Duration::days(5), now), "5d ago");
        assert_eq!(relative_time(now - Duration::weeks(3), now), "3w ago");
    }

    struct HistoryTestView {
        recent: Entity<RecentRuns>,
    }

    impl Render for HistoryTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            v_flex().p_4().child(self.recent.clone())
        }
    }

    #[gpui_kit::test]
    fn renders_empty_state_then_rows_and_clears_with_a_named_dialog(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light);
            // The shell installs the registry at bootstrap (7.4); row
            // rendering reads tool icons and titles from it.
            cx.set_global(ToolRegistryGlobal::new(ToolRegistry::new()));
        });
        let state = cx.update(|cx| cx.new(|_| RecentRunsState::new()));
        let opened = Rc::new(RefCell::new(None));

        let (_view, cx) = cx.add_window_view(|window, cx| {
            let recent = cx.new(|cx| {
                let opened = opened.clone();
                RecentRuns::with_history(state.clone(), window, cx)
                    .on_open_tool(move |tool_id, _, _| *opened.borrow_mut() = Some(tool_id.to_string()))
            });
            let inner = cx.new(|_| HistoryTestView { recent });
            Root::new(inner, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        // Empty: muted empty state, no clear command, no rows.
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("recent-runs"), "empty")).is_some());
            assert!(window.try_find((ElementId::from("recent-runs"), "clear")).is_none());
        });

        // After a run: the row appears and activating it reopens the tool.
        cx.update(|_, cx| {
            state.update(cx, |state, cx| state.record("uuid", "10 IDs", cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("recent-runs"), "empty")).is_none());
            assert!(window.try_find((ElementId::from("recent-runs"), "clear")).is_some());

            let run_id = state.read(cx).runs()[0].id().clone();
            window.click((ElementId::from("recent-run"), run_id), cx);
            assert_eq!(opened.borrow().as_deref(), Some("uuid"));
        });

        // The clear command opens the named dialog; Escape dismisses without
        // clearing (focus return and confirmation are the dialog's contract).
        cx.update(|window, cx| {
            window.click((ElementId::from("recent-runs"), "clear"), cx);
            assert!(window.has_active_dialog(cx));
            window.press("escape", cx);
            assert!(!state.read(cx).is_empty());
        });
    }
}

//! `base64_view` — the `base64` tool page (README Phase 7.6): encode and
//! decode, standard and URL-safe. Pattern A: the form (mode, alphabet, input)
//! on the left, the result pane on the right.
//!
//! The page calls the core `convert_base64` entry point directly. The run
//! button follows the selected mode (`Encode`/`Decode`), and a result is
//! cleared when the mode or alphabet changes because it belongs to the
//! operation that produced it. Decode errors surface inline on the input
//! field with copy that mirrors the core `Base64Error` messages.

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window, div,
};
use oxide_dev_tools_core::{Base64Alphabet, Base64Error, Base64Kind, Base64Options, convert_base64};

use crate::shared::registry::registry;
use crate::shared::run_seam::{RunReporter, report_run, summarize};
use crate::shared::{OutputPanel, ToolField, ToolPage};

/// The two operations behind the page's tabs, mirroring `Base64Kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base64Mode {
    Encode,
    Decode,
}

impl Base64Mode {
    /// The tab position for this mode.
    fn index(self) -> usize {
        match self {
            Self::Encode => 0,
            Self::Decode => 1,
        }
    }

    fn from_index(ix: usize) -> Self {
        match ix {
            1 => Self::Decode,
            _ => Self::Encode,
        }
    }

    /// The run-button label key: the action follows the selected mode.
    fn action_key(self) -> &'static str {
        match self {
            Self::Encode => "app.codecs.base64.encode",
            Self::Decode => "app.codecs.base64.decode",
        }
    }
}

/// The `base64` tool page.
pub struct Base64View {
    mode: Base64Mode,
    alphabet_items: Vec<SharedString>,
    alphabet_select: Entity<SelectState<SearchableVec<SharedString>>>,
    input: Entity<TextareaState>,
    result: SharedString,
    error: Option<SharedString>,
    on_run: Option<RunReporter>,
    _subscriptions: Vec<Subscription>,
}

impl Base64View {
    /// Creates the page with the core defaults (encode, standard alphabet).
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let alphabet_items: Vec<SharedString> = [
            "app.codecs.base64.alphabet_standard",
            "app.codecs.base64.alphabet_url_safe",
        ]
        .iter()
        .map(|key| rust_i18n::t!(*key).into_owned().into())
        .collect();
        let alphabet_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(alphabet_items.clone()),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });
        let input = cx.new(|cx| TextareaState::new(window, cx));

        let subscriptions = vec![
            cx.subscribe(&alphabet_select, Self::on_alphabet_confirm),
            cx.subscribe(&input, Self::on_input_change),
            // Resizes re-run the render so the panes switch between the
            // side-by-side and stacked layouts.
            cx.observe_window_bounds(window, |_, _, cx| cx.notify()),
        ];

        Self {
            mode: Base64Mode::Encode,
            alphabet_items,
            alphabet_select,
            input,
            result: SharedString::default(),
            error: None,
            on_run: None,
            _subscriptions: subscriptions,
        }
    }

    /// Registers the run reporter the shell's recent-runs history consumes.
    pub fn on_run(mut self, reporter: RunReporter) -> Self {
        self.on_run = Some(reporter);
        self
    }

    fn on_alphabet_confirm(
        &mut self,
        _: Entity<SelectState<SearchableVec<SharedString>>>,
        event: &SelectEvent<SearchableVec<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, SelectEvent::Confirm(_)) {
            // A result belongs to the alphabet that produced it.
            self.result = SharedString::default();
            self.error = None;
            cx.notify();
        }
    }

    /// Keeps the run button's disabled state in step with the input; an error
    /// describes the previous run's text, so editing it clears the error.
    fn on_input_change(&mut self, _: Entity<TextareaState>, _: &InputEvent, cx: &mut Context<Self>) {
        self.error = None;
        cx.notify();
    }

    /// The selected alphabet, mirroring `Base64Alphabet`.
    fn alphabet(&self, cx: &App) -> Base64Alphabet {
        match self.alphabet_select.read(cx).selected_value() {
            Some(value) if value == &self.alphabet_items[1] => Base64Alphabet::UrlSafe,
            _ => Base64Alphabet::Standard,
        }
    }

    /// Runs the codec with the current form state.
    fn run(&mut self, _event: &gpui_kit::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.read(cx).value().to_string();
        if input.trim().is_empty() {
            return;
        }
        let options = Base64Options {
            input,
            alphabet: self.alphabet(cx),
        };
        let kind = match self.mode {
            Base64Mode::Encode => Base64Kind::Encode(options),
            Base64Mode::Decode => Base64Kind::Decode(options),
        };

        match convert_base64(kind) {
            Ok(value) => {
                report_run(&self.on_run, "base64", &summarize(&value), cx);
                self.result = value.into();
                self.error = None;
            }
            Err(error) => {
                self.result = SharedString::default();
                self.error = Some(error_copy(&error));
            }
        }
        cx.notify();
    }

    /// The decision area: the primary action for the selected mode.
    fn render_actions(&self, empty_input: bool, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new((ElementId::from("base64"), "run"))
            .primary()
            .label(rust_i18n::t!(self.mode.action_key()))
            .disabled(empty_input)
            .w_full()
            .on_click(cx.listener(Self::run))
    }
}

/// Maps a core base64 error onto the page's validation copy.
fn error_copy(error: &Base64Error) -> SharedString {
    match error {
        Base64Error::InvalidInput => rust_i18n::t!("app.codecs.base64.validation_invalid")
            .into_owned()
            .into(),
        Base64Error::InvalidUtf8 => rust_i18n::t!("app.codecs.base64.validation_utf8").into_owned().into(),
    }
}

impl Render for Base64View {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = registry(cx)
            .find("base64")
            .map(|meta| meta.title())
            .unwrap_or_else(|| rust_i18n::t!("app.tools.codecs.base64.title").into_owned());
        let empty_input = self.input.read(cx).value().trim().is_empty();

        let alphabet_field = ToolField::new(
            "base64-alphabet",
            rust_i18n::t!("app.codecs.base64.alphabet_label"),
            Select::new(&self.alphabet_select)
                .id("base64-alphabet-select")
                .accessibility_label(rust_i18n::t!("app.codecs.base64.alphabet_label"))
                .w_full(),
        )
        .help(rust_i18n::t!("app.codecs.base64.alphabet_help"));

        let input_help = match self.mode {
            Base64Mode::Encode => rust_i18n::t!("app.codecs.base64.input_help_encode"),
            Base64Mode::Decode => rust_i18n::t!("app.codecs.base64.input_help_decode"),
        };
        // A decode error belongs to the input that carried the invalid data:
        // it replaces the help line with a discoverable Alert node.
        let input_field = ToolField::new(
            "base64-input",
            rust_i18n::t!("app.codecs.base64.input_label"),
            Textarea::new(&self.input).aria_label(rust_i18n::t!("app.codecs.base64.input_label")),
        )
        .help(input_help);
        let input_field = match self.error.clone() {
            Some(message) => input_field.validation(message),
            None => input_field,
        };

        let form = v_flex()
            .gap_4()
            .child(
                h_flex().w_full().justify_center().child(
                    div().id("base64-mode-bar").test_support().child(
                        TabBar::new("base64-mode")
                            .segmented()
                            .selected_index(self.mode.index())
                            .children([
                                Tab::new().label(rust_i18n::t!("app.codecs.base64.encode")),
                                Tab::new().label(rust_i18n::t!("app.codecs.base64.decode")),
                            ])
                            .on_click(cx.listener(|this, index, _, cx| {
                                let mode = Base64Mode::from_index(*index);
                                if mode != this.mode {
                                    this.mode = mode;
                                    // The result belongs to the mode that produced it.
                                    this.result = SharedString::default();
                                    this.error = None;
                                    cx.notify();
                                }
                            }))
                            .border_1()
                            .border_color(cx.theme().border),
                    ),
                ),
            )
            .child(alphabet_field)
            .child(input_field)
            .child(self.render_actions(empty_input, cx));

        ToolPage::new(title, form, OutputPanel::new("base64-output").content(self.result.clone()))
            .form_id("base64-form")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Role, TestAppContext, VisualTestContext};

    fn install(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| crate::shell::bootstrap::install_registry(cx).expect("seed registers"));
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });
    }

    #[gpui_kit::test]
    fn encode_and_decode_roundtrip_through_the_page(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(Base64View::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("base64-alphabet").is_some());
            assert!(window.try_find("base64-input").is_some());

            // The run button names the currently selected action.
            let run = window
                .try_find((ElementId::from("base64"), "run"))
                .expect("run button is rendered");
            assert_eq!(run.role(), Some(Role::Button));
            assert_eq!(run.label(), Some(rust_i18n::t!("app.codecs.base64.encode").as_ref()));

            // The textarea is named by the visible field label.
            let input = view.read(cx).input.clone();
            let textarea = window
                .try_find(("input", input.entity_id()))
                .expect("the input textarea is rendered");
            assert_eq!(textarea.role(), Some(Role::MultilineTextInput));
            assert_eq!(textarea.label(), Some(rust_i18n::t!("app.codecs.base64.input_label").as_ref()));

            input.update(cx, |state, cx| state.set_value("hello", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("base64"), "run"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            let content = window
                .try_find((ElementId::from("base64-output"), "content"))
                .expect("output content is rendered");
            assert_eq!(content.role(), Some(Role::Status));
            assert_eq!(content.label(), Some(view.read(cx).result.as_ref()));
            assert_eq!(view.read(cx).result.as_ref(), "aGVsbG8=");

            window.within("base64-mode").click(1usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            // Switching modes clears the result of the previous operation and
            // keeps the typed input.
            assert!(window.try_find((ElementId::from("base64-output"), "content")).is_none());
            let run = window
                .try_find((ElementId::from("base64"), "run"))
                .expect("run button is rendered");
            assert_eq!(run.label(), Some(rust_i18n::t!("app.codecs.base64.decode").as_ref()));
            assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "hello");

            let input = view.read(cx).input.clone();
            input.update(cx, |state, cx| state.set_value("aGVsbG8=", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("base64"), "run"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(view.read(cx).result.as_ref(), "hello");
        });
    }

    #[gpui_kit::test]
    fn decode_errors_surface_the_page_copy(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(Base64View::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("base64-mode").click(1usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            let input = view.read(cx).input.clone();
            input.update(cx, |state, cx| state.set_value("aGVsbG8!", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("base64"), "run"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(
                view.read(cx).error.as_deref(),
                Some(rust_i18n::t!("app.codecs.base64.validation_invalid").as_ref())
            );
            let alert = window
                .try_find((ElementId::from("base64-input"), "alert-icon"))
                .expect("the input validation renders");
            assert_eq!(alert.role(), Some(Role::Alert));
            assert_eq!(alert.label(), Some(rust_i18n::t!("app.codecs.base64.validation_invalid").as_ref()));
            assert!(window.try_find((ElementId::from("base64-output"), "content")).is_none());

            // 0xFF 0xFE is valid base64 but not valid UTF-8.
            let input = view.read(cx).input.clone();
            input.update(cx, |state, cx| state.set_value("//4=", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("base64"), "run"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(
                view.read(cx).error.as_deref(),
                Some(rust_i18n::t!("app.codecs.base64.validation_utf8").as_ref())
            );
            assert_eq!(view.read(cx).result.as_ref(), "");
        });
    }

    #[gpui_kit::test]
    fn run_waits_for_input_before_enabling(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(Base64View::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            // Empty input: activation is inert and no result appears.
            window.click((ElementId::from("base64"), "run"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(view.read(cx).result.as_ref(), "");
            assert!(view.read(cx).error.is_none());
            assert!(window.try_find((ElementId::from("base64-output"), "content")).is_none());

            let input = view.read(cx).input.clone();
            input.update(cx, |state, cx| state.set_value("hi", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("base64"), "run"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(view.read(cx).result.as_ref(), "aGk=");
        });
    }

    #[gpui_kit::test]
    fn url_safe_alphabet_encodes_without_padding(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(Base64View::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let alphabet = view.read(cx).alphabet_select.clone();
            alphabet.update(cx, |state, cx| {
                state.set_selected_index(Some(gpui_kit::component::IndexPath::new(1)), window, cx);
            });
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            let input = view.read(cx).input.clone();
            input.update(cx, |state, cx| state.set_value("hello", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("base64"), "run"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert_eq!(view.read(cx).result.as_ref(), "aGVsbG8");
        });
    }

    #[gpui_kit::test]
    fn long_result_wraps_inside_the_output_pane(cx: &mut TestAppContext) {
        install(cx);
        // Wide window: the side-by-side layout is the one where an oversized
        // result can widen the pane past the window edge.
        let bounds = gpui_kit::Bounds {
            origin: gpui_kit::Point::default(),
            size: gpui_kit::Size {
                width: gpui_kit::px(1200.),
                height: gpui_kit::px(800.),
            },
        };
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(
                gpui_kit::WindowOptions {
                    window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Base64View::new(window, cx)),
            )
            .expect("open test window")
        });
        let cx: &mut VisualTestContext = VisualTestContext::from_window(window, cx).into_mut();

        // One unbreakable base64 token far wider than the pane: the shared
        // panel clips it and keeps its own bounds inside the window.
        let long = "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVowMTIzNDU2Nzg5".repeat(50);
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.result = long.into();
                cx.notify();
            });
        });

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let viewport = window.viewport_size().width;
            let panel = window.try_find("base64-output").expect("output panel renders").bounds();
            let content = window
                .try_find((ElementId::from("base64-output"), "content"))
                .expect("result content renders")
                .bounds();
            assert!(
                panel.size.width <= viewport,
                "the output pane must stay inside the window: pane {:?}, viewport {:?}",
                panel.size.width,
                viewport,
            );
            assert!(
                content.size.width <= panel.size.width,
                "clipped content must stay inside the pane: content {:?}, pane {:?}",
                content.size.width,
                panel.size.width,
            );
        });
    }
}

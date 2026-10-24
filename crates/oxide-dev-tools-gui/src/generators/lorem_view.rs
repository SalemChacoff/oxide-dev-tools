//! `lorem_view` — the `lorem` tool page: words, sentences, and paragraphs.
//!
//! Three modes behind a `TabBar`, mirroring the core's `LoremKind` variants.
//! Validation copy mirrors the core `LoremError` messages.

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{InputState, NumberInput};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Disableable as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Window, div, prelude::FluentBuilder as _, px,
};
use oxide_dev_tools_core::{LoremKind, ParagraphOptions, SentenceOptions, WordOptions, generate_lorem};

use super::{RunReporter, report_run, summarize};
use crate::shared::registry::registry;
use crate::shared::{OutputPanel, ToolField};

/// The three generator types behind the page's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoremMode {
    Words,
    Sentences,
    Paragraphs,
}

impl LoremMode {
    fn index(self) -> usize {
        match self {
            Self::Words => 0,
            Self::Sentences => 1,
            Self::Paragraphs => 2,
        }
    }

    fn from_index(ix: usize) -> Self {
        match ix {
            1 => Self::Sentences,
            2 => Self::Paragraphs,
            _ => Self::Words,
        }
    }

    /// The count default for this mode, mirroring the core `XOptions` defaults.
    fn default_count(self) -> &'static str {
        match self {
            Self::Words => "10",
            Self::Sentences => "3",
            Self::Paragraphs => "3",
        }
    }
}

/// The `lorem` tool page.
pub struct LoremView {
    mode: LoremMode,
    count: Entity<InputState>,
    min_words: Entity<InputState>,
    max_words: Entity<InputState>,
    sentences_per_paragraph: Entity<InputState>,
    start_with_lorem: bool,
    result: SharedString,
    error: Option<SharedString>,
    on_run: Option<RunReporter>,
}

impl LoremView {
    /// Creates the page with core-default options selected.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let count = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(rust_i18n::t!("app.generators.lorem.count_placeholder").into_owned())
        });
        count.update(cx, |input, cx| input.set_value("10", window, cx));
        let min_words = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(rust_i18n::t!("app.generators.lorem.min_words_placeholder").into_owned())
        });
        min_words.update(cx, |input, cx| input.set_value("4", window, cx));
        let max_words = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(rust_i18n::t!("app.generators.lorem.max_words_placeholder").into_owned())
        });
        max_words.update(cx, |input, cx| input.set_value("12", window, cx));
        let sentences_per_paragraph = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(rust_i18n::t!("app.generators.lorem.sentences_placeholder").into_owned())
        });
        sentences_per_paragraph.update(cx, |input, cx| input.set_value("4", window, cx));

        Self {
            mode: LoremMode::Words,
            count,
            min_words,
            max_words,
            sentences_per_paragraph,
            start_with_lorem: false,
            result: SharedString::default(),
            error: None,
            on_run: None,
        }
    }

    /// Registers the run reporter the shell's recent-runs history consumes.
    pub fn on_run(mut self, reporter: RunReporter) -> Self {
        self.on_run = Some(reporter);
        self
    }

    /// Parses a positive count, mirroring the core's nonzero requirement.
    fn parse_count(value: SharedString) -> Result<usize, ()> {
        match value.trim().parse::<usize>() {
            Ok(count) if count > 0 => Ok(count),
            _ => Err(()),
        }
    }

    /// Inline validation for the count field.
    fn count_validation(&self, cx: &App) -> Option<SharedString> {
        if Self::parse_count(self.count.read(cx).value()).is_ok() {
            None
        } else {
            Some(
                rust_i18n::t!("app.generators.lorem.validation_count")
                    .into_owned()
                    .into(),
            )
        }
    }

    /// Inline validation for the minimum-words field.
    fn min_words_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != LoremMode::Sentences {
            return None;
        }
        match Self::parse_count(self.min_words.read(cx).value()) {
            Ok(_) => None,
            Err(()) => Some(
                rust_i18n::t!("app.generators.lorem.validation_count")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Inline validation for the maximum-words field; also flags an inverted
    /// min/max range, mirroring the core `InvalidRange` message.
    fn max_words_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != LoremMode::Sentences {
            return None;
        }
        let max = match Self::parse_count(self.max_words.read(cx).value()) {
            Ok(max) => max,
            Err(()) => {
                return Some(
                    rust_i18n::t!("app.generators.lorem.validation_count")
                        .into_owned()
                        .into(),
                );
            }
        };
        if let Ok(min) = Self::parse_count(self.min_words.read(cx).value())
            && min > max
        {
            return Some(
                rust_i18n::t!("app.generators.lorem.validation_range")
                    .into_owned()
                    .into(),
            );
        }
        None
    }

    /// Inline validation for the sentences-per-paragraph field.
    fn sentences_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != LoremMode::Paragraphs {
            return None;
        }
        match Self::parse_count(self.sentences_per_paragraph.read(cx).value()) {
            Ok(_) => None,
            Err(()) => Some(
                rust_i18n::t!("app.generators.lorem.validation_count")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Builds the core `LoremKind` for the current form state.
    fn build_kind(&self, cx: &App) -> Result<LoremKind, String> {
        let start_with_lorem = self.start_with_lorem;
        match self.mode {
            LoremMode::Words => {
                let count = Self::parse_count(self.count.read(cx).value())
                    .map_err(|()| rust_i18n::t!("app.generators.lorem.validation_count").into_owned())?;
                Ok(LoremKind::Words(WordOptions {
                    count,
                    start_with_lorem,
                }))
            }
            LoremMode::Sentences => {
                let count = Self::parse_count(self.count.read(cx).value())
                    .map_err(|()| rust_i18n::t!("app.generators.lorem.validation_count").into_owned())?;
                let min_words = Self::parse_count(self.min_words.read(cx).value())
                    .map_err(|()| rust_i18n::t!("app.generators.lorem.validation_count").into_owned())?;
                let max_words = Self::parse_count(self.max_words.read(cx).value())
                    .map_err(|()| rust_i18n::t!("app.generators.lorem.validation_count").into_owned())?;
                Ok(LoremKind::Sentences(SentenceOptions {
                    count,
                    min_words,
                    max_words,
                    start_with_lorem,
                }))
            }
            LoremMode::Paragraphs => {
                let count = Self::parse_count(self.count.read(cx).value())
                    .map_err(|()| rust_i18n::t!("app.generators.lorem.validation_count").into_owned())?;
                let sentences_per_paragraph = Self::parse_count(self.sentences_per_paragraph.read(cx).value())
                    .map_err(|()| rust_i18n::t!("app.generators.lorem.validation_count").into_owned())?;
                Ok(LoremKind::Paragraphs(ParagraphOptions {
                    count,
                    sentences_per_paragraph,
                    start_with_lorem,
                }))
            }
        }
    }

    /// Runs the generator with the current form state.
    fn run(&mut self, _event: &gpui_kit::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        match self.build_kind(cx) {
            Ok(kind) => match generate_lorem(kind) {
                Ok(value) => {
                    report_run(&self.on_run, "lorem", &summarize(&value), cx);
                    self.result = value.into();
                }
                Err(err) => self.error = Some(err.to_string().into()),
            },
            Err(message) => self.error = Some(message.into()),
        }
        cx.notify();
    }

    /// The decision area: run errors above the primary action.
    fn render_actions(&self, invalid: bool, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .when_some(self.error.clone(), |this, error| {
                this.child(Alert::error((ElementId::from("lorem"), "run-error"), error))
            })
            .child(
                Button::new((ElementId::from("lorem"), "generate"))
                    .primary()
                    .label(rust_i18n::t!("app.common.generate"))
                    .disabled(invalid)
                    .w_full()
                    .on_click(cx.listener(Self::run)),
            )
    }
}

impl Render for LoremView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = registry(cx)
            .find("lorem")
            .map(|meta| meta.title())
            .unwrap_or_else(|| rust_i18n::t!("app.tools.generators.lorem.title").into_owned());

        let count_validation = self.count_validation(cx);
        let min_validation = self.min_words_validation(cx);
        let max_validation = self.max_words_validation(cx);
        let sentences_validation = self.sentences_validation(cx);
        let invalid = count_validation.is_some()
            || min_validation.is_some()
            || max_validation.is_some()
            || sentences_validation.is_some();

        let count_field = ToolField::new(
            "lorem-count",
            rust_i18n::t!("app.generators.lorem.count"),
            NumberInput::new(&self.count).placeholder(rust_i18n::t!("app.generators.lorem.count_placeholder")),
        )
        .help(match self.mode {
            LoremMode::Words => rust_i18n::t!("app.generators.lorem.count_help_words"),
            LoremMode::Sentences => rust_i18n::t!("app.generators.lorem.count_help_sentences"),
            LoremMode::Paragraphs => rust_i18n::t!("app.generators.lorem.count_help_paragraphs"),
        });
        let count_field = match count_validation {
            Some(message) => count_field.validation(message),
            None => count_field,
        };

        let min_field = ToolField::new(
            "lorem-min-words",
            rust_i18n::t!("app.generators.lorem.min_words"),
            NumberInput::new(&self.min_words).placeholder(rust_i18n::t!("app.generators.lorem.min_words_placeholder")),
        );
        let min_field = match min_validation {
            Some(message) => min_field.validation(message),
            None => min_field,
        };

        let max_field = ToolField::new(
            "lorem-max-words",
            rust_i18n::t!("app.generators.lorem.max_words"),
            NumberInput::new(&self.max_words).placeholder(rust_i18n::t!("app.generators.lorem.max_words_placeholder")),
        );
        let max_field = match max_validation {
            Some(message) => max_field.validation(message),
            None => max_field,
        };

        let sentences_field = ToolField::new(
            "lorem-sentences",
            rust_i18n::t!("app.generators.lorem.sentences_per_paragraph"),
            NumberInput::new(&self.sentences_per_paragraph)
                .placeholder(rust_i18n::t!("app.generators.lorem.sentences_placeholder")),
        );
        let sentences_field = match sentences_validation {
            Some(message) => sentences_field.validation(message),
            None => sentences_field,
        };

        v_flex()
            .id("tool-page")
            .test_support()
            .flex_1()
            .min_h_0()
            .w_full()
            .px_6()
            .pb_6()
            .child(
                h_flex()
                    .justify_center()
                    .child(div().text_lg().font_semibold().child(title)),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .pt_4()
                    .gap_4()
                    .items_stretch()
                    .w_full()
                    .mx_auto()
                    .max_w(px(1060.))
                    .child(
                        v_flex()
                            .w_96()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .id("lorem-form")
                            .gap_4()
                            .child(
                                TabBar::new("lorem-mode")
                                    .selected_index(self.mode.index())
                                    .children([
                                        Tab::new().label(rust_i18n::t!("app.generators.lorem.words")),
                                        Tab::new().label(rust_i18n::t!("app.generators.lorem.sentences")),
                                        Tab::new().label(rust_i18n::t!("app.generators.lorem.paragraphs")),
                                    ])
                                    .on_click(cx.listener(|this, index, window, cx| {
                                        let mode = LoremMode::from_index(*index);
                                        if mode != this.mode {
                                            this.mode = mode;
                                            let count = this.count.clone();
                                            count.update(cx, |input, cx| {
                                                input.set_value(mode.default_count(), window, cx);
                                            });
                                            cx.notify();
                                        }
                                    })),
                            )
                            .child(count_field)
                            .when(self.mode == LoremMode::Sentences, |this| this.child(min_field).child(max_field))
                            .when(self.mode == LoremMode::Paragraphs, |this| this.child(sentences_field))
                            .child(
                                Checkbox::new("lorem-start")
                                    .checked(self.start_with_lorem)
                                    .label(rust_i18n::t!("app.generators.lorem.start").into_owned())
                                    .on_change(cx.listener(|this, checked, _, cx| {
                                        this.start_with_lorem = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(self.render_actions(invalid, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(OutputPanel::new("lorem-output").content(self.result.clone())),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{TestAppContext, VisualTestContext};

    fn install(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| crate::shell::bootstrap::install_registry(cx).expect("seed registers"));
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });
    }

    #[gpui_kit::test]
    fn words_tab_generates_from_the_core_default(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(LoremView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("lorem-count").is_some());
            assert!(window.try_find("lorem-min-words").is_none());

            window.click((ElementId::from("lorem"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("lorem-output"), "content")).is_some());
        });
    }

    #[gpui_kit::test]
    fn sentences_tab_validates_an_inverted_word_range(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(LoremView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("lorem-mode").click(1usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("lorem-min-words").is_some());
            let min_words = view.read(cx).min_words.clone();
            min_words.update(cx, |input, cx| input.set_value("12", window, cx));
            let max_words = view.read(cx).max_words.clone();
            max_words.update(cx, |input, cx| input.set_value("4", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(
                window
                    .try_find((ElementId::from("lorem-max-words"), "alert-icon"))
                    .is_some()
            );
            // The invalid form's run is inert: the output stays empty.
            window.click((ElementId::from("lorem"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("lorem-output"), "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn zero_count_shows_inline_validation(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(LoremView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let count = view.read(cx).count.clone();
            count.update(cx, |input, cx| input.set_value("0", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(
                window
                    .try_find((ElementId::from("lorem-count"), "alert-icon"))
                    .is_some()
            );
            // The invalid form's run is inert: the output stays empty.
            window.click((ElementId::from("lorem"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("lorem-output"), "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn paragraphs_tab_shows_the_sentences_field(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(LoremView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("lorem-mode").click(2usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("lorem-sentences").is_some());
            assert!(window.try_find("lorem-min-words").is_none());
        });
    }
}

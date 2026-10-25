//! `sample_view` — the `sample` tool page: PDF, PNG, and JPG fixtures.
//!
//! The only page whose result is binary: generation runs on the background
//! executor with a real loading state, the output pane shows a size summary,
//! and `Export…` opens a save dialog with the named-object overwrite
//! confirmation the interface lexicon requires.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState, NumberInput};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Disableable as _, WindowExt as _, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, ElementId, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, div, prelude::FluentBuilder as _,
};
use oxide_dev_tools_core::{JpgOptions, PdfOptions, PngOptions, SampleKind, TamperKind, generate_sample_file};

use super::{RunReporter, report_run, summarize};
use crate::shared::registry::registry;
use crate::shared::{OutputPanel, ToolField, ToolPage};

/// The three file formats behind the page's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SampleMode {
    Pdf,
    Png,
    Jpg,
}

impl SampleMode {
    fn index(self) -> usize {
        match self {
            Self::Pdf => 0,
            Self::Png => 1,
            Self::Jpg => 2,
        }
    }

    fn from_index(ix: usize) -> Self {
        match ix {
            1 => Self::Png,
            2 => Self::Jpg,
            _ => Self::Pdf,
        }
    }

    /// The exported file name for this format.
    fn file_name(self) -> &'static str {
        match self {
            Self::Pdf => "sample.pdf",
            Self::Png => "sample.png",
            Self::Jpg => "sample.jpg",
        }
    }
}

/// The tamper select's choices, in select order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TamperChoice {
    None,
    MagicBytes,
    Truncate,
    ZeroFill,
    TextFill,
}

impl TamperChoice {
    const ALL: [Self; 5] = [
        Self::None,
        Self::MagicBytes,
        Self::Truncate,
        Self::ZeroFill,
        Self::TextFill,
    ];

    fn from_index(ix: usize) -> Self {
        Self::ALL[ix.min(Self::ALL.len() - 1)]
    }

    /// The core tamper kind; `None` keeps the file intact.
    fn build(self) -> Option<TamperKind> {
        match self {
            Self::None => None,
            Self::MagicBytes => Some(TamperKind::MagicBytes),
            Self::Truncate => Some(TamperKind::Truncate),
            Self::ZeroFill => Some(TamperKind::ZeroFill),
            Self::TextFill => Some(TamperKind::TextFill),
        }
    }
}

/// The `sample` tool page.
pub struct SampleView {
    mode: SampleMode,
    tamper: TamperChoice,
    tamper_items: Vec<SharedString>,
    tamper_select: Entity<SelectState<SearchableVec<SharedString>>>,
    size: Entity<InputState>,
    pages: Entity<InputState>,
    text: Entity<InputState>,
    width: Entity<InputState>,
    height: Entity<InputState>,
    color: Entity<InputState>,
    export_path: Entity<InputState>,
    result: Option<Vec<u8>>,
    loading: bool,
    error: Option<SharedString>,
    on_run: Option<RunReporter>,
    _subscriptions: Vec<Subscription>,
}

impl SampleView {
    /// Creates the page with core-default options selected.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tamper_items: Vec<SharedString> = [
            "app.generators.sample.tamper_none",
            "app.generators.sample.tamper_magic",
            "app.generators.sample.tamper_truncate",
            "app.generators.sample.tamper_zero",
            "app.generators.sample.tamper_text",
        ]
        .iter()
        .map(|key| rust_i18n::t!(*key).into_owned().into())
        .collect();
        let tamper_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(tamper_items.clone()),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });

        let subscriptions = vec![
            cx.subscribe(&tamper_select, Self::on_tamper_confirm),
            // Resizes re-run the render so the panes switch between the
            // side-by-side and stacked layouts.
            cx.observe_window_bounds(window, |_, _, cx| cx.notify()),
        ];

        let pages = cx.new(|cx| InputState::new(window, cx));
        pages.update(cx, |input, cx| input.set_value("1", window, cx));

        Self {
            mode: SampleMode::Pdf,
            tamper: TamperChoice::None,
            tamper_items,
            tamper_select,
            size: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(rust_i18n::t!("app.generators.sample.size_placeholder").into_owned())
            }),
            pages,
            text: cx.new(|cx| InputState::new(window, cx)),
            width: cx.new(|cx| InputState::new(window, cx)),
            height: cx.new(|cx| InputState::new(window, cx)),
            color: cx.new(|cx| InputState::new(window, cx)),
            export_path: cx.new(|cx| InputState::new(window, cx)),
            result: None,
            loading: false,
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

    fn on_tamper_confirm(
        &mut self,
        _: Entity<SelectState<SearchableVec<SharedString>>>,
        event: &SelectEvent<SearchableVec<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        if let SelectEvent::Confirm(Some(value)) = event
            && let Some(ix) = self.tamper_items.iter().position(|item| item == value)
        {
            let tamper = TamperChoice::from_index(ix);
            if tamper != self.tamper {
                self.tamper = tamper;
                cx.notify();
            }
        }
    }

    /// Parses the optional size field; empty means the smallest valid file.
    fn parse_size(&self, cx: &App) -> Result<Option<u64>, ()> {
        let text = self.size.read(cx).value();
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        text.parse::<u64>().map(Some).map_err(|_| ())
    }

    /// Parses a required positive integer field.
    fn parse_positive(text: SharedString) -> Result<u32, ()> {
        match text.trim().parse::<u32>() {
            Ok(value) if value > 0 => Ok(value),
            _ => Err(()),
        }
    }

    /// Parses an optional positive integer field.
    fn parse_optional_positive(text: SharedString) -> Result<Option<u32>, ()> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        Self::parse_positive(text.into()).map(Some)
    }

    /// Inline validation for the size field.
    fn size_validation(&self, cx: &App) -> Option<SharedString> {
        match self.parse_size(cx) {
            Ok(_) => None,
            Err(()) => Some(
                rust_i18n::t!("app.generators.sample.validation_number")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Inline validation for the pages field (PDF mode).
    fn pages_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != SampleMode::Pdf {
            return None;
        }
        match Self::parse_positive(self.pages.read(cx).value()) {
            Ok(_) => None,
            Err(()) => Some(
                rust_i18n::t!("app.generators.sample.validation_number")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Inline validation for the width field (PNG mode).
    fn width_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != SampleMode::Png {
            return None;
        }
        match Self::parse_optional_positive(self.width.read(cx).value()) {
            Ok(_) => None,
            Err(()) => Some(
                rust_i18n::t!("app.generators.sample.validation_number")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Inline validation for the height field (PNG mode).
    fn height_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != SampleMode::Png {
            return None;
        }
        match Self::parse_optional_positive(self.height.read(cx).value()) {
            Ok(_) => None,
            Err(()) => Some(
                rust_i18n::t!("app.generators.sample.validation_number")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Builds the core `SampleKind` for the current form state.
    fn build_kind(&self, cx: &App) -> Result<SampleKind, SharedString> {
        let number = rust_i18n::t!("app.generators.sample.validation_number").into_owned();
        let size = self.parse_size(cx).map_err(|()| number.clone())?;
        match self.mode {
            SampleMode::Pdf => {
                let pages = Self::parse_positive(self.pages.read(cx).value()).map_err(|()| number.clone())?;
                let text = {
                    let text = self.text.read(cx).value();
                    if text.trim().is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    }
                };
                Ok(SampleKind::Pdf(PdfOptions { size, pages, text }))
            }
            SampleMode::Png => {
                let width = Self::parse_optional_positive(self.width.read(cx).value()).map_err(|()| number.clone())?;
                let height =
                    Self::parse_optional_positive(self.height.read(cx).value()).map_err(|()| number.clone())?;
                let color = {
                    let color = self.color.read(cx).value();
                    if color.trim().is_empty() {
                        None
                    } else {
                        Some(color.to_string())
                    }
                };
                Ok(SampleKind::Png(PngOptions {
                    size,
                    width,
                    height,
                    color,
                }))
            }
            SampleMode::Jpg => Ok(SampleKind::Jpg(JpgOptions { size })),
        }
    }

    /// Runs the generator on the background executor.
    fn run(&mut self, _event: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.generate(window, cx);
    }

    /// The run flow behind the button, also reachable from tests.
    fn generate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let kind = match self.build_kind(cx) {
            Ok(kind) => kind,
            Err(message) => {
                self.error = Some(message);
                cx.notify();
                return;
            }
        };
        let tamper = self.tamper.build();
        self.loading = true;
        cx.notify();

        let task = cx.background_spawn(async move { generate_sample_file(kind, tamper) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |view, cx| {
                    view.loading = false;
                    match result {
                        Ok(bytes) => {
                            report_run(&view.on_run, "sample", &summarize(&format!("{} bytes", bytes.len())), cx);
                            view.result = Some(bytes);
                        }
                        Err(err) => view.error = Some(err.to_string().into()),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        let _ = window;
    }

    /// The export command: opens the save dialog with the path prefilled.
    fn open_export_dialog(&self, window: &mut Window, cx: &mut Context<Self>) {
        let file_name = self.mode.file_name();
        self.export_path
            .update(cx, |input, cx| input.set_value(file_name, window, cx));

        let weak = cx.weak_entity();
        let path_field = self.export_path.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let field = path_field.clone();
            dialog
                .title(div().child(rust_i18n::t!("app.generators.sample.export_title", name = file_name)))
                .content(move |content, _, _| {
                    content.child(ToolField::new(
                        "sample-export-path",
                        rust_i18n::t!("app.generators.sample.path"),
                        Input::new(&field),
                    ))
                })
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(rust_i18n::t!("app.common.save"))
                        .cancel_text(rust_i18n::t!("app.common.cancel"))
                        .show_cancel(true)
                        .on_ok({
                            let weak = weak.clone();
                            move |_, window, cx| {
                                let Some(view) = weak.upgrade() else {
                                    return true;
                                };
                                let ready = view.read_with(cx, |view, cx| {
                                    let path = view.export_path.read(cx).value();
                                    let path = path.trim();
                                    if path.is_empty() {
                                        return None;
                                    }
                                    view.result.as_ref().map(|bytes| (path.to_string(), bytes.clone()))
                                });
                                let Some((path, bytes)) = ready else {
                                    return false;
                                };
                                view.update(cx, |view, cx| {
                                    if std::path::Path::new(&path).exists() {
                                        view.open_overwrite_dialog(path, bytes, window, cx);
                                    } else {
                                        view.save_bytes(path, bytes, window, cx);
                                    }
                                });
                                true
                            }
                        }),
                )
        });
    }

    /// The named overwrite confirmation before replacing an existing file.
    fn open_overwrite_dialog(&self, path: String, bytes: Vec<u8>, window: &mut Window, cx: &mut Context<Self>) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let weak = cx.weak_entity();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            dialog
                .confirm()
                .title(rust_i18n::t!("app.generators.sample.overwrite_title", name = name))
                .description(rust_i18n::t!("app.generators.sample.overwrite_body"))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(rust_i18n::t!("app.common.overwrite"))
                        .ok_variant(gpui_kit::component::button::ButtonVariant::Danger)
                        .cancel_text(rust_i18n::t!("app.common.cancel"))
                        .show_cancel(true)
                        .on_ok({
                            let weak = weak.clone();
                            let path = path.clone();
                            let bytes = bytes.clone();
                            move |_, window, cx| {
                                if let Some(view) = weak.upgrade() {
                                    view.update(cx, |view, cx| {
                                        view.save_bytes(path.clone(), bytes.clone(), window, cx)
                                    });
                                }
                                true
                            }
                        }),
                )
        });
    }

    /// Writes the bytes to the path on the background executor and reports
    /// the outcome as a notification.
    fn save_bytes(&self, path: String, bytes: Vec<u8>, window: &mut Window, cx: &mut Context<Self>) {
        let window_handle = window.window_handle();
        let task_path = path.clone();
        let task =
            cx.background_spawn(async move { std::fs::write(&task_path, &bytes).map_err(|err| err.to_string()) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |_, cx| {
                    let note = match result {
                        Ok(()) => Notification::success(rust_i18n::t!("app.generators.sample.saved", path = &path)),
                        Err(err) => Notification::error(rust_i18n::t!(
                            "app.generators.sample.save_failed",
                            path = &path,
                            error = &err
                        )),
                    };
                    let _ = window_handle.update(cx, |_, window, cx| {
                        window.push_notification(note, cx);
                    });
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// The decision area: run errors above the primary action.
    fn render_actions(&self, invalid: bool, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .when_some(self.error.clone(), |this, error| {
                this.child(Alert::error((ElementId::from("sample"), "run-error"), error))
            })
            .child(
                Button::new((ElementId::from("sample"), "generate"))
                    .primary()
                    .label(rust_i18n::t!("app.common.generate"))
                    .disabled(invalid || self.loading)
                    .w_full()
                    .on_click(cx.listener(Self::run)),
            )
    }
}

impl Render for SampleView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = registry(cx)
            .find("sample")
            .map(|meta| meta.title())
            .unwrap_or_else(|| rust_i18n::t!("app.tools.generators.sample.title").into_owned());

        let size_validation = self.size_validation(cx);
        let pages_validation = self.pages_validation(cx);
        let width_validation = self.width_validation(cx);
        let height_validation = self.height_validation(cx);
        let invalid = size_validation.is_some()
            || pages_validation.is_some()
            || width_validation.is_some()
            || height_validation.is_some();

        let size_field = ToolField::new(
            "sample-size",
            rust_i18n::t!("app.generators.sample.size"),
            NumberInput::new(&self.size).placeholder(rust_i18n::t!("app.generators.sample.size_placeholder")),
        )
        .help(rust_i18n::t!("app.generators.sample.size_help"));
        let size_field = match size_validation {
            Some(message) => size_field.validation(message),
            None => size_field,
        };

        let pages_field =
            ToolField::new("sample-pages", rust_i18n::t!("app.generators.sample.pages"), NumberInput::new(&self.pages));
        let pages_field = match pages_validation {
            Some(message) => pages_field.validation(message),
            None => pages_field,
        };

        let width_field =
            ToolField::new("sample-width", rust_i18n::t!("app.generators.sample.width"), NumberInput::new(&self.width));
        let width_field = match width_validation {
            Some(message) => width_field.validation(message),
            None => width_field,
        };

        let height_field = ToolField::new(
            "sample-height",
            rust_i18n::t!("app.generators.sample.height"),
            NumberInput::new(&self.height),
        );
        let height_field = match height_validation {
            Some(message) => height_field.validation(message),
            None => height_field,
        };

        let panel = OutputPanel::new("sample-output").loading(self.loading);
        let panel = match &self.result {
            Some(bytes) => panel.binary(bytes.clone()),
            None => panel,
        };
        let panel = panel.on_export({
            let weak = cx.weak_entity();
            move |_, window, cx| {
                if let Some(view) = weak.upgrade() {
                    view.update(cx, |view, cx| view.open_export_dialog(window, cx));
                }
            }
        });

        let form = v_flex()
            .gap_4()
            .child(
                TabBar::new("sample-mode")
                    .selected_index(self.mode.index())
                    .children([
                        Tab::new().label(rust_i18n::t!("app.generators.sample.pdf")),
                        Tab::new().label(rust_i18n::t!("app.generators.sample.png")),
                        Tab::new().label(rust_i18n::t!("app.generators.sample.jpg")),
                    ])
                    .on_click(cx.listener(|this, index, _, cx| {
                        let mode = SampleMode::from_index(*index);
                        if mode != this.mode {
                            this.mode = mode;
                            cx.notify();
                        }
                    })),
            )
            .child(
                ToolField::new(
                    "sample-tamper",
                    rust_i18n::t!("app.generators.sample.tamper"),
                    Select::new(&self.tamper_select).id("sample-tamper-select").w_full(),
                )
                .help(rust_i18n::t!("app.generators.sample.tamper_help")),
            )
            .child(size_field)
            .when(self.mode == SampleMode::Pdf, |this| {
                this.child(pages_field).child(
                    ToolField::new("sample-text", rust_i18n::t!("app.generators.sample.text"), Input::new(&self.text))
                        .help(rust_i18n::t!("app.generators.sample.text_help")),
                )
            })
            .when(self.mode == SampleMode::Png, |this| {
                this.child(width_field).child(height_field).child(
                    ToolField::new(
                        "sample-color",
                        rust_i18n::t!("app.generators.sample.color"),
                        Input::new(&self.color),
                    )
                    .help(rust_i18n::t!("app.generators.sample.color_help")),
                )
            })
            .child(self.render_actions(invalid, cx));

        ToolPage::new(title, form, panel).form_id("sample-form")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::Root;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{TestAppContext, VisualTestContext};

    fn install(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| crate::shell::bootstrap::install_registry(cx).expect("seed registers"));
        cx.update(|cx| {
            crate::theme::apply(cx, crate::theme::ThemePreference::System, gpui_kit::WindowAppearance::Light)
        });
    }

    /// A sized window root: the page's `flex_1` layout needs a definite
    /// parent height, which the shell provides in the real app.
    struct PageTestView {
        page: Entity<SampleView>,
    }

    impl Render for PageTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            v_flex().size_full().p_4().child(self.page.clone())
        }
    }

    /// Opens the page with a sized root and returns its view handle.
    fn open(cx: &mut TestAppContext) -> (Entity<SampleView>, &mut VisualTestContext) {
        let (root, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| SampleView::new(window, cx));
            PageTestView { page }
        });
        let view = root.read_with(cx, |root, _| root.page.clone());
        (view, cx)
    }

    #[gpui_kit::test]
    fn renders_the_form_and_generates_a_pdf_with_a_size_summary(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = open(cx);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("sample-tamper").is_some());
            assert!(window.try_find("sample-pages").is_some());
            assert!(window.try_find("sample-width").is_none());

            window.click((ElementId::from("sample"), "generate"), cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("sample-output"), "size")).is_some());
            assert!(window.try_find((ElementId::from("sample-output"), "export")).is_some());
            assert!(window.try_find((ElementId::from("sample-output"), "copy")).is_none());
        });
    }

    #[gpui_kit::test]
    fn png_tab_shows_the_image_fields(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = open(cx);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("sample-mode").click(1usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("sample-width").is_some());
            assert!(window.try_find("sample-height").is_some());
            assert!(window.try_find("sample-color").is_some());
            assert!(window.try_find("sample-pages").is_none());
        });
    }

    #[gpui_kit::test]
    fn invalid_pages_shows_inline_validation_and_disables_generate(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(SampleView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let pages = view.read(cx).pages.clone();
            pages.update(cx, |input, cx| input.set_value("", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(
                window
                    .try_find((ElementId::from("sample-pages"), "alert-icon"))
                    .is_some()
            );
            // The invalid form's run is inert: the output stays empty.
            window.click((ElementId::from("sample"), "generate"), cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("sample-output"), "size")).is_none());
        });
    }

    #[gpui_kit::test]
    fn export_opens_the_save_dialog_with_the_file_name_prefilled(cx: &mut TestAppContext) {
        install(cx);
        let handle = std::cell::RefCell::new(None);
        // The dialog overlay needs the component `Root`; the page's run and
        // export flows are invoked directly, since a Root-wrapped test window
        // has no definite height for the page's `flex_1` layout.
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| SampleView::new(window, cx));
            *handle.borrow_mut() = Some(page.clone());
            Root::new(page, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        let view = handle.borrow().clone().expect("page handle captured");

        cx.update(|window, cx| {
            window.render_frame(cx);
            view.update(cx, |view, cx| view.generate(window, cx));
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find((ElementId::from("sample-output"), "size")).is_some());
            view.update(cx, |view, cx| view.open_export_dialog(window, cx));
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.has_active_dialog(cx));
            assert!(window.try_find("sample-export-path").is_some());
        });
    }
}

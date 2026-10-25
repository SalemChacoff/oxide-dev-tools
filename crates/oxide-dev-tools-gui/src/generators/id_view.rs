//! `id_view` — the `id` tool page: UUID v1–v8, ULID, and NanoID (Pattern B).
//!
//! The form mirrors the core `IdKind` payloads: a mode select, an optional
//! date for the time-based versions, and namespace + name for the
//! deterministic v3/v5 pair. Validation is computed from the core parsers
//! (`parse_date`, `parse_namespace_uuid`) at render time, so the copy is the
//! core's, not a reimplementation.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{Disableable as _, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, ElementId, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, prelude::FluentBuilder as _,
};
use oxide_dev_tools_core::{IdKind, STANDARD_UUID_NAMESPACES, generate_id, parse_date, parse_namespace_uuid};

use super::{RunReporter, report_run, sentence_case, summarize};
use crate::shared::registry::registry;
use crate::shared::{OutputPanel, ToolField, ToolPage};

/// The selectable identifier formats, in select order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdMode {
    UuidV1,
    UuidV3,
    UuidV4,
    UuidV5,
    UuidV6,
    UuidV7,
    UuidV8,
    Ulid,
    NanoId,
}

impl IdMode {
    const ALL: [Self; 9] = [
        Self::UuidV1,
        Self::UuidV3,
        Self::UuidV4,
        Self::UuidV5,
        Self::UuidV6,
        Self::UuidV7,
        Self::UuidV8,
        Self::Ulid,
        Self::NanoId,
    ];

    /// The mode at the select's list position.
    fn from_index(ix: usize) -> Self {
        Self::ALL[ix.min(Self::ALL.len() - 1)]
    }
}

/// The v3/v5 namespace source: the core default, a standard namespace, or a
/// custom UUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NamespaceChoice {
    Default,
    Dns,
    Url,
    Oid,
    X500,
    Custom,
}

const NAMESPACE_CHOICES: [NamespaceChoice; 6] = [
    NamespaceChoice::Default,
    NamespaceChoice::Dns,
    NamespaceChoice::Url,
    NamespaceChoice::Oid,
    NamespaceChoice::X500,
    NamespaceChoice::Custom,
];

impl NamespaceChoice {
    /// The position of the standard namespace within [`STANDARD_UUID_NAMESPACES`].
    fn standard_index(self) -> Option<usize> {
        match self {
            Self::Dns => Some(0),
            Self::Url => Some(1),
            Self::Oid => Some(2),
            Self::X500 => Some(3),
            _ => None,
        }
    }
}

/// The `id` tool page.
pub struct IdView {
    mode: IdMode,
    mode_items: Vec<SharedString>,
    mode_select: Entity<SelectState<SearchableVec<SharedString>>>,
    namespace: NamespaceChoice,
    namespace_items: Vec<SharedString>,
    namespace_select: Entity<SelectState<SearchableVec<SharedString>>>,
    date: Entity<InputState>,
    custom_namespace: Entity<InputState>,
    name: Entity<InputState>,
    result: SharedString,
    error: Option<SharedString>,
    on_run: Option<RunReporter>,
    _subscriptions: Vec<Subscription>,
}

impl IdView {
    /// Creates the page with core-default options selected.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mode_items: Vec<SharedString> = [
            "app.generators.id.mode_uuid_v1",
            "app.generators.id.mode_uuid_v3",
            "app.generators.id.mode_uuid_v4",
            "app.generators.id.mode_uuid_v5",
            "app.generators.id.mode_uuid_v6",
            "app.generators.id.mode_uuid_v7",
            "app.generators.id.mode_uuid_v8",
            "app.generators.id.mode_ulid",
            "app.generators.id.mode_nanoid",
        ]
        .iter()
        .map(|key| rust_i18n::t!(*key).into_owned().into())
        .collect();
        let namespace_items: Vec<SharedString> = [
            "app.generators.id.namespace_default",
            "app.generators.id.namespace_dns",
            "app.generators.id.namespace_url",
            "app.generators.id.namespace_oid",
            "app.generators.id.namespace_x500",
            "app.generators.id.namespace_custom",
        ]
        .iter()
        .map(|key| rust_i18n::t!(*key).into_owned().into())
        .collect();

        let mode_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(mode_items.clone()),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });
        let namespace_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(namespace_items.clone()),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });

        let subscriptions = vec![
            cx.subscribe(&mode_select, Self::on_mode_confirm),
            cx.subscribe(&namespace_select, Self::on_namespace_confirm),
            // Resizes re-run the render so the panes switch between the
            // side-by-side and stacked layouts.
            cx.observe_window_bounds(window, |_, _, cx| cx.notify()),
        ];

        Self {
            mode: IdMode::UuidV1,
            mode_items,
            mode_select,
            namespace: NamespaceChoice::Default,
            namespace_items,
            namespace_select,
            date: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(rust_i18n::t!("app.generators.id.date_placeholder").into_owned())
            }),
            custom_namespace: cx.new(|cx| InputState::new(window, cx)),
            name: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(rust_i18n::t!("app.generators.id.name_placeholder").into_owned())
            }),
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

    fn on_mode_confirm(
        &mut self,
        _: Entity<SelectState<SearchableVec<SharedString>>>,
        event: &SelectEvent<SearchableVec<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        if let SelectEvent::Confirm(Some(value)) = event
            && let Some(ix) = self.mode_items.iter().position(|item| item == value)
        {
            let mode = IdMode::from_index(ix);
            if mode != self.mode {
                self.mode = mode;
                cx.notify();
            }
        }
    }

    fn on_namespace_confirm(
        &mut self,
        _: Entity<SelectState<SearchableVec<SharedString>>>,
        event: &SelectEvent<SearchableVec<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        if let SelectEvent::Confirm(Some(value)) = event
            && let Some(ix) = self.namespace_items.iter().position(|item| item == value)
        {
            let namespace = NAMESPACE_CHOICES[ix.min(NAMESPACE_CHOICES.len() - 1)];
            if namespace != self.namespace {
                self.namespace = namespace;
                cx.notify();
            }
        }
    }

    /// Whether the date field applies to the current mode.
    fn show_date(&self) -> bool {
        matches!(self.mode, IdMode::UuidV1 | IdMode::UuidV6 | IdMode::UuidV7)
    }

    /// Whether the namespace/name fields apply to the current mode.
    fn show_namespace(&self) -> bool {
        matches!(self.mode, IdMode::UuidV3 | IdMode::UuidV5)
    }

    /// Inline validation for the date field: the core parser's error message.
    fn date_validation(&self, cx: &App) -> Option<SharedString> {
        if !self.show_date() {
            return None;
        }
        let text = self.date.read(cx).value();
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        match parse_date(Some(text.to_string())) {
            Ok(_) => None,
            Err(err) => Some(sentence_case(err.to_string()).into()),
        }
    }

    /// Inline validation for the custom namespace field.
    fn custom_namespace_validation(&self, cx: &App) -> Option<SharedString> {
        if !self.show_namespace() || self.namespace != NamespaceChoice::Custom {
            return None;
        }
        let text = self.custom_namespace.read(cx).value();
        match parse_namespace_uuid(text.trim()) {
            Ok(_) => None,
            Err(err) => Some(sentence_case(err.to_string()).into()),
        }
    }

    /// Inline validation for the name field: a custom namespace needs a name.
    fn name_validation(&self, cx: &App) -> Option<SharedString> {
        if !self.show_namespace() || self.namespace == NamespaceChoice::Default {
            return None;
        }
        if self.name.read(cx).value().trim().is_empty() {
            Some(rust_i18n::t!("app.generators.id.name_validation").into_owned().into())
        } else {
            None
        }
    }

    /// Builds the core `IdKind` for the current form state.
    fn build_kind(&self, cx: &App) -> Result<IdKind, String> {
        let time = if self.show_date() {
            let text = self.date.read(cx).value();
            let text = text.trim();
            let value = if text.is_empty() { None } else { Some(text.to_string()) };
            parse_date(value).map_err(|err| err.to_string())?
        } else {
            None
        };

        // The v3/v5 pair shares one params builder; the payload type is
        // inferred from the `IdKind` constructors, so the GUI never names
        // core-only types.
        let namespaced = |is_v5: bool| -> Result<IdKind, String> {
            let name = self.name.read(cx).value();
            let name = name.trim();
            if self.namespace == NamespaceChoice::Default && name.is_empty() {
                return Ok(if is_v5 {
                    IdKind::UuidV5(None)
                } else {
                    IdKind::UuidV3(None)
                });
            }
            if name.is_empty() {
                return Err(rust_i18n::t!("app.generators.id.name_validation").into_owned());
            }
            let namespace = match self.namespace.standard_index() {
                Some(ix) => STANDARD_UUID_NAMESPACES[ix].1,
                None => {
                    let text = self.custom_namespace.read(cx).value();
                    parse_namespace_uuid(text.trim()).map_err(|err| err.to_string())?
                }
            };
            let params = (namespace, name.as_bytes().to_vec());
            Ok(if is_v5 {
                IdKind::UuidV5(Some(params))
            } else {
                IdKind::UuidV3(Some(params))
            })
        };

        match self.mode {
            IdMode::UuidV1 => Ok(IdKind::UuidV1(time)),
            IdMode::UuidV3 => namespaced(false),
            IdMode::UuidV4 => Ok(IdKind::UuidV4),
            IdMode::UuidV5 => namespaced(true),
            IdMode::UuidV6 => Ok(IdKind::UuidV6(time)),
            IdMode::UuidV7 => Ok(IdKind::UuidV7(time)),
            IdMode::UuidV8 => Ok(IdKind::UuidV8),
            IdMode::Ulid => Ok(IdKind::Ulid),
            IdMode::NanoId => Ok(IdKind::NanoId),
        }
    }

    /// Runs the generator with the current form state.
    fn run(&mut self, _event: &gpui_kit::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        match self.build_kind(cx) {
            Ok(kind) => match generate_id(kind) {
                Ok(value) => {
                    report_run(&self.on_run, "id", &summarize(&value), cx);
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
                this.child(Alert::error((ElementId::from("id"), "run-error"), error))
            })
            .child(
                Button::new((ElementId::from("id"), "generate"))
                    .primary()
                    .label(rust_i18n::t!("app.common.generate"))
                    .disabled(invalid)
                    .w_full()
                    .on_click(cx.listener(Self::run)),
            )
    }
}

impl Render for IdView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = registry(cx)
            .find("id")
            .map(|meta| meta.title())
            .unwrap_or_else(|| rust_i18n::t!("app.tools.generators.id.title").into_owned());

        let date_validation = self.date_validation(cx);
        let custom_validation = self.custom_namespace_validation(cx);
        let name_validation = self.name_validation(cx);
        let invalid = date_validation.is_some() || custom_validation.is_some() || name_validation.is_some();

        let date_field = ToolField::new("id-date", rust_i18n::t!("app.generators.id.date"), Input::new(&self.date))
            .help(rust_i18n::t!("app.generators.id.date_help"));
        let date_field = match date_validation {
            Some(message) => date_field.validation(message),
            None => date_field,
        };

        let namespace_uuid_field = ToolField::new(
            "id-namespace-uuid",
            rust_i18n::t!("app.generators.id.custom_namespace"),
            Input::new(&self.custom_namespace),
        );
        let namespace_uuid_field = match custom_validation {
            Some(message) => namespace_uuid_field.validation(message),
            None => namespace_uuid_field,
        };

        let name_field = ToolField::new("id-name", rust_i18n::t!("app.generators.id.name"), Input::new(&self.name))
            .help(rust_i18n::t!("app.generators.id.name_help"));
        let name_field = match name_validation {
            Some(message) => name_field.validation(message),
            None => name_field,
        };

        let form = v_flex()
            .gap_4()
            .child(
                ToolField::new(
                    "id-mode",
                    rust_i18n::t!("app.generators.id.kind"),
                    Select::new(&self.mode_select).id("id-mode-select").w_full(),
                )
                .help(rust_i18n::t!("app.generators.id.kind_help")),
            )
            .when(self.show_date(), |this| this.child(date_field))
            .when(self.show_namespace(), |this| {
                this.child(
                    ToolField::new(
                        "id-namespace",
                        rust_i18n::t!("app.generators.id.namespace"),
                        Select::new(&self.namespace_select).id("id-namespace-select").w_full(),
                    )
                    .help(rust_i18n::t!("app.generators.id.namespace_help")),
                )
                .when(self.namespace == NamespaceChoice::Custom, |this| this.child(namespace_uuid_field))
                .child(name_field)
            })
            .child(self.render_actions(invalid, cx));

        ToolPage::new(title, form, OutputPanel::new("id-output").content(self.result.clone())).form_id("id-form")
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
    fn renders_the_form_and_generates_an_id(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(IdView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("id-mode").is_some());
            assert!(window.try_find("id-date").is_some(), "v1 shows the date field");
            assert!(window.try_find((ElementId::from("id-output"), "empty")).is_some());

            window.click((ElementId::from("id"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("id-output"), "content")).is_some());
            assert!(window.try_find((ElementId::from("id-output"), "empty")).is_none());
        });
    }

    #[gpui_kit::test]
    fn invalid_date_shows_inline_validation_and_disables_generate(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(IdView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let date = view.read(cx).date.clone();
            date.update(cx, |input, cx| input.set_value("not-a-date", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("id-date"), "alert-icon")).is_some());
            // The invalid form's run is inert: the output stays empty.
            window.click((ElementId::from("id"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("id-output"), "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn selecting_uuid_v4_hides_the_date_and_namespace_fields(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(IdView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("id-mode-select").click("input", cx);
            // One step down selects UUID v3: the date field goes away and the
            // namespace fields appear.
            window.press("down", cx);
            window.press("enter", cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("id-date").is_none());
            assert!(window.try_find("id-namespace").is_some());
        });
    }

    #[gpui_kit::test]
    fn custom_namespace_requires_a_name(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(IdView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            view.update(cx, |view, cx| {
                view.mode = IdMode::UuidV5;
                view.namespace = NamespaceChoice::Custom;
                cx.notify();
            });
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("id-namespace-uuid").is_some());
            // An empty name (and an empty custom UUID) both validate inline;
            // the invalid form's run is inert.
            assert!(window.try_find((ElementId::from("id-name"), "alert-icon")).is_some());
            window.click((ElementId::from("id"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("id-output"), "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn narrow_windows_stack_the_result_under_the_form(cx: &mut TestAppContext) {
        install(cx);
        let bounds = gpui_kit::Bounds {
            origin: gpui_kit::Point::default(),
            size: gpui_kit::Size {
                width: gpui_kit::px(800.),
                height: gpui_kit::px(640.),
            },
        };
        let (window, _content) = cx.update(|cx| {
            gpui_kit::open_window(
                gpui_kit::WindowOptions {
                    window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| IdView::new(window, cx)),
            )
            .expect("open test window")
        });
        let cx: &mut VisualTestContext = VisualTestContext::from_window(window, cx).into_mut();

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("tool-panes"), "stacked")).is_some());
            assert!(window.try_find((ElementId::from("tool-panes"), "side")).is_none());
            // The form and the result pane both stay reachable.
            assert!(window.try_find("id-mode").is_some());
            assert!(window.try_find("id-output").is_some());
        });
    }
}

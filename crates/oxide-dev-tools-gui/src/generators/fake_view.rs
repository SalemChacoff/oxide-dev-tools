//! `fake_view` — the `fake` tool page: personas, names, emails, phones,
//! addresses, and companies.
//!
//! One mode select over the core's twelve `FakeKind` variants plus a shared
//! count; the output joins generated values with newlines.

use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{InputState, NumberInput};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{Disableable as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, px,
};
use oxide_dev_tools_core::{FakeKind, FakeOptions, generate_fake};

use super::{RunReporter, report_run, summarize};
use crate::shared::registry::registry;
use crate::shared::{OutputPanel, ToolField};

/// The selectable fake-data kinds, in select order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FakeMode {
    Person,
    Name,
    Surname,
    FullName,
    Email,
    Phone,
    Address,
    City,
    Country,
    Company,
    JobTitle,
    Username,
}

impl FakeMode {
    const ALL: [Self; 12] = [
        Self::Person,
        Self::Name,
        Self::Surname,
        Self::FullName,
        Self::Email,
        Self::Phone,
        Self::Address,
        Self::City,
        Self::Country,
        Self::Company,
        Self::JobTitle,
        Self::Username,
    ];

    /// The mode at the select's list position.
    fn from_index(ix: usize) -> Self {
        Self::ALL[ix.min(Self::ALL.len() - 1)]
    }

    /// The core kind for this mode at the given count.
    fn build(self, count: usize) -> FakeKind {
        let options = FakeOptions { count };
        match self {
            Self::Person => FakeKind::Person(options),
            Self::Name => FakeKind::Name(options),
            Self::Surname => FakeKind::Surname(options),
            Self::FullName => FakeKind::FullName(options),
            Self::Email => FakeKind::Email(options),
            Self::Phone => FakeKind::Phone(options),
            Self::Address => FakeKind::Address(options),
            Self::City => FakeKind::City(options),
            Self::Country => FakeKind::Country(options),
            Self::Company => FakeKind::Company(options),
            Self::JobTitle => FakeKind::JobTitle(options),
            Self::Username => FakeKind::Username(options),
        }
    }
}

/// The `fake` tool page.
pub struct FakeView {
    mode: FakeMode,
    mode_items: Vec<SharedString>,
    mode_select: Entity<SelectState<SearchableVec<SharedString>>>,
    count: Entity<InputState>,
    result: SharedString,
    error: Option<SharedString>,
    on_run: Option<RunReporter>,
    _subscriptions: Vec<Subscription>,
}

impl FakeView {
    /// Creates the page with core-default options selected.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mode_items: Vec<SharedString> = [
            "app.generators.fake.mode_person",
            "app.generators.fake.mode_name",
            "app.generators.fake.mode_surname",
            "app.generators.fake.mode_full_name",
            "app.generators.fake.mode_email",
            "app.generators.fake.mode_phone",
            "app.generators.fake.mode_address",
            "app.generators.fake.mode_city",
            "app.generators.fake.mode_country",
            "app.generators.fake.mode_company",
            "app.generators.fake.mode_job_title",
            "app.generators.fake.mode_username",
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

        let subscriptions = vec![cx.subscribe(&mode_select, Self::on_mode_confirm)];

        let count = cx.new(|cx| {
            InputState::new(window, cx).placeholder(rust_i18n::t!("app.generators.fake.count_placeholder").into_owned())
        });
        count.update(cx, |input, cx| input.set_value("1", window, cx));

        Self {
            mode: FakeMode::Person,
            mode_items,
            mode_select,
            count,
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
            let mode = FakeMode::from_index(ix);
            if mode != self.mode {
                self.mode = mode;
                cx.notify();
            }
        }
    }

    /// Inline validation for the count field, mirroring the core `ZeroCount`.
    fn count_validation(&self, cx: &App) -> Option<SharedString> {
        match self.count.read(cx).value().trim().parse::<usize>() {
            Ok(count) if count > 0 => None,
            _ => Some(
                rust_i18n::t!("app.generators.fake.validation_count")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Runs the generator with the current form state.
    fn run(&mut self, _event: &gpui_kit::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let count = match self.count.read(cx).value().trim().parse::<usize>() {
            Ok(count) if count > 0 => count,
            _ => {
                cx.notify();
                return;
            }
        };
        match generate_fake(self.mode.build(count)) {
            Ok(value) => {
                report_run(&self.on_run, "fake", &summarize(&value), cx);
                self.result = value.into();
            }
            Err(err) => self.error = Some(err.to_string().into()),
        }
        cx.notify();
    }

    /// The decision area: run errors above the primary action.
    fn render_actions(&self, invalid: bool, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .when_some(self.error.clone(), |this, error| {
                this.child(Alert::error((ElementId::from("fake"), "run-error"), error))
            })
            .child(
                Button::new((ElementId::from("fake"), "generate"))
                    .primary()
                    .label(rust_i18n::t!("app.common.generate"))
                    .disabled(invalid)
                    .w_full()
                    .on_click(cx.listener(Self::run)),
            )
    }
}

impl Render for FakeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = registry(cx)
            .find("fake")
            .map(|meta| meta.title())
            .unwrap_or_else(|| rust_i18n::t!("app.tools.generators.fake.title").into_owned());

        let count_validation = self.count_validation(cx);

        let count_invalid = count_validation.is_some();
        let count_field = ToolField::new(
            "fake-count",
            rust_i18n::t!("app.generators.fake.count"),
            NumberInput::new(&self.count).placeholder(rust_i18n::t!("app.generators.fake.count_placeholder")),
        )
        .help(rust_i18n::t!("app.generators.fake.count_help"));
        let count_field = match count_validation {
            Some(message) => count_field.validation(message),
            None => count_field,
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
                            .id("fake-form")
                            .gap_4()
                            .child(ToolField::new(
                                "fake-mode",
                                rust_i18n::t!("app.generators.fake.kind"),
                                Select::new(&self.mode_select).id("fake-mode-select").w_full(),
                            ))
                            .child(count_field)
                            .child(self.render_actions(count_invalid, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(OutputPanel::new("fake-output").content(self.result.clone())),
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
    fn renders_the_form_and_generates_a_persona(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(FakeView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("fake-mode").is_some());
            assert!(window.try_find("fake-count").is_some());

            window.click((ElementId::from("fake"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("fake-output"), "content")).is_some());
        });
    }

    #[gpui_kit::test]
    fn zero_count_shows_inline_validation_and_disables_generate(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(FakeView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            let count = view.read(cx).count.clone();
            count.update(cx, |input, cx| input.set_value("0", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("fake-count"), "alert-icon")).is_some());
            // The invalid form's run is inert: the output stays empty.
            window.click((ElementId::from("fake"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("fake-output"), "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn count_above_one_joins_values_with_newlines(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(FakeView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            // Names join with newlines (persona cards are multiline blocks).
            view.update(cx, |view, cx| {
                view.mode = FakeMode::Name;
                cx.notify();
            });
            let count = view.read(cx).count.clone();
            count.update(cx, |input, cx| input.set_value("3", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("fake"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            let result = view.read(cx).result.clone();
            assert_eq!(result.lines().count(), 3);
        });
    }
}

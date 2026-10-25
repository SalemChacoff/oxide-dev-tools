//! `key_view` — the `key` tool page: passwords, tokens, and JWTs (HS256).
//!
//! One page, three modes behind a `TabBar` (the core's `KeyKind` variants
//! plus the JWT generator the CLI exposes under the same command). Core
//! messages surface inline where a field owns the error; everything else
//! lands in the run-error alert.

use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputState, NumberInput, Textarea, TextareaState};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Disableable as _, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, ElementId, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, prelude::FluentBuilder as _,
};
use oxide_dev_tools_core::{
    JwtOptions, KeyKind, PasswordOptions, TokenEncoding, TokenOptions, generate_jwt, generate_key, parse_exp,
};

use super::{RunReporter, report_run, sentence_case, summarize};
use crate::shared::registry::registry;
use crate::shared::{OutputPanel, ToolField, ToolPage};

/// The three generator types behind the page's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMode {
    Password,
    Token,
    Jwt,
}

impl KeyMode {
    /// The tab position for this mode.
    fn index(self) -> usize {
        match self {
            Self::Password => 0,
            Self::Token => 1,
            Self::Jwt => 2,
        }
    }

    fn from_index(ix: usize) -> Self {
        match ix {
            1 => Self::Token,
            2 => Self::Jwt,
            _ => Self::Password,
        }
    }

    /// The length default for this mode, mirroring the core `XOptions` defaults.
    fn default_length(self) -> &'static str {
        match self {
            Self::Password => "16",
            Self::Token => "32",
            Self::Jwt => "16",
        }
    }
}

/// The `key` tool page.
pub struct KeyView {
    mode: KeyMode,
    length: Entity<InputState>,
    lowercase: bool,
    uppercase: bool,
    digits: bool,
    special: bool,
    encoding_items: Vec<SharedString>,
    encoding_select: Entity<SelectState<SearchableVec<SharedString>>>,
    payload: Entity<TextareaState>,
    secret: Entity<InputState>,
    expiry: Entity<InputState>,
    result: SharedString,
    error: Option<SharedString>,
    on_run: Option<RunReporter>,
    _subscriptions: Vec<Subscription>,
}

impl KeyView {
    /// Creates the page with core-default options selected.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let encoding_items: Vec<SharedString> =
            ["app.generators.key.encoding_hex", "app.generators.key.encoding_base64"]
                .iter()
                .map(|key| rust_i18n::t!(*key).into_owned().into())
                .collect();
        let encoding_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(encoding_items.clone()),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });

        let subscriptions = vec![
            cx.subscribe(&encoding_select, Self::on_encoding_confirm),
            // Resizes re-run the render so the panes switch between the
            // side-by-side and stacked layouts.
            cx.observe_window_bounds(window, |_, _, cx| cx.notify()),
        ];

        let length = cx.new(|cx| {
            InputState::new(window, cx).placeholder(rust_i18n::t!("app.generators.key.length_placeholder").into_owned())
        });
        length.update(cx, |input, cx| input.set_value("16", window, cx));

        Self {
            mode: KeyMode::Password,
            length,
            lowercase: true,
            uppercase: true,
            digits: true,
            special: false,
            encoding_items,
            encoding_select,
            payload: cx.new(|cx| TextareaState::new(window, cx)),
            secret: cx.new(|cx| InputState::new(window, cx)),
            expiry: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(rust_i18n::t!("app.generators.key.expiry_placeholder").into_owned())
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

    fn on_encoding_confirm(
        &mut self,
        _: Entity<SelectState<SearchableVec<SharedString>>>,
        event: &SelectEvent<SearchableVec<SharedString>>,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, SelectEvent::Confirm(_)) {
            cx.notify();
        }
    }

    /// The selected token encoding, mirroring the core [`TokenEncoding`].
    fn token_encoding(&self, cx: &App) -> TokenEncoding {
        match self.encoding_select.read(cx).selected_value() {
            Some(value) if value == &self.encoding_items[1] => TokenEncoding::Base64,
            _ => TokenEncoding::Hex,
        }
    }

    /// Inline validation for the length field (password and token modes).
    fn length_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode == KeyMode::Jwt {
            return None;
        }
        let text = self.length.read(cx).value();
        match text.trim().parse::<usize>() {
            Ok(length) if length > 0 => None,
            _ => Some(
                rust_i18n::t!("app.generators.key.validation_number")
                    .into_owned()
                    .into(),
            ),
        }
    }

    /// Inline validation for the secret field (JWT mode).
    fn secret_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != KeyMode::Jwt {
            return None;
        }
        if self.secret.read(cx).value().trim().is_empty() {
            Some(
                rust_i18n::t!("app.generators.key.secret_validation")
                    .into_owned()
                    .into(),
            )
        } else {
            None
        }
    }

    /// Inline validation for the expiry field: the core parser's message.
    fn expiry_validation(&self, cx: &App) -> Option<SharedString> {
        if self.mode != KeyMode::Jwt {
            return None;
        }
        let text = self.expiry.read(cx).value();
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        match parse_exp(Some(text.to_string())) {
            Ok(_) => None,
            Err(err) => Some(sentence_case(err.to_string()).into()),
        }
    }

    /// Parses the length field; empty or non-numeric fails.
    fn parsed_length(&self, cx: &App) -> Result<usize, ()> {
        self.length.read(cx).value().trim().parse::<usize>().map_err(|_| ())
    }

    /// Runs the generator with the current form state.
    fn run(&mut self, _event: &gpui_kit::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let outcome = match self.mode {
            KeyMode::Password => {
                let length = match self.parsed_length(cx) {
                    Ok(length) => length,
                    Err(()) => {
                        cx.notify();
                        return;
                    }
                };
                generate_key(KeyKind::Password(PasswordOptions {
                    length,
                    lowercase: self.lowercase,
                    uppercase: self.uppercase,
                    digits: self.digits,
                    special: self.special,
                }))
                .map_err(|err| err.to_string())
            }
            KeyMode::Token => {
                let length = match self.parsed_length(cx) {
                    Ok(length) => length,
                    Err(()) => {
                        cx.notify();
                        return;
                    }
                };
                generate_key(KeyKind::Token(TokenOptions {
                    length,
                    encoding: self.token_encoding(cx),
                }))
                .map_err(|err| err.to_string())
            }
            KeyMode::Jwt => {
                let exp = match parse_exp(trimmed(self.expiry.read(cx).value())) {
                    Ok(exp) => exp,
                    Err(err) => {
                        self.error = Some(err.to_string().into());
                        cx.notify();
                        return;
                    }
                };
                generate_jwt(JwtOptions {
                    payload: self.payload.read(cx).value().to_string(),
                    secret: self.secret.read(cx).value().to_string(),
                    exp,
                })
                .map_err(|err| err.to_string())
            }
        };

        match outcome {
            Ok(value) => {
                report_run(&self.on_run, "key", &summarize(&value), cx);
                self.result = value.into();
            }
            Err(message) => self.error = Some(message.into()),
        }
        cx.notify();
    }

    /// The decision area: run errors above the primary action.
    fn render_actions(&self, invalid: bool, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .when_some(self.error.clone(), |this, error| {
                this.child(Alert::error((ElementId::from("key"), "run-error"), error))
            })
            .child(
                Button::new((ElementId::from("key"), "generate"))
                    .primary()
                    .label(rust_i18n::t!("app.common.generate"))
                    .disabled(invalid)
                    .w_full()
                    .on_click(cx.listener(Self::run)),
            )
    }
}

/// The trimmed value of an input, as an owned `Option`.
fn trimmed(value: SharedString) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

impl Render for KeyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = registry(cx)
            .find("key")
            .map(|meta| meta.title())
            .unwrap_or_else(|| rust_i18n::t!("app.tools.generators.key.title").into_owned());

        let length_validation = self.length_validation(cx);
        let secret_validation = self.secret_validation(cx);
        let expiry_validation = self.expiry_validation(cx);
        let invalid = length_validation.is_some() || secret_validation.is_some() || expiry_validation.is_some();

        let length_field = ToolField::new(
            "key-length",
            rust_i18n::t!("app.generators.key.length"),
            NumberInput::new(&self.length).placeholder(rust_i18n::t!("app.generators.key.length_placeholder")),
        )
        .help(match self.mode {
            KeyMode::Password => rust_i18n::t!("app.generators.key.length_help_password"),
            KeyMode::Token => rust_i18n::t!("app.generators.key.length_help_token"),
            KeyMode::Jwt => rust_i18n::t!("app.generators.key.length_help_password"),
        });
        let length_field = match length_validation {
            Some(message) => length_field.validation(message),
            None => length_field,
        };

        let secret_field =
            ToolField::new("key-secret", rust_i18n::t!("app.generators.key.secret"), Input::new(&self.secret))
                .help(rust_i18n::t!("app.generators.key.secret_help"));
        let secret_field = match secret_validation {
            Some(message) => secret_field.validation(message),
            None => secret_field,
        };

        let expiry_field =
            ToolField::new("key-expiry", rust_i18n::t!("app.generators.key.expiry"), Input::new(&self.expiry))
                .help(rust_i18n::t!("app.generators.key.expiry_help"));
        let expiry_field = match expiry_validation {
            Some(message) => expiry_field.validation(message),
            None => expiry_field,
        };

        // One mode's fields render per frame; the match arms own the fields
        // they need so nothing is built (or moved) twice.
        let mode_fields: Vec<gpui_kit::AnyElement> = match self.mode {
            KeyMode::Password => vec![
                length_field.into_any_element(),
                v_flex()
                    .gap_2()
                    .child(
                        Checkbox::new("key-lowercase")
                            .checked(self.lowercase)
                            .label(rust_i18n::t!("app.generators.key.lowercase").into_owned())
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.lowercase = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("key-uppercase")
                            .checked(self.uppercase)
                            .label(rust_i18n::t!("app.generators.key.uppercase").into_owned())
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.uppercase = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("key-digits")
                            .checked(self.digits)
                            .label(rust_i18n::t!("app.generators.key.digits").into_owned())
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.digits = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("key-special")
                            .checked(self.special)
                            .label(rust_i18n::t!("app.generators.key.special").into_owned())
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.special = *checked;
                                cx.notify();
                            })),
                    )
                    .into_any_element(),
            ],
            KeyMode::Token => vec![
                length_field.into_any_element(),
                ToolField::new(
                    "key-encoding",
                    rust_i18n::t!("app.generators.key.encoding"),
                    Select::new(&self.encoding_select).id("key-encoding-select").w_full(),
                )
                .into_any_element(),
            ],
            KeyMode::Jwt => vec![
                ToolField::new(
                    "key-payload",
                    rust_i18n::t!("app.generators.key.payload"),
                    Textarea::new(&self.payload),
                )
                .help(rust_i18n::t!("app.generators.key.payload_help"))
                .into_any_element(),
                secret_field.into_any_element(),
                expiry_field.into_any_element(),
            ],
        };

        let form = v_flex()
            .gap_4()
            .child(
                TabBar::new("key-mode")
                    .selected_index(self.mode.index())
                    .children([
                        Tab::new().label(rust_i18n::t!("app.generators.key.password")),
                        Tab::new().label(rust_i18n::t!("app.generators.key.token")),
                        Tab::new().label(rust_i18n::t!("app.generators.key.jwt")),
                    ])
                    .on_click(cx.listener(|this, index, window, cx| {
                        let mode = KeyMode::from_index(*index);
                        if mode != this.mode {
                            this.mode = mode;
                            let length = this.length.clone();
                            length.update(cx, |input, cx| {
                                input.set_value(mode.default_length(), window, cx);
                            });
                            cx.notify();
                        }
                    })),
            )
            .children(mode_fields)
            .child(self.render_actions(invalid, cx));

        ToolPage::new(title, form, OutputPanel::new("key-output").content(self.result.clone())).form_id("key-form")
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
    fn password_tab_renders_all_character_sets_and_generates(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(KeyView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("key-length").is_some());
            assert!(window.try_find("key-lowercase").is_some());
            assert!(window.try_find("key-uppercase").is_some());
            assert!(window.try_find("key-digits").is_some());
            assert!(window.try_find("key-special").is_some());
            assert!(window.try_find("key-payload").is_none());

            window.click((ElementId::from("key"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("key-output"), "content")).is_some());
        });
    }

    #[gpui_kit::test]
    fn jwt_tab_shows_claims_fields_and_validates_an_empty_secret(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(KeyView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("key-mode").click(2usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("key-payload").is_some());
            assert!(window.try_find("key-secret").is_some());
            assert!(window.try_find("key-expiry").is_some());
            assert!(window.try_find("key-lowercase").is_none());
            // The secret starts empty: inline validation, and the invalid
            // form's run is inert.
            assert!(window.try_find((ElementId::from("key-secret"), "alert-icon")).is_some());
            window.click((ElementId::from("key"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find((ElementId::from("key-output"), "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn jwt_run_surfaces_core_payload_errors(cx: &mut TestAppContext) {
        install(cx);
        let (view, cx) = cx.add_window_view(KeyView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("key-mode").click(2usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            let payload = view.read(cx).payload.clone();
            payload.update(cx, |state, cx| state.set_value("not json", window, cx));
            let secret = view.read(cx).secret.clone();
            secret.update(cx, |state, cx| state.set_value("s3cret", window, cx));
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.click((ElementId::from("key"), "generate"), cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            // The core's message lands in the page's error slot.
            assert!(view.read(cx).error.is_some());
            assert!(window.try_find((ElementId::from("key-output"), "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn token_tab_shows_the_encoding_select(cx: &mut TestAppContext) {
        install(cx);
        let (_view, cx) = cx.add_window_view(KeyView::new);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.within("key-mode").click(1usize, cx);
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
            assert!(window.try_find("key-encoding").is_some());
            assert!(window.try_find("key-special").is_none());
        });
    }
}

//! oxide-design theme module — the visual language of `docs/design/phase7-ui.md`
//! §3 and §6, implemented on the gpui-kit `Theme` system.
//!
//! Rules enforced here:
//! - Raw color values live only in this module ([`palette`]). Every other
//!   module reads gpui-kit slots via `cx.theme()` or the app-level tokens from
//!   [`OxideTheme::tokens`].
//! - Theme switching goes through [`apply`]: it swaps the two [`ThemeConfig`]s
//!   on the gpui-kit theme global and flips the mode, without touching focus
//!   or window state.
//!
//! The values mirror the static mock (`docs/design/mock/design.css`), which is
//! the eyes-on acceptance reference for this design.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::Theme;
use gpui_kit::component::theme::{ThemeConfig, ThemeConfigColors, ThemeMode};
use gpui_kit::{App, Global, Hsla, SharedString, WindowAppearance, rgb};

/// Palette literals from the mock stylesheet, the single source for both the
/// gpui-kit config strings and the resolved [`OxideTokens`]. Entries that
/// differ between modes carry a `_LIGHT` / `_DARK` suffix.
mod palette {
    pub const PRIMARY: u32 = 0xCE_42_2B;
    pub const PRIMARY_HOVER_LIGHT: u32 = 0xB8_3A_26;
    pub const PRIMARY_HOVER_DARK: u32 = 0xDD_5A_3F;
    pub const BACKGROUND_LIGHT: u32 = 0xFF_FF_FF;
    pub const BACKGROUND_DARK: u32 = 0x0E_0E_0F;
    pub const SURFACE_LIGHT: u32 = 0xF7_F7_F8;
    pub const SURFACE_DARK: u32 = 0x17_18_1A;
    pub const SURFACE_2_LIGHT: u32 = 0xEF_EF_F1;
    pub const SURFACE_2_DARK: u32 = 0x20_22_24;
    pub const BORDER_LIGHT: u32 = 0xE4_E5_E7;
    pub const BORDER_DARK: u32 = 0x2A_2C_2F;
    pub const FOREGROUND_LIGHT: u32 = 0x17_18_1A;
    pub const FOREGROUND_DARK: u32 = 0xF2_F2_F3;
    pub const MUTED_LIGHT: u32 = 0x6B_6E_73;
    pub const MUTED_DARK: u32 = 0x8E_90_94;
    pub const DANGER_LIGHT: u32 = 0xB9_1C_1C;
    pub const DANGER_DARK: u32 = 0xE5_48_4D;
    pub const SUCCESS_LIGHT: u32 = 0x15_80_3D;
    pub const SUCCESS_DARK: u32 = 0x30_A4_6C;
    pub const ON_PRIMARY: u32 = 0xFF_FF_FF;

    /// Alpha channels, quantized to 8-bit from the mock's float alphas
    /// (9% -> 23, 35% -> 89, 8% -> 20, 12% -> 31, all over 255).
    pub const ALPHA_TINT: u8 = 23;
    pub const ALPHA_RING: u8 = 89;
    pub const ALPHA_DANGER_TINT_LIGHT: u8 = 20;
    pub const ALPHA_DANGER_TINT_DARK: u8 = 31;
    pub const ALPHA_SUCCESS_TINT_LIGHT: u8 = 23;
    pub const ALPHA_SUCCESS_TINT_DARK: u8 = 31;
}

/// The oxide-design semantic token set, resolved per mode.
///
/// gpui-kit models most roles in [`ThemeConfigColors`]; the extra slots here
/// (`surface`, `surface_2`, the tints, `radius_s`) have no library equivalent,
/// so the active set is published through [`OxideTheme`] for application
/// components.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct OxideTokens {
    pub primary: Hsla,
    pub primary_hover: Hsla,
    /// Primary at ~9% alpha: hover/selected tints (cards, active nav items).
    pub primary_tint: Hsla,
    /// Primary at ~35% alpha: focus rings and selection.
    pub primary_ring: Hsla,
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_2: Hsla,
    pub border: Hsla,
    pub foreground: Hsla,
    pub muted: Hsla,
    pub danger: Hsla,
    pub danger_tint: Hsla,
    pub success: Hsla,
    pub success_tint: Hsla,
    /// Radius for compact elements (kbd chips, segmented tabs), in px.
    pub radius_s: usize,
}

/// Mode-varying palette entries (the mock's light/dark splits).
struct ModePalette {
    primary_hover: u32,
    background: u32,
    surface: u32,
    surface_2: u32,
    border: u32,
    foreground: u32,
    muted: u32,
    danger: u32,
    success: u32,
    danger_tint_alpha: u8,
    success_tint_alpha: u8,
}

impl ModePalette {
    const LIGHT: Self = Self {
        primary_hover: palette::PRIMARY_HOVER_LIGHT,
        background: palette::BACKGROUND_LIGHT,
        surface: palette::SURFACE_LIGHT,
        surface_2: palette::SURFACE_2_LIGHT,
        border: palette::BORDER_LIGHT,
        foreground: palette::FOREGROUND_LIGHT,
        muted: palette::MUTED_LIGHT,
        danger: palette::DANGER_LIGHT,
        success: palette::SUCCESS_LIGHT,
        danger_tint_alpha: palette::ALPHA_DANGER_TINT_LIGHT,
        success_tint_alpha: palette::ALPHA_SUCCESS_TINT_LIGHT,
    };

    const DARK: Self = Self {
        primary_hover: palette::PRIMARY_HOVER_DARK,
        background: palette::BACKGROUND_DARK,
        surface: palette::SURFACE_DARK,
        surface_2: palette::SURFACE_2_DARK,
        border: palette::BORDER_DARK,
        foreground: palette::FOREGROUND_DARK,
        muted: palette::MUTED_DARK,
        danger: palette::DANGER_DARK,
        success: palette::SUCCESS_DARK,
        danger_tint_alpha: palette::ALPHA_DANGER_TINT_DARK,
        success_tint_alpha: palette::ALPHA_SUCCESS_TINT_DARK,
    };
}

fn solid(value: u32) -> Hsla {
    Hsla::from(rgb(value))
}

fn tint(value: u32, alpha: u8) -> Hsla {
    solid(value).opacity(f32::from(alpha) / 255.0)
}

/// 6-digit hex string for a config slot.
fn hex(value: u32) -> SharedString {
    SharedString::from(format!("#{value:06X}"))
}

/// 8-digit hex string carrying an alpha channel.
fn hex_alpha(value: u32, alpha: u8) -> SharedString {
    SharedString::from(format!("#{value:06X}{alpha:02X}"))
}

impl OxideTokens {
    /// Light-mode tokens, mirroring the mock's light values.
    pub fn light() -> Self {
        Self::from_mode_palette(ModePalette::LIGHT)
    }

    /// Dark-mode tokens, mirroring the mock's dark values.
    pub fn dark() -> Self {
        Self::from_mode_palette(ModePalette::DARK)
    }

    fn from_mode_palette(mode: ModePalette) -> Self {
        Self {
            primary: solid(palette::PRIMARY),
            primary_hover: solid(mode.primary_hover),
            primary_tint: tint(palette::PRIMARY, palette::ALPHA_TINT),
            primary_ring: tint(palette::PRIMARY, palette::ALPHA_RING),
            background: solid(mode.background),
            surface: solid(mode.surface),
            surface_2: solid(mode.surface_2),
            border: solid(mode.border),
            foreground: solid(mode.foreground),
            muted: solid(mode.muted),
            danger: solid(mode.danger),
            danger_tint: tint(mode.danger, mode.danger_tint_alpha),
            success: solid(mode.success),
            success_tint: tint(mode.success, mode.success_tint_alpha),
            radius_s: 4,
        }
    }
}

/// The user's theme preference.
///
/// gpui-kit's [`ThemeMode`] has no `System` variant, so following the OS
/// appearance is resolved here from the window appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemePreference {
    /// Follow the system appearance (the default).
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    fn resolve(self, appearance: WindowAppearance) -> ThemeMode {
        match self {
            Self::System => ThemeMode::from(appearance),
            Self::Light => ThemeMode::Light,
            Self::Dark => ThemeMode::Dark,
        }
    }
}

/// Global publishing the active token set.
///
/// Installed by [`apply`]; reads happen through [`OxideTheme::tokens`].
pub struct OxideTheme(Arc<OxideTokens>);

impl Global for OxideTheme {}

impl OxideTheme {
    /// Returns the active token set. Requires a prior [`apply`] call, so a
    /// missing global here means bootstrap wiring broke — and panics.
    pub fn tokens(cx: &App) -> Arc<OxideTokens> {
        cx.global::<OxideTheme>().0.clone()
    }
}

/// Applies oxide-design to the gpui-kit theme global and publishes the active
/// token set.
///
/// Runs at bootstrap (after `gpui_kit::init`) and again whenever the
/// preference or the system appearance changes. It swaps both theme configs so
/// later mode flips pick up oxide-design automatically, and it never touches
/// focus or window state.
pub fn apply(cx: &mut App, preference: ThemePreference, appearance: WindowAppearance) {
    let mode = preference.resolve(appearance);
    {
        let theme = Theme::global_mut(cx);
        theme.light_theme = Rc::new(build_config(ThemeMode::Light));
        theme.dark_theme = Rc::new(build_config(ThemeMode::Dark));
    }
    Theme::change(mode, None, cx);
    let tokens = if mode.is_dark() {
        OxideTokens::dark()
    } else {
        OxideTokens::light()
    };
    cx.set_global(OxideTheme(Arc::new(tokens)));
}

/// Builds the gpui-kit config for one mode. Design tokens map onto the library
/// slots by role; slots with no design token (info, warning, charts) keep the
/// gpui-kit fallbacks until the design review gives them values.
fn build_config(mode: ThemeMode) -> ThemeConfig {
    let dark = mode.is_dark();
    let mut colors = ThemeConfigColors::default();
    assign_surfaces(dark, &mut colors);
    assign_brand(dark, &mut colors);
    assign_actions_and_status(dark, &mut colors);

    ThemeConfig {
        is_default: false,
        name: SharedString::from("oxide-design"),
        mode,
        font_size: None,
        font_family: None,
        mono_font_family: None,
        mono_font_size: None,
        radius: Some(6),
        radius_lg: Some(8),
        shadow: Some(false),
        colors,
        highlight: None,
    }
}

/// Neutral structure: `surface` fills containers, `surface_2` fills hover and
/// stripe slots, `border` draws every hairline, `muted` reads as muted text.
fn assign_surfaces(dark: bool, colors: &mut ThemeConfigColors) {
    let background = if dark {
        palette::BACKGROUND_DARK
    } else {
        palette::BACKGROUND_LIGHT
    };
    let surface = if dark {
        palette::SURFACE_DARK
    } else {
        palette::SURFACE_LIGHT
    };
    let surface_2 = if dark {
        palette::SURFACE_2_DARK
    } else {
        palette::SURFACE_2_LIGHT
    };
    let border = if dark {
        palette::BORDER_DARK
    } else {
        palette::BORDER_LIGHT
    };
    let foreground = if dark {
        palette::FOREGROUND_DARK
    } else {
        palette::FOREGROUND_LIGHT
    };
    let muted = if dark {
        palette::MUTED_DARK
    } else {
        palette::MUTED_LIGHT
    };

    colors.background = Some(hex(background));
    colors.foreground = Some(hex(foreground));
    colors.muted_foreground = Some(hex(muted));

    // Container fills.
    colors.accordion = Some(hex(surface));
    colors.button = Some(hex(surface));
    colors.group_box = Some(hex(surface));
    colors.list = Some(hex(surface));
    colors.popover = Some(hex(surface));
    colors.sidebar = Some(hex(surface));
    colors.tab = Some(hex(surface));
    colors.tab_bar = Some(hex(surface));
    colors.tab_bar_segmented = Some(hex(surface));
    colors.table = Some(hex(surface));
    colors.title_bar = Some(hex(surface));
    colors.status_bar = Some(hex(surface));

    // Hairlines.
    colors.border = Some(hex(border));
    colors.input = Some(hex(border));
    colors.sidebar_border = Some(hex(border));
    colors.table_row_border = Some(hex(border));
    colors.title_bar_border = Some(hex(border));
    colors.status_bar_border = Some(hex(border));
    colors.window_border = Some(hex(border));

    // Hover and stripe fills.
    colors.list_hover = Some(hex(surface_2));
    colors.list_even = Some(hex(surface_2));
    colors.list_head = Some(hex(surface_2));
    colors.table_hover = Some(hex(surface_2));
    colors.table_even = Some(hex(surface_2));
    colors.table_head = Some(hex(surface_2));
    colors.muted = Some(hex(surface_2));
    colors.skeleton = Some(hex(surface_2));
    colors.slider_bar = Some(hex(surface_2));
    colors.scrollbar = Some(hex(surface_2));
    colors.description_list_label = Some(hex(surface_2));

    // Foreground roles.
    colors.group_box_foreground = Some(hex(foreground));
    colors.group_box_title_foreground = Some(hex(muted));
    colors.popover_foreground = Some(hex(foreground));
    colors.sidebar_foreground = Some(hex(foreground));
    colors.tab_foreground = Some(hex(muted));
    colors.table_head_foreground = Some(hex(muted));
    colors.description_list_label_foreground = Some(hex(muted));
    colors.scrollbar_thumb = Some(hex(muted));
    colors.scrollbar_thumb_hover = Some(hex(muted));
}

/// The brand color: primary actions, active navigation, focus rings, and
/// selection all read from the orange accent.
fn assign_brand(dark: bool, colors: &mut ThemeConfigColors) {
    let primary_hover = if dark {
        palette::PRIMARY_HOVER_DARK
    } else {
        palette::PRIMARY_HOVER_LIGHT
    };

    colors.primary = Some(hex(palette::PRIMARY));
    colors.primary_hover = Some(hex(primary_hover));
    colors.primary_active = Some(hex(primary_hover));
    colors.primary_foreground = Some(hex(palette::ON_PRIMARY));

    // Tints and rings (alpha over the accent).
    colors.accent = Some(hex_alpha(palette::PRIMARY, palette::ALPHA_TINT));
    colors.ring = Some(hex_alpha(palette::PRIMARY, palette::ALPHA_RING));
    colors.selection = Some(hex_alpha(palette::PRIMARY, palette::ALPHA_TINT));
    colors.list_active = Some(hex_alpha(palette::PRIMARY, palette::ALPHA_TINT));
    colors.table_active = Some(hex_alpha(palette::PRIMARY, palette::ALPHA_TINT));
    colors.sidebar_accent = Some(hex_alpha(palette::PRIMARY, palette::ALPHA_TINT));
    colors.drop_target = Some(hex_alpha(palette::PRIMARY, palette::ALPHA_TINT));

    // Accent-foreground and navigation roles.
    colors.accent_foreground = Some(hex(palette::PRIMARY));
    colors.link = Some(hex(palette::PRIMARY));
    colors.caret = Some(hex(palette::PRIMARY));
    colors.list_active_border = Some(hex(palette::PRIMARY));
    colors.table_active_border = Some(hex(palette::PRIMARY));
    colors.sidebar_accent_foreground = Some(hex(palette::PRIMARY));
    colors.sidebar_primary = Some(hex(palette::PRIMARY));
    colors.sidebar_primary_foreground = Some(hex(palette::ON_PRIMARY));
    colors.drag_border = Some(hex(palette::PRIMARY));

    // Small indicators and controls.
    colors.switch = Some(hex(palette::PRIMARY));
    colors.switch_thumb = Some(hex(palette::ON_PRIMARY));
    colors.progress_bar = Some(hex(palette::PRIMARY));
    colors.slider_thumb = Some(hex(palette::PRIMARY));
    colors.tab_active = Some(hex(palette::PRIMARY));
    colors.tab_active_foreground = Some(hex(palette::ON_PRIMARY));
}

/// Action variants and the two status hues. Destructive keeps the danger hue
/// and never borrows the brand orange; verdicts read the success hue.
fn assign_actions_and_status(dark: bool, colors: &mut ThemeConfigColors) {
    let danger = if dark {
        palette::DANGER_DARK
    } else {
        palette::DANGER_LIGHT
    };
    let success = if dark {
        palette::SUCCESS_DARK
    } else {
        palette::SUCCESS_LIGHT
    };

    // Default, secondary, and primary buttons.
    colors.button_foreground = Some(hex(if dark {
        palette::FOREGROUND_DARK
    } else {
        palette::FOREGROUND_LIGHT
    }));
    colors.button_hover = Some(hex(if dark {
        palette::SURFACE_2_DARK
    } else {
        palette::SURFACE_2_LIGHT
    }));
    colors.button_active = Some(hex(if dark {
        palette::SURFACE_2_DARK
    } else {
        palette::SURFACE_2_LIGHT
    }));
    colors.button_secondary = Some(hex(if dark {
        palette::SURFACE_DARK
    } else {
        palette::SURFACE_LIGHT
    }));
    colors.button_secondary_hover = Some(hex(if dark {
        palette::SURFACE_2_DARK
    } else {
        palette::SURFACE_2_LIGHT
    }));
    colors.button_secondary_active = Some(hex(if dark {
        palette::SURFACE_2_DARK
    } else {
        palette::SURFACE_2_LIGHT
    }));
    colors.button_secondary_foreground = Some(hex(if dark {
        palette::FOREGROUND_DARK
    } else {
        palette::FOREGROUND_LIGHT
    }));
    colors.button_primary = Some(hex(palette::PRIMARY));
    colors.button_primary_hover = Some(hex(if dark {
        palette::PRIMARY_HOVER_DARK
    } else {
        palette::PRIMARY_HOVER_LIGHT
    }));
    colors.button_primary_active = Some(hex(if dark {
        palette::PRIMARY_HOVER_DARK
    } else {
        palette::PRIMARY_HOVER_LIGHT
    }));
    colors.button_primary_foreground = Some(hex(palette::ON_PRIMARY));

    // Destructive and validation verdict hues.
    colors.danger = Some(hex(danger));
    colors.danger_hover = Some(hex(danger));
    colors.danger_active = Some(hex(danger));
    colors.danger_foreground = Some(hex(danger));
    colors.button_danger = Some(hex(danger));
    colors.button_danger_hover = Some(hex(danger));
    colors.button_danger_active = Some(hex(danger));
    colors.button_danger_foreground = Some(hex(palette::ON_PRIMARY));
    colors.success = Some(hex(success));
    colors.success_hover = Some(hex(success));
    colors.success_active = Some(hex(success));
    colors.success_foreground = Some(hex(success));
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    use gpui_kit::component::ActiveTheme as _;

    fn hsla(value: u32) -> Hsla {
        solid(value)
    }

    #[test]
    fn light_tokens_match_the_mock() {
        let tokens = OxideTokens::light();
        assert_eq!(tokens.primary, hsla(0xCE_42_2B));
        assert_eq!(tokens.primary_hover, hsla(0xB8_3A_26));
        assert_eq!(tokens.primary_tint, hsla(0xCE_42_2B).opacity(23.0 / 255.0));
        assert_eq!(tokens.primary_ring, hsla(0xCE_42_2B).opacity(89.0 / 255.0));
        assert_eq!(tokens.background, hsla(0xFF_FF_FF));
        assert_eq!(tokens.surface, hsla(0xF7_F7_F8));
        assert_eq!(tokens.surface_2, hsla(0xEF_EF_F1));
        assert_eq!(tokens.border, hsla(0xE4_E5_E7));
        assert_eq!(tokens.foreground, hsla(0x17_18_1A));
        assert_eq!(tokens.muted, hsla(0x6B_6E_73));
        assert_eq!(tokens.danger, hsla(0xB9_1C_1C));
        assert_eq!(tokens.danger_tint, hsla(0xB9_1C_1C).opacity(20.0 / 255.0));
        assert_eq!(tokens.success, hsla(0x15_80_3D));
        assert_eq!(tokens.success_tint, hsla(0x15_80_3D).opacity(23.0 / 255.0));
        assert_eq!(tokens.radius_s, 4);
    }

    #[test]
    fn dark_tokens_match_the_mock() {
        let tokens = OxideTokens::dark();
        assert_eq!(tokens.primary, hsla(0xCE_42_2B));
        assert_eq!(tokens.primary_hover, hsla(0xDD_5A_3F));
        assert_eq!(tokens.background, hsla(0x0E_0E_0F));
        assert_eq!(tokens.surface, hsla(0x17_18_1A));
        assert_eq!(tokens.surface_2, hsla(0x20_22_24));
        assert_eq!(tokens.border, hsla(0x2A_2C_2F));
        assert_eq!(tokens.foreground, hsla(0xF2_F2_F3));
        assert_eq!(tokens.muted, hsla(0x8E_90_94));
        assert_eq!(tokens.danger, hsla(0xE5_48_4D));
        assert_eq!(tokens.danger_tint, hsla(0xE5_48_4D).opacity(31.0 / 255.0));
        assert_eq!(tokens.success, hsla(0x30_A4_6C));
        assert_eq!(tokens.success_tint, hsla(0x30_A4_6C).opacity(31.0 / 255.0));
    }

    #[test]
    fn config_light_carries_the_design_values() {
        let config = build_config(ThemeMode::Light);
        assert_eq!(config.name, "oxide-design");
        assert_eq!(config.mode, ThemeMode::Light);
        assert_eq!(config.radius, Some(6));
        assert_eq!(config.radius_lg, Some(8));
        assert_eq!(config.shadow, Some(false));

        let colors = &config.colors;
        assert_eq!(colors.primary.as_deref(), Some("#CE422B"));
        assert_eq!(colors.primary_hover.as_deref(), Some("#B83A26"));
        assert_eq!(colors.primary_foreground.as_deref(), Some("#FFFFFF"));
        assert_eq!(colors.background.as_deref(), Some("#FFFFFF"));
        assert_eq!(colors.sidebar.as_deref(), Some("#F7F7F8"));
        assert_eq!(colors.list_hover.as_deref(), Some("#EFEFF1"));
        assert_eq!(colors.border.as_deref(), Some("#E4E5E7"));
        assert_eq!(colors.foreground.as_deref(), Some("#17181A"));
        assert_eq!(colors.muted_foreground.as_deref(), Some("#6B6E73"));
        assert_eq!(colors.accent.as_deref(), Some("#CE422B17"));
        assert_eq!(colors.ring.as_deref(), Some("#CE422B59"));
        assert_eq!(colors.danger.as_deref(), Some("#B91C1C"));
        assert_eq!(colors.success.as_deref(), Some("#15803D"));
    }

    #[test]
    fn config_dark_carries_the_design_values() {
        let config = build_config(ThemeMode::Dark);
        assert_eq!(config.mode, ThemeMode::Dark);

        let colors = &config.colors;
        assert_eq!(colors.primary_hover.as_deref(), Some("#DD5A3F"));
        assert_eq!(colors.background.as_deref(), Some("#0E0E0F"));
        assert_eq!(colors.sidebar.as_deref(), Some("#17181A"));
        assert_eq!(colors.list_hover.as_deref(), Some("#202224"));
        assert_eq!(colors.border.as_deref(), Some("#2A2C2F"));
        assert_eq!(colors.foreground.as_deref(), Some("#F2F2F3"));
        assert_eq!(colors.muted_foreground.as_deref(), Some("#8E9094"));
        assert_eq!(colors.danger.as_deref(), Some("#E5484D"));
        assert_eq!(colors.success.as_deref(), Some("#30A46C"));
    }

    #[test]
    fn preference_resolves_system_overrides_and_vibrant_appearances() {
        use WindowAppearance::{Dark, Light, VibrantDark, VibrantLight};

        assert_eq!(ThemePreference::System.resolve(Light), ThemeMode::Light);
        assert_eq!(ThemePreference::System.resolve(Dark), ThemeMode::Dark);
        assert_eq!(ThemePreference::System.resolve(VibrantLight), ThemeMode::Light);
        assert_eq!(ThemePreference::System.resolve(VibrantDark), ThemeMode::Dark);
        assert_eq!(ThemePreference::Light.resolve(Dark), ThemeMode::Light);
        assert_eq!(ThemePreference::Dark.resolve(Light), ThemeMode::Dark);
    }

    #[gpui_kit::test]
    fn apply_switches_mode_and_publishes_tokens(cx: &mut TestAppContext) {
        cx.update(|app| {
            gpui_kit::init(app);

            apply(app, ThemePreference::System, WindowAppearance::Dark);
            assert!(app.theme().is_dark());
            assert_eq!(app.theme().primary, hsla(0xCE_42_2B));
            assert_eq!(OxideTheme::tokens(app).background, hsla(0x0E_0E_0F));

            apply(app, ThemePreference::Light, WindowAppearance::Dark);
            assert!(!app.theme().is_dark());
            assert_eq!(OxideTheme::tokens(app).background, hsla(0xFF_FF_FF));
            assert_eq!(app.theme().primary, hsla(0xCE_42_2B));
        });
    }
}

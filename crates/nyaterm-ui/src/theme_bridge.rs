//! Bridge from NyaTerm's persisted theme palette to gpui-component's theme.

use gpui::{App, Hsla, rgb};
use gpui_component::Theme;

use crate::theme::ThemePalette;

fn color(rgb_value: u32) -> Hsla {
    rgb(rgb_value).into()
}

pub fn apply_component_theme(palette: ThemePalette, cx: &mut App) {
    if !cx.has_global::<Theme>() {
        return;
    }

    let component_theme = Theme::global_mut(cx);
    component_theme.colors.background = color(palette.bg);
    component_theme.colors.foreground = color(palette.text);
    component_theme.colors.muted = color(palette.surface_elevated);
    component_theme.colors.muted_foreground = color(palette.text_muted);
    component_theme.colors.border = color(palette.border);
    component_theme.colors.input = color(palette.border);
    component_theme.colors.caret = color(palette.focus_ring);
    component_theme.colors.primary = color(palette.primary);
    component_theme.colors.primary_hover = color(palette.primary_hover);
    component_theme.colors.primary_active = color(palette.primary_hover);
    component_theme.colors.primary_foreground = color(palette.on_primary);
    component_theme.colors.secondary = color(palette.surface_elevated);
    component_theme.colors.secondary_hover = color(palette.hover);
    component_theme.colors.secondary_active = color(palette.hover);
    component_theme.colors.secondary_foreground = color(palette.text);
    component_theme.colors.danger = color(palette.danger);
    component_theme.colors.danger_hover = color(palette.danger);
    component_theme.colors.danger_active = color(palette.danger);
    component_theme.colors.danger_foreground = color(palette.on_primary);
    component_theme.colors.ring = color(palette.focus_ring);
    component_theme.colors.accent = color(palette.hover);
    component_theme.colors.accent_foreground = color(palette.text);
    component_theme.colors.popover = color(palette.surface_elevated);
    component_theme.colors.popover_foreground = color(palette.text);
    component_theme.colors.title_bar = color(palette.surface);
    component_theme.colors.title_bar_border = color(palette.border);
    component_theme.colors.selection = color(palette.terminal_selection);
    component_theme.colors.scrollbar = color(palette.bg);
    component_theme.colors.scrollbar_thumb = color(palette.border);
    component_theme.colors.scrollbar_thumb_hover = color(palette.text_dimmed);
    component_theme.colors.switch = color(palette.border);
    component_theme.colors.switch_thumb = color(palette.surface);
    component_theme.colors.tab = color(palette.input);
    component_theme.colors.tab_active = color(palette.hover);
    component_theme.colors.tab_active_foreground = color(palette.text);
    component_theme.colors.tab_foreground = color(palette.text_muted);
}

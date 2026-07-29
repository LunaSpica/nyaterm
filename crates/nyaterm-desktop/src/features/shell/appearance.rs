use std::{path::Path, sync::Arc};

use gpui::{App, Context, PathPromptOptions, SharedString, font, px, rgb, rgba};
use nyaterm_core::{
    AppSettingsSummary, ConnectionStore, ResolvedKeywordHighlightRule,
    merge_keyword_highlight_rules_for_paint,
};

use crate::features::NyaTermApp;
pub(in crate::features) use crate::theme::{ThemePalette, theme_palette};

const TERMINAL_FONT_SIZE_MIN: i16 = 8;
const TERMINAL_FONT_SIZE_MAX: i16 = 72;

impl NyaTermApp {
    pub(in crate::features) fn apply_gpui_settings(&mut self, settings: AppSettingsSummary) {
        self.settings.summary = settings;
        self.invalidate_paint_theme_caches();
    }

    pub(in crate::features) fn gpui_terminal_font_family(&self) -> String {
        gpui_platform_font_family(
            &self.settings.summary.terminal_font_family,
            gpui_terminal_font_fallback(),
            true,
        )
    }

    pub(in crate::features) fn gpui_ui_font_family(&self) -> String {
        let raw = if self.settings.summary.ui_font_family.trim().is_empty() {
            self.settings.summary.terminal_font_family.as_str()
        } else {
            self.settings.summary.ui_font_family.as_str()
        };
        gpui_platform_font_family(raw, gpui_ui_font_fallback(), false)
    }

    pub(in crate::features) fn theme_palette(&self) -> ThemePalette {
        theme_palette(&self.settings.summary.theme)
    }

    pub(in crate::features) fn wallpaper_enabled(&self) -> bool {
        self.settings
            .summary
            .background_image_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|path| !path.is_empty() && Path::new(path).is_file())
    }

    /// Wallpaper opacity applies to surface backgrounds, not their contents.
    pub(in crate::features) fn shell_surface_color(&self, color: u32) -> gpui::Rgba {
        if !self.wallpaper_enabled() {
            return rgb(color);
        }
        let alpha = ((self.settings.summary.background_content_opacity.min(100) as f32 / 100.0)
            * 255.0)
            .round() as u32;
        rgba((color << 8) | alpha.min(0xff))
    }

    /// Tauri's terminal and explicitly transparent surfaces reveal wallpaper.
    pub(in crate::features) fn shell_transparent_color(&self, color: u32) -> gpui::Rgba {
        if self.wallpaper_enabled() {
            rgba(color << 8)
        } else {
            rgb(color)
        }
    }

    pub(in crate::features) fn terminal_theme_is_dark(&self) -> bool {
        let palette = self.terminal_theme_palette();
        let r = ((palette.terminal_bg >> 16) & 0xff) as f32;
        let g = ((palette.terminal_bg >> 8) & 0xff) as f32;
        let b = (palette.terminal_bg & 0xff) as f32;
        let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        lum < 0.5
    }

    pub(in crate::features) fn resolved_keyword_highlight_rules(
        &self,
    ) -> Arc<Vec<ResolvedKeywordHighlightRule>> {
        if self.settings.summary.terminal_low_latency_mode {
            return Arc::new(Vec::new());
        }
        if let Some(cached) = self.terminal.paint.cached_keyword_highlight_rules.as_ref() {
            return cached.clone();
        }
        // Cache miss (settings path / first call without ensure): build once without storing.
        if !self.settings.keyword_config.enabled {
            return Arc::new(Vec::new());
        }
        Arc::new(merge_keyword_highlight_rules_for_paint(
            &self.settings.keyword_config.rules,
            &self.settings.keyword_config.builtin_rules,
            self.terminal_theme_is_dark(),
        ))
    }

    /// Populate paint caches used by every terminal/chrome rebuild.
    pub(in crate::features) fn ensure_paint_theme_caches(&mut self) {
        self.ensure_terminal_theme_palette_cache();
        self.ensure_keyword_highlight_rules_cache();
    }

    fn ensure_keyword_highlight_rules_cache(&mut self) {
        if self.settings.summary.terminal_low_latency_mode {
            self.terminal.paint.cached_keyword_highlight_rules = Some(Arc::new(Vec::new()));
            return;
        }
        if self.terminal.paint.cached_keyword_highlight_rules.is_some() {
            return;
        }
        let rules = if !self.settings.keyword_config.enabled {
            Arc::new(Vec::new())
        } else {
            // terminal_theme_is_dark uses palette; ensure palette first.
            self.ensure_terminal_theme_palette_cache();
            Arc::new(merge_keyword_highlight_rules_for_paint(
                &self.settings.keyword_config.rules,
                &self.settings.keyword_config.builtin_rules,
                self.terminal_theme_is_dark(),
            ))
        };
        self.terminal.paint.cached_keyword_highlight_rules = Some(rules);
    }

    /// Terminal surface palette: follows optional `terminal_theme`, else UI theme.
    pub(in crate::features) fn terminal_theme_palette(&self) -> ThemePalette {
        let ui_theme = self.settings.summary.theme.as_str();
        let terminal_theme = self
            .settings
            .summary
            .terminal_theme
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("");
        let contrast = self.settings.summary.minimum_contrast_ratio.as_str();
        if let Some((cached_ui, cached_term, cached_contrast, palette)) =
            self.terminal.paint.cached_terminal_theme_palette.as_ref()
        {
            if cached_ui == ui_theme && cached_term == terminal_theme && cached_contrast == contrast
            {
                return *palette;
            }
        }
        Self::compute_terminal_theme_palette(
            ui_theme,
            if terminal_theme.is_empty() {
                None
            } else {
                Some(terminal_theme)
            },
            contrast,
        )
    }

    fn ensure_terminal_theme_palette_cache(&mut self) {
        let ui_theme = self.settings.summary.theme.clone();
        let terminal_theme = self
            .settings
            .summary
            .terminal_theme
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        let contrast = self.settings.summary.minimum_contrast_ratio.clone();
        if let Some((cached_ui, cached_term, cached_contrast, _)) =
            self.terminal.paint.cached_terminal_theme_palette.as_ref()
        {
            if cached_ui == &ui_theme
                && cached_term == &terminal_theme
                && cached_contrast == &contrast
            {
                return;
            }
        }
        let palette = Self::compute_terminal_theme_palette(
            &ui_theme,
            if terminal_theme.is_empty() {
                None
            } else {
                Some(terminal_theme.as_str())
            },
            &contrast,
        );
        self.terminal.paint.cached_terminal_theme_palette =
            Some((ui_theme, terminal_theme, contrast, palette));
    }

    fn compute_terminal_theme_palette(
        ui_theme: &str,
        terminal_theme: Option<&str>,
        minimum_contrast_ratio: &str,
    ) -> ThemePalette {
        let id = terminal_theme.unwrap_or(ui_theme);
        let id = if id == "catppuccin" {
            "catppuccin-mocha"
        } else {
            id
        };
        let mut palette = theme_palette(id);
        palette.apply_minimum_contrast_ratio(parse_minimum_contrast_ratio(minimum_contrast_ratio));
        palette
    }

    pub(in crate::features) fn invalidate_paint_theme_caches(&mut self) {
        self.terminal.paint.cached_terminal_theme_palette = None;
        self.terminal.paint.cached_keyword_highlight_rules = None;
    }

    pub(in crate::features) fn refresh_visible_terminal_surfaces(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let ids = self
            .visible_terminal_session_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        for session_id in ids {
            self.sync_terminal_surface_paint(&session_id, cx);
        }
    }

    pub(in crate::features) fn update_appearance_theme(
        &mut self,
        theme: &str,
        cx: &mut Context<Self>,
    ) {
        // Normalize legacy Settings id "catppuccin" to Tauri mocha id.
        let theme = if theme == "catppuccin" {
            "catppuccin-mocha"
        } else {
            theme
        };
        self.settings.summary.theme = theme.to_string();
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn update_terminal_font_family(
        &mut self,
        family: &str,
        cx: &mut Context<Self>,
    ) {
        if self.settings.summary.terminal_font_family == family {
            return;
        }
        self.settings.summary.terminal_font_family = family.to_string();
        self.invalidate_terminal_cell_metrics(cx);
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn invalidate_terminal_cell_metrics(&mut self, cx: &mut Context<Self>) {
        self.terminal.layout.cell_metrics = None;
        // Refresh the measured metrics before resizing the terminal. Using the
        // font-size fallback here makes the app and surface briefly disagree;
        // that is especially visible while scrolled or dragging a selection.
        self.refresh_terminal_cell_metrics(cx);
        self.sync_terminal_cell_metrics_to_screens();
        for view in self.terminal.view.views.values_mut() {
            view.render_cache.clear();
        }
        self.resize_all_known_terminal_surfaces();
        self.refresh_visible_terminal_surfaces(cx);
    }

    pub(in crate::features) fn adjust_terminal_font_size(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.summary.terminal_font_size as i16 + delta)
            .clamp(TERMINAL_FONT_SIZE_MIN, TERMINAL_FONT_SIZE_MAX);
        if self.settings.summary.terminal_font_size == next as u16 {
            return;
        }
        self.settings.summary.terminal_font_size = next as u16;
        self.invalidate_terminal_cell_metrics(cx);
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn reset_terminal_font_size(&mut self, cx: &mut Context<Self>) {
        let default_size = AppSettingsSummary::default().terminal_font_size;
        if self.settings.summary.terminal_font_size == default_size {
            return;
        }
        self.settings.summary.terminal_font_size = default_size;
        self.invalidate_terminal_cell_metrics(cx);
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn set_cursor_style(
        &mut self,
        style: &'static str,
        cx: &mut Context<Self>,
    ) {
        let normalized = match style {
            "underline" | "bar" => style,
            _ => "block",
        };
        self.settings.summary.cursor_style = normalized.to_string();
        self.save_appearance_settings(cx);
        self.terminal.view.status = format!("cursor style → {normalized}");
    }

    pub(in crate::features) fn toggle_cursor_blink(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.cursor_blink = !self.settings.summary.cursor_blink;
        self.save_appearance_settings(cx);
        self.terminal.view.status = if self.settings.summary.cursor_blink {
            "cursor blink on".to_string()
        } else {
            "cursor blink off".to_string()
        };
    }

    pub(in crate::features) fn set_terminal_theme(
        &mut self,
        theme: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.terminal_theme =
            theme.map(str::trim).filter(|s| !s.is_empty()).map(|s| {
                if s == "catppuccin" {
                    "catppuccin-mocha".to_string()
                } else {
                    s.to_string()
                }
            });
        self.save_appearance_settings(cx);
        self.terminal.view.status = match self.settings.summary.terminal_theme.as_deref() {
            Some(id) => format!("terminal theme → {id}"),
            None => "terminal theme → follow UI".to_string(),
        };
    }

    pub(in crate::features) fn set_minimum_contrast_ratio(
        &mut self,
        ratio: &'static str,
        cx: &mut Context<Self>,
    ) {
        let ratio = match ratio {
            "3" | "4.5" | "7" | "21" => ratio,
            _ => "1",
        };
        if self.settings.summary.minimum_contrast_ratio == ratio {
            return;
        }
        self.settings.summary.minimum_contrast_ratio = ratio.to_string();
        self.save_appearance_settings(cx);
        self.terminal.view.status = format!("minimum contrast → {ratio}");
    }

    pub(in crate::features) fn update_ui_font_family(
        &mut self,
        family: &str,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.ui_font_family = family.to_string();
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn set_appearance_font_stack_entry(
        &mut self,
        terminal: bool,
        index: usize,
        family: String,
        cx: &mut Context<Self>,
    ) {
        let raw = if terminal {
            &self.settings.summary.terminal_font_family
        } else {
            &self.settings.summary.ui_font_family
        };
        let fallback = if terminal { "JetBrains Mono" } else { "Inter" };
        let mut fonts = appearance_font_stack(raw, fallback);
        let Some(font) = fonts.get_mut(index) else {
            return;
        };
        *font = family;
        self.settings.close_appearance_menu();
        self.save_appearance_font_stack(terminal, fonts, cx);
    }

    pub(in crate::features) fn add_appearance_fallback_font(
        &mut self,
        terminal: bool,
        cx: &mut Context<Self>,
    ) {
        let raw = if terminal {
            &self.settings.summary.terminal_font_family
        } else {
            &self.settings.summary.ui_font_family
        };
        let fallback = if terminal { "JetBrains Mono" } else { "Inter" };
        let mut fonts = appearance_font_stack(raw, fallback);
        let next = if terminal {
            self.settings.terminal_font_options().first()
        } else {
            self.settings.ui_font_options().first()
        }
        .cloned()
        .unwrap_or_else(|| fallback.to_string());
        fonts.push(next);
        self.save_appearance_font_stack(terminal, fonts, cx);
    }

    pub(in crate::features) fn remove_appearance_font_stack_entry(
        &mut self,
        terminal: bool,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let raw = if terminal {
            &self.settings.summary.terminal_font_family
        } else {
            &self.settings.summary.ui_font_family
        };
        let fallback = if terminal { "JetBrains Mono" } else { "Inter" };
        let mut fonts = appearance_font_stack(raw, fallback);
        if index >= fonts.len() {
            return;
        }
        fonts.remove(index);
        if fonts.is_empty() {
            fonts.push(fallback.to_string());
        }
        self.settings.close_appearance_menu();
        self.save_appearance_font_stack(terminal, fonts, cx);
    }

    fn save_appearance_font_stack(
        &mut self,
        terminal: bool,
        fonts: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        let stack = fonts.join(", ");
        if terminal {
            self.update_terminal_font_family(&stack, cx);
        } else {
            self.update_ui_font_family(&stack, cx);
        }
    }

    pub(in crate::features) fn adjust_ui_font_size(&mut self, delta: i16, cx: &mut Context<Self>) {
        let next = (self.settings.summary.ui_font_size as i16 + delta).clamp(12, 24) as u16;
        self.settings.summary.ui_font_size = next;
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn set_terminal_font_weight(
        &mut self,
        weight: u16,
        cx: &mut Context<Self>,
    ) {
        let weight = match weight {
            300 | 400 | 500 | 600 | 700 | 800 => weight,
            _ => 400,
        };
        if self.settings.summary.terminal_font_weight == weight {
            return;
        }
        self.settings.summary.terminal_font_weight = weight;
        self.invalidate_terminal_cell_metrics(cx);
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn set_terminal_font_weight_bold(
        &mut self,
        weight: u16,
        cx: &mut Context<Self>,
    ) {
        let weight = match weight {
            300 | 400 | 500 | 600 | 700 | 800 => weight,
            _ => 700,
        };
        if self.settings.summary.terminal_font_weight_bold == weight {
            return;
        }
        self.settings.summary.terminal_font_weight_bold = weight;
        self.invalidate_terminal_cell_metrics(cx);
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn zoom_terminal_in(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(1, cx);
    }

    pub(in crate::features) fn zoom_terminal_out(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(-1, cx);
    }

    pub(in crate::features) fn prompt_background_image(&mut self, cx: &mut Context<Self>) {
        if self.settings.summary.background_image_path.is_some() {
            // allow replace
        }
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(
                self.tr("settings.selectBackgroundImage"),
            )),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal.view.status = "selecting wallpaper image".to_string();
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = path {
                    this.settings.summary.background_image_path = Some(path.display().to_string());
                    if this.settings.summary.background_image_fit.trim().is_empty() {
                        this.settings.summary.background_image_fit = "cover".to_string();
                    }
                    this.save_appearance_settings(cx);
                    this.terminal.view.status = "wallpaper image selected".to_string();
                } else {
                    this.terminal.view.status = "wallpaper selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn clear_background_image(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.background_image_path = None;
        self.settings.close_appearance_menu();
        self.save_appearance_settings(cx);
        self.terminal.view.status = "wallpaper cleared".to_string();
    }

    pub(in crate::features) fn set_background_image_fit(
        &mut self,
        fit: &'static str,
        cx: &mut Context<Self>,
    ) {
        let normalized = match fit {
            "contain" => "contain",
            "stretch" | "fill" => "stretch",
            "tile" => "tile",
            _ => "cover",
        };
        self.settings.summary.background_image_fit = normalized.to_string();
        self.save_appearance_settings(cx);
        self.terminal.view.status = format!("wallpaper fit → {normalized}");
    }

    pub(in crate::features) fn set_background_image_opacity(
        &mut self,
        value: u8,
        cx: &mut Context<Self>,
    ) {
        let next = value.min(100);
        if self.settings.summary.background_image_opacity == next {
            return;
        }
        self.settings.summary.background_image_opacity = next;
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn set_background_content_opacity(
        &mut self,
        value: u8,
        cx: &mut Context<Self>,
    ) {
        let next = value.min(100);
        if self.settings.summary.background_content_opacity == next {
            return;
        }
        self.settings.summary.background_content_opacity = next;
        self.save_appearance_settings(cx);
    }

    fn save_appearance_settings(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            self.refresh_visible_terminal_surfaces(cx);
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_appearance_settings(&self.settings.summary))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.refresh_visible_terminal_surfaces(cx);
                self.settings
                    .set_store_message("appearance settings saved".to_string());
                self.settings.set_store_ready(true);
                self.terminal.view.status = "appearance settings saved".to_string();
            }
            Err(error) => {
                self.settings
                    .set_store_message(format!("appearance settings save failed: {error}"));
                self.settings.set_store_ready(false);
                self.terminal.view.status = self.settings.store_status().message.to_string();
            }
        }
        cx.notify();
    }
}

pub(in crate::features) fn appearance_font_stack(raw: &str, fallback: &str) -> Vec<String> {
    let fonts = raw
        .split(',')
        .map(trim_gpui_font_family)
        .filter(|family| !family.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if fonts.is_empty() {
        vec![fallback.to_string()]
    } else {
        fonts
    }
}

pub(in crate::features) fn appearance_font_options(cx: &App) -> (Vec<String>, Vec<String>) {
    let text_system = cx.text_system();
    let system_fonts = text_system.all_font_names();
    let mut ui_fonts = Vec::new();
    let mut terminal_fonts = Vec::new();

    for family in ["JetBrains Mono", "Noto Sans SC Variable", "Inter"] {
        push_unique_font(&mut ui_fonts, family.to_string());
    }
    for family in &system_fonts {
        push_unique_font(&mut ui_fonts, family.clone());
    }

    push_unique_font(&mut terminal_fonts, "JetBrains Mono".to_string());
    for family in &system_fonts {
        let font_id = text_system.resolve_font(&font(SharedString::from(family.clone())));
        let font_size = px(14.);
        let widths = ['i', 'W', '0']
            .into_iter()
            .filter_map(|ch| text_system.advance(font_id, font_size, ch).ok())
            .map(|advance| f32::from(advance.width))
            .collect::<Vec<_>>();
        let monospace = widths.len() == 3
            && widths
                .iter()
                .all(|width| (*width - widths[0]).abs() <= 0.05);
        if monospace {
            push_unique_font(&mut terminal_fonts, family.clone());
        }
    }
    push_unique_font(&mut terminal_fonts, "monospace".to_string());

    (ui_fonts, terminal_fonts)
}

fn push_unique_font(fonts: &mut Vec<String>, family: String) {
    if !fonts
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&family))
    {
        fonts.push(family);
    }
}

fn parse_minimum_contrast_ratio(raw: &str) -> f32 {
    match raw.trim() {
        "3" => 3.0,
        "4.5" => 4.5,
        "7" => 7.0,
        "21" => 21.0,
        _ => 1.0,
    }
}

fn gpui_platform_font_family(raw: &str, fallback: &str, monospace: bool) -> String {
    gpui_platform_font_family_for_target(raw, fallback, monospace, cfg!(target_os = "windows"))
}

fn gpui_platform_font_family_for_target(
    raw: &str,
    fallback: &str,
    monospace: bool,
    is_windows: bool,
) -> String {
    let primary = raw
        .split(',')
        .map(trim_gpui_font_family)
        .find(|family| !family.is_empty())
        .unwrap_or(fallback);
    if is_windows && windows_gpui_font_should_fallback(primary, monospace) {
        fallback.to_string()
    } else {
        primary.to_string()
    }
}

fn trim_gpui_font_family(value: &str) -> &str {
    value.trim().trim_matches(|ch| ch == '"' || ch == '\'')
}

fn gpui_terminal_font_fallback() -> &'static str {
    if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "monospace"
    }
}

pub(in crate::features) fn gpui_code_font_family() -> &'static str {
    if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "JetBrains Mono"
    }
}

fn gpui_ui_font_fallback() -> &'static str {
    if cfg!(target_os = "windows") {
        "Microsoft YaHei UI"
    } else {
        "system-ui"
    }
}

fn windows_gpui_font_should_fallback(family: &str, monospace: bool) -> bool {
    if matches!(family, "monospace" | "system-ui" | "sans-serif") {
        return true;
    }
    if monospace {
        matches!(
            family,
            "JetBrains Mono"
                | "Fira Code"
                | "FiraCode Nerd Font Mono"
                | "Iosevka"
                | "Maple Mono CN"
        )
    } else {
        matches!(
            family,
            "Inter" | "JetBrains Mono" | "Noto Sans SC Variable" | "微软雅黑"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        appearance_font_stack, gpui_code_font_family, gpui_platform_font_family_for_target,
    };

    #[test]
    fn windows_terminal_font_family_uses_primary_stack_entry() {
        assert_eq!(
            gpui_platform_font_family_for_target("Cascadia Mono, Consolas", "Consolas", true, true,),
            "Cascadia Mono"
        );
    }

    #[test]
    fn windows_ui_font_family_collapses_known_missing_stack() {
        assert_eq!(
            gpui_platform_font_family_for_target(
                "JetBrains Mono, Noto Sans SC Variable, 微软雅黑",
                "Microsoft YaHei UI",
                false,
                true,
            ),
            "Microsoft YaHei UI"
        );
    }

    #[test]
    fn windows_code_font_uses_installed_platform_default() {
        if cfg!(target_os = "windows") {
            assert_eq!(gpui_code_font_family(), "Consolas");
        }
    }

    #[test]
    fn appearance_font_stack_preserves_fallback_order() {
        assert_eq!(
            appearance_font_stack("JetBrains Mono, Noto Sans SC Variable, Inter", "Inter"),
            vec!["JetBrains Mono", "Noto Sans SC Variable", "Inter"]
        );
    }

    #[test]
    fn non_windows_font_family_uses_first_family() {
        assert_eq!(
            gpui_platform_font_family_for_target(
                "JetBrains Mono, monospace",
                "monospace",
                true,
                false,
            ),
            "JetBrains Mono"
        );
    }
}

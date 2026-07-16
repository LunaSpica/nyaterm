use super::*;

pub(in crate::features) use crate::theme::{ThemePalette, theme_palette};

const TERMINAL_FONT_SIZE_MIN: i16 = 8;
const TERMINAL_FONT_SIZE_MAX: i16 = 72;

impl NyaTermApp {
    pub(in crate::features) fn apply_gpui_settings(&mut self, mut settings: AppSettingsSummary) {
        normalize_gpui_font_settings_for_platform(&mut settings);
        self.settings = settings;
        self.invalidate_paint_theme_caches();
    }

    pub(in crate::features) fn gpui_terminal_font_family(&self) -> String {
        gpui_platform_font_family(
            &self.settings.terminal_font_family,
            gpui_terminal_font_fallback(),
            true,
        )
    }

    pub(in crate::features) fn gpui_ui_font_family(&self) -> String {
        let raw = if self.settings.ui_font_family.trim().is_empty() {
            self.settings.terminal_font_family.as_str()
        } else {
            self.settings.ui_font_family.as_str()
        };
        gpui_platform_font_family(raw, gpui_ui_font_fallback(), false)
    }

    pub(in crate::features) fn theme_palette(&self) -> ThemePalette {
        theme_palette(&self.settings.theme)
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
    ) -> std::sync::Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>> {
        if let Some(cached) = self.cached_keyword_highlight_rules.as_ref() {
            return cached.clone();
        }
        // Cache miss (settings path / first call without ensure): build once without storing.
        if !self.keyword_highlights.enabled {
            return std::sync::Arc::new(Vec::new());
        }
        std::sync::Arc::new(nyaterm_core::merge_keyword_highlight_rules_for_paint(
            &self.keyword_highlights.rules,
            &self.keyword_highlights.builtin_rules,
            self.terminal_theme_is_dark(),
        ))
    }

    /// Populate paint caches used by every terminal/chrome rebuild.
    pub(in crate::features) fn ensure_paint_theme_caches(&mut self) {
        self.ensure_terminal_theme_palette_cache();
        self.ensure_keyword_highlight_rules_cache();
    }

    fn ensure_keyword_highlight_rules_cache(&mut self) {
        if self.cached_keyword_highlight_rules.is_some() {
            return;
        }
        let rules = if !self.keyword_highlights.enabled {
            std::sync::Arc::new(Vec::new())
        } else {
            // terminal_theme_is_dark uses palette; ensure palette first.
            self.ensure_terminal_theme_palette_cache();
            std::sync::Arc::new(nyaterm_core::merge_keyword_highlight_rules_for_paint(
                &self.keyword_highlights.rules,
                &self.keyword_highlights.builtin_rules,
                self.terminal_theme_is_dark(),
            ))
        };
        self.cached_keyword_highlight_rules = Some(rules);
    }

    /// Terminal surface palette: follows optional `terminal_theme`, else UI theme.
    pub(in crate::features) fn terminal_theme_palette(&self) -> ThemePalette {
        let ui_theme = self.settings.theme.as_str();
        let terminal_theme = self
            .settings
            .terminal_theme
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("");
        let contrast = self.settings.minimum_contrast_ratio.as_str();
        if let Some((cached_ui, cached_term, cached_contrast, palette)) =
            self.cached_terminal_theme_palette.as_ref()
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
        let ui_theme = self.settings.theme.clone();
        let terminal_theme = self
            .settings
            .terminal_theme
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        let contrast = self.settings.minimum_contrast_ratio.clone();
        if let Some((cached_ui, cached_term, cached_contrast, _)) =
            self.cached_terminal_theme_palette.as_ref()
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
        self.cached_terminal_theme_palette = Some((ui_theme, terminal_theme, contrast, palette));
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
        self.cached_terminal_theme_palette = None;
        self.cached_keyword_highlight_rules = None;
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
        self.settings.theme = theme.to_string();
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn update_terminal_font_family(
        &mut self,
        family: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self.settings.terminal_font_family == family {
            return;
        }
        self.settings.terminal_font_family = family.to_string();
        self.invalidate_terminal_cell_metrics();
        self.save_appearance_settings(cx);
    }

    fn invalidate_terminal_cell_metrics(&mut self) {
        self.terminal_cell_metrics = None;
        self.sync_terminal_cell_metrics_to_screens();
        self.resize_all_known_terminal_surfaces();
    }

    pub(in crate::features) fn adjust_terminal_font_size(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.terminal_font_size as i16 + delta)
            .clamp(TERMINAL_FONT_SIZE_MIN, TERMINAL_FONT_SIZE_MAX);
        if self.settings.terminal_font_size == next as u16 {
            return;
        }
        self.settings.terminal_font_size = next as u16;
        self.invalidate_terminal_cell_metrics();
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn reset_terminal_font_size(&mut self, cx: &mut Context<Self>) {
        let default_size = AppSettingsSummary::default().terminal_font_size;
        if self.settings.terminal_font_size == default_size {
            return;
        }
        self.settings.terminal_font_size = default_size;
        self.invalidate_terminal_cell_metrics();
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
        self.settings.cursor_style = normalized.to_string();
        self.save_appearance_settings(cx);
        self.terminal_status = format!("cursor style → {normalized}");
    }

    pub(in crate::features) fn toggle_cursor_blink(&mut self, cx: &mut Context<Self>) {
        self.settings.cursor_blink = !self.settings.cursor_blink;
        self.save_appearance_settings(cx);
        self.terminal_status = if self.settings.cursor_blink {
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
        self.settings.terminal_theme = theme.map(str::trim).filter(|s| !s.is_empty()).map(|s| {
            if s == "catppuccin" {
                "catppuccin-mocha".to_string()
            } else {
                s.to_string()
            }
        });
        self.save_appearance_settings(cx);
        self.terminal_status = match self.settings.terminal_theme.as_deref() {
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
        if self.settings.minimum_contrast_ratio == ratio {
            return;
        }
        self.settings.minimum_contrast_ratio = ratio.to_string();
        self.save_appearance_settings(cx);
        self.terminal_status = format!("minimum contrast → {ratio}");
    }

    pub(in crate::features) fn update_ui_font_family(
        &mut self,
        family: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.ui_font_family = family.to_string();
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn adjust_ui_font_size(&mut self, delta: i16, cx: &mut Context<Self>) {
        let next = (self.settings.ui_font_size as i16 + delta).clamp(12, 24) as u16;
        self.settings.ui_font_size = next;
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
        if self.settings.terminal_font_weight == weight {
            return;
        }
        self.settings.terminal_font_weight = weight;
        self.invalidate_terminal_cell_metrics();
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
        if self.settings.terminal_font_weight_bold == weight {
            return;
        }
        self.settings.terminal_font_weight_bold = weight;
        self.invalidate_terminal_cell_metrics();
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn zoom_terminal_in(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(1, cx);
    }

    pub(in crate::features) fn zoom_terminal_out(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(-1, cx);
    }

    pub(in crate::features) fn prompt_background_image(&mut self, cx: &mut Context<Self>) {
        if self.settings.background_image_path.is_some() {
            // allow replace
        }
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select wallpaper image")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal_status = "selecting wallpaper image".to_string();
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = path {
                    this.settings.background_image_path = Some(path.display().to_string());
                    if this.settings.background_image_fit.trim().is_empty() {
                        this.settings.background_image_fit = "cover".to_string();
                    }
                    this.save_appearance_settings(cx);
                    this.terminal_status = "wallpaper image selected".to_string();
                } else {
                    this.terminal_status = "wallpaper selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn clear_background_image(&mut self, cx: &mut Context<Self>) {
        self.settings.background_image_path = None;
        self.save_appearance_settings(cx);
        self.terminal_status = "wallpaper cleared".to_string();
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
        self.settings.background_image_fit = normalized.to_string();
        self.save_appearance_settings(cx);
        self.terminal_status = format!("wallpaper fit → {normalized}");
    }

    pub(in crate::features) fn adjust_background_image_opacity(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.background_image_opacity as i16 + delta).clamp(5, 100) as u8;
        self.settings.background_image_opacity = next;
        self.save_appearance_settings(cx);
    }

    pub(in crate::features) fn adjust_background_content_opacity(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.background_content_opacity as i16 + delta).clamp(20, 100) as u8;
        self.settings.background_content_opacity = next;
        self.save_appearance_settings(cx);
    }

    fn save_appearance_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_appearance_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.store_status.message = "appearance settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "appearance settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("appearance settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }
}

pub(in crate::features) fn normalize_gpui_font_settings_for_platform(
    settings: &mut AppSettingsSummary,
) {
    normalize_gpui_font_settings_for_target(settings, cfg!(target_os = "windows"));
}

fn normalize_gpui_font_settings_for_target(settings: &mut AppSettingsSummary, is_windows: bool) {
    settings.terminal_font_family = gpui_platform_font_family_for_target(
        &settings.terminal_font_family,
        gpui_terminal_font_fallback(),
        true,
        is_windows,
    );
    settings.ui_font_family = gpui_platform_font_family_for_target(
        &settings.ui_font_family,
        gpui_ui_font_fallback(),
        false,
        is_windows,
    );
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
    if is_windows && (raw.contains(',') || windows_gpui_font_should_fallback(primary, monospace)) {
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
    use super::*;

    #[test]
    fn windows_terminal_font_family_collapses_comma_stack() {
        assert_eq!(
            gpui_platform_font_family_for_target(
                "FiraCode Nerd Font Mono, Maple Mono CN",
                "Consolas",
                true,
                true,
            ),
            "Consolas"
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
    fn windows_font_settings_are_normalized_before_gpui_render() {
        let mut settings = AppSettingsSummary {
            terminal_font_family: "FiraCode Nerd Font Mono, Maple Mono CN".to_string(),
            ui_font_family: "JetBrains Mono, Noto Sans SC Variable, 微软雅黑".to_string(),
            ..AppSettingsSummary::default()
        };

        normalize_gpui_font_settings_for_target(&mut settings, true);

        assert_eq!(settings.terminal_font_family, "Consolas");
        assert_eq!(settings.ui_font_family, "Microsoft YaHei UI");
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

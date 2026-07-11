use super::*;

pub(in crate::ui::view) use crate::ui::theme::{ThemePalette, theme_palette};

const TERMINAL_FONT_SIZE_MIN: i16 = 8;
const TERMINAL_FONT_SIZE_MAX: i16 = 72;



impl NyaTermApp {
    pub(in crate::ui::view) fn theme_palette(&self) -> ThemePalette {
        theme_palette(&self.settings.theme)
    }

    pub(in crate::ui::view) fn terminal_theme_is_dark(&self) -> bool {
        let palette = self.terminal_theme_palette();
        let r = ((palette.terminal_bg >> 16) & 0xff) as f32;
        let g = ((palette.terminal_bg >> 8) & 0xff) as f32;
        let b = (palette.terminal_bg & 0xff) as f32;
        let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        lum < 0.5
    }

    pub(in crate::ui::view) fn resolved_keyword_highlight_rules(
        &self,
    ) -> Vec<nyaterm_domain::ResolvedKeywordHighlightRule> {
        if !self.keyword_highlights.enabled {
            return Vec::new();
        }
        nyaterm_domain::merge_keyword_highlight_rules_for_paint(
            &self.keyword_highlights.rules,
            &self.keyword_highlights.builtin_rules,
            self.terminal_theme_is_dark(),
        )
    }

    /// Terminal surface palette: follows optional `terminal_theme`, else UI theme.
    pub(in crate::ui::view) fn terminal_theme_palette(&self) -> ThemePalette {
        let id = self
            .settings
            .terminal_theme
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(self.settings.theme.as_str());
        let id = if id == "catppuccin" {
            "catppuccin-mocha"
        } else {
            id
        };
        let mut palette = theme_palette(id);
        palette.apply_minimum_contrast_ratio(
            parse_minimum_contrast_ratio(&self.settings.minimum_contrast_ratio),
        );
        palette
    }

    pub(in crate::ui::view) fn update_appearance_theme(
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

    pub(in crate::ui::view) fn update_terminal_font_family(
        &mut self,
        family: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.terminal_font_family = family.to_string();
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_terminal_font_size(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.terminal_font_size as i16 + delta)
            .clamp(TERMINAL_FONT_SIZE_MIN, TERMINAL_FONT_SIZE_MAX);
        self.settings.terminal_font_size = next as u16;
        self.terminal_cell_metrics = None;
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn reset_terminal_font_size(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_font_size = AppSettingsSummary::default().terminal_font_size;
        self.terminal_cell_metrics = None;
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn set_cursor_style(
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

    pub(in crate::ui::view) fn toggle_cursor_blink(&mut self, cx: &mut Context<Self>) {
        self.settings.cursor_blink = !self.settings.cursor_blink;
        self.save_appearance_settings(cx);
        self.terminal_status = if self.settings.cursor_blink {
            "cursor blink on".to_string()
        } else {
            "cursor blink off".to_string()
        };
    }

    pub(in crate::ui::view) fn set_terminal_theme(
        &mut self,
        theme: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.settings.terminal_theme = theme
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
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

    pub(in crate::ui::view) fn set_minimum_contrast_ratio(
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

    pub(in crate::ui::view) fn update_ui_font_family(
        &mut self,
        family: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.ui_font_family = family.to_string();
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_ui_font_size(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.ui_font_size as i16 + delta).clamp(12, 24) as u16;
        self.settings.ui_font_size = next;
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn set_terminal_font_weight(
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
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn set_terminal_font_weight_bold(
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
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn zoom_terminal_in(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(1, cx);
    }

    pub(in crate::ui::view) fn zoom_terminal_out(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(-1, cx);
    }


    pub(in crate::ui::view) fn prompt_background_image(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn clear_background_image(&mut self, cx: &mut Context<Self>) {
        self.settings.background_image_path = None;
        self.save_appearance_settings(cx);
        self.terminal_status = "wallpaper cleared".to_string();
    }

    pub(in crate::ui::view) fn set_background_image_fit(
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

    pub(in crate::ui::view) fn adjust_background_image_opacity(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.background_image_opacity as i16 + delta).clamp(5, 100) as u8;
        self.settings.background_image_opacity = next;
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_background_content_opacity(
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
                self.settings = settings;
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


fn parse_minimum_contrast_ratio(raw: &str) -> f32 {
    match raw.trim() {
        "3" => 3.0,
        "4.5" => 4.5,
        "7" => 7.0,
        "21" => 21.0,
        _ => 1.0,
    }
}

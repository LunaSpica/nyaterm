use super::*;

pub(in crate::ui::view) use crate::ui::theme::{ThemePalette, theme_palette};

const TERMINAL_FONT_SIZE_MIN: i16 = 8;
const TERMINAL_FONT_SIZE_MAX: i16 = 72;



impl NyaTermApp {
    pub(in crate::ui::view) fn theme_palette(&self) -> ThemePalette {
        theme_palette(&self.settings.theme)
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

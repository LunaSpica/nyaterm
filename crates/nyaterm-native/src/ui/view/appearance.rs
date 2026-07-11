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
        theme: &'static str,
        cx: &mut Context<Self>,
    ) {
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
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn reset_terminal_font_size(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_font_size = AppSettingsSummary::default().terminal_font_size;
        self.save_appearance_settings(cx);
    }

    pub(in crate::ui::view) fn zoom_terminal_in(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(1, cx);
    }

    pub(in crate::ui::view) fn zoom_terminal_out(&mut self, cx: &mut Context<Self>) {
        self.adjust_terminal_font_size(-1, cx);
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

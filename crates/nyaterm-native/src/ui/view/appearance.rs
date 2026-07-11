use super::*;

const TERMINAL_FONT_SIZE_MIN: i16 = 8;
const TERMINAL_FONT_SIZE_MAX: i16 = 72;


/// Shell chrome palette keyed by appearance.theme (Tauri theme ids).
/// Field names map to Tauri `ThemeColors` / CSS vars (`--df-*`).
#[derive(Debug, Clone, Copy)]
pub(in crate::ui::view) struct ThemePalette {
    pub bg: u32,
    pub surface: u32,
    pub surface_elevated: u32,
    pub section_header: u32,
    pub hover: u32,
    pub input: u32,
    pub border: u32,
    pub text: u32,
    pub text_muted: u32,
    pub text_dimmed: u32,
    pub accent: u32,
    pub success: u32,
    pub warning: u32,
    pub danger: u32,
}

pub(in crate::ui::view) fn theme_palette(theme: &str) -> ThemePalette {
    match theme {
        "github-light" => ThemePalette {
            bg: 0xffffff,
            surface: 0xf2f5f8,
            surface_elevated: 0xffffff,
            section_header: 0xeef3f8,
            hover: 0xe6edf5,
            input: 0xf7f9fb,
            border: 0xc3ccd6,
            text: 0x1f2328,
            text_muted: 0x656d76,
            text_dimmed: 0x8b949e,
            accent: 0x0969da,
            success: 0x1a7f37,
            warning: 0x9a6700,
            danger: 0xcf222e,
        },
        // Settings id "catppuccin" maps to Tauri catppuccin-mocha.
        "catppuccin" | "catppuccin-mocha" => ThemePalette {
            bg: 0x221827,
            surface: 0x1a1420,
            surface_elevated: 0x33263c,
            section_header: 0x24192b,
            hover: 0x33263c,
            input: 0x1d1624,
            border: 0x4d3a57,
            text: 0xcdd6f4,
            text_muted: 0xa6adc8,
            text_dimmed: 0x6c7086,
            accent: 0xcba6f7,
            success: 0xa6e3a1,
            warning: 0xf9e2af,
            danger: 0xf38ba8,
        },
        // github-dark + unknown
        _ => ThemePalette {
            bg: 0x0d1117,
            surface: 0x161b22,
            surface_elevated: 0x21262d,
            section_header: 0x12171f,
            hover: 0x1c2128,
            input: 0x0d1117,
            border: 0x30363d,
            text: 0xc9d1d9,
            text_muted: 0x8b949e,
            text_dimmed: 0x6e7681,
            accent: 0x58a6ff,
            success: 0x3fb950,
            warning: 0xd29922,
            danger: 0xff7b72,
        },
    }
}

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

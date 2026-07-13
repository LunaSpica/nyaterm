use super::*;

impl NyaTermApp {
    pub(in crate::features) fn appearance_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let font_size_label = format!("{} px", self.settings.terminal_font_size);
        let ui_font_size_label = format!("{} px", self.settings.ui_font_size);
        let terminal_theme = self.settings.terminal_theme.clone().unwrap_or_default();
        let follow_ui = terminal_theme.trim().is_empty();
        let contrast = self.settings.minimum_contrast_ratio.clone();
        let ui_font = self.settings.ui_font_family.clone();
        let font_weight = self.settings.terminal_font_weight;
        let font_weight_bold = self.settings.terminal_font_weight_bold;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Theme"),
                Some("UI theme, optional terminal theme override, contrast, and panel layout."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "UI theme",
                        Some(SharedString::from(
                            "Color scheme used by the native shell chrome (Tauri theme list).",
                        )),
                        {
                            let current = self.settings.theme.clone();
                            APPEARANCE_THEME_IDS.iter().fold(
                                div().flex().flex_wrap().gap_1().max_w(px(520.)),
                                |row, theme_id| {
                                    let theme_id_owned = (*theme_id).to_string();
                                    let selected = current == *theme_id
                                        || (current == "catppuccin"
                                            && *theme_id == "catppuccin-mocha");
                                    row.child(settings_choice_chip(
                                        palette,
                                        format!("appearance-theme-{theme_id}"),
                                        appearance_theme_label(theme_id),
                                        selected,
                                        cx.listener(move |this, _, _, cx| {
                                            this.update_appearance_theme(&theme_id_owned, cx);
                                        }),
                                    ))
                                },
                            )
                        },
                    ))
                    .child(settings_form_row(
                        palette,
                        "Terminal theme",
                        Some(SharedString::from(
                            "Override terminal colors, or follow the UI theme.",
                        )),
                        {
                            let mut row = div().flex().flex_wrap().gap_1().max_w(px(520.)).child(
                                settings_choice_chip(
                                    palette,
                                    "appearance-term-theme-follow",
                                    "Follow UI",
                                    follow_ui,
                                    cx.listener(|this, _, _, cx| {
                                        this.set_terminal_theme(None, cx);
                                    }),
                                ),
                            );
                            for theme_id in APPEARANCE_THEME_IDS {
                                let theme_id_owned = (*theme_id).to_string();
                                let selected = !follow_ui
                                    && (terminal_theme == *theme_id
                                        || (terminal_theme == "catppuccin"
                                            && *theme_id == "catppuccin-mocha"));
                                row = row.child(settings_choice_chip(
                                    palette,
                                    format!("appearance-term-theme-{theme_id}"),
                                    appearance_theme_label(theme_id),
                                    selected,
                                    cx.listener(move |this, _, _, cx| {
                                        this.set_terminal_theme(Some(&theme_id_owned), cx);
                                    }),
                                ));
                            }
                            row
                        },
                    ))
                    .child(settings_form_row(
                        palette,
                        "Minimum contrast",
                        Some(SharedString::from(
                            "Boost terminal fg/ANSI contrast against the terminal background.",
                        )),
                        div().flex().flex_wrap().gap_1().children(
                            ["1", "3", "4.5", "7", "21"].into_iter().map(|ratio| {
                                let selected = contrast == ratio;
                                let id = format!("appearance-contrast-{ratio}");
                                let label: &'static str = match ratio {
                                    "1" => "Off",
                                    "3" => "3:1",
                                    "4.5" => "4.5:1",
                                    "7" => "7:1",
                                    _ => "21:1",
                                };
                                settings_choice_chip(
                                    palette,
                                    id,
                                    label,
                                    selected,
                                    cx.listener(move |this, _, _, cx| {
                                        this.set_minimum_contrast_ratio(ratio, cx);
                                    }),
                                )
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Multi-open panels",
                        Some(SharedString::from(
                            "Stack several left/right panels instead of replacing the active one.",
                        )),
                        settings_switch(
                            palette,
                            "appearance-panel-multi-open",
                            self.panel_multi_open,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_panel_multi_open(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Background image"),
                Some("Optional shell wallpaper (Tauri Appearance background image)."),
                {
                    let path_label = self
                        .settings
                        .background_image_path
                        .as_deref()
                        .map(|p| {
                            if p.chars().count() > 56 {
                                format!(
                                    "…{}",
                                    p.chars()
                                        .rev()
                                        .take(52)
                                        .collect::<String>()
                                        .chars()
                                        .rev()
                                        .collect::<String>()
                                )
                            } else {
                                p.to_string()
                            }
                        })
                        .unwrap_or_else(|| "No image selected".to_string());
                    let has_image = self.settings.background_image_path.is_some();
                    let image_opacity = format!("{}%", self.settings.background_image_opacity);
                    let content_opacity = format!("{}%", self.settings.background_content_opacity);
                    let fit = self.settings.background_image_fit.clone();
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(settings_form_row(
                            palette,
                            "Image",
                            Some(SharedString::from(path_label)),
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    "appearance-wallpaper-browse",
                                    "Browse",
                                    cx.listener(|this, _, _, cx| {
                                        this.prompt_background_image(cx);
                                    }),
                                ))
                                .when(has_image, |this| {
                                    this.child(small_button(
                                        palette,
                                        "appearance-wallpaper-clear",
                                        "Clear",
                                        cx.listener(|this, _, _, cx| {
                                            this.clear_background_image(cx);
                                        }),
                                    ))
                                }),
                        ))
                        .child(settings_form_row(
                            palette,
                            "Fit",
                            Some(SharedString::from(
                                "How the wallpaper is scaled in the shell.",
                            )),
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_1()
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-cover",
                                    "Cover",
                                    fit == "cover",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("cover", cx);
                                    }),
                                ))
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-contain",
                                    "Contain",
                                    fit == "contain",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("contain", cx);
                                    }),
                                ))
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-stretch",
                                    "Stretch",
                                    fit == "stretch" || fit == "fill",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("stretch", cx);
                                    }),
                                ))
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-tile",
                                    "Tile",
                                    fit == "tile",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("tile", cx);
                                    }),
                                )),
                        ))
                        .child(settings_form_row(
                            palette,
                            "Image opacity",
                            Some(SharedString::from(
                                "Wallpaper strength over the theme background.",
                            )),
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    "appearance-wallpaper-opacity-dec",
                                    "−",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_image_opacity(-5, cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .min_w(px(48.))
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text))
                                        .child(image_opacity),
                                )
                                .child(small_button(
                                    palette,
                                    "appearance-wallpaper-opacity-inc",
                                    "+",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_image_opacity(5, cx);
                                    }),
                                )),
                        ))
                        .child(settings_form_row(
                            palette,
                            "Content opacity",
                            Some(SharedString::from(
                                "How solid chrome stays when a wallpaper is active.",
                            )),
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    "appearance-content-opacity-dec",
                                    "−",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_content_opacity(-5, cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .min_w(px(48.))
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text))
                                        .child(content_opacity),
                                )
                                .child(small_button(
                                    palette,
                                    "appearance-content-opacity-inc",
                                    "+",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_content_opacity(5, cx);
                                    }),
                                )),
                        ))
                },
            ))
            .child(settings_form_section(
                palette,
                Some("UI font"),
                Some("Chrome font family and size (Tauri ui_font_*)."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "UI font family",
                        Some(SharedString::from(
                            "Sans face for menus, settings, and chrome.",
                        )),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "appearance-ui-font-inter",
                                "Inter",
                                ui_font == "Inter" || ui_font.is_empty(),
                                cx.listener(|this, _, _, cx| {
                                    this.update_ui_font_family("Inter", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "appearance-ui-font-system",
                                "System UI",
                                ui_font == "system-ui" || ui_font == "System UI",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ui_font_family("system-ui", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "appearance-ui-font-segoe",
                                "Segoe UI",
                                ui_font == "Segoe UI",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ui_font_family("Segoe UI", cx);
                                }),
                            ))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(if ui_font.is_empty() {
                                        "Inter".to_string()
                                    } else {
                                        ui_font
                                    }),
                            ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "UI font size",
                        Some(SharedString::from("Chrome text size (12–24 px).")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "appearance-ui-font-minus",
                                "−",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ui_font_size(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(48.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text))
                                    .child(ui_font_size_label),
                            )
                            .child(small_button(
                                palette,
                                "appearance-ui-font-plus",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ui_font_size(1, cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Terminal font"),
                Some("Monospace face, size, and weights for the GPUI terminal surface."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Font family",
                        Some(SharedString::from(
                            "Monospace face used by the GPUI terminal surface.",
                        )),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "appearance-font-jetbrains",
                                "JetBrains Mono",
                                self.settings.terminal_font_family == "JetBrains Mono",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("JetBrains Mono", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "appearance-font-iosevka",
                                "Iosevka",
                                self.settings.terminal_font_family == "Iosevka",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("Iosevka", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "appearance-font-fira",
                                "Fira Code",
                                self.settings.terminal_font_family == "Fira Code",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("Fira Code", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "appearance-font-monospace",
                                "monospace",
                                self.settings.terminal_font_family == "monospace",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("monospace", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Font size",
                        Some(SharedString::from(
                            "Zoom the terminal text without leaving Settings.",
                        )),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "appearance-font-minus",
                                "−",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_font_size(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(48.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text))
                                    .child(font_size_label),
                            )
                            .child(small_button(
                                palette,
                                "appearance-font-plus",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_font_size(1, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "appearance-font-reset",
                                "Reset",
                                cx.listener(|this, _, _, cx| {
                                    this.reset_terminal_font_size(cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Regular weight",
                        Some(SharedString::from("Base terminal font weight.")),
                        div().flex().flex_wrap().gap_1().children(
                            [300_u16, 400, 500, 600, 700, 800]
                                .into_iter()
                                .map(|weight| {
                                    let selected = font_weight == weight;
                                    let id = format!("appearance-font-weight-{weight}");
                                    let label: &'static str = match weight {
                                        300 => "300",
                                        400 => "400",
                                        500 => "500",
                                        600 => "600",
                                        700 => "700",
                                        _ => "800",
                                    };
                                    settings_choice_chip(
                                        palette,
                                        id,
                                        label,
                                        selected,
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_terminal_font_weight(weight, cx);
                                        }),
                                    )
                                }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Bold weight",
                        Some(SharedString::from(
                            "Weight used for bold/bright ANSI text when supported.",
                        )),
                        div().flex().flex_wrap().gap_1().children(
                            [300_u16, 400, 500, 600, 700, 800]
                                .into_iter()
                                .map(|weight| {
                                    let selected = font_weight_bold == weight;
                                    let id = format!("appearance-font-weight-bold-{weight}");
                                    let label: &'static str = match weight {
                                        300 => "300",
                                        400 => "400",
                                        500 => "500",
                                        600 => "600",
                                        700 => "700",
                                        _ => "800",
                                    };
                                    settings_choice_chip(
                                        palette,
                                        id,
                                        label,
                                        selected,
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_terminal_font_weight_bold(weight, cx);
                                        }),
                                    )
                                }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Cursor"),
                Some("Terminal caret style (Tauri Appearance cursor settings)."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Cursor style",
                        Some(SharedString::from(
                            "Block, underline, or vertical bar caret.",
                        )),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "appearance-cursor-block",
                                "Block",
                                self.settings.cursor_style == "block",
                                cx.listener(|this, _, _, cx| {
                                    this.set_cursor_style("block", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "appearance-cursor-underline",
                                "Underline",
                                self.settings.cursor_style == "underline",
                                cx.listener(|this, _, _, cx| {
                                    this.set_cursor_style("underline", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "appearance-cursor-bar",
                                "Bar",
                                self.settings.cursor_style == "bar",
                                cx.listener(|this, _, _, cx| {
                                    this.set_cursor_style("bar", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Cursor blink",
                        Some(SharedString::from("Blink the caret on the active session.")),
                        settings_switch(
                            palette,
                            "appearance-cursor-blink",
                            self.settings.cursor_blink,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_cursor_blink(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("X11 display"),
                Some("Optional DISPLAY override for remote X11 forwarding helpers."),
                settings_form_row(
                    palette,
                    "DISPLAY",
                    Some(SharedString::from(
                        "Empty uses the environment; pick a common override when needed.",
                    )),
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .child(settings_choice_chip(
                            palette,
                            "appearance-x11-auto",
                            "Auto",
                            self.settings.x11_display.trim().is_empty(),
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
                            "appearance-x11-localhost0",
                            "localhost:0",
                            self.settings.x11_display == "localhost:0",
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("localhost:0", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
                            "appearance-x11-localhost1",
                            "localhost:1",
                            self.settings.x11_display == "localhost:1",
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("localhost:1", cx);
                            }),
                        )),
                ),
            ))
    }
}

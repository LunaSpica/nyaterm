use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, SharedString, Window, div, prelude::*, px,
    rgb, rgba, svg,
};
use nyaterm_core::truncate_preview;

use crate::features::{ChromeTooltip, NyaTermApp, appearance_font_stack, gpui_code_font_family};
use crate::theme::{APPEARANCE_THEME_IDS, ThemePalette, appearance_theme_label};

use super::super::{settings_form_row, settings_form_section, settings_switch};

impl NyaTermApp {
    pub(in crate::features) fn appearance_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let font_size_label = self.settings.terminal_font_size.to_string();
        let ui_font_size_label = self.settings.ui_font_size.to_string();

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(settings_form_section(
                palette,
                None,
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.theme"),
                        Some(SharedString::from(self.tr("settings.themeDesc"))),
                        self.appearance_theme_select(false, cx),
                    ))
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.terminalTheme"),
                        Some(SharedString::from(self.tr("settings.terminalThemeDesc"))),
                        self.appearance_theme_select(true, cx),
                    ))
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.minimumContrastRatio"),
                        Some(SharedString::from(
                            self.tr("settings.minimumContrastRatioDesc"),
                        )),
                        self.appearance_contrast_select(cx),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.panelMultiOpen"),
                        Some(SharedString::from(self.tr("settings.panelMultiOpenDesc"))),
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
                Some(self.tr("settings.backgroundImage")),
                Some(self.tr("settings.backgroundImageDesc")),
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
                        .unwrap_or_else(|| self.tr("settings.backgroundImageEmpty").to_string());
                    let has_image = self.settings.background_image_path.is_some();
                    div()
                        .flex()
                        .flex_col()
                        .gap_5()
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_h(px(36.))
                                        .min_w_0()
                                        .flex_1()
                                        .px_3()
                                        .py_2()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.input))
                                        .font_family(crate::features::gpui_code_font_family())
                                        .text_size(px(11.))
                                        .text_color(rgb(if has_image {
                                            palette.text
                                        } else {
                                            palette.text_muted
                                        }))
                                        .child(path_label),
                                )
                                .child(appearance_icon_text_button(
                                    palette,
                                    "appearance-wallpaper-browse",
                                    "icons/conn/folder.svg",
                                    self.tr("settings.selectBackgroundImage"),
                                    cx.listener(|this, _, _, cx| {
                                        this.prompt_background_image(cx);
                                    }),
                                ))
                                .when(has_image, |this| {
                                    this.child(appearance_icon_text_button(
                                        palette,
                                        "appearance-wallpaper-clear",
                                        "icons/fe/delete.svg",
                                        self.tr("settings.removeBackgroundImage"),
                                        cx.listener(|this, _, _, cx| {
                                            this.clear_background_image(cx);
                                        }),
                                    ))
                                }),
                        )
                        .child(appearance_settings_field(
                            palette,
                            self.tr("settings.backgroundImageFit"),
                            Some(SharedString::from(
                                self.tr("settings.backgroundImageFitDesc"),
                            )),
                            self.appearance_background_fit_select(has_image, cx),
                        ))
                        .child(self.appearance_opacity_slider(false, has_image, cx))
                        .child(self.appearance_opacity_slider(true, has_image, cx))
                },
            ))
            .child(self.appearance_font_stack_settings_section(false, cx))
            .child(self.appearance_font_stack_settings_section(true, cx))
            .child(settings_form_section(
                palette,
                None,
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.fontSize"),
                        None,
                        appearance_number_stepper(
                            palette,
                            "appearance-font-minus",
                            "appearance-font-plus",
                            font_size_label,
                            cx.listener(|this, _, _, cx| {
                                this.adjust_terminal_font_size(-1, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_terminal_font_size(1, cx);
                            }),
                        ),
                    ))
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.terminalFontWeight"),
                        Some(SharedString::from(
                            self.tr("settings.terminalFontWeightDesc"),
                        )),
                        self.appearance_font_weight_select(false, cx),
                    ))
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.terminalFontWeightBold"),
                        Some(SharedString::from(
                            self.tr("settings.terminalFontWeightBoldDesc"),
                        )),
                        self.appearance_font_weight_select(true, cx),
                    ))
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.uiFontSize"),
                        None,
                        appearance_number_stepper(
                            palette,
                            "appearance-ui-font-minus",
                            "appearance-ui-font-plus",
                            ui_font_size_label,
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ui_font_size(-1, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ui_font_size(1, cx);
                            }),
                        ),
                    ))
                    .child(appearance_settings_field(
                        palette,
                        self.tr("settings.cursorStyle"),
                        None,
                        self.appearance_cursor_style_select(cx),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.cursorBlink"),
                        None,
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
    }

    fn appearance_font_stack_settings_section(
        &mut self,
        terminal: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let (title, desc, raw, fallback, mut options) = if terminal {
            (
                self.tr("settings.terminalFontFamily"),
                self.tr("settings.terminalFontFamilyDesc"),
                self.settings.terminal_font_family.clone(),
                "JetBrains Mono",
                self.appearance_terminal_font_options.clone(),
            )
        } else {
            (
                self.tr("settings.uiFontFamily"),
                self.tr("settings.uiFontFamilyDesc"),
                self.settings.ui_font_family.clone(),
                "Inter",
                self.appearance_ui_font_options.clone(),
            )
        };
        let fonts = appearance_font_stack(&raw, fallback);
        for family in &fonts {
            if !options
                .iter()
                .any(|option| option.eq_ignore_ascii_case(family))
            {
                options.insert(0, family.clone());
            }
        }
        let menu_open = self.appearance_menu_open.clone();
        let kind = if terminal { "terminal" } else { "ui" };
        let add_id = if terminal {
            "appearance-terminal-font-add"
        } else {
            "appearance-ui-font-add"
        };
        let add_action = appearance_icon_text_button(
            palette,
            add_id,
            "icons/conn/add.svg",
            self.tr("settings.addFallbackFont"),
            cx.listener(move |this, _, _, cx| {
                this.add_appearance_fallback_font(terminal, cx);
            }),
        );
        let built_in_label = self.tr("settings.fontBuiltIn");
        let primary_label = self.tr("settings.fontPrimary");
        let fallback_label = self.tr("settings.fontFallback");
        let remove_label = self.tr("common.remove");

        appearance_form_section_with_action(
            palette,
            title,
            desc,
            add_action,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .children(fonts.into_iter().enumerate().map(|(index, family)| {
                    let menu_id = format!("appearance-{kind}-font-{index}");
                    let open = menu_open.as_deref() == Some(menu_id.as_str());
                    let menu_id_for_toggle = menu_id.clone();
                    let toggle: AppearanceClickHandler =
                        Box::new(cx.listener(move |this, _, _, cx| {
                            if this.appearance_menu_open.as_deref()
                                == Some(menu_id_for_toggle.as_str())
                            {
                                this.appearance_menu_open = None;
                            } else {
                                this.appearance_menu_open = Some(menu_id_for_toggle.clone());
                            }
                            cx.notify();
                        }));
                    let select_options = options
                        .iter()
                        .map(|option| {
                            let selected = option.eq_ignore_ascii_case(&family);
                            let built_in = if terminal {
                                option.eq_ignore_ascii_case("JetBrains Mono")
                            } else {
                                ["JetBrains Mono", "Noto Sans SC Variable", "Inter"]
                                    .iter()
                                    .any(|font| option.eq_ignore_ascii_case(font))
                            };
                            let label = if built_in {
                                format!("{option} ({built_in_label})")
                            } else {
                                option.clone()
                            };
                            let value = option.clone();
                            let handler: AppearanceClickHandler =
                                Box::new(cx.listener(move |this, _, _, cx| {
                                    this.set_appearance_font_stack_entry(
                                        terminal,
                                        index,
                                        value.clone(),
                                        cx,
                                    );
                                }));
                            AppearanceSelectOption {
                                label,
                                font_family: option.clone(),
                                selected,
                                on_click: handler,
                            }
                        })
                        .collect::<Vec<_>>();
                    let delete_id = format!("appearance-{kind}-font-delete-{index}");
                    let delete: AppearanceClickHandler =
                        Box::new(cx.listener(move |this, _, _, cx| {
                            this.remove_appearance_font_stack_entry(terminal, index, cx);
                        }));
                    let row_label = if index == 0 {
                        primary_label.to_string()
                    } else {
                        format!("{fallback_label} {index}")
                    };

                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.input))
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .w(px(96.))
                                        .flex_none()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight(500.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(row_label),
                                )
                                .child(
                                    appearance_select_control(
                                        palette,
                                        menu_id,
                                        family,
                                        open,
                                        toggle,
                                        select_options,
                                    )
                                    .flex_1()
                                    .min_w(px(220.)),
                                )
                                .child(appearance_icon_button(
                                    palette,
                                    delete_id,
                                    "icons/fe/delete.svg",
                                    remove_label,
                                    delete,
                                )),
                        )
                })),
        )
    }

    fn appearance_opacity_slider(
        &mut self,
        content: bool,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let value = if content {
            self.settings.background_content_opacity
        } else {
            self.settings.background_image_opacity
        };
        let label = if content {
            self.tr("settings.backgroundContentOpacity")
        } else {
            self.tr("settings.backgroundImageOpacity")
        };
        let desc = if content {
            self.tr("settings.backgroundContentOpacityDesc")
                .replace("{{value}}", "82%")
        } else {
            self.tr("settings.backgroundImageOpacityDesc").to_string()
        };
        let kind = if content { "content" } else { "image" };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight(500.))
                                    .text_color(rgb(palette.text))
                                    .child(label),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(desc),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .px_2()
                            .py_1()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child(format!("{value}%")),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "appearance-{kind}-opacity-track"
                    )))
                    .h(px(10.))
                    .w_full()
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .overflow_hidden()
                    .flex()
                    .opacity(if enabled { 1.0 } else { 0.5 })
                    .children((0_u8..=100).map(|percent| {
                        div()
                            .id(SharedString::from(format!(
                                "appearance-{kind}-opacity-{percent}"
                            )))
                            .h_full()
                            .flex_1()
                            .bg(if percent < value {
                                rgb(palette.primary)
                            } else {
                                rgb(palette.input)
                            })
                            .when(enabled, |this| {
                                this.cursor_pointer().on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if content {
                                            this.set_background_content_opacity(percent, cx);
                                        } else {
                                            this.set_background_image_opacity(percent, cx);
                                        }
                                    },
                                ))
                            })
                    })),
            )
    }

    fn appearance_theme_select(
        &mut self,
        terminal: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let id = if terminal {
            "appearance-terminal-theme"
        } else {
            "appearance-ui-theme"
        };
        let current = if terminal {
            self.settings
                .terminal_theme
                .as_deref()
                .filter(|theme| !theme.trim().is_empty())
        } else {
            Some(self.settings.theme.as_str())
        };
        let value = current
            .map(appearance_theme_label)
            .unwrap_or_else(|| self.tr("settings.followUiTheme"))
            .to_string();
        let mut options = Vec::new();
        if terminal {
            let handler: AppearanceClickHandler = Box::new(cx.listener(|this, _, _, cx| {
                this.appearance_menu_open = None;
                this.set_terminal_theme(None, cx);
            }));
            options.push(AppearancePlainSelectOption {
                label: self.tr("settings.followUiTheme").to_string(),
                selected: current.is_none(),
                on_click: handler,
            });
        }
        for theme_id in APPEARANCE_THEME_IDS {
            let selected = current == Some(*theme_id)
                || (current == Some("catppuccin") && *theme_id == "catppuccin-mocha");
            let theme = (*theme_id).to_string();
            let handler: AppearanceClickHandler = if terminal {
                Box::new(cx.listener(move |this, _, _, cx| {
                    this.appearance_menu_open = None;
                    this.set_terminal_theme(Some(&theme), cx);
                }))
            } else {
                Box::new(cx.listener(move |this, _, _, cx| {
                    this.appearance_menu_open = None;
                    this.update_appearance_theme(&theme, cx);
                }))
            };
            options.push(AppearancePlainSelectOption {
                label: appearance_theme_label(theme_id).to_string(),
                selected,
                on_click: handler,
            });
        }

        appearance_plain_select_control(
            palette,
            id,
            value,
            self.appearance_menu_open.as_deref() == Some(id),
            true,
            appearance_menu_toggle_handler(id, cx),
            options,
        )
    }

    fn appearance_contrast_select(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let id = "appearance-minimum-contrast";
        let current = self.settings.minimum_contrast_ratio.clone();
        let label_for = |ratio: &str| match ratio {
            "3" => self.tr("settings.minimumContrastRatio_3"),
            "4.5" => self.tr("settings.minimumContrastRatio_4_5"),
            "7" => self.tr("settings.minimumContrastRatio_7"),
            "21" => self.tr("settings.minimumContrastRatio_21"),
            _ => self.tr("settings.minimumContrastRatio_1"),
        };
        let options = ["1", "3", "4.5", "7", "21"]
            .into_iter()
            .map(|ratio| {
                let handler: AppearanceClickHandler =
                    Box::new(cx.listener(move |this, _, _, cx| {
                        this.appearance_menu_open = None;
                        this.set_minimum_contrast_ratio(ratio, cx);
                    }));
                AppearancePlainSelectOption {
                    label: label_for(ratio).to_string(),
                    selected: current == ratio,
                    on_click: handler,
                }
            })
            .collect();
        appearance_plain_select_control(
            palette,
            id,
            label_for(&current).to_string(),
            self.appearance_menu_open.as_deref() == Some(id),
            true,
            appearance_menu_toggle_handler(id, cx),
            options,
        )
    }

    fn appearance_background_fit_select(
        &mut self,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let id = "appearance-background-fit";
        let current = match self.settings.background_image_fit.as_str() {
            "contain" => "contain",
            "stretch" | "fill" => "stretch",
            "tile" => "tile",
            _ => "cover",
        };
        let label_for = |fit: &str| match fit {
            "contain" => self.tr("settings.backgroundImageFit_contain"),
            "stretch" => self.tr("settings.backgroundImageFit_stretch"),
            "tile" => self.tr("settings.backgroundImageFit_tile"),
            _ => self.tr("settings.backgroundImageFit_cover"),
        };
        let options = ["cover", "contain", "stretch", "tile"]
            .into_iter()
            .map(|fit| {
                let handler: AppearanceClickHandler =
                    Box::new(cx.listener(move |this, _, _, cx| {
                        this.appearance_menu_open = None;
                        this.set_background_image_fit(fit, cx);
                    }));
                AppearancePlainSelectOption {
                    label: label_for(fit).to_string(),
                    selected: current == fit,
                    on_click: handler,
                }
            })
            .collect();
        appearance_plain_select_control(
            palette,
            id,
            label_for(current).to_string(),
            self.appearance_menu_open.as_deref() == Some(id),
            enabled,
            appearance_menu_toggle_handler(id, cx),
            options,
        )
    }

    fn appearance_font_weight_select(
        &mut self,
        bold: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let id = if bold {
            "appearance-font-weight-bold"
        } else {
            "appearance-font-weight"
        };
        let current = if bold {
            self.settings.terminal_font_weight_bold
        } else {
            self.settings.terminal_font_weight
        };
        let label_for = |weight| match weight {
            300 => self.tr("settings.fontWeight_300"),
            500 => self.tr("settings.fontWeight_500"),
            600 => self.tr("settings.fontWeight_600"),
            700 => self.tr("settings.fontWeight_700"),
            800 => self.tr("settings.fontWeight_800"),
            _ => self.tr("settings.fontWeight_400"),
        };
        let options = [300_u16, 400, 500, 600, 700, 800]
            .into_iter()
            .map(|weight| {
                let handler: AppearanceClickHandler = if bold {
                    Box::new(cx.listener(move |this, _, _, cx| {
                        this.appearance_menu_open = None;
                        this.set_terminal_font_weight_bold(weight, cx);
                    }))
                } else {
                    Box::new(cx.listener(move |this, _, _, cx| {
                        this.appearance_menu_open = None;
                        this.set_terminal_font_weight(weight, cx);
                    }))
                };
                AppearancePlainSelectOption {
                    label: label_for(weight).to_string(),
                    selected: current == weight,
                    on_click: handler,
                }
            })
            .collect();
        appearance_plain_select_control(
            palette,
            id,
            label_for(current).to_string(),
            self.appearance_menu_open.as_deref() == Some(id),
            true,
            appearance_menu_toggle_handler(id, cx),
            options,
        )
    }

    fn appearance_cursor_style_select(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let id = "appearance-cursor-style";
        let current = self.settings.cursor_style.clone();
        let label_for = |style: &str| match style {
            "underline" => self.tr("settings.cursorUnderline"),
            "bar" => self.tr("settings.cursorBar"),
            _ => self.tr("settings.cursorBlock"),
        };
        let options = ["block", "underline", "bar"]
            .into_iter()
            .map(|style| {
                let handler: AppearanceClickHandler =
                    Box::new(cx.listener(move |this, _, _, cx| {
                        this.appearance_menu_open = None;
                        this.set_cursor_style(style, cx);
                    }));
                AppearancePlainSelectOption {
                    label: label_for(style).to_string(),
                    selected: current == style,
                    on_click: handler,
                }
            })
            .collect();
        appearance_plain_select_control(
            palette,
            id,
            label_for(&current).to_string(),
            self.appearance_menu_open.as_deref() == Some(id),
            true,
            appearance_menu_toggle_handler(id, cx),
            options,
        )
    }
}

type AppearanceClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

fn appearance_menu_toggle_handler(
    id: &'static str,
    cx: &mut Context<NyaTermApp>,
) -> AppearanceClickHandler {
    Box::new(cx.listener(move |this, _, _, cx| {
        if this.appearance_menu_open.as_deref() == Some(id) {
            this.appearance_menu_open = None;
        } else {
            this.appearance_menu_open = Some(id.to_string());
        }
        cx.notify();
    }))
}

struct AppearanceSelectOption {
    label: String,
    font_family: String,
    selected: bool,
    on_click: AppearanceClickHandler,
}

struct AppearancePlainSelectOption {
    label: String,
    selected: bool,
    on_click: AppearanceClickHandler,
}

fn appearance_select_control(
    palette: ThemePalette,
    id: String,
    value: String,
    open: bool,
    on_toggle: AppearanceClickHandler,
    options: Vec<AppearanceSelectOption>,
) -> gpui::Div {
    let hover = palette.hover;
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .child(
            div()
                .id(SharedString::from(format!("{id}-trigger")))
                .h(px(34.))
                .w_full()
                .px_3()
                .rounded_sm()
                .border_1()
                .border_color(if open {
                    rgb(palette.link)
                } else {
                    rgb(palette.border)
                })
                .bg(rgb(palette.bg))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .cursor_pointer()
                .hover(move |this| this.bg(rgb(hover)))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .overflow_hidden()
                        .font_family(value.clone())
                        .text_size(px(12.))
                        .text_color(rgb(palette.text))
                        .child(truncate_preview(&value, 44)),
                )
                .child(
                    svg()
                        .size(px(14.))
                        .flex_none()
                        .path("icons/chevron-down.svg")
                        .text_color(rgb(palette.text_dimmed)),
                )
                .on_click(on_toggle),
        )
        .when(open, |this| {
            this.child(
                div()
                    .id(SharedString::from(format!("{id}-options")))
                    .mt_1()
                    .max_h(px(240.))
                    .overflow_scroll()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface_elevated))
                    .p_1()
                    .children(options.into_iter().enumerate().map(|(index, option)| {
                        div()
                            .id(SharedString::from(format!("{id}-option-{index}")))
                            .h(px(30.))
                            .px_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .bg(if option.selected {
                                rgb(palette.hover)
                            } else {
                                rgb(0x00000000)
                            })
                            .font_family(option.font_family)
                            .text_size(px(11.))
                            .text_color(rgb(palette.text))
                            .cursor_pointer()
                            .hover(move |this| this.bg(rgb(hover)))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .overflow_hidden()
                                    .child(option.label),
                            )
                            .when(option.selected, |this| {
                                this.child(
                                    svg()
                                        .size(px(13.))
                                        .flex_none()
                                        .path("icons/check.svg")
                                        .text_color(rgb(palette.primary)),
                                )
                            })
                            .on_click(option.on_click)
                    })),
            )
        })
}

fn appearance_plain_select_control(
    palette: ThemePalette,
    id: &'static str,
    value: String,
    open: bool,
    enabled: bool,
    on_toggle: AppearanceClickHandler,
    options: Vec<AppearancePlainSelectOption>,
) -> impl IntoElement {
    let hover = palette.hover;
    div()
        .w_full()
        .max_w(px(360.))
        .flex()
        .flex_col()
        .opacity(if enabled { 1.0 } else { 0.5 })
        .child(
            div()
                .id(SharedString::from(format!("{id}-trigger")))
                .h(px(34.))
                .w_full()
                .px_3()
                .rounded_sm()
                .border_1()
                .border_color(if open {
                    rgb(palette.link)
                } else {
                    rgb(palette.border)
                })
                .bg(rgb(palette.bg))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .overflow_hidden()
                        .text_size(px(12.))
                        .text_color(rgb(palette.text))
                        .child(value),
                )
                .child(
                    svg()
                        .size(px(14.))
                        .flex_none()
                        .path("icons/chevron-down.svg")
                        .text_color(rgb(palette.text_dimmed)),
                )
                .when(enabled, |this| {
                    this.cursor_pointer()
                        .hover(move |this| this.bg(rgb(hover)))
                        .on_click(on_toggle)
                }),
        )
        .when(open && enabled, |this| {
            this.child(
                div()
                    .id(SharedString::from(format!("{id}-options")))
                    .mt_1()
                    .max_h(px(240.))
                    .overflow_scroll()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface_elevated))
                    .p_1()
                    .children(options.into_iter().enumerate().map(|(index, option)| {
                        div()
                            .id(SharedString::from(format!("{id}-option-{index}")))
                            .min_h(px(30.))
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .bg(if option.selected {
                                rgb(palette.hover)
                            } else {
                                rgb(0x00000000)
                            })
                            .text_size(px(11.))
                            .text_color(rgb(palette.text))
                            .cursor_pointer()
                            .hover(move |this| this.bg(rgb(hover)))
                            .child(div().min_w_0().flex_1().child(option.label))
                            .when(option.selected, |this| {
                                this.child(
                                    svg()
                                        .size(px(13.))
                                        .flex_none()
                                        .path("icons/check.svg")
                                        .text_color(rgb(palette.primary)),
                                )
                            })
                            .on_click(option.on_click)
                    })),
            )
        })
}

fn appearance_form_section_with_action(
    palette: ThemePalette,
    title: &'static str,
    desc: &'static str,
    action: impl IntoElement,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .rounded_lg()
        .border_1()
        .border_color(rgba((palette.border << 8) | 0xb3))
        .bg(rgba((palette.surface << 8) | 0x99))
        .overflow_hidden()
        .child(
            div()
                .px_4()
                .py_4()
                .border_b_1()
                .border_color(rgba((palette.surface_elevated << 8) | 0x99))
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(palette.text))
                                .child(title),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(desc),
                        ),
                )
                .child(action),
        )
        .child(div().px_4().py_4().child(content))
}

fn appearance_settings_field(
    palette: ThemePalette,
    label: impl Into<SharedString>,
    desc: Option<SharedString>,
    control: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight(500.))
                .text_color(rgb(palette.text))
                .child(label.into()),
        )
        .when_some(desc, |this, desc| {
            this.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(desc),
            )
        })
        .child(div().w_full().max_w(px(576.)).child(control))
}

fn appearance_number_stepper(
    palette: ThemePalette,
    minus_id: &'static str,
    plus_id: &'static str,
    value: String,
    on_minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover = palette.hover;
    div()
        .h(px(34.))
        .w_full()
        .max_w(px(360.))
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .flex()
        .items_center()
        .child(
            div()
                .id(minus_id)
                .w(px(34.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_r_1()
                .border_color(rgb(palette.border))
                .text_size(px(13.))
                .text_color(rgb(palette.text_muted))
                .cursor_pointer()
                .hover(move |this| this.bg(rgb(hover)))
                .child("-")
                .on_click(on_minus),
        )
        .child(
            div()
                .flex_1()
                .text_center()
                .font_family(gpui_code_font_family())
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(palette.text))
                .child(value),
        )
        .child(
            div()
                .id(plus_id)
                .w(px(34.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_l_1()
                .border_color(rgb(palette.border))
                .text_size(px(13.))
                .text_color(rgb(palette.text_muted))
                .cursor_pointer()
                .hover(move |this| this.bg(rgb(hover)))
                .child("+")
                .on_click(on_plus),
        )
}

fn appearance_icon_text_button(
    palette: ThemePalette,
    id: &'static str,
    icon_path: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let destructive = icon_path == "icons/fe/delete.svg";
    let hover = if destructive {
        rgba((palette.danger << 8) | 0x18)
    } else {
        rgb(palette.hover)
    };
    div()
        .id(id)
        .h(px(28.))
        .px_2()
        .rounded_sm()
        .flex()
        .items_center()
        .gap_1()
        .text_size(px(11.))
        .text_color(rgb(if destructive {
            palette.danger
        } else {
            palette.primary
        }))
        .cursor_pointer()
        .hover(move |this| this.bg(hover))
        .child(
            svg()
                .size(px(14.))
                .path(icon_path)
                .text_color(rgb(if destructive {
                    palette.danger
                } else {
                    palette.primary
                })),
        )
        .child(label)
        .on_click(on_click)
}

fn appearance_icon_button(
    palette: ThemePalette,
    id: String,
    icon_path: &'static str,
    tooltip: &'static str,
    on_click: AppearanceClickHandler,
) -> impl IntoElement {
    let hover = rgba((palette.danger << 8) | 0x18);
    div()
        .id(SharedString::from(id))
        .size(px(28.))
        .flex_none()
        .rounded_sm()
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(palette.danger))
        .cursor_pointer()
        .hover(move |this| this.bg(hover))
        .child(
            svg()
                .size(px(15.))
                .path(icon_path)
                .text_color(rgb(palette.danger)),
        )
        .tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(tooltip)).into())
        .on_click(on_click)
}

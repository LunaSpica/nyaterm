use super::*;

impl NyaTermApp {
    pub(in crate::features) fn keyword_highlights_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let is_dark = self.terminal_theme_is_dark();
        let rules = self.keyword_highlights.rules.clone();
        let active = rules.iter().filter(|rule| rule.enabled).count();
        let expanded = self.keyword_highlight_expanded_id.clone();
        let edit_id = self.keyword_highlight_edit_id.clone();
        let edit_field = self.keyword_highlight_edit_field;
        let prompt = match self.keyword_highlight_path_prompt {
            Some(KeywordHighlightPathPromptKind::Import) => "selecting import file",
            None => "legacy JSON import",
        };
        let builtin_ids = nyaterm_core::builtin_keyword_rule_ids();

        settings_form_section(
            palette,
            Some("Keyword highlights"),
            Some("Match terminal output keywords with colored highlights (Tauri TerminalTab)."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    "Enabled",
                    Some(SharedString::from(format!(
                        "{active}/{} custom rules · {prompt}",
                        rules.len()
                    ))),
                    settings_switch(
                        palette,
                        "settings-keyword-highlights-enabled",
                        self.keyword_highlights.enabled,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_keyword_highlights(cx);
                        }),
                    ),
                ))
                .when(self.keyword_highlights.enabled, |this| {
                    this.child(
                        div()
                            .pl_3()
                            .ml_1()
                            .border_l_1()
                            .border_color(rgb(palette.border))
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(settings_form_row(
                                palette,
                                "Across wrapped lines",
                                Some(SharedString::from(
                                    "Continue matches across soft-wrapped terminal lines.",
                                )),
                                settings_switch(
                                    palette,
                                    "settings-keyword-highlights-wrap",
                                    self.keyword_highlights.across_wrapped_lines,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_keyword_highlights_wrapped(cx);
                                    }),
                                ),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text))
                                            .child("Built-in rules"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text_muted))
                                            .child(
                                                "Toggle catalog rules used when no custom rule matches.",
                                            ),
                                    )
                                    .child(
                                        div()
                                            .grid()
                                            .grid_cols(2)
                                            .gap_2()
                                            .children(builtin_ids.iter().map(|id| {
                                                let id = (*id).to_string();
                                                let label =
                                                    nyaterm_core::builtin_keyword_rule_label(&id);
                                                let swatch =
                                                    nyaterm_core::builtin_keyword_rule_swatch(
                                                        &id, is_dark,
                                                    );
                                                let enabled = self
                                                    .keyword_highlights
                                                    .builtin_rules
                                                    .get(&id)
                                                    .copied()
                                                    .unwrap_or(true);
                                                let color =
                                                    parse_keyword_swatch(swatch).unwrap_or(0x79c0ff);
                                                let rid = id.clone();
                                                div()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(rgb(palette.border))
                                                    .bg(rgb(palette.bg))
                                                    .px_3()
                                                    .py_2()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap_2()
                                                            .min_w_0()
                                                            .child(
                                                                div()
                                                                    .size(px(10.))
                                                                    .rounded_full()
                                                                    .bg(rgb(color))
                                                                    .border_1()
                                                                    .border_color(rgb(
                                                                        palette.border,
                                                                    )),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(12.))
                                                                    .text_color(rgb(
                                                                        palette.text_muted,
                                                                    ))
                                                                    .overflow_hidden()
                                                                    .child(label),
                                                            ),
                                                    )
                                                    .child(settings_switch(
                                                        palette,
                                                        format!(
                                                            "settings-keyword-builtin-{id}"
                                                        ),
                                                        enabled,
                                                        cx.listener(move |this, _, _, cx| {
                                                            this.toggle_keyword_highlight_builtin(
                                                                rid.clone(),
                                                                cx,
                                                            );
                                                        }),
                                                    ))
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text))
                                            .child("Custom rules"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(small_button(
                                                palette,
                                                "settings-keyword-highlights-import",
                                                "Import",
                                                cx.listener(|this, _, _, cx| {
                                                    this.prompt_keyword_highlight_import(cx);
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                "settings-keyword-highlights-add",
                                                "Add",
                                                cx.listener(|this, _, window, cx| {
                                                    this.add_keyword_highlight_rule(window, cx);
                                                }),
                                            )),
                                    ),
                            )
                            .when(rules.is_empty(), |this| {
                                this.child(
                                    div()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.input))
                                        .px_4()
                                        .py_6()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .child(
                                                    "No custom rules — Add one or Import legacy JSON.",
                                                ),
                                        ),
                                )
                            })
                            .children(rules.into_iter().map(|rule| {
                                let is_open = expanded.as_deref() == Some(rule.id.as_str());
                                let pattern_count =
                                    rule.patterns.iter().filter(|p| !p.trim().is_empty()).count();
                                let swatch = if is_dark {
                                    rule.color_dark.as_str()
                                } else {
                                    rule.color_light.as_str()
                                };
                                let color = parse_keyword_swatch(swatch).unwrap_or(0x79c0ff);
                                let rule_id = rule.id.clone();
                                let rule_id_toggle = rule.id.clone();
                                let rule_id_delete = rule.id.clone();
                                let name_active = edit_id.as_deref() == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::Name;
                                let patterns_active = edit_id.as_deref()
                                    == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::Patterns;
                                let dark_active = edit_id.as_deref() == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::ColorDark;
                                let light_active = edit_id.as_deref() == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::ColorLight;
                                let name_value = if rule.name.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.name.clone()
                                };
                                let patterns_value = if rule.patterns.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.patterns.join("\n")
                                };
                                let dark_value = if rule.color_dark.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.color_dark.clone()
                                };
                                let light_value = if rule.color_light.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.color_light.clone()
                                };
                                let palette_dark = nyaterm_core::keyword_highlight_color_palette(true);
                                let palette_light =
                                    nyaterm_core::keyword_highlight_color_palette(false);

                                div()
                                    .id(SharedString::from(format!(
                                        "settings-keyword-rule-{}",
                                        rule.id
                                    )))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.input))
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "settings-keyword-rule-header-{}",
                                                rule.id
                                            )))
                                            .px_3()
                                            .py_2()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .cursor_pointer()
                                            .hover(|this| this.bg(rgb(palette.hover)))
                                            .on_click(cx.listener({
                                                let rule_id = rule_id.clone();
                                                move |this, _, _, cx| {
                                                    this.expand_keyword_highlight_rule(
                                                        rule_id.clone(),
                                                        cx,
                                                    );
                                                }
                                            }))
                                            .child(
                                                div()
                                                    .size(px(10.))
                                                    .rounded_full()
                                                    .bg(rgb(color))
                                                    .border_1()
                                                    .border_color(rgb(palette.border))
                                                    .flex_none(),
                                            )
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight(600.))
                                                    .text_color(rgb(palette.text))
                                                    .overflow_hidden()
                                                    .child(if rule.name.trim().is_empty() {
                                                        "Untitled rule".to_string()
                                                    } else {
                                                        rule.name.clone()
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(format!("{pattern_count} patterns")),
                                            )
                                            .child(settings_switch(
                                                palette,
                                                format!("settings-keyword-rule-enabled-{}", rule.id),
                                                rule.enabled,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.toggle_keyword_highlight_rule(
                                                        rule_id_toggle.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                format!("settings-keyword-rule-delete-{}", rule.id),
                                                "Delete",
                                                cx.listener(move |this, _, _, cx| {
                                                    this.remove_keyword_highlight_rule(
                                                        rule_id_delete.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(rgb(palette.text_dimmed))
                                                    .child(if is_open { "▾" } else { "▸" }),
                                            ),
                                    )
                                    .when(is_open, |this| {
                                        this.child(
                                            div()
                                                .border_t_1()
                                                .border_color(rgb(palette.border))
                                                .bg(rgb(palette.bg))
                                                .px_3()
                                                .py_3()
                                                .flex()
                                                .flex_col()
                                                .gap_3()
                                                .track_focus(&self.keyword_highlight_focus)
                                                .on_key_down(cx.listener(
                                                    |this, event: &KeyDownEvent, _, cx| {
                                                        this.handle_keyword_highlight_key_down(
                                                            event, cx,
                                                        );
                                                    },
                                                ))
                                                .child(settings_form_row(
                                                    palette,
                                                    "Name",
                                                    None,
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "settings-keyword-rule-name-{}",
                                                            rule.id
                                                        )))
                                                        .min_w(px(160.))
                                                        .h(px(28.))
                                                        .px_2()
                                                        .rounded_md()
                                                        .border_1()
                                                        .border_color(if name_active {
                                                            rgb(palette.link)
                                                        } else {
                                                            rgb(palette.border)
                                                        })
                                                        .bg(rgb(palette.input))
                                                        .flex()
                                                        .items_center()
                                                        .text_size(px(12.))
                                                        .text_color(rgb(palette.text))
                                                        .cursor_pointer()
                                                        .child(name_value)
                                                        .on_click(cx.listener({
                                                            let rule_id = rule_id.clone();
                                                            move |this, _, window, cx| {
                                                                this.focus_keyword_highlight_field(
                                                                    rule_id.clone(),
                                                                    KeywordHighlightEditorField::Name,
                                                                    window,
                                                                    cx,
                                                                );
                                                            }
                                                        })),
                                                ))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .text_size(px(11.))
                                                                .text_color(rgb(
                                                                    palette.text_muted,
                                                                ))
                                                                .child(
                                                                    "Patterns (one regex per line)",
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .id(SharedString::from(format!(
                                                                    "settings-keyword-rule-patterns-{}",
                                                                    rule.id
                                                                )))
                                                                .min_h(px(72.))
                                                                .px_2()
                                                                .py_2()
                                                                .rounded_md()
                                                                .border_1()
                                                                .border_color(if patterns_active {
                                                                    rgb(palette.link)
                                                                } else {
                                                                    rgb(palette.border)
                                                                })
                                                                .bg(rgb(palette.input))
                                                                .font_family(crate::features::gpui_code_font_family())
                                                                .text_size(px(11.))
                                                                .text_color(rgb(palette.text))
                                                                .line_height(px(16.))
                                                                .cursor_pointer()
                                                                .child(patterns_value)
                                                                .on_click(cx.listener({
                                                                    let rule_id = rule_id.clone();
                                                                    move |this, _, window, cx| {
                                                                        this.focus_keyword_highlight_field(
                                                                            rule_id.clone(),
                                                                            KeywordHighlightEditorField::Patterns,
                                                                            window,
                                                                            cx,
                                                                        );
                                                                    }
                                                                })),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_2()
                                                        .child(settings_form_row(
                                                            palette,
                                                            "Dark color",
                                                            None,
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2()
                                                                .child(
                                                                    div()
                                                                        .size(px(20.))
                                                                        .rounded_md()
                                                                        .bg(rgb(
                                                                            parse_keyword_swatch(
                                                                                &rule.color_dark,
                                                                            )
                                                                            .unwrap_or(0x79c0ff),
                                                                        ))
                                                                        .border_1()
                                                                        .border_color(rgb(
                                                                            palette.border,
                                                                        )),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id(SharedString::from(
                                                                            format!(
                                                                                "settings-keyword-rule-dark-{}",
                                                                                rule.id
                                                                            ),
                                                                        ))
                                                                        .min_w(px(88.))
                                                                        .h(px(28.))
                                                                        .px_2()
                                                                        .rounded_md()
                                                                        .border_1()
                                                                        .border_color(
                                                                            if dark_active {
                                                                                rgb(palette.link)
                                                                            } else {
                                                                                rgb(palette.border)
                                                                            },
                                                                        )
                                                                        .bg(rgb(palette.input))
                                                                        .font_family(
                                                                            crate::features::gpui_code_font_family(),
                                                                        )
                                                                        .text_size(px(11.))
                                                                        .text_color(rgb(
                                                                            palette.text,
                                                                        ))
                                                                        .flex()
                                                                        .items_center()
                                                                        .cursor_pointer()
                                                                        .child(dark_value)
                                                                        .on_click(cx.listener({
                                                                            let rule_id =
                                                                                rule_id.clone();
                                                                            move |this,
                                                                                  _,
                                                                                  window,
                                                                                  cx| {
                                                                                this.focus_keyword_highlight_field(
                                                                                    rule_id.clone(),
                                                                                    KeywordHighlightEditorField::ColorDark,
                                                                                    window,
                                                                                    cx,
                                                                                );
                                                                            }
                                                                        })),
                                                                ),
                                                        ))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .flex_wrap()
                                                                .gap_1()
                                                                .children(palette_dark.iter().map(
                                                                    |swatch| {
                                                                        let color = parse_keyword_swatch(swatch)
                                                                            .unwrap_or(0x79c0ff);
                                                                        let rid = rule_id.clone();
                                                                        let value = (*swatch).to_string();
                                                                        div()
                                                                            .id(SharedString::from(
                                                                                format!(
                                                                                    "settings-keyword-swatch-dark-{}-{swatch}",
                                                                                    rule.id
                                                                                ),
                                                                            ))
                                                                            .size(px(16.))
                                                                            .rounded_sm()
                                                                            .bg(rgb(color))
                                                                            .border_1()
                                                                            .border_color(rgb(
                                                                                palette.border,
                                                                            ))
                                                                            .cursor_pointer()
                                                                            .on_click(cx.listener(
                                                                                move |this, _, _, cx| {
                                                                                    this.set_keyword_highlight_rule_color(
                                                                                        rid.clone(),
                                                                                        true,
                                                                                        &value,
                                                                                        cx,
                                                                                    );
                                                                                },
                                                                            ))
                                                                    },
                                                                )),
                                                        )
                                                        .child(settings_form_row(
                                                            palette,
                                                            "Light color",
                                                            None,
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2()
                                                                .child(
                                                                    div()
                                                                        .size(px(20.))
                                                                        .rounded_md()
                                                                        .bg(rgb(
                                                                            parse_keyword_swatch(
                                                                                &rule.color_light,
                                                                            )
                                                                            .unwrap_or(0x0969da),
                                                                        ))
                                                                        .border_1()
                                                                        .border_color(rgb(
                                                                            palette.border,
                                                                        )),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id(SharedString::from(
                                                                            format!(
                                                                                "settings-keyword-rule-light-{}",
                                                                                rule.id
                                                                            ),
                                                                        ))
                                                                        .min_w(px(88.))
                                                                        .h(px(28.))
                                                                        .px_2()
                                                                        .rounded_md()
                                                                        .border_1()
                                                                        .border_color(
                                                                            if light_active {
                                                                                rgb(palette.link)
                                                                            } else {
                                                                                rgb(palette.border)
                                                                            },
                                                                        )
                                                                        .bg(rgb(palette.input))
                                                                        .font_family(
                                                                            crate::features::gpui_code_font_family(),
                                                                        )
                                                                        .text_size(px(11.))
                                                                        .text_color(rgb(
                                                                            palette.text,
                                                                        ))
                                                                        .flex()
                                                                        .items_center()
                                                                        .cursor_pointer()
                                                                        .child(light_value)
                                                                        .on_click(cx.listener({
                                                                            let rule_id =
                                                                                rule_id.clone();
                                                                            move |this,
                                                                                  _,
                                                                                  window,
                                                                                  cx| {
                                                                                this.focus_keyword_highlight_field(
                                                                                    rule_id.clone(),
                                                                                    KeywordHighlightEditorField::ColorLight,
                                                                                    window,
                                                                                    cx,
                                                                                );
                                                                            }
                                                                        })),
                                                                ),
                                                        ))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .flex_wrap()
                                                                .gap_1()
                                                                .children(
                                                                    palette_light.iter().map(
                                                                        |swatch| {
                                                                            let color =
                                                                                parse_keyword_swatch(
                                                                                    swatch,
                                                                                )
                                                                                .unwrap_or(
                                                                                    0x0969da,
                                                                                );
                                                                            let rid =
                                                                                rule_id.clone();
                                                                            let value =
                                                                                (*swatch)
                                                                                    .to_string();
                                                                            div()
                                                                                .id(SharedString::from(
                                                                                    format!(
                                                                                        "settings-keyword-swatch-light-{}-{swatch}",
                                                                                        rule.id
                                                                                    ),
                                                                                ))
                                                                                .size(px(16.))
                                                                                .rounded_sm()
                                                                                .bg(rgb(color))
                                                                                .border_1()
                                                                                .border_color(rgb(
                                                                                    palette.border,
                                                                                ))
                                                                                .cursor_pointer()
                                                                                .on_click(
                                                                                    cx.listener(
                                                                                        move |this,
                                                                                              _,
                                                                                              _,
                                                                                              cx| {
                                                                                            this.set_keyword_highlight_rule_color(
                                                                                                rid.clone(),
                                                                                                false,
                                                                                                &value,
                                                                                                cx,
                                                                                            );
                                                                                        },
                                                                                    ),
                                                                                )
                                                                        },
                                                                    ),
                                                                ),
                                                        ),
                                                ),
                                        )
                                    })
                            })),
                    )
                }),
        )
    }
}

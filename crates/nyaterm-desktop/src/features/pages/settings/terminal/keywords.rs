use super::*;

use crate::models::KeywordHighlightEditorField;

impl NyaTermApp {
    pub(in crate::features) fn keyword_highlights_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let is_dark = self.terminal_theme_is_dark();
        let rules = self.keyword_highlights.rules.clone();
        let keyword_highlighting_enabled = self.keyword_highlights.enabled;
        let expanded = self.keyword_highlight_expanded_id.clone();
        let edit_id = self.keyword_highlight_edit_id.clone();
        let edit_field = self.keyword_highlight_edit_field;
        let builtin_ids = nyaterm_core::builtin_keyword_rule_ids();
        let pattern_count_template = self.tr("settings.keywordHighlightPatternCount");
        let untitled_rule_label = self.tr("settings.keywordHighlightNewRule");

        settings_form_section(
            palette,
            None,
            None,
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(settings_form_row(
                    palette,
                    self.tr("settings.keywordHighlightingExperimental"),
                    Some(SharedString::from(
                        self.tr("settings.keywordHighlightingExperimentalDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-keyword-highlights-enabled",
                        keyword_highlighting_enabled,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_keyword_highlights(cx);
                        }),
                    ),
                ))
                .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(settings_form_row(
                                palette,
                                self.tr("settings.keywordHighlightWrappedLines"),
                                Some(SharedString::from(
                                    self.tr("settings.keywordHighlightWrappedLinesDesc"),
                                )),
                                settings_switch_with_enabled(
                                    palette,
                                    "settings-keyword-highlights-wrap",
                                    self.keyword_highlights.across_wrapped_lines,
                                    keyword_highlighting_enabled,
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
                                            .child(self.tr(
                                                "settings.keywordHighlightBuiltinRules",
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr(
                                                "settings.keywordHighlightBuiltinNote",
                                            )),
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
                                                    .child(settings_switch_with_enabled(
                                                        palette,
                                                        format!(
                                                            "settings-keyword-builtin-{id}"
                                                        ),
                                                        enabled,
                                                        keyword_highlighting_enabled,
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
                                    .flex_col()
                                    .gap_3()
                                    .opacity(if keyword_highlighting_enabled {
                                        1.0
                                    } else {
                                        0.5
                                    })
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
                                                    .child(self.tr(
                                                        "settings.keywordHighlightRules",
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(keyword_highlight_action_button(
                                                        palette,
                                                        "settings-keyword-highlights-import",
                                                        "icons/fe/upload.svg",
                                                        self.tr(
                                                            "settings.keywordHighlightImport",
                                                        ),
                                                        keyword_highlighting_enabled,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.prompt_keyword_highlight_import(cx);
                                                        }),
                                                    ))
                                                    .child(keyword_highlight_action_button(
                                                        palette,
                                                        "settings-keyword-highlights-add",
                                                        "icons/conn/add.svg",
                                                        self.tr("common.add"),
                                                        keyword_highlighting_enabled,
                                                        cx.listener(|this, _, window, cx| {
                                                            this.add_keyword_highlight_rule(
                                                                window, cx,
                                                            );
                                                        }),
                                                    )),
                                            ),
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
                                                .child(self.tr(
                                                    "settings.keywordHighlightNoRules",
                                                )),
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
                                            .when(keyword_highlighting_enabled, |this| {
                                                this.cursor_pointer()
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
                                            })
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
                                                        untitled_rule_label.to_string()
                                                    } else {
                                                        rule.name.clone()
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(
                                                        pattern_count_template.replace(
                                                            "{{count}}",
                                                            &pattern_count.to_string(),
                                                        ),
                                                    ),
                                            )
                                            .child(keyword_highlight_rule_switch(
                                                palette,
                                                format!("settings-keyword-rule-enabled-{}", rule.id),
                                                rule.enabled,
                                                keyword_highlighting_enabled,
                                                cx.listener(move |this, _, _, cx| {
                                                    cx.stop_propagation();
                                                    this.toggle_keyword_highlight_rule(
                                                        rule_id_toggle.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(keyword_highlight_icon_button(
                                                palette,
                                                format!("settings-keyword-rule-delete-{}", rule.id),
                                                "icons/fe/delete.svg",
                                                self.tr("common.delete"),
                                                keyword_highlighting_enabled,
                                                cx.listener(move |this, _, _, cx| {
                                                    cx.stop_propagation();
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
                                                    .child(
                                                        svg()
                                                            .size(px(14.))
                                                            .flex_none()
                                                            .path(if is_open {
                                                                "icons/chevron-down.svg"
                                                            } else {
                                                                "icons/fe/forward.svg"
                                                            })
                                                            .text_color(rgb(palette.text_dimmed)),
                                                    ),
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
                                                .when(keyword_highlighting_enabled, |this| {
                                                    this.track_focus(
                                                        &self.keyword_highlight_focus,
                                                    )
                                                    .on_key_down(cx.listener(
                                                        |this,
                                                         event: &KeyDownEvent,
                                                         _,
                                                         cx| {
                                                            this.handle_keyword_highlight_key_down(
                                                                event, cx,
                                                            );
                                                        },
                                                    ))
                                                })
                                                .child(settings_form_row(
                                                    palette,
                                                    self.tr(
                                                        "settings.keywordHighlightRuleName",
                                                    ),
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
                                                        .child(name_value)
                                                        .when(
                                                            keyword_highlighting_enabled,
                                                            |this| {
                                                                this.cursor_pointer().on_click(
                                                                    cx.listener({
                                                                        let rule_id =
                                                                            rule_id.clone();
                                                                        move |this,
                                                                              _,
                                                                              window,
                                                                              cx| {
                                                                            this.focus_keyword_highlight_field(
                                                                                rule_id.clone(),
                                                                                KeywordHighlightEditorField::Name,
                                                                                window,
                                                                                cx,
                                                                            );
                                                                        }
                                                                    }),
                                                                )
                                                            },
                                                        ),
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
                                                                .child(self.tr(
                                                                    "settings.keywordHighlightRulePatterns",
                                                                )),
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
                                                                .child(patterns_value)
                                                                .when(
                                                                    keyword_highlighting_enabled,
                                                                    |this| {
                                                                        this.cursor_pointer()
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
                                                                            }))
                                                                    },
                                                                ),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_2()
                                                        .child(settings_form_row(
                                                            palette,
                                                            self.tr(
                                                                "settings.keywordHighlightDarkPalette",
                                                            ),
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
                                                                        .child(dark_value)
                                                                        .when(
                                                                            keyword_highlighting_enabled,
                                                                            |this| {
                                                                                this.cursor_pointer().on_click(cx.listener({
                                                                                    let rule_id = rule_id.clone();
                                                                                    move |this, _, window, cx| {
                                                                                        this.focus_keyword_highlight_field(
                                                                                            rule_id.clone(),
                                                                                            KeywordHighlightEditorField::ColorDark,
                                                                                            window,
                                                                                            cx,
                                                                                        );
                                                                                    }
                                                                                }))
                                                                            },
                                                                        ),
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
                                                                            .when(
                                                                                keyword_highlighting_enabled,
                                                                                |this| {
                                                                                    this.cursor_pointer().on_click(cx.listener(
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
                                                                            )
                                                                    },
                                                                )),
                                                        )
                                                        .child(settings_form_row(
                                                            palette,
                                                            self.tr(
                                                                "settings.keywordHighlightLightPalette",
                                                            ),
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
                                                                        .child(light_value)
                                                                        .when(
                                                                            keyword_highlighting_enabled,
                                                                            |this| {
                                                                                this.cursor_pointer().on_click(cx.listener({
                                                                                    let rule_id = rule_id.clone();
                                                                                    move |this, _, window, cx| {
                                                                                        this.focus_keyword_highlight_field(
                                                                                            rule_id.clone(),
                                                                                            KeywordHighlightEditorField::ColorLight,
                                                                                            window,
                                                                                            cx,
                                                                                        );
                                                                                    }
                                                                                }))
                                                                            },
                                                                        ),
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
                                                                                .when(
                                                                                    keyword_highlighting_enabled,
                                                                                    |this| {
                                                                                        this.cursor_pointer().on_click(
                                                                                            cx.listener(
                                                                                                move |this, _, _, cx| {
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
                                                                                )
                                                                        },
                                                                    ),
                                                                ),
                                                        ),
                                                ),
                                        )
                                    })
                            })),
                ),
        )
    }
}

fn keyword_highlight_action_button(
    palette: ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover_bg = palette.hover;
    let hover_text = palette.text;

    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface_elevated))
        .text_color(rgb(palette.text))
        .text_xs()
        .child(svg().size(px(14.)).flex_none().path(icon_path))
        .child(div().ml_1().child(label))
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(move |this| this.bg(rgb(hover_bg)).text_color(rgb(hover_text)))
                .on_click(on_click)
        })
}

fn keyword_highlight_icon_button(
    palette: ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    tooltip: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover_bg = rgba((palette.danger << 8) | 0x18);

    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_color(rgb(palette.danger))
        .child(svg().size(px(15.)).path(icon_path))
        .tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(tooltip)).into())
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(move |this| this.bg(hover_bg))
                .on_click(on_click)
        })
}

fn keyword_highlight_rule_switch(
    palette: ThemePalette,
    id: impl Into<String>,
    checked: bool,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_bg = palette.primary;
    let off_bg = palette.border;
    let on_hover = palette.primary_hover;
    let off_hover = palette.hover;

    div()
        .id(SharedString::from(id.into()))
        .h(px(22.))
        .w(px(40.))
        .flex()
        .items_center()
        .rounded_full()
        .px(px(2.))
        .bg(if checked { rgb(on_bg) } else { rgb(off_bg) })
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(move |this| {
                    this.bg(if checked {
                        rgb(on_hover)
                    } else {
                        rgb(off_hover)
                    })
                })
                .on_click(on_click)
        })
        .child(
            div()
                .size(px(18.))
                .rounded_full()
                .bg(rgb(0xffffff))
                .when(checked, |this| this.ml_auto()),
        )
}

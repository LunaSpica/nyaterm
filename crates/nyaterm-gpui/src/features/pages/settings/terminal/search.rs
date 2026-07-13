use super::*;

impl NyaTermApp {
    pub(in crate::features) fn terminal_search_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Terminal search"),
                Some("Default flags for in-buffer find."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Search mode",
                        Some(SharedString::from(
                            "Buffer searches the live terminal; History searches session logs.",
                        )),
                        div()
                            .flex()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "settings-search-mode-buffer",
                                "Buffer",
                                self.terminal_search_mode == TerminalSearchMode::Buffer,
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_mode = TerminalSearchMode::Buffer;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "settings-search-mode-history",
                                "History",
                                self.terminal_search_mode == TerminalSearchMode::History,
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_mode = TerminalSearchMode::History;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Case sensitive",
                        None,
                        settings_switch(
                            palette,
                            "settings-search-case",
                            self.terminal_search_case_sensitive,
                            cx.listener(|this, _, _, cx| {
                                this.terminal_search_case_sensitive =
                                    !this.terminal_search_case_sensitive;
                                this.terminal_search_active_index = 0;
                                cx.notify();
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Regular expression",
                        None,
                        settings_switch(
                            palette,
                            "settings-search-regex",
                            self.terminal_search_regex,
                            cx.listener(|this, _, _, cx| {
                                this.terminal_search_regex = !this.terminal_search_regex;
                                this.terminal_search_active_index = 0;
                                cx.notify();
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Whole word",
                        None,
                        settings_switch(
                            palette,
                            "settings-search-word",
                            self.terminal_search_whole_word,
                            cx.listener(|this, _, _, cx| {
                                this.terminal_search_whole_word = !this.terminal_search_whole_word;
                                this.terminal_search_active_index = 0;
                                cx.notify();
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Open search",
                        Some(SharedString::from(
                            "Focus the terminal search bar in the workspace.",
                        )),
                        small_button(
                            palette,
                            "settings-search-open",
                            "Open",
                            cx.listener(|this, _, window, cx| {
                                this.open_terminal_search(window, cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Command search"),
                Some("Shared matcher sources for history and quick commands."),
                settings_form_row(
                    palette,
                    "Catalog",
                    Some(SharedString::from(format!(
                        "{} history · {} quick commands",
                        self.command_history.len(),
                        self.quick_commands.len()
                    ))),
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .child("Native fuzzy"),
                ),
            ))
            .child(self.online_search_engines_settings_section(cx))
            .child(self.keyword_highlights_settings_section(cx))
    }

    pub(in crate::features) fn online_search_engines_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let engines = self.settings.search_custom_engines.clone();
        let menu_count = engines.iter().filter(|engine| engine.show_in_menu).count();
        let edit_index = self.search_engine_edit_index;
        let expanded_index = self.search_engine_expanded_index;
        let edit_field = self.search_engine_edit_field;

        settings_form_section(
            palette,
            Some("Online search engines"),
            Some("Engines shown on the terminal selection context menu (Tauri Search tab)."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    "Catalog",
                    Some(SharedString::from(format!(
                        "{} engines · {} in menu",
                        engines.len(),
                        menu_count
                    ))),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(small_button(
                            palette,
                            "settings-search-engine-add",
                            "Add",
                            cx.listener(|this, _, window, cx| {
                                this.add_search_engine(cx);
                                window.focus(&this.search_engine_focus);
                            }),
                        ))
                        .child(small_button(
                            palette,
                            "settings-search-engine-reset",
                            "Reset",
                            cx.listener(|this, _, _, cx| {
                                this.reset_search_engines(cx);
                            }),
                        )),
                ))
                .when(engines.is_empty(), |this| {
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
                                        "No custom engines — Add one for selection online search.",
                                    ),
                            ),
                    )
                })
                .children(engines.into_iter().enumerate().map(|(index, engine)| {
                    let is_open = expanded_index == Some(index);
                    let name_active =
                        edit_index == Some(index) && edit_field == SearchEngineEditorField::Name;
                    let url_active =
                        edit_index == Some(index) && edit_field == SearchEngineEditorField::Url;
                    let name_value = if engine.name.is_empty() {
                        " ".to_string()
                    } else {
                        engine.name.clone()
                    };
                    let url_value = if engine.url_template.is_empty() {
                        " ".to_string()
                    } else {
                        engine.url_template.clone()
                    };
                    let has_placeholder = engine.url_template.contains("%s");
                    let icon_label = search_engine_icon_label(engine.icon.as_deref());
                    let icon_color = search_engine_icon_color(engine.icon.as_deref());
                    div()
                        .id(SharedString::from(format!(
                            "settings-search-engine-{index}"
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
                                    "settings-search-engine-header-{index}"
                                )))
                                .px_3()
                                .py_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.expand_search_engine(index, cx);
                                }))
                                .child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "settings-search-engine-icon-{index}"
                                        )))
                                        .size(px(22.))
                                        .rounded_md()
                                        .bg(rgb(palette.bg))
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(icon_color))
                                        .child(icon_label)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.cycle_search_engine_icon(index, cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(palette.text))
                                                .overflow_hidden()
                                                .child(if engine.name.trim().is_empty() {
                                                    "Unnamed engine".to_string()
                                                } else {
                                                    engine.name.clone()
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_muted))
                                                .overflow_hidden()
                                                .child(if engine.url_template.trim().is_empty() {
                                                    "No URL template".to_string()
                                                } else {
                                                    truncate_preview(&engine.url_template, 48)
                                                }),
                                        ),
                                )
                                .child(settings_switch(
                                    palette,
                                    format!("settings-search-engine-menu-{index}"),
                                    engine.show_in_menu,
                                    cx.listener(move |this, _, _, cx| {
                                        this.toggle_search_engine_in_menu(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("settings-search-engine-test-{index}"),
                                    "Test",
                                    cx.listener(move |this, _, _, cx| {
                                        this.test_search_engine(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("settings-search-engine-del-{index}"),
                                    "Delete",
                                    cx.listener(move |this, _, _, cx| {
                                        this.remove_search_engine(index, cx);
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
                                    .gap_2()
                                    .child(
                                        transfer_input(
                                            format!("settings-search-engine-name-{index}"),
                                            "Name",
                                            name_value,
                                            name_active,
                                            palette,
                                        )
                                        .track_focus(&self.search_engine_focus)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.focus_search_engine_field(
                                                index,
                                                SearchEngineEditorField::Name,
                                                window,
                                                cx,
                                            );
                                        }))
                                        .on_key_down(
                                            cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                                cx.stop_propagation();
                                                this.handle_search_engine_key_down(event, cx);
                                            }),
                                        ),
                                    )
                                    .child(
                                        transfer_input(
                                            format!("settings-search-engine-url-{index}"),
                                            "URL template (%s = query)",
                                            url_value,
                                            url_active,
                                            palette,
                                        )
                                        .track_focus(&self.search_engine_focus)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.focus_search_engine_field(
                                                index,
                                                SearchEngineEditorField::Url,
                                                window,
                                                cx,
                                            );
                                        }))
                                        .on_key_down(
                                            cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                                cx.stop_propagation();
                                                this.handle_search_engine_key_down(event, cx);
                                            }),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(if has_placeholder {
                                                palette.text_muted
                                            } else {
                                                palette.danger
                                            }))
                                            .child(if has_placeholder {
                                                "Ready — Test opens the URL with query \"nyaterm\"."
                                                    .to_string()
                                            } else {
                                                "URL should include %s for the selected text"
                                                    .to_string()
                                            }),
                                    ),
                            )
                        })
                })),
        )
    }

}

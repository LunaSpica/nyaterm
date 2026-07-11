use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn terminal_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let session_id = self.active_session_id.clone().unwrap_or_default();
        self.terminal_canvas_for(session_id, false, cx)
    }

    pub(in crate::ui::view) fn terminal_canvas_for(
        &self,
        session_id: String,
        show_pane_chrome: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let is_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        let mut output = div().flex().flex_col().gap_1();
        let lines = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.screen.lines())
            .unwrap_or_else(|| self.terminal_screen.lines());
        let search_matches = if is_active
            && self.terminal_search_open
            && self.terminal_search_mode == TerminalSearchMode::Buffer
        {
            self.terminal_buffer_matches().unwrap_or_default()
        } else {
            Vec::new()
        };
        let active_match_line = search_matches
            .get(
                self.terminal_search_active_index
                    .min(search_matches.len().saturating_sub(1)),
            )
            .map(|search_match| search_match.line_index);
        let matched_lines = search_matches
            .iter()
            .map(|search_match| search_match.line_index)
            .collect::<HashSet<_>>();
        for (line_index, line) in lines.into_iter().enumerate() {
            let line = if line.is_empty() {
                " ".to_string()
            } else {
                line
            };
            let line = if self.settings.terminal_show_line_numbers {
                format!("{:>5}  {line}", line_index + 1)
            } else {
                line
            };
            output = output.child(terminal_line_element(
                &line,
                &self.keyword_highlights,
                matched_lines.contains(&line_index),
                active_match_line == Some(line_index),
            ));
        }
        let pane_title = self
            .session_display_name(&session_id)
            .unwrap_or_else(|| short_id(&session_id).to_string());
        let sync_group_label = self.active_sync_group_label(&session_id);
        let output_session_id = session_id.clone();
        let terminal_font_family = self.settings.terminal_font_family.clone();
        let terminal_font_size = self.settings.terminal_font_size as f32;

        div()
            .flex_1()
            .min_h_0()
            .font_family(terminal_font_family)
            .text_size(px(terminal_font_size))
            .text_color(rgb(0xc8d3f5))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .relative()
                    .bg(rgb(0x07090d))
                    .when(show_pane_chrome, |this| {
                        this.border_1()
                            .border_color(if is_active {
                                rgb(0x3b82f6)
                            } else {
                                rgb(0x202633)
                            })
                            .child(
                                div()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .px_2()
                                    .border_b_1()
                                    .border_color(rgb(0x202633))
                                    .bg(if is_active {
                                        rgb(0x101b2d)
                                    } else {
                                        rgb(0x0d1118)
                                    })
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_xs()
                                            .font_weight(FontWeight(800.))
                                            .text_color(if is_active {
                                                rgb(palette.text)
                                            } else {
                                                rgb(palette.text_muted)
                                            })
                                            .child(truncate_preview(&pane_title, 42)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .when_some(sync_group_label.clone(), |this, label| {
                                                this.child(
                                                    div()
                                                        .rounded_sm()
                                                        .px_2()
                                                        .py_1()
                                                        .text_xs()
                                                        .text_color(rgb(0xc4b5fd))
                                                        .bg(rgb(0x2e2148))
                                                        .child(truncate_preview(&label, 18)),
                                                )
                                            })
                                            .child(status_pill(
                                                if is_active { "active" } else { "pane" },
                                                if is_active {
                                                    rgb(0x6ee7b7)
                                                } else {
                                                    rgb(0x93c5fd)
                                                },
                                                rgb(0x17233a),
                                            )),
                                    ),
                            )
                    })
                    .track_focus(&self.terminal_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.mark_user_activity();
                        if this.handle_global_shortcut(event, window, cx) {
                            return;
                        }
                        let keystroke = &event.keystroke;
                        let primary = keystroke.modifiers.control || keystroke.modifiers.platform;
                        if primary
                            && !keystroke.modifiers.alt
                            && !keystroke.modifiers.function
                            && matches!(keystroke.key.as_str(), "v" | "V")
                        {
                            this.paste_from_clipboard(window, cx);
                            return;
                        }
                        if let Some(bytes) = this.terminal_key_bytes_for_event(event) {
                            this.send_terminal_input(bytes, cx);
                        }
                    }))
                    .child(
                        div()
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .border_b_1()
                            .border_color(rgb(0x1d2430))
                            .bg(rgb(0x0d1118))
                            .child(small_button(palette, 
                                "terminal-start-local",
                                "Start Local",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-probe",
                                "Probe",
                                cx.listener(|this, _, _, cx| {
                                    this.send_probe_command(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-paste",
                                "Paste",
                                cx.listener(|this, _, window, cx| {
                                    this.paste_from_clipboard(window, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-actions",
                                "Actions",
                                cx.listener(|this, _, window, cx| {
                                    this.open_terminal_actions(window, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-reconnect",
                                "Reconnect",
                                cx.listener(|this, _, window, cx| {
                                    this.reconnect_active_session(window, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-duplicate-run",
                                "Dup+Run",
                                cx.listener(|this, _, window, cx| {
                                    this.open_startup_command_dialog(window, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_active_session(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-clear",
                                "Clear",
                                cx.listener(|this, _, _, cx| {
                                    this.clear_terminal(cx);
                                }),
                            ))
                            .child(div().flex_1())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.terminal_status.clone()),
                            ),
                    )
                    .child(
                        div()
                            .id(SharedString::from("terminal-output"))
                            .flex_1()
                            .min_h_0()
                            .p(if self.settings.terminal_show_workspace_padding {
                                px(16.)
                            } else {
                                px(8.)
                            })
                            .overflow_hidden()
                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                this.activate_workspace_pane(output_session_id.clone(), cx);
                                if event.is_right_click() {
                                    if this.settings.interaction_right_click_paste {
                                        this.paste_from_clipboard(window, cx);
                                    } else {
                                        this.open_terminal_actions(window, cx);
                                    }
                                    cx.stop_propagation();
                                    return;
                                }
                                window.focus(&this.terminal_focus);
                                this.terminal_status = "terminal focused".to_string();
                                cx.notify();
                            }))
                            .child(output),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(px(8.))
                            .left(px(10.))
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(status_pill(
                                if self.settings.terminal_show_timestamps {
                                    "timestamps"
                                } else {
                                    "plain"
                                },
                                rgb(0x93c5fd),
                                rgb(0x17253b),
                            ))
                            .child(status_pill(
                                if self.settings.terminal_hardware_acceleration {
                                    "gpu"
                                } else {
                                    "cpu"
                                },
                                rgb(0x6ee7b7),
                                rgb(0x12342a),
                            )),
                    )
                    .when(is_active && self.terminal_search_open, |this| {
                        this.child(self.terminal_search_bar(cx))
                    }),
            )
    }

    fn terminal_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let buffer_matches = self.terminal_buffer_matches();
        let history_results = self.terminal_history_search_results();
        let (status, is_error) = match self.terminal_search_mode {
            TerminalSearchMode::Buffer => match &buffer_matches {
                Ok(matches) if self.terminal_search_query.trim().is_empty() => {
                    ("idle".to_string(), false)
                }
                Ok(matches) if matches.is_empty() => ("not found".to_string(), false),
                Ok(matches) => (
                    format!(
                        "{}/{}",
                        self.terminal_search_active_index
                            .min(matches.len().saturating_sub(1))
                            + 1,
                        matches.len()
                    ),
                    false,
                ),
                Err(error) => (truncate_preview(error, 40), true),
            },
            TerminalSearchMode::History => match &history_results {
                Ok(response) if self.terminal_search_query.trim().is_empty() => {
                    ("idle".to_string(), false)
                }
                Ok(response) if response.results.is_empty() => ("not found".to_string(), false),
                Ok(response) => (
                    format!(
                        "{} result(s){}",
                        response.total,
                        if response.truncated { " truncated" } else { "" }
                    ),
                    false,
                ),
                Err(error) => (truncate_preview(error, 40), true),
            },
        };
        let input_display = if self.terminal_search_query.is_empty() {
            "Find".to_string()
        } else {
            self.terminal_search_query.clone()
        };
        let mut history_rows = div().flex().flex_col().gap_1();
        if self.terminal_search_mode == TerminalSearchMode::History
            && !self.terminal_search_query.trim().is_empty()
        {
            match history_results {
                Ok(response) if response.results.is_empty() => {
                    history_rows = history_rows.child(
                        div()
                            .px_2()
                            .py_2()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child("No history matches."),
                    );
                }
                Ok(response) => {
                    for result in response.results.into_iter().take(5) {
                        history_rows = history_rows.child(
                            div()
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(0x202633))
                                .bg(rgb(0x0b111b))
                                .p_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(800.))
                                        .text_color(rgb(palette.text))
                                        .child(format!("line {}", result.line_number)),
                                )
                                .child(
                                    div()
                                        .mt_1()
                                        .font_family("JetBrains Mono")
                                        .text_xs()
                                        .text_color(rgb(0xaeb7c8))
                                        .line_height(px(18.))
                                        .child(truncate_preview(&result.preview, 96)),
                                ),
                        );
                    }
                }
                Err(error) => {
                    history_rows = history_rows.child(
                        div()
                            .px_2()
                            .py_2()
                            .text_xs()
                            .text_color(rgb(0xfca5a5))
                            .child(truncate_preview(&error, 96)),
                    );
                }
            }
        }

        div()
            .id(SharedString::from("terminal-search-bar"))
            .absolute()
            .top(px(44.))
            .right(px(8.))
            .w(px(420.))
            .max_w_full()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x303848))
            .bg(rgb(0x0b0f16))
            .shadow_lg()
            .p_2()
            .track_focus(&self.terminal_search_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.terminal_search_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_terminal_search_key_down(event, window, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(mode_button(
                        "terminal-search-mode-buffer",
                        TerminalSearchMode::Buffer.label(),
                        self.terminal_search_mode == TerminalSearchMode::Buffer, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_mode = TerminalSearchMode::Buffer;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-mode-history",
                        TerminalSearchMode::History.label(),
                        self.terminal_search_mode == TerminalSearchMode::History, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_mode = TerminalSearchMode::History;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_error {
                                rgb(0xfca5a5)
                            } else {
                                rgb(0x8f98aa)
                            })
                            .child(status),
                    )
                    .child(icon_button(
                        "terminal-search-close",
                        "x", self.theme_palette(),cx.listener(|this, _, window, cx| {
                            this.close_terminal_search(window, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id(SharedString::from("terminal-search-input"))
                            .h(px(28.))
                            .min_w_0()
                            .flex_1()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x303848))
                            .bg(rgb(palette.input))
                            .px_2()
                            .flex()
                            .items_center()
                            .text_xs()
                            .text_color(if self.terminal_search_query.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .child(input_display),
                    )
                    .child(mode_button(
                        "terminal-search-case",
                        "Aa",
                        self.terminal_search_case_sensitive, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_case_sensitive =
                                !this.terminal_search_case_sensitive;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-regex",
                        ".*",
                        self.terminal_search_regex, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_regex = !this.terminal_search_regex;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-word",
                        "Word",
                        self.terminal_search_whole_word, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_whole_word = !this.terminal_search_whole_word;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(icon_button(
                        "terminal-search-prev",
                        "^", self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.navigate_terminal_search(-1, cx);
                        }),
                    ))
                    .child(icon_button(
                        "terminal-search-next",
                        "v", self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.navigate_terminal_search(1, cx);
                        }),
                    )),
            )
            .child(history_rows)
    }
}

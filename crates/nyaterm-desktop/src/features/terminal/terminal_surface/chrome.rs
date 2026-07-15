use super::*;
use crate::features::terminal_runtime::terminal_scroll_track_ratio;

impl NyaTermApp {
    pub(in crate::features) fn terminal_scrollbar_element(
        &self,
        session_id: &str,
        is_active: bool,
        scroll_offset: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use gpui::relative;
        let palette = self.theme_palette();
        let max = self
            .terminal_views
            .get(session_id)
            .map(|view| view.scrollback_len_for_ui())
            .unwrap_or_else(|| {
                if session_id.is_empty() {
                    self.terminal_screen.scrollback_len()
                } else {
                    0
                }
            });
        let viewport_rows = self
            .terminal_views
            .get(session_id)
            .map(|view| {
                // viewport height equals live screen rows
                view.viewport_rows_for_ui()
            })
            .unwrap_or_else(|| self.active_terminal_page_rows().max(1));
        let show = max > 0;
        let thumb_ratio = if max == 0 {
            1.0
        } else {
            let viewport = viewport_rows as f32;
            (viewport / (viewport + max as f32)).clamp(0.12, 1.0)
        };
        let travel = (1.0 - thumb_ratio).max(0.0);
        let thumb_top_ratio = if max == 0 {
            0.0
        } else {
            travel * (1.0 - (scroll_offset as f32 / max as f32).clamp(0.0, 1.0))
        };
        let track_id = format!("terminal-scrollbar-track-{session_id}");
        let thumb_id = format!("terminal-scrollbar-thumb-{session_id}");

        div()
            .id(SharedString::from(format!(
                "terminal-scrollbar-{session_id}"
            )))
            .w(px(10.))
            .flex_none()
            .h_full()
            .py(px(2.))
            .pr(px(2.))
            .opacity(if show { 1.0 } else { 0.35 })
            .child(
                div()
                    .id(SharedString::from(track_id))
                    .relative()
                    .size_full()
                    .rounded_full()
                    .bg(rgb(palette.border))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, {
                        let session_id = session_id.to_string();
                        cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                            if !session_id.is_empty() {
                                this.activate_workspace_pane(session_id.clone(), cx);
                            }
                            let drag_session_id =
                                (!session_id.is_empty()).then_some(session_id.clone());
                            this.begin_terminal_scrollbar_drag(drag_session_id.clone(), cx);
                            let Some(bounds) = (if session_id.is_empty() {
                                this.terminal_surface_bounds
                            } else {
                                this.terminal_session_surface_bounds
                                    .get(&session_id)
                                    .copied()
                                    .or(this.terminal_surface_bounds)
                            }) else {
                                return;
                            };
                            let ratio = terminal_scroll_track_ratio(bounds, event.position.y);
                            this.set_terminal_scroll_from_track_ratio_for_session(
                                drag_session_id.as_deref(),
                                ratio,
                                cx,
                            );
                            cx.stop_propagation();
                        })
                    })
                    .when(show, |this| {
                        this.child(
                            div()
                                .id(SharedString::from(thumb_id))
                                .absolute()
                                .left(px(1.))
                                .right(px(1.))
                                .top(relative(thumb_top_ratio))
                                .h(relative(thumb_ratio))
                                .min_h(px(18.))
                                .rounded_full()
                                .bg(rgb(if is_active {
                                    palette.accent
                                } else {
                                    palette.text_muted
                                }))
                                .opacity(0.85),
                        )
                    }),
            )
    }

    pub(in crate::features) fn terminal_search_bar(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let buffer_matches = self.terminal_buffer_matches();
        let history_results = self.terminal_history_search_results();
        let history_pending = self.terminal_history_search_pending_for_current_query();
        let (status, is_error) = match self.terminal_search_mode {
            TerminalSearchMode::Buffer => match &buffer_matches {
                Ok(matches) if self.terminal_search_query.trim().is_empty() => {
                    ("idle".to_string(), false)
                }
                Ok(matches) if matches.is_empty() => ("not found".to_string(), false),
                Ok(matches) => {
                    let count = matches.len();
                    let count_label = if count >= 1000 {
                        "1000+".to_string()
                    } else {
                        count.to_string()
                    };
                    (
                        format!(
                            "{}/{}",
                            self.terminal_search_active_index
                                .min(count.saturating_sub(1))
                                + 1,
                            count_label
                        ),
                        false,
                    )
                }
                Err(error) => (truncate_preview(error, 40), true),
            },
            TerminalSearchMode::History if history_pending => ("searching".to_string(), false),
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
        let mut history_rows = div()
            .id(SharedString::from("terminal-search-history-results"))
            .mt_1()
            .max_h(px(260.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_1();
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
                            .text_color(rgb(palette.text_muted))
                            .child("No history matches."),
                    );
                }
                Ok(response) => {
                    history_rows = history_rows.child(
                        div()
                            .px_1()
                            .pb_1()
                            .text_xs()
                            .text_color(rgb(palette.text_dimmed))
                            .child(format!(
                                "{} match(es) · {} ms{}",
                                response.total,
                                response.elapsed_ms,
                                if response.truncated {
                                    " · truncated"
                                } else {
                                    ""
                                }
                            )),
                    );
                    for result in response.results.into_iter().take(8) {
                        let before = result.before.join("\n");
                        let after = result.after.join("\n");
                        let mut context_parts = Vec::new();
                        if !before.trim().is_empty() {
                            context_parts.push(truncate_preview(&before, 120));
                        }
                        context_parts.push(format!("> {}", truncate_preview(&result.preview, 120)));
                        if !after.trim().is_empty() {
                            context_parts.push(truncate_preview(&after, 120));
                        }
                        let context = context_parts.join("\n");
                        history_rows = history_rows.child(
                            div()
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .p_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!("line {}", result.line_number)),
                                )
                                .child(
                                    div()
                                        .mt_1()
                                        .font_family(crate::features::gpui_code_font_family())
                                        .text_xs()
                                        .text_color(rgb(palette.text))
                                        .line_height(px(16.))
                                        .child(truncate_preview(&result.preview, 96)),
                                )
                                .when(
                                    !result.before.is_empty() || !result.after.is_empty(),
                                    |this| {
                                        this.child(
                                            div()
                                                .mt_1()
                                                .font_family(
                                                    crate::features::gpui_code_font_family(),
                                                )
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .line_height(px(14.))
                                                .child(context),
                                        )
                                    },
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
                            .text_color(rgb(palette.danger))
                            .child(truncate_preview(&error, 96)),
                    );
                }
            }
        }

        div()
            .id(SharedString::from("terminal-search-bar"))
            .absolute()
            .top(px(8.))
            .right(px(8.))
            .w(px(420.))
            .max_w_full()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.terminal_bg))
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
                        self.terminal_search_mode == TerminalSearchMode::Buffer,
                        self.theme_palette(),
                        cx.listener(|this, _, _, cx| {
                            this.terminal_search_mode = TerminalSearchMode::Buffer;
                            this.terminal_search_active_index = 0;
                            this.request_active_terminal_search();
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-mode-history",
                        TerminalSearchMode::History.label(),
                        self.terminal_search_mode == TerminalSearchMode::History,
                        self.theme_palette(),
                        cx.listener(|this, _, _, cx| {
                            this.terminal_search_mode = TerminalSearchMode::History;
                            this.terminal_search_active_index = 0;
                            this.request_active_terminal_search();
                            cx.notify();
                        }),
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_error {
                                rgb(palette.danger)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .child(status),
                    )
                    .child(icon_button(
                        "terminal-search-close",
                        "x",
                        self.theme_palette(),
                        cx.listener(|this, _, window, cx| {
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
                            .border_color(rgb(palette.border))
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
                        self.terminal_search_case_sensitive,
                        self.theme_palette(),
                        cx.listener(|this, _, _, cx| {
                            this.terminal_search_case_sensitive =
                                !this.terminal_search_case_sensitive;
                            this.terminal_search_active_index = 0;
                            this.request_active_terminal_search();
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-regex",
                        ".*",
                        self.terminal_search_regex,
                        self.theme_palette(),
                        cx.listener(|this, _, _, cx| {
                            this.terminal_search_regex = !this.terminal_search_regex;
                            this.terminal_search_active_index = 0;
                            this.request_active_terminal_search();
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-word",
                        "Word",
                        self.terminal_search_whole_word,
                        self.theme_palette(),
                        cx.listener(|this, _, _, cx| {
                            this.terminal_search_whole_word = !this.terminal_search_whole_word;
                            this.terminal_search_active_index = 0;
                            this.request_active_terminal_search();
                            cx.notify();
                        }),
                    ))
                    .when(
                        self.terminal_search_mode == TerminalSearchMode::Buffer,
                        |this| {
                            this.child(icon_button(
                                "terminal-search-prev",
                                "^",
                                self.theme_palette(),
                                cx.listener(|this, _, _, cx| {
                                    this.navigate_terminal_search(-1, cx);
                                }),
                            ))
                            .child(icon_button(
                                "terminal-search-next",
                                "v",
                                self.theme_palette(),
                                cx.listener(|this, _, _, cx| {
                                    this.navigate_terminal_search(1, cx);
                                }),
                            ))
                        },
                    ),
            )
            .child(history_rows)
    }
}
